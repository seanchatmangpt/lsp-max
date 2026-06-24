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
//
// Tests 1–11: ported from crates/lsp-max-compositor/tests/routing_integration.rs
// Tests 12–21: gap-filling coverage for OPEN scenarios identified by static analysis

use lsp_max_compositor_routing::config::CompositorConfig;
use lsp_max_compositor_routing::fanout::{dispatch_strategy, servers_for_uri, DispatchStrategy};
use lsp_max_compositor_routing::registry::{ChildServer, ChildTier, ExtensionRouter};

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

    assert!(ids.contains(&"anti-llm-cheat-lsp"), "anti-llm-cheat-lsp must serve .rs, got: {:?}", ids);
    assert!(ids.contains(&"wasm4pm-lsp"), "wasm4pm-lsp must serve .rs, got: {:?}", ids);
    assert!(!ids.contains(&"ggen-lsp"), "ggen-lsp must NOT serve .rs, got: {:?}", ids);
}

// ── 2. Compound extension routing ────────────────────────────────────────────

#[test]
fn compound_ocel_json_routes_to_primary_server() {
    let router = ExtensionRouter::new();
    router.register(".ocel.json", primary_server("wasm4pm-lsp", &[".ocel.json"]));
    router.register(".json", diag_server("anti-llm-cheat-lsp", &[".json"]));

    let servers = servers_for_uri(&router, "file:///workspace/trace.ocel.json");
    let ids: Vec<&str> = servers.iter().map(|s| s.id.as_str()).collect();

    assert!(ids.contains(&"wasm4pm-lsp"), "wasm4pm-lsp must serve .ocel.json, got: {:?}", ids);
    let wasm4pm = servers.iter().find(|s| s.id == "wasm4pm-lsp").unwrap();
    assert!(
        matches!(wasm4pm.tier, ChildTier::Primary),
        "wasm4pm-lsp must be Primary tier for .ocel.json"
    );
}

// ── 3. Tier ordering ─────────────────────────────────────────────────────────

#[test]
fn tier_ordering_primary_secondary_diag() {
    let router = ExtensionRouter::new();
    router.register(".rs", diag_server("anti-llm-cheat-lsp", &[".rs"]));
    router.register(".rs", secondary_server("wasm4pm-lsp", &[".rs"]));
    router.register(".rs", primary_server("ggen-lsp", &[".rs"]));

    let result = servers_for_uri(&router, "file:///workspace/main.rs");
    assert_eq!(result.len(), 3);
    assert!(matches!(result[0].tier, ChildTier::Primary), "index 0 must be Primary");
    assert!(matches!(result[1].tier, ChildTier::Secondary), "index 1 must be Secondary");
    assert!(matches!(result[2].tier, ChildTier::DiagnosticsOnly), "index 2 must be DiagnosticsOnly");
    assert_eq!(result[0].id, "ggen-lsp");
    assert_eq!(result[1].id, "wasm4pm-lsp");
    assert_eq!(result[2].id, "anti-llm-cheat-lsp");
}

// ── 4. FirstSuccess filter ───────────────────────────────────────────────────

#[test]
fn first_success_filter_excludes_diag_only() {
    let router = ExtensionRouter::new();
    router.register(".rs", primary_server("ggen-lsp", &[".rs"]));
    router.register(".rs", diag_server("anti-llm-cheat-lsp", &[".rs"]));
    router.register(".rs", diag_server("wasm4pm-lsp", &[".rs"]));

    assert_eq!(dispatch_strategy("textDocument/hover"), DispatchStrategy::FirstSuccess);

    let all_servers = servers_for_uri(&router, "file:///workspace/main.rs");
    let primary_candidates: Vec<&str> = all_servers
        .iter()
        .filter(|s| matches!(s.tier, ChildTier::Primary))
        .map(|s| s.id.as_str())
        .collect();

    assert_eq!(primary_candidates, vec!["ggen-lsp"]);
}

// ── 5. No double-registration ────────────────────────────────────────────────

#[test]
fn no_double_registration_across_extension_keys() {
    let router = ExtensionRouter::new();
    router.register(".ocel.json", diag_server("wasm4pm-lsp", &[".ocel.json", ".json"]));
    router.register(".json", diag_server("wasm4pm-lsp", &[".ocel.json", ".json"]));
    router.register(".json", diag_server("anti-llm-cheat-lsp", &[".json"]));

    let servers = servers_for_uri(&router, "file:///workspace/trace.ocel.json");
    let ids: Vec<&str> = servers.iter().map(|s| s.id.as_str()).collect();
    let count = ids.iter().filter(|&&id| id == "wasm4pm-lsp").count();
    assert_eq!(count, 1, "wasm4pm-lsp must appear exactly once; got: {:?}", ids);
}

// ── 6. from_config dotted .rs key ──────────────────────────────────────────

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
    assert!(ids.contains(&"anti-llm-cheat-lsp"), "anti-llm-cheat-lsp must serve .rs; got: {:?}", ids);
    assert!(ids.contains(&"wasm4pm-lsp"), "wasm4pm-lsp must serve .rs; got: {:?}", ids);
    assert!(!ids.contains(&"ggen-lsp"), "ggen-lsp must NOT serve .rs; got: {:?}", ids);
}

// ── 7. from_config compound .ocel.json key ───────────────────────────────────

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
    assert!(ids.contains(&"wasm4pm-lsp"), "wasm4pm-lsp must serve .ocel.json; got: {:?}", ids);
    assert!(!ids.contains(&"ggen-lsp"), "ggen-lsp must NOT serve .ocel.json; got: {:?}", ids);
}

// ── 8. from_config .ttl routes to ggen-lsp as Primary ────────────────────────

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
    assert!(ids.contains(&"ggen-lsp"), "ggen-lsp must serve .ttl; got: {:?}", ids);
    assert!(!ids.contains(&"wasm4pm-lsp"), "wasm4pm-lsp must NOT serve .ttl; got: {:?}", ids);
    assert!(!ids.contains(&"anti-llm-cheat-lsp"), "anti-llm-cheat-lsp must NOT serve .ttl; got: {:?}", ids);
    let ggen = servers.iter().find(|s| s.id == "ggen-lsp").unwrap();
    assert!(matches!(ggen.tier, ChildTier::Primary), "ggen-lsp must be Primary for .ttl");
}

// ── 9. from_config no double-registration across secondary keys ───────────────

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
    let count = ids.iter().filter(|&&id| id == "wasm4pm-lsp").count();
    assert_eq!(count, 1, "wasm4pm-lsp must appear once for .ts; got: {:?}", ids);
}

// ── 10. Dispatch strategy: diagnostics → FanAll ──────────────────────────────

#[test]
fn diagnostics_method_maps_to_fan_all() {
    assert_eq!(dispatch_strategy("textDocument/publishDiagnostics"), DispatchStrategy::FanAll);
    assert_eq!(dispatch_strategy("textDocument/diagnostic"), DispatchStrategy::FanAll);
}

// ── 11. Empty router for unregistered extension ───────────────────────────────

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
    assert!(servers.is_empty(), "no server registered for .rs; must be empty, got: {:?}",
        servers.iter().map(|s| s.id.as_str()).collect::<Vec<_>>());
}

// ── 12. No extension — bare filename ─────────────────────────────────────────

#[test]
fn no_extension_filename_returns_empty() {
    let router = ExtensionRouter::new();
    router.register(".rs", diag_server("anti-llm-cheat-lsp", &[".rs"]));
    let servers = servers_for_uri(&router, "file:///workspace/Makefile");
    assert!(servers.is_empty(), "Makefile has no extension; must return empty, got: {:?}",
        servers.iter().map(|s| s.id.as_str()).collect::<Vec<_>>());
}

// ── 13. Hidden file matches dotted key registration ──────────────────────────

#[test]
fn hidden_file_matches_dotted_key_registration() {
    let router = ExtensionRouter::new();
    router.register(".gitignore", diag_server("lint-lsp", &[".gitignore"]));
    let servers = servers_for_uri(&router, "file:///workspace/.gitignore");
    let ids: Vec<&str> = servers.iter().map(|s| s.id.as_str()).collect();
    assert!(ids.contains(&"lint-lsp"),
        ".gitignore must match server registered under \".gitignore\" key, got: {:?}", ids);
}

// ── 14. Empty registry ───────────────────────────────────────────────────────

#[test]
fn empty_registry_returns_empty_for_any_uri() {
    let router = ExtensionRouter::new();
    let servers = servers_for_uri(&router, "file:///workspace/main.rs");
    assert!(servers.is_empty(), "empty registry must return empty for any URI");
}

// ── 15. Case sensitivity ─────────────────────────────────────────────────────

#[test]
fn extension_matching_is_case_sensitive() {
    let router = ExtensionRouter::new();
    router.register(".rs", diag_server("anti-llm-cheat-lsp", &[".rs"]));
    let servers = servers_for_uri(&router, "file:///workspace/MAIN.RS");
    assert!(servers.is_empty(),
        "uppercase .RS must NOT match lowercase .rs (byte-exact matching); got: {:?}",
        servers.iter().map(|s| s.id.as_str()).collect::<Vec<_>>());
}

// ── 16. Directory URI (trailing slash) ───────────────────────────────────────

#[test]
fn directory_uri_trailing_slash_returns_empty() {
    let router = ExtensionRouter::new();
    router.register(".rs", diag_server("anti-llm-cheat-lsp", &[".rs"]));
    let servers = servers_for_uri(&router, "file:///workspace/");
    assert!(servers.is_empty(),
        "directory URI (trailing slash) has no filename; must return empty, got: {:?}",
        servers.iter().map(|s| s.id.as_str()).collect::<Vec<_>>());
}

// ── 17. ggen-lsp serves .rq extension ────────────────────────────────────────

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
    assert!(ids.contains(&"ggen-lsp"), "ggen-lsp must serve .rq files, got: {:?}", ids);
    let ggen = servers.iter().find(|s| s.id == "ggen-lsp").unwrap();
    assert!(matches!(ggen.tier, ChildTier::Primary),
        "ggen-lsp must be Primary tier for .rq (priority=full), got: {:?}", ggen.tier);
}

// ── 18. semantic priority maps to Primary ────────────────────────────────────
//
// from_config checks `entry.priority == "diagnostics-only"` directly; anything
// else (including "semantic") goes through the else branch → Primary.

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
    assert!(matches!(servers[0].tier, ChildTier::Primary),
        "priority=semantic must produce Primary tier, got: {:?}", servers[0].tier);
}

// ── 19. LSP 3.18 unmatched methods default to PrimaryOnly ────────────────────

#[test]
fn lsp318_unmatched_methods_default_to_primary_only() {
    assert_eq!(dispatch_strategy("textDocument/inlayHint"), DispatchStrategy::PrimaryOnly,
        "inlayHint must fall through to PrimaryOnly");
    assert_eq!(dispatch_strategy("textDocument/typeDefinition"), DispatchStrategy::PrimaryOnly,
        "typeDefinition must fall through to PrimaryOnly");
    assert_eq!(dispatch_strategy("workspace/diagnostic"), DispatchStrategy::PrimaryOnly,
        "workspace/diagnostic must fall through to PrimaryOnly");
    assert_eq!(dispatch_strategy("textDocument/selectionRange"), DispatchStrategy::PrimaryOnly,
        "selectionRange must fall through to PrimaryOnly");
}

// ── 20. Same server, same key, registered twice — deduplicates ───────────────

#[test]
fn same_key_duplicate_registration_deduplicates() {
    let router = ExtensionRouter::new();
    router.register(".rs", diag_server("anti-llm-cheat-lsp", &[".rs"]));
    router.register(".rs", diag_server("anti-llm-cheat-lsp", &[".rs"])); // duplicate
    let servers = servers_for_uri(&router, "file:///workspace/main.rs");
    let count = servers.iter().filter(|s| s.id == "anti-llm-cheat-lsp").count();
    assert_eq!(count, 1, "same server registered twice under same key must appear once; got {} entries", count);
}

// ── 21. URI with query string and fragment — routes by base extension ─────────

#[test]
fn uri_with_query_and_fragment_routes_by_base_extension() {
    let router = ExtensionRouter::new();
    router.register(".rs", diag_server("anti-llm-cheat-lsp", &[".rs"]));

    for (label, uri) in [
        ("query", "file:///workspace/main.rs?version=2"),
        ("fragment", "file:///workspace/main.rs#L42"),
        ("both", "file:///workspace/main.rs?v=1#L10"),
    ] {
        let servers = servers_for_uri(&router, uri);
        let ids: Vec<&str> = servers.iter().map(|s| s.id.as_str()).collect();
        assert!(ids.contains(&"anti-llm-cheat-lsp"),
            "URI with {} must route by base .rs extension, got: {:?}", label, ids);
    }
}
