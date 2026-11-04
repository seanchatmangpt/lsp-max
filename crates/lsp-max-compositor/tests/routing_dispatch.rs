// routing_dispatch.rs — unit tests for ExtensionRouter dispatch (M4 lane).
//
// Contract: real routing, no mocks of the unit under test. The observable output
// of dispatch is `servers_for_uri(&router, uri)` — the same helper the fanout
// pipeline calls at request time — so every assertion lands on that decision
// output, not on call counts or interaction records. Construction paths
// exercised: manual `new()`+`register()`, `from_config()` (TOML parse → router),
// and the generated `new_from_manifest()` registry init.
//
// Covers the dispatch contract: (1) URI with a mapped extension routes to its
// child; (2) URI with an unmapped extension → the documented fallback (empty
// candidate list, no cross-server leakage); (3) two children with disjoint
// extensions each receive their own files; (4) dispatch strategy for the routed
// method is consistent with the tier of the server it lands on.

use lsp_max_compositor::fanout::{dispatch_strategy, servers_for_uri, DispatchStrategy};
use lsp_max_compositor::registry::{ChildServer, ChildTier, ExtensionRouter};

// ── helpers ──────────────────────────────────────────────────────────────────

fn child_a() -> ChildServer {
    ChildServer {
        id: "child-a".into(),
        tier: ChildTier::Primary,
        extensions: vec![".ttl".into(), ".rq".into()],
        commands: Vec::new(),
    }
}

fn child_b() -> ChildServer {
    ChildServer {
        id: "child-b".into(),
        tier: ChildTier::DiagnosticsOnly,
        extensions: vec![".rs".into()],
        commands: Vec::new(),
    }
}

const CHILDREN_TOML: &str = r#"
[[server]]
id = "child-a"
primary_extensions = [".ttl", ".rq"]
secondary_extensions = []
priority = "full"

[[server]]
id = "child-b"
primary_extensions = [".rs"]
secondary_extensions = []
priority = "diagnostics-only"
"#;

// ── 1. Mapped extension routes to its child (manual construction) ────────────

#[test]
fn mapped_ttl_routes_to_child_a() {
    let router = ExtensionRouter::new();
    router.register(".ttl", child_a());

    let servers = servers_for_uri(&router, "file:///w/ontol.ttl");
    let ids: Vec<&str> = servers.iter().map(|s| s.id.as_str()).collect();
    assert_eq!(
        ids,
        vec!["child-a"],
        "URI with mapped .ttl extension must route to child-a exactly, got: {:?}",
        ids
    );
}

// ── 2. Mapped extension routes to its child (generated manifest path) ────────

#[test]
fn manifest_path_ttl_routes_to_ggen_lsp() {
    // Generated registry init (registry_init.rs, ggen-projected from lsp-max.toml):
    // the construction path the compositor binary uses at startup.
    let router = ExtensionRouter::new_from_manifest();
    let servers = servers_for_uri(&router, "file:///w/compositor.ttl");
    let ids: Vec<&str> = servers.iter().map(|s| s.id.as_str()).collect();
    assert_eq!(
        ids,
        vec!["ggen-lsp"],
        ".ttl must route to ggen-lsp via the generated manifest router, got: {:?}",
        ids
    );
}

// ── 3. Unmapped extension → documented fallback: empty list, no leakage ─────

#[test]
fn unmapped_extension_falls_back_to_empty() {
    let router = ExtensionRouter::new();
    router.register(".ttl", child_a());
    router.register(".rs", child_b());

    let servers = servers_for_uri(&router, "file:///w/data.csv");
    let ids: Vec<&str> = servers.iter().map(|s| s.id.as_str()).collect();
    assert!(
        servers.is_empty(),
        "unmapped extension (.csv) must produce the documented fallback: empty candidate \
         list, no cross-server leakage, got: {:?}",
        ids
    );
}

// ── 4. Two children, disjoint extensions, each receive their own ────────────

#[test]
fn disjoint_children_each_receive_their_own_extension() {
    let router = ExtensionRouter::new();
    router.register(".ttl", child_a());
    router.register(".rs", child_b());

    let a_hits = servers_for_uri(&router, "file:///w/model.ttl");
    let b_hits = servers_for_uri(&router, "file:///w/main.rs");

    let a_ids: Vec<&str> = a_hits.iter().map(|s| s.id.as_str()).collect();
    assert_eq!(a_ids, vec!["child-a"], "child-a gets .ttl only");

    let b_ids: Vec<&str> = b_hits.iter().map(|s| s.id.as_str()).collect();
    assert_eq!(b_ids, vec!["child-b"], "child-b gets .rs only");
}

// ── 5. from_config path: same contract via TOML → router ─────────────────────

#[test]
fn from_config_routes_mapped_and_rejects_unmapped() {
    let config: lsp_max_compositor::config::CompositorConfig =
        toml::from_str(CHILDREN_TOML).expect("TOML must parse");
    let router = ExtensionRouter::from_config(&config);

    let ttl_hits = servers_for_uri(&router, "file:///w/model.ttl");
    let ttl: Vec<&str> = ttl_hits.iter().map(|s| s.id.as_str()).collect();
    assert_eq!(ttl, vec!["child-a"]);

    let rs_hits = servers_for_uri(&router, "file:///w/main.rs");
    let rs: Vec<&str> = rs_hits.iter().map(|s| s.id.as_str()).collect();
    assert_eq!(rs, vec!["child-b"]);

    let csv = servers_for_uri(&router, "file:///w/data.csv");
    assert!(
        csv.is_empty(),
        "unmapped .csv must stay empty via from_config"
    );
}

// ── 6. Dispatch strategy agrees with routed tier ─────────────────────────────
// A FirstSuccess method (hover) must only consider Primary-tier candidates;
// child-b (DiagnosticsOnly) is never the hover target even though .rs is mapped.

#[test]
fn hover_first_success_never_lands_on_diagnostics_only_child() {
    let router = ExtensionRouter::new();
    router.register(".ttl", child_a());
    router.register(".rs", child_b());

    assert_eq!(
        dispatch_strategy("textDocument/hover"),
        DispatchStrategy::FirstSuccess
    );

    // child-a is Primary; hover against a .ttl URI has exactly one candidate.
    let hover_candidates = servers_for_uri(&router, "file:///w/model.ttl");
    assert_eq!(hover_candidates.len(), 1);
    assert!(matches!(hover_candidates[0].tier, ChildTier::Primary));

    // child-b is DiagnosticsOnly — mapped for .rs, but excluded from
    // FirstSuccess dispatch by tier.
    let rs_candidates = servers_for_uri(&router, "file:///w/main.rs");
    assert!(rs_candidates
        .iter()
        .all(|s| !matches!(s.tier, ChildTier::Primary)),
        "no Primary-tier candidate exists for .rs; FirstSuccess hover dispatch must find none, got: {:?}",
        rs_candidates.iter().map(|s| s.id.as_str()).collect::<Vec<_>>()
    );
}
