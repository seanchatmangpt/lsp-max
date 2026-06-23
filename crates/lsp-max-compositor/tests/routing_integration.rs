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
// slot maps to ChildTier::Primary. ggen-lsp and anti-llm-cheat-lsp do not
// register `.ocel.json` as a primary extension and must not appear.

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
