// Priority semantics tests — "diagnostics-only" vs "full" child declarations.
//
// Tests the REAL filters, not stubs:
// - registry::ChildTier::from_priority / ExtensionRouter::from_config (tier mapping)
// - fanout::dispatch_strategy / servers_for_uri (per-method request surface)
// - routing::should_forward_notification (notification gating per tier)
// - capability_merge::merge_capabilities (DiagnosticsOnly contributes no capabilities)
//
// Chicago discipline: real collaborators, assert on final observable state.

#![cfg(feature = "full")]

use lsp_max_compositor::capability_merge::merge_capabilities;
use lsp_max_compositor::fanout::{dispatch_strategy, servers_for_uri, DispatchStrategy};
use lsp_max_compositor::registry::{ChildServer, ChildTier, ExtensionRouter};
use lsp_max_compositor::routing::should_forward_notification;

/// Build a config from raw TOML the way `lsp-max.toml` declares it — this is
/// the exact path H3's manifest takes, so misdeclaration behavior is tested
/// end-to-end from the declaration string.
fn router_from_toml(toml_src: &str) -> ExtensionRouter {
    let config: lsp_max_compositor::config::CompositorConfig =
        toml::from_str(toml_src).expect("config parse");
    ExtensionRouter::from_config(&config)
}

fn ids(servers: &[ChildServer]) -> Vec<String> {
    servers.iter().map(|s| s.id.clone()).collect()
}

// ---------------------------------------------------------------------------
// (1) diagnostics-only child: which requests does it receive?
// ---------------------------------------------------------------------------

#[test]
fn diagnostics_only_declaration_maps_every_extension_to_diagnostics_only_tier() {
    let router = router_from_toml(
        r#"
[[server]]
id = "config-lsp"
priority = "diagnostics-only"
primary_extensions = [".json"]
secondary_extensions = [".jsonc"]
"#,
    );
    // Primary-declared extension still lands in DiagnosticsOnly tier.
    let primary = router.servers_for_filename("settings.json");
    assert_eq!(ids(&primary), vec!["config-lsp"]);
    assert!(matches!(primary[0].tier, ChildTier::DiagnosticsOnly));

    // Secondary-declared extension too — from_config forces BOTH lists into
    // DiagnosticsOnly when priority == "diagnostics-only".
    let secondary = router.servers_for_filename("settings.jsonc");
    assert_eq!(ids(&secondary), vec!["config-lsp"]);
    assert!(matches!(secondary[0].tier, ChildTier::DiagnosticsOnly));
}

#[test]
fn diagnostics_only_child_receives_diagnostics_but_not_read_surface() {
    // Diagnostics-only child on .json, alone on that extension.
    let router = router_from_toml(
        r#"
[[server]]
id = "config-lsp"
priority = "diagnostics-only"
primary_extensions = [".json"]
secondary_extensions = []
"#,
    );

    let servers = servers_for_uri(&router, "file:///w/settings.json");

    // Publish side: FanAll reaches it (diagnostics union is its whole surface).
    assert_eq!(
        dispatch_strategy("textDocument/publishDiagnostics"),
        DispatchStrategy::FanAll
    );
    assert_eq!(
        dispatch_strategy("textDocument/diagnostic"),
        DispatchStrategy::FanAll
    );
    assert_eq!(
        ids(&servers),
        vec!["config-lsp"],
        "FanAll candidate set includes the diagnostics-only child"
    );

    // Read surface: hover/completion/definition are FirstSuccess = Primary-tier
    // candidates only. The child IS returned by servers_for_uri (tier order)
    // but is filtered out of FirstSuccess candidacy by its tier.
    assert_eq!(
        dispatch_strategy("textDocument/hover"),
        DispatchStrategy::FirstSuccess
    );
    assert_eq!(
        dispatch_strategy("textDocument/completion"),
        DispatchStrategy::FirstSuccess
    );
    assert_eq!(
        dispatch_strategy("textDocument/definition"),
        DispatchStrategy::FirstSuccess
    );
    let primary_candidates: Vec<&ChildServer> = servers
        .iter()
        .filter(|s| matches!(s.tier, ChildTier::Primary))
        .collect();
    assert!(
        primary_candidates.is_empty(),
        "diagnostics-only child must never be a FirstSuccess candidate: {servers:?}"
    );

    // Unknown-method class (formatting, code action, rename, …) is PrimaryOnly
    // — also excluded by tier.
    assert_eq!(
        dispatch_strategy("textDocument/formatting"),
        DispatchStrategy::PrimaryOnly
    );
    assert_eq!(
        dispatch_strategy("textDocument/rename"),
        DispatchStrategy::PrimaryOnly
    );

    // State-mutating notifications: didOpen/didChange/didClose are NOT forwarded
    // to DiagnosticsOnly (should_forward_notification is the real gate).
    assert!(!should_forward_notification(
        "textDocument/didOpen",
        &ChildTier::DiagnosticsOnly
    ));
    assert!(!should_forward_notification(
        "textDocument/didChange",
        &ChildTier::DiagnosticsOnly
    ));
    assert!(!should_forward_notification(
        "textDocument/didClose",
        &ChildTier::DiagnosticsOnly
    ));
    // didSave IS forwarded (non-mutating for the child's view).
    assert!(should_forward_notification(
        "textDocument/didSave",
        &ChildTier::DiagnosticsOnly
    ));
}

#[test]
fn diagnostics_only_child_contributes_no_capabilities() {
    // The merged client-facing capability surface must not advertise hover or
    // completion from a DiagnosticsOnly child even when it is the only child.
    let diag_caps = lsp_max::lsp_types::ServerCapabilities {
        hover_provider: Some(lsp_max::lsp_types::HoverProviderCapability::Simple(true)),
        completion_provider: Some(lsp_max::lsp_types::CompletionOptions::default()),
        ..Default::default()
    };
    let merged = merge_capabilities(&[(ChildTier::DiagnosticsOnly, diag_caps)]);
    assert_eq!(
        merged.hover_provider, None,
        "DiagnosticsOnly must not contribute hover capability"
    );
    assert_eq!(
        merged.completion_provider, None,
        "DiagnosticsOnly must not contribute completion capability"
    );
}

// ---------------------------------------------------------------------------
// (2) full child: full request surface
// ---------------------------------------------------------------------------

#[test]
fn full_declaration_maps_to_primary_and_receives_full_surface() {
    let router = router_from_toml(
        r#"
[[server]]
id = "json-lsp"
priority = "full"
primary_extensions = [".json"]
secondary_extensions = [".jsonc"]
"#,
    );

    let servers = servers_for_uri(&router, "file:///w/settings.json");
    assert_eq!(ids(&servers), vec!["json-lsp"]);
    assert!(matches!(servers[0].tier, ChildTier::Primary));

    // Full surface: read methods are Primary candidates, notifications forwarded,
    // diagnostics fanned.
    assert!(
        servers
            .iter()
            .filter(|s| matches!(s.tier, ChildTier::Primary))
            .count()
            == 1
    );
    assert!(should_forward_notification(
        "textDocument/didOpen",
        &ChildTier::Primary
    ));
    assert!(should_forward_notification(
        "textDocument/didChange",
        &ChildTier::Primary
    ));
    assert!(should_forward_notification(
        "textDocument/didClose",
        &ChildTier::Primary
    ));
    assert_eq!(
        dispatch_strategy("textDocument/hover"),
        DispatchStrategy::FirstSuccess
    );

    // Secondary-declared extension lands in Secondary tier for a "full" child.
    let secondary = router.servers_for_filename("settings.jsonc");
    assert_eq!(ids(&secondary), vec!["json-lsp"]);
    assert!(matches!(secondary[0].tier, ChildTier::Secondary));
}

#[test]
fn full_child_capabilities_advertise_read_surface() {
    let full_caps = lsp_max::lsp_types::ServerCapabilities {
        hover_provider: Some(lsp_max::lsp_types::HoverProviderCapability::Simple(true)),
        completion_provider: Some(lsp_max::lsp_types::CompletionOptions::default()),
        ..Default::default()
    };
    let merged = merge_capabilities(&[(ChildTier::Primary, full_caps)]);
    assert!(
        merged.hover_provider.is_some(),
        "Primary (full) child's hover capability must survive the merge"
    );
    assert!(merged.completion_provider.is_some());
}

// ---------------------------------------------------------------------------
// (3) mixed set: full child on .json + diagnostics-only child on .md
//     (plus the deployment case: both co-tenants on .json)
// ---------------------------------------------------------------------------

#[test]
fn mixed_full_and_diagnostics_only_children_behave_per_declaration() {
    // Per-extension separation: each file kind gets exactly its declared child.
    let router = router_from_toml(
        r#"
[[server]]
id = "json-lsp"
priority = "full"
primary_extensions = [".json"]
secondary_extensions = []

[[server]]
id = "md-diag"
priority = "diagnostics-only"
primary_extensions = [".md"]
secondary_extensions = []
"#,
    );

    // .json traffic → json-lsp only.
    let json_servers = servers_for_uri(&router, "file:///w/a.json");
    assert_eq!(ids(&json_servers), vec!["json-lsp"]);
    assert!(matches!(json_servers[0].tier, ChildTier::Primary));

    // .md traffic → md-diag only, DiagnosticsOnly tier.
    let md_servers = servers_for_uri(&router, "file:///w/b.md");
    assert_eq!(ids(&md_servers), vec!["md-diag"]);
    assert!(matches!(md_servers[0].tier, ChildTier::DiagnosticsOnly));

    // Cross-check: hover on .json is a Primary candidate (json-lsp), hover on
    // .md has no Primary candidate (md-diag filtered out).
    let json_primary: Vec<_> = json_servers
        .iter()
        .filter(|s| matches!(s.tier, ChildTier::Primary))
        .collect();
    assert_eq!(json_primary.len(), 1);
    let md_primary: Vec<_> = md_servers
        .iter()
        .filter(|s| matches!(s.tier, ChildTier::Primary))
        .collect();
    assert!(md_primary.is_empty());
}

#[test]
fn cotenant_deployment_full_json_tool_plus_diagnostics_only_validator() {
    // The H3 config-lsp deployment shape: a general JSON tool (full) and the
    // config validator (diagnostics-only) co-tenants on .json.
    let router = router_from_toml(
        r#"
[[server]]
id = "json-lsp"
priority = "full"
primary_extensions = [".json"]
secondary_extensions = []

[[server]]
id = "config-lsp"
priority = "diagnostics-only"
primary_extensions = [".json"]
secondary_extensions = []
"#,
    );

    let servers = servers_for_uri(&router, "file:///w/settings.json");
    assert_eq!(
        ids(&servers),
        vec!["json-lsp", "config-lsp"],
        "tier order: Primary before DiagnosticsOnly"
    );

    // Full child sees the complete surface.
    assert!(matches!(servers[0].tier, ChildTier::Primary));
    // Validator receives diagnostics fan (FanAll reaches both).
    assert_eq!(
        dispatch_strategy("textDocument/diagnostic"),
        DispatchStrategy::FanAll
    );
    // …but is excluded from hover/completion candidacy.
    let primary: Vec<_> = servers
        .iter()
        .filter(|s| matches!(s.tier, ChildTier::Primary))
        .map(|s| s.id.clone())
        .collect();
    assert_eq!(
        primary,
        vec!["json-lsp"],
        "hover/completion candidacy stays with the full child"
    );

    // Notifications: didOpen fans to the full child, is withheld from the validator.
    assert!(should_forward_notification(
        "textDocument/didOpen",
        &servers[0].tier
    ));
    assert!(!should_forward_notification(
        "textDocument/didOpen",
        &servers[1].tier
    ));

    // Merged capability surface: validator's caps never leak into the aggregate
    // even alongside a full child.
    let full_caps = lsp_max::lsp_types::ServerCapabilities {
        hover_provider: Some(lsp_max::lsp_types::HoverProviderCapability::Simple(true)),
        ..Default::default()
    };
    let diag_caps = lsp_max::lsp_types::ServerCapabilities {
        hover_provider: Some(lsp_max::lsp_types::HoverProviderCapability::Simple(true)),
        completion_provider: Some(lsp_max::lsp_types::CompletionOptions::default()),
        ..Default::default()
    };
    let merged = merge_capabilities(&[
        (ChildTier::Primary, full_caps),
        (ChildTier::DiagnosticsOnly, diag_caps),
    ]);
    assert!(merged.hover_provider.is_some());
    assert!(
        merged.completion_provider.is_none(),
        "validator's completion capability must not leak through a co-tenant full child"
    );
}

// ---------------------------------------------------------------------------
// (4) misdeclaration: diagnostics-only child receiving a full-surface request
// ---------------------------------------------------------------------------

#[test]
fn misdeclared_diagnostics_only_child_full_surface_requests_are_refused_by_tier() {
    // H3 shape (diagnostics-only config-lsp on .json) but the client asks for
    // full-surface methods anyway. Documented behavior: the tier filter
    // refuses candidacy — the request is never delivered, there is no silent
    // pass-through.
    let router = router_from_toml(
        r#"
[[server]]
id = "config-lsp"
priority = "diagnostics-only"
primary_extensions = [".json"]
secondary_extensions = []
"#,
    );

    let servers = servers_for_uri(&router, "file:///w/settings.json");
    assert_eq!(servers.len(), 1);

    for method in [
        "textDocument/hover",
        "textDocument/completion",
        "textDocument/definition",
        "textDocument/formatting",
        "textDocument/rename",
    ] {
        let full_surface = matches!(
            dispatch_strategy(method),
            DispatchStrategy::FirstSuccess | DispatchStrategy::PrimaryOnly
        );
        assert!(
            full_surface,
            "sanity: {method} should classify as a full-surface dispatch class"
        );
        let candidates: Vec<_> = servers
            .iter()
            .filter(|s| matches!(s.tier, ChildTier::Primary))
            .collect();
        assert!(
            candidates.is_empty(),
            "{method} on a diagnostics-only child must have zero Primary candidates — refusal by tier filter"
        );
    }

    // The publish side is the ONLY surface that still reaches it.
    assert_eq!(
        dispatch_strategy("textDocument/diagnostic"),
        DispatchStrategy::FanAll
    );
    assert_eq!(ids(&servers), vec!["config-lsp"]);
}

#[test]
fn misdeclaration_priority_string_typo_falls_back_to_diagnostics_only() {
    // from_priority: anything unrecognized (typos, "FULL", "Full", "") maps to
    // DiagnosticsOnly. A typo'd "ful" declaration silently DEMOTES the child —
    // the conservative direction (read surface withheld, diagnostics kept).
    for typo in ["ful", "FULL", "Full", "", "diagnosticsonly", "primary "] {
        let tier = ChildTier::from_priority(typo);
        assert!(
            matches!(tier, ChildTier::DiagnosticsOnly),
            "priority {typo:?} must map to DiagnosticsOnly (conservative fallback)"
        );
        // A typo'd full declaration therefore loses the read surface:
        assert!(!should_forward_notification("textDocument/didOpen", &tier));
        assert_eq!(
            ChildTier::from_priority("diagnostics-only"),
            tier,
            "typo'd priority is indistinguishable from an explicit diagnostics-only declaration"
        );
    }

    // The recognized spellings still map as documented.
    assert!(matches!(
        ChildTier::from_priority("full"),
        ChildTier::Primary
    ));
    assert!(matches!(
        ChildTier::from_priority("semantic"),
        ChildTier::Primary
    ));
    assert!(matches!(
        ChildTier::from_priority("secondary"),
        ChildTier::Secondary
    ));
    assert!(matches!(ChildTier::from_priority("lsif"), ChildTier::Lsif));
}

#[test]
fn typo_priority_is_refused_at_load_and_from_config_matches_from_priority() {
    // Fixed hazard (was: from_config escalated any non-"diagnostics-only" string
    // to Primary while from_priority demoted typos). Now the exact-string
    // validation refuses the typo at TOML load with a typed error naming the
    // value and the accepted set; and from_config maps through from_priority,
    // so the two paths agree on every spelling — a typo demotes on both.
    let toml_src = r#"
[[server]]
id = "typo-lsp"
priority = "ful"
primary_extensions = [".json"]
secondary_extensions = []
"#;
    let parse_err = toml::from_str::<lsp_max_compositor::config::CompositorConfig>(toml_src)
        .expect_err("typo'd priority must be refused at load, not escalated");
    let msg = parse_err.to_string();
    assert!(
        msg.contains("\"ful\""),
        "refusal must name the invalid value, got: {msg}"
    );
    for accepted in ["full", "diagnostics-only", "semantic", "secondary", "lsif"] {
        assert!(
            msg.contains(accepted),
            "refusal must name accepted value {accepted:?}, got: {msg}"
        );
    }

    // Direct construction (bypassing serde) now also demotes — parity: the
    // tier from_config assigns equals from_priority's verdict for any string.
    let config = lsp_max_compositor::config::CompositorConfig {
        server: vec![lsp_max_compositor::config::ServerEntry {
            id: "typo-lsp".into(),
            primary_extensions: vec![".json".into()],
            secondary_extensions: vec![],
            priority: "ful".into(),
            andon_code_prefixes: None,
            command: None,
            args: None,
        }],
    };
    let router = ExtensionRouter::from_config(&config);
    let servers = router.servers_for_filename("a.json");
    assert_eq!(ids(&servers), vec!["typo-lsp"]);
    assert_eq!(
        servers[0].tier,
        ChildTier::from_priority("ful"),
        "from_config and from_priority must agree on every spelling, typos included"
    );
    assert!(
        matches!(servers[0].tier, ChildTier::DiagnosticsOnly),
        "typo demotes conservatively on both paths (no silent escalation)"
    );
}
