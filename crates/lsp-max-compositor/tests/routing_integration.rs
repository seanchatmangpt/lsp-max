// routing_integration.rs — integration tests for multi-LSP extension routing.
//
// These tests exercise the ExtensionRouter + fanout pipeline at the integration
// boundary: the same path that CompositorConfig::from_toml_file → from_config
// walks at startup. The scenarios correspond to the three child servers declared
// in lsp-max.toml:
//
//   wasm4pm-lsp         priority=diagnostics-only  primary=[.ocel.json …]  secondary=[.rs …]
//   anti-llm-cheat-lsp  priority=diagnostics-only  primary=[.rs …]
//   ggen-lsp            priority=full              primary=[.ttl .rq .tera]
//
// None of these tests spawn real processes. They exercise the routing layer in
// isolation — config parse → router populate → URI lookup — which is the
// admission boundary below the live JSON-RPC transcript.

use lsp_max_compositor::config::CompositorConfig;
use lsp_max_compositor::fanout::{dispatch_strategy, servers_for_uri, DispatchStrategy};
use lsp_max_compositor::registry::{ChildServer, ChildTier, ExtensionRouter};

// ── helpers ──────────────────────────────────────────────────────────────────

fn diag_server(id: &str, exts: &[&str]) -> ChildServer {
    ChildServer {
        id: id.into(),
        tier: ChildTier::DiagnosticsOnly,
        extensions: exts.iter().map(|s| s.to_string()).collect(),
    }
}

fn primary_server(id: &str, exts: &[&str]) -> ChildServer {
    ChildServer {
        id: id.into(),
        tier: ChildTier::Primary,
        extensions: exts.iter().map(|s| s.to_string()).collect(),
    }
}

fn secondary_server(id: &str, exts: &[&str]) -> ChildServer {
    ChildServer {
        id: id.into(),
        tier: ChildTier::Secondary,
        extensions: exts.iter().map(|s| s.to_string()).collect(),
    }
}

// ── 1. Dotted extension routing ───────────────────────────────────────────────
//
// `.rs` extension maps to anti-llm-cheat-lsp and wasm4pm-lsp (both DiagnosticsOnly
// per lsp-max.toml); ggen-lsp is NOT returned for .rs because it only registers
// .ttl, .rq, .tera.

#[test]
fn dotted_rs_routes_to_anti_llm_and_wasm4pm_not_ggen() {
    let router = ExtensionRouter::new();
    router.register(".rs", diag_server("anti-llm-cheat-lsp", &[".rs"]));
    router.register(".rs", diag_server("wasm4pm-lsp", &[".rs"]));
    router.register(".ttl", primary_server("ggen-lsp", &[".ttl", ".rq", ".tera"]));
    router.register(".rq", primary_server("ggen-lsp", &[".ttl", ".rq", ".tera"]));
    router.register(".tera", primary_server("ggen-lsp", &[".ttl", ".rq", ".tera"]));

    let servers = servers_for_uri(&router, "file:///workspace/main.rs");
    let ids: Vec<&str> = servers.iter().map(|s| s.id.as_str()).collect();

    assert!(
        ids.contains(&"anti-llm-cheat-lsp"),
        "anti-llm-cheat-lsp must serve .rs files, got: {:?}",
        ids
    );
    assert!(
        ids.contains(&"wasm4pm-lsp"),
        "wasm4pm-lsp must serve .rs files (secondary_extensions slot), got: {:?}",
        ids
    );
    assert!(
        !ids.contains(&"ggen-lsp"),
        "ggen-lsp must NOT serve .rs files, got: {:?}",
        ids
    );
}

// ── 2. Compound extension routing ────────────────────────────────────────────
//
// `.ocel.json` is a primary extension for wasm4pm-lsp. When that server is
// configured with priority="full" (not diagnostics-only), the primary_extensions
// slot maps to ChildTier::Primary. ggen-lsp and anti-llm-cheat-lsp must not appear for .ocel.json.

#[test]
fn compound_ocel_json_routes_to_primary_server() {
    let router = ExtensionRouter::new();
    // Simulate a "full"-priority wasm4pm-lsp for .ocel.json (Primary tier).
    router.register(".ocel.json", primary_server("wasm4pm-lsp", &[".ocel.json"]));
    // anti-llm-cheat-lsp covers .json but NOT .ocel.json specifically.
    router.register(".json", diag_server("anti-llm-cheat-lsp", &[".json"]));

    let servers = servers_for_uri(&router, "file:///workspace/trace.ocel.json");
    let ids: Vec<&str> = servers.iter().map(|s| s.id.as_str()).collect();

    assert!(
        ids.contains(&"wasm4pm-lsp"),
        "wasm4pm-lsp must serve .ocel.json files, got: {:?}",
        ids
    );
    // anti-llm-cheat-lsp registers .json — trace.ocel.json ends with .json so it
    // also matches. That is expected fan-out behavior: both serve.
    // The key invariant: wasm4pm-lsp is present and at Primary tier.
    let wasm4pm = servers.iter().find(|s| s.id == "wasm4pm-lsp").unwrap();
    assert!(
        matches!(wasm4pm.tier, ChildTier::Primary),
        "wasm4pm-lsp must be Primary tier for .ocel.json primary_extensions slot"
    );
}

// ── 3. Tier ordering ─────────────────────────────────────────────────────────
//
// servers_for_uri returns results sorted Primary < Secondary < DiagnosticsOnly
// regardless of registration order.

#[test]
fn tier_ordering_primary_secondary_diag() {
    let router = ExtensionRouter::new();
    // Register deliberately out of order: DiagnosticsOnly first, then Secondary,
    // then Primary.
    router.register(".rs", diag_server("anti-llm-cheat-lsp", &[".rs"]));
    router.register(".rs", secondary_server("wasm4pm-lsp", &[".rs"]));
    router.register(".rs", primary_server("ggen-lsp", &[".rs"]));

    let result = servers_for_uri(&router, "file:///workspace/main.rs");

    assert_eq!(result.len(), 3, "all three servers must be returned");
    assert!(
        matches!(result[0].tier, ChildTier::Primary),
        "index 0 must be Primary, got: {:?}",
        result[0].tier
    );
    assert!(
        matches!(result[1].tier, ChildTier::Secondary),
        "index 1 must be Secondary, got: {:?}",
        result[1].tier
    );
    assert!(
        matches!(result[2].tier, ChildTier::DiagnosticsOnly),
        "index 2 must be DiagnosticsOnly, got: {:?}",
        result[2].tier
    );
    assert_eq!(result[0].id, "ggen-lsp");
    assert_eq!(result[1].id, "wasm4pm-lsp");
    assert_eq!(result[2].id, "anti-llm-cheat-lsp");
}

// ── 4. FirstSuccess filter ───────────────────────────────────────────────────
//
// When dispatch_strategy returns FirstSuccess (hover, completion, definition),
// the caller filters to Primary-only candidates. DiagnosticsOnly servers that
// share an extension must not appear in that filtered set.

#[test]
fn first_success_filter_excludes_diag_only() {
    let router = ExtensionRouter::new();
    router.register(".rs", primary_server("ggen-lsp", &[".rs"]));
    router.register(".rs", diag_server("anti-llm-cheat-lsp", &[".rs"]));
    router.register(".rs", diag_server("wasm4pm-lsp", &[".rs"]));

    assert_eq!(
        dispatch_strategy("textDocument/hover"),
        DispatchStrategy::FirstSuccess,
        "hover must map to FirstSuccess"
    );

    let all_servers = servers_for_uri(&router, "file:///workspace/main.rs");
    let primary_candidates: Vec<&str> = all_servers
        .iter()
        .filter(|s| matches!(s.tier, ChildTier::Primary))
        .map(|s| s.id.as_str())
        .collect();

    assert_eq!(
        primary_candidates,
        vec!["ggen-lsp"],
        "FirstSuccess candidates must be Primary-only; DiagnosticsOnly servers excluded"
    );
}

// ── 5. No double-registration ────────────────────────────────────────────────
//
// A server registered under multiple matching extension keys must appear exactly
// once in the servers_for_uri result (deduplicated by id).

#[test]
fn no_double_registration_across_extension_keys() {
    let router = ExtensionRouter::new();
    // wasm4pm-lsp is registered under both .ocel.json and .json.
    router.register(
        ".ocel.json",
        diag_server("wasm4pm-lsp", &[".ocel.json", ".json"]),
    );
    router.register(
        ".json",
        diag_server("wasm4pm-lsp", &[".ocel.json", ".json"]),
    );
    // anti-llm-cheat-lsp is registered under .json only.
    router.register(".json", diag_server("anti-llm-cheat-lsp", &[".json"]));

    let servers = servers_for_uri(&router, "file:///workspace/trace.ocel.json");
    let ids: Vec<&str> = servers.iter().map(|s| s.id.as_str()).collect();

    let wasm4pm_count = ids.iter().filter(|&&id| id == "wasm4pm-lsp").count();
    assert_eq!(
        wasm4pm_count, 1,
        "wasm4pm-lsp must appear exactly once even when registered under multiple matching keys; got ids: {:?}",
        ids
    );
}

// ── 6. from_config routing — dotted .rs key ──────────────────────────────────
//
// Parses a TOML string matching the lsp-max.toml format and verifies that the
// resulting ExtensionRouter routes .rs to the two diagnostics-only servers and
// excludes ggen-lsp (which only covers .ttl/.rq/.tera).

#[test]
fn from_config_dotted_rs_routes_to_correct_servers() {
    let toml = r#"
[[server]]
id = "wasm4pm-lsp"
primary_extensions = [".ocel.json", ".receipt.json", ".ocel.jsonl"]
secondary_extensions = [".rs", ".ts", ".json"]
priority = "diagnostics-only"

[[server]]
id = "anti-llm-cheat-lsp"
primary_extensions = [".rs", ".ts", ".tsx", ".json", ".jsonl", ".md"]
secondary_extensions = []
priority = "diagnostics-only"

[[server]]
id = "ggen-lsp"
primary_extensions = [".ttl", ".rq", ".tera"]
secondary_extensions = []
priority = "full"
"#;

    let config: CompositorConfig = toml::from_str(toml).expect("TOML must parse");
    let router = ExtensionRouter::from_config(&config);

    let servers = servers_for_uri(&router, "file:///workspace/main.rs");
    let ids: Vec<&str> = servers.iter().map(|s| s.id.as_str()).collect();

    assert!(
        ids.contains(&"anti-llm-cheat-lsp"),
        "from_config: anti-llm-cheat-lsp must serve .rs (primary_extensions slot), got: {:?}",
        ids
    );
    assert!(
        ids.contains(&"wasm4pm-lsp"),
        "from_config: wasm4pm-lsp must serve .rs (secondary_extensions slot), got: {:?}",
        ids
    );
    assert!(
        !ids.contains(&"ggen-lsp"),
        "from_config: ggen-lsp must NOT serve .rs, got: {:?}",
        ids
    );
}

// ── 7. from_config routing — compound .ocel.json key ─────────────────────────
//
// Parses the same TOML and verifies that .ocel.json routes to wasm4pm-lsp
// (registered there as primary_extensions). Because priority=diagnostics-only,
// wasm4pm-lsp is DiagnosticsOnly tier even for its primary_extensions slot.
// ggen-lsp must not appear.

#[test]
fn from_config_compound_ocel_json_routes_to_wasm4pm_not_ggen() {
    let toml = r#"
[[server]]
id = "wasm4pm-lsp"
primary_extensions = [".ocel.json", ".receipt.json", ".ocel.jsonl"]
secondary_extensions = [".rs", ".ts", ".json"]
priority = "diagnostics-only"

[[server]]
id = "anti-llm-cheat-lsp"
primary_extensions = [".rs", ".ts", ".tsx", ".json", ".jsonl", ".md"]
secondary_extensions = []
priority = "diagnostics-only"

[[server]]
id = "ggen-lsp"
primary_extensions = [".ttl", ".rq", ".tera"]
secondary_extensions = []
priority = "full"
"#;

    let config: CompositorConfig = toml::from_str(toml).expect("TOML must parse");
    let router = ExtensionRouter::from_config(&config);

    let servers = servers_for_uri(&router, "file:///workspace/trace.ocel.json");
    let ids: Vec<&str> = servers.iter().map(|s| s.id.as_str()).collect();

    assert!(
        ids.contains(&"wasm4pm-lsp"),
        "from_config: wasm4pm-lsp must serve .ocel.json, got: {:?}",
        ids
    );
    // ggen-lsp must NOT appear — it only covers .ttl/.rq/.tera.
    assert!(
        !ids.contains(&"ggen-lsp"),
        "from_config: ggen-lsp must NOT serve .ocel.json, got: {:?}",
        ids
    );
}

// ── 8. from_config routing — .ttl routes to ggen-lsp as Primary ──────────────
//
// ggen-lsp has priority="full", so its primary_extensions (.ttl/.rq/.tera) map
// to ChildTier::Primary. wasm4pm-lsp and anti-llm-cheat-lsp must not appear for .ttl.

#[test]
fn from_config_ttl_routes_to_ggen_as_primary() {
    let toml = r#"
[[server]]
id = "wasm4pm-lsp"
primary_extensions = [".ocel.json", ".receipt.json", ".ocel.jsonl"]
secondary_extensions = [".rs", ".ts", ".json"]
priority = "diagnostics-only"

[[server]]
id = "anti-llm-cheat-lsp"
primary_extensions = [".rs", ".ts", ".tsx", ".json", ".jsonl", ".md"]
secondary_extensions = []
priority = "diagnostics-only"

[[server]]
id = "ggen-lsp"
primary_extensions = [".ttl", ".rq", ".tera"]
secondary_extensions = []
priority = "full"
"#;

    let config: CompositorConfig = toml::from_str(toml).expect("TOML must parse");
    let router = ExtensionRouter::from_config(&config);

    let servers = servers_for_uri(&router, "file:///workspace/compositor.ttl");
    let ids: Vec<&str> = servers.iter().map(|s| s.id.as_str()).collect();

    assert!(
        ids.contains(&"ggen-lsp"),
        "from_config: ggen-lsp must serve .ttl files, got: {:?}",
        ids
    );
    assert!(
        !ids.contains(&"wasm4pm-lsp"),
        "from_config: wasm4pm-lsp must NOT serve .ttl files, got: {:?}",
        ids
    );
    assert!(
        !ids.contains(&"anti-llm-cheat-lsp"),
        "from_config: anti-llm-cheat-lsp must NOT serve .ttl files, got: {:?}",
        ids
    );

    let ggen = servers.iter().find(|s| s.id == "ggen-lsp").unwrap();
    assert!(
        matches!(ggen.tier, ChildTier::Primary),
        "ggen-lsp must be Primary tier for .ttl (priority=full primary_extensions), got: {:?}",
        ggen.tier
    );
}

// ── 9. from_config no double-registration across secondary keys ───────────────
//
// wasm4pm-lsp is registered under .rs, .ts, and .json as secondary_extensions.
// For a .ts file, it must appear exactly once.

#[test]
fn from_config_no_double_registration_for_secondary_extensions() {
    let toml = r#"
[[server]]
id = "wasm4pm-lsp"
primary_extensions = [".ocel.json"]
secondary_extensions = [".rs", ".ts", ".json"]
priority = "diagnostics-only"

[[server]]
id = "anti-llm-cheat-lsp"
primary_extensions = [".rs", ".ts", ".tsx", ".json", ".jsonl", ".md"]
secondary_extensions = []
priority = "diagnostics-only"
"#;

    let config: CompositorConfig = toml::from_str(toml).expect("TOML must parse");
    let router = ExtensionRouter::from_config(&config);

    let servers = servers_for_uri(&router, "file:///workspace/lib.ts");
    let ids: Vec<&str> = servers.iter().map(|s| s.id.as_str()).collect();

    let wasm4pm_count = ids.iter().filter(|&&id| id == "wasm4pm-lsp").count();
    assert_eq!(
        wasm4pm_count, 1,
        "wasm4pm-lsp must appear exactly once for .ts even with multiple secondary extensions; got: {:?}",
        ids
    );
}

// ── 10. Dispatch strategy covers diagnostics method ───────────────────────────
//
// textDocument/publishDiagnostics must map to FanAll — all registered servers
// for a given extension contribute their diagnostics.

#[test]
fn diagnostics_method_maps_to_fan_all() {
    assert_eq!(
        dispatch_strategy("textDocument/publishDiagnostics"),
        DispatchStrategy::FanAll,
        "textDocument/publishDiagnostics must use FanAll so all DiagnosticsOnly servers contribute"
    );
    assert_eq!(
        dispatch_strategy("textDocument/diagnostic"),
        DispatchStrategy::FanAll,
        "textDocument/diagnostic must use FanAll"
    );
}

// ── 11. Empty router for unregistered extension ───────────────────────────────
//
// An extension that no server registers must return an empty list — no panic,
// no fallback to unrelated servers.

#[test]
fn unregistered_extension_returns_empty() {
    let toml = r#"
[[server]]
id = "ggen-lsp"
primary_extensions = [".ttl", ".rq", ".tera"]
secondary_extensions = []
priority = "full"
"#;

    let config: CompositorConfig = toml::from_str(toml).expect("TOML must parse");
    let router = ExtensionRouter::from_config(&config);

    let servers = servers_for_uri(&router, "file:///workspace/main.rs");
    assert!(
        servers.is_empty(),
        "no server registered for .rs; result must be empty, got: {:?}",
        servers.iter().map(|s| s.id.as_str()).collect::<Vec<_>>()
    );
}

// ── 12. No extension — bare filename ────────────────────────────────────────
// A filename with no dot (e.g. "Makefile", "README") should match no server
// and return an empty list — not panic.
#[test]
fn no_extension_filename_returns_empty() {
    let router = ExtensionRouter::new();
    router.register(".rs", diag_server("anti-llm-cheat-lsp", &[".rs"]));

    let servers = servers_for_uri(&router, "file:///workspace/Makefile");
    assert!(
        servers.is_empty(),
        "Makefile has no extension; must return empty, got: {:?}",
        servers.iter().map(|s| s.id.as_str()).collect::<Vec<_>>()
    );
}

// ── 13. Hidden file (leading-dot filename) ────────────────────────────────
// ".gitignore" should NOT match a server registered under ".gitignore" via
// the bare-key path if its dot-boundary check is correct, and SHOULD match
// if registered under the dotted key ".gitignore".
#[test]
fn hidden_file_matches_dotted_key_registration() {
    let router = ExtensionRouter::new();
    router.register(".gitignore", diag_server("lint-lsp", &[".gitignore"]));

    let servers = servers_for_uri(&router, "file:///workspace/.gitignore");
    let ids: Vec<&str> = servers.iter().map(|s| s.id.as_str()).collect();
    assert!(
        ids.contains(&"lint-lsp"),
        ".gitignore must match a server registered under \".gitignore\" key, got: {:?}",
        ids
    );
}

// ── 14. Empty registry ───────────────────────────────────────────────────
// A router with zero registrations must return empty for any URI — no panic.
#[test]
fn empty_registry_returns_empty_for_any_uri() {
    let router = ExtensionRouter::new();
    let servers = servers_for_uri(&router, "file:///workspace/main.rs");
    assert!(
        servers.is_empty(),
        "empty registry must return empty for any URI"
    );
}

// ── 15. Case sensitivity ─────────────────────────────────────────────────
// Extension matching is byte-exact. ".RS" must NOT match a server registered
// under ".rs". This is a silent miss on case-insensitive filesystems — the
// test documents the current behaviour so regressions are caught.
#[test]
fn extension_matching_is_case_sensitive() {
    let router = ExtensionRouter::new();
    router.register(".rs", diag_server("anti-llm-cheat-lsp", &[".rs"]));

    let servers = servers_for_uri(&router, "file:///workspace/MAIN.RS");
    assert!(
        servers.is_empty(),
        "uppercase .RS must NOT match lowercase .rs registration (case-sensitive); got: {:?}",
        servers.iter().map(|s| s.id.as_str()).collect::<Vec<_>>()
    );
}

// ── 16. Directory URI (trailing slash) ───────────────────────────────────
// A directory URI such as "file:///workspace/" has no filename segment.
// filename_from_uri returns "" and servers_for_filename("") must return empty.
#[test]
fn directory_uri_trailing_slash_returns_empty() {
    let router = ExtensionRouter::new();
    router.register(".rs", diag_server("anti-llm-cheat-lsp", &[".rs"]));

    let servers = servers_for_uri(&router, "file:///workspace/");
    assert!(
        servers.is_empty(),
        "directory URI (trailing slash) must return empty — no filename to match, got: {:?}",
        servers.iter().map(|s| s.id.as_str()).collect::<Vec<_>>()
    );
}

// ── 17. Ggen rq extension ──────────────────────────────────────────────
// ggen-lsp registers .ttl, .rq, .tera. A .rq file must route to ggen-lsp
// specifically (not just .ttl, which is already tested).
#[test]
fn ggen_lsp_serves_rq_extension() {
    let toml = r#"
[[server]]
id = "ggen-lsp"
primary_extensions = [".ttl", ".rq", ".tera"]
secondary_extensions = []
priority = "full"
"#;
    let config: CompositorConfig = toml::from_str(toml).expect("TOML must parse");
    let router = ExtensionRouter::from_config(&config);

    let servers = servers_for_uri(&router, "file:///workspace/query.rq");
    let ids: Vec<&str> = servers.iter().map(|s| s.id.as_str()).collect();
    assert!(
        ids.contains(&"ggen-lsp"),
        "ggen-lsp must serve .rq files, got: {:?}",
        ids
    );
    let ggen = servers.iter().find(|s| s.id == "ggen-lsp").unwrap();
    assert!(
        matches!(ggen.tier, ChildTier::Primary),
        "ggen-lsp must be Primary tier for .rq (priority=full), got: {:?}",
        ggen.tier
    );
}

// ── 18. semantic priority maps to Primary ────────────────────────────────
// `priority = "semantic"` must produce ChildTier::Primary for primary_extensions,
// same as `priority = "full"`. from_config uses from_priority() for this.
// NOTE: from_config currently checks `entry.priority == "diagnostics-only"` directly
// rather than delegating to ChildTier::from_priority(). This test documents the
// gap: "semantic" goes through the else branch → treated as full/Primary.
#[test]
fn semantic_priority_maps_to_primary_tier() {
    let toml = r#"
[[server]]
id = "semantic-lsp"
primary_extensions = [".rs"]
secondary_extensions = []
priority = "semantic"
"#;
    let config: CompositorConfig = toml::from_str(toml).expect("TOML must parse");
    let router = ExtensionRouter::from_config(&config);

    let servers = servers_for_uri(&router, "file:///workspace/main.rs");
    assert_eq!(servers.len(), 1, "exactly one server for .rs");
    assert!(
        matches!(servers[0].tier, ChildTier::Primary),
        "priority=semantic must produce Primary tier, got: {:?}",
        servers[0].tier
    );
}

// ── 19. LSP 3.18 methods fall through to PrimaryOnly ─────────────────────
// Methods not explicitly matched in dispatch_strategy map to PrimaryOnly.
// This covers LSP 3.18 additions not yet enumerated (inlayHint, typeDefinition, etc.).
#[test]
fn lsp318_unmatched_methods_default_to_primary_only() {
    assert_eq!(
        dispatch_strategy("textDocument/inlayHint"),
        DispatchStrategy::PrimaryOnly,
        "inlayHint must fall through to PrimaryOnly"
    );
    assert_eq!(
        dispatch_strategy("textDocument/typeDefinition"),
        DispatchStrategy::PrimaryOnly,
        "typeDefinition must fall through to PrimaryOnly"
    );
    assert_eq!(
        dispatch_strategy("workspace/diagnostic"),
        DispatchStrategy::PrimaryOnly,
        "workspace/diagnostic must fall through to PrimaryOnly"
    );
    assert_eq!(
        dispatch_strategy("textDocument/selectionRange"),
        DispatchStrategy::PrimaryOnly,
        "selectionRange must fall through to PrimaryOnly"
    );
}

// ── 20. Same server ID, same key registered twice ───────────────────────
// Calling register() twice for the same extension key and same server id
// must produce exactly one entry in servers_for_uri (deduplication holds).
#[test]
fn same_key_duplicate_registration_deduplicates() {
    let router = ExtensionRouter::new();
    router.register(".rs", diag_server("anti-llm-cheat-lsp", &[".rs"]));
    router.register(".rs", diag_server("anti-llm-cheat-lsp", &[".rs"])); // duplicate

    let servers = servers_for_uri(&router, "file:///workspace/main.rs");
    let count = servers.iter().filter(|s| s.id == "anti-llm-cheat-lsp").count();
    assert_eq!(
        count, 1,
        "same server registered under same key twice must appear exactly once; got {} entries",
        count
    );
}

// ── 21. URI with query string and fragment ────────────────────────────────
// Query strings and fragments must be stripped before extension matching.
#[test]
fn uri_with_query_and_fragment_routes_by_base_extension() {
    let router = ExtensionRouter::new();
    router.register(".rs", diag_server("anti-llm-cheat-lsp", &[".rs"]));

    let servers_q = servers_for_uri(&router, "file:///workspace/main.rs?version=2");
    let servers_f = servers_for_uri(&router, "file:///workspace/main.rs#L42");
    let servers_qf = servers_for_uri(&router, "file:///workspace/main.rs?v=1#L10");

    for (label, servers) in [("query", &servers_q), ("fragment", &servers_f), ("both", &servers_qf)] {
        let ids: Vec<&str> = servers.iter().map(|s| s.id.as_str()).collect();
        assert!(
            ids.contains(&"anti-llm-cheat-lsp"),
            "URI with {} must still route by base .rs extension, got: {:?}",
            label, ids
        );
    }
}
