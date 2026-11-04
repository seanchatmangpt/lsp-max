// Priority-string validation at config load (CL-G, M9 hazard fix).
//
// Hazard: `ExtensionRouter::from_config` escalated any non-"diagnostics-only"
// priority string to Primary tier while `ChildTier::from_priority` demoted
// typos to DiagnosticsOnly — a typo'd "ful" silently escalated the child.
// Fix: exact-string validation at TOML load (typed refusal naming the value
// and the accepted set), and from_config routes through from_priority so both
// paths agree on every accepted spelling.
//
// Chicago discipline: real TOML parses, real router, final-state asserts.

#![cfg(feature = "full")]

use lsp_max_compositor::config::{CompositorConfig, ServerEntry};
use lsp_max_compositor::registry::{ChildTier, ExtensionRouter};
use lsp_max_compositor::{ChildServer, ExtensionRouter as Router};

const ACCEPTED: &[&str] = &["full", "diagnostics-only", "semantic", "secondary", "lsif"];

fn manifest(priority: &str) -> String {
    format!(
        r#"
[[server]]
id = "probe-lsp"
priority = "{}"
primary_extensions = [".json"]
secondary_extensions = [".md"]
"#,
        priority
    )
}

fn parse(src: &str) -> Result<CompositorConfig, Box<dyn std::error::Error>> {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("lsp-max.toml");
    std::fs::write(&path, src).expect("write manifest");
    CompositorConfig::from_toml_file(&path)
}

// `parse` writes to a tempdir that lives for the duration of the call, so
// `from_toml_file` reads a real file on disk (Chicago: real collaborators).
#[test]
fn typo_priority_refused_at_load_naming_value_and_accepted_set() {
    for typo in ["ful", "FULL", "diagnostic-only", "primary", "high", ""] {
        let err = parse(&manifest(typo)).expect_err(&format!(
            "priority {typo:?} is outside the accepted set; load must refuse it"
        ));
        let msg = err.to_string();
        assert!(
            msg.contains(&format!("\"{typo}\"")),
            "refusal must name the invalid value {typo:?}, got: {msg}"
        );
        for accepted in ACCEPTED {
            assert!(
                msg.contains(accepted),
                "refusal must list accepted value {accepted:?}, got: {msg}"
            );
        }
    }
}

#[test]
fn every_accepted_priority_parses_and_maps_to_its_tier() {
    let expected = [
        ("full", ChildTier::Primary),
        ("semantic", ChildTier::Primary),
        ("secondary", ChildTier::Secondary),
        ("lsif", ChildTier::Lsif),
        ("diagnostics-only", ChildTier::DiagnosticsOnly),
    ];
    for (priority, want) in expected {
        let config = parse(&manifest(priority)).unwrap_or_else(|_| {
            panic!("accepted priority {priority:?} must parse (valid set unchanged)")
        });
        assert_eq!(
            config.server[0].priority, priority,
            "parsed value round-trips"
        );
        assert_eq!(
            ChildTier::from_priority(&config.server[0].priority),
            want,
            "from_priority({priority:?}) tier"
        );
        let router = ExtensionRouter::from_config(&config);
        let primary_ext_servers = router.servers_for_filename("a.json");
        let secondary_ext_servers = router.servers_for_filename("doc.md");
        assert_eq!(
            primary_ext_servers[0].tier, want,
            "from_config primary-ext tier for {priority:?}"
        );
        let secondary_want = match want {
            ChildTier::DiagnosticsOnly | ChildTier::Lsif => want.clone(),
            _ => ChildTier::Secondary,
        };
        assert_eq!(
            secondary_ext_servers[0].tier, secondary_want,
            "from_config secondary-ext tier for {priority:?}"
        );
    }
}

#[test]
fn from_config_and_from_priority_agree_on_every_accepted_string() {
    for priority in ACCEPTED {
        let config: CompositorConfig = toml::from_str(&manifest(priority)).expect("parse");
        let router = ExtensionRouter::from_config(&config);
        let servers = router.servers_for_filename("a.json");
        assert_eq!(servers.len(), 1);
        assert_eq!(
            servers[0].tier,
            ChildTier::from_priority(priority),
            "from_config primary-ext tier must equal from_priority({priority:?})"
        );
    }
}

#[test]
fn direct_construction_typo_demotes_on_both_paths() {
    // Struct literals bypass serde; from_config must still agree with
    // from_priority (conservative demotion, never escalation).
    let entry = ServerEntry {
        id: "typo-lsp".into(),
        primary_extensions: vec![".json".into()],
        secondary_extensions: vec![],
        priority: "ful".into(),
        andon_code_prefixes: None,
        command: None,
        args: None,
    };
    let config = CompositorConfig {
        server: vec![entry],
    };
    let router = ExtensionRouter::from_config(&config);
    let servers = router.servers_for_filename("a.json");
    assert_eq!(servers[0].tier, ChildTier::from_priority("ful"));
    assert!(matches!(servers[0].tier, ChildTier::DiagnosticsOnly));

    // The same ChildServer-shaped registration parity holds for the tier type
    // used by the merge path.
    let cs = ChildServer {
        id: "typo-lsp".into(),
        tier: ChildTier::from_priority("ful"),
        extensions: vec![],
        commands: Vec::new(),
    };
    let r2 = Router::new();
    r2.register(".json", cs);
    assert_eq!(
        r2.servers_for_filename("a.json")[0].tier,
        ChildTier::DiagnosticsOnly
    );
}

#[test]
fn valid_mixed_manifest_still_parses() {
    // The production spellings from lsp-max/lsp-max.toml and
    // claude-code-config-lsp/lsp-max.toml parse unchanged.
    let src = r#"
[[server]]
id = "wasm4pm-lsp"
priority = "diagnostics-only"
primary_extensions = [".rs"]
secondary_extensions = [".md"]

[[server]]
id = "ggen-lsp"
priority = "full"
primary_extensions = [".ttl", ".rq"]
secondary_extensions = []
"#;
    let config = parse(src).expect("valid manifest must parse");
    assert_eq!(config.server.len(), 2);
}
