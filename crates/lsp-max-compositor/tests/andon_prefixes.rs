// andon_prefixes.rs — `andon_code_prefixes` manifest key → runtime semantics.
//
// Chicago tests: a real `lsp-max.toml` (tempfile on disk) parsed through the
// real `CompositorConfig::from_toml_file`, wired through the real
// `MergeContext::from_config`, and asserted on final observable state:
// attribution route, `has_andon_block`, merged-stream ordering/dedup, and
// survival of the lsp-max client's ANDON-law injection
// (`LSPMAX-ANDON-PUSH-MISSING`, src/service/client/lsp_methods.rs).
//
// Semantics under test (read from src/config.rs + src/merge.rs):
//   - A server that declares `andon_code_prefixes` is classified STRICTLY by
//     its own set (AndonRoute::PerServer on match, NotAndon otherwise — the
//     workspace union is never borrowed).
//   - A server without the key inherits the static defaults
//     (WASM4PM-/ANTI-LLM-/GGEN-) and classifies through the union fallback.
//   - `has_andon_block` requires severity == 1 (Error) AND an andon match.
//   - Merged-stream ordering puts andon Errors first; law-code dedup lets the
//     lower severity number win at a duplicate key.
//   - "LSPMAX-" is not a declared prefix in any family under test, so the
//     client-injected LSPMAX-ANDON-PUSH-MISSING code classifies NotAndon and
//     is never dropped by the compositor's merge (dedup is key-scoped, and
//     its Error severity wins same-key ties against non-andon codes).

use lsp_max::lsp_types::{Diagnostic, DiagnosticSeverity, NumberOrString, Position, Range};
use lsp_max_compositor::config::CompositorConfig;
use lsp_max_compositor::merge::{merge_for_upstream, AndonRoute, DiagnosticEntry, MergeContext};
use lsp_max_compositor::ChildTier;

// ── manifest fixture: real TOML on disk ──────────────────────────────────────

const MANIFEST: &str = r#"
[[server]]
id = "config-lsp"
primary_extensions = ["json"]
secondary_extensions = []
priority = "full"
andon_code_prefixes = ["CCC-"]
command = "true"

[[server]]
id = "ggen-lsp"
primary_extensions = ["ttl"]
secondary_extensions = ["md"]
priority = "full"
command = "true"
"#;

fn config_from_manifest() -> CompositorConfig {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("lsp-max.toml");
    std::fs::write(&path, MANIFEST).expect("write manifest");
    CompositorConfig::from_toml_file(&path).expect("parse manifest")
}

// ── helpers ──────────────────────────────────────────────────────────────────

fn entry(
    uri: &str,
    line: u32,
    code: &str,
    severity: u8,
    server_id: Option<&str>,
) -> DiagnosticEntry {
    DiagnosticEntry {
        uri: uri.to_string(),
        line,
        character: 0,
        end_line: line,
        end_character: 0,
        severity,
        code: code.to_string(),
        message: format!("finding {code}"),
        source_tier: ChildTier::Secondary,
        server_id: server_id.map(|s| s.to_string()),
    }
}

fn lsp_diag(code: &str, severity: DiagnosticSeverity, source: Option<&str>) -> Diagnostic {
    Diagnostic {
        range: Range {
            start: Position {
                line: 0,
                character: 0,
            },
            end: Position {
                line: 0,
                character: 4,
            },
        },
        code: Some(NumberOrString::String(code.to_string())),
        severity: Some(severity),
        source: source.map(|s| s.to_string()),
        message: format!("finding {code}"),
        ..Default::default()
    }
}

fn codes(v: &[Diagnostic]) -> Vec<String> {
    v.iter()
        .map(|d| match &d.code {
            Some(NumberOrString::String(s)) => s.clone(),
            _ => "<none>".to_string(),
        })
        .collect()
}

// ── 1. Manifest key → per-server prefix map ─────────────────────────────────

#[test]
fn manifest_key_routes_to_per_server_prefix_map() {
    let config = config_from_manifest();
    let map = config.per_server_andon_prefixes();

    assert_eq!(
        map.get("config-lsp").map(|v| v.as_slice()),
        Some(["CCC-".to_string()].as_slice()),
        "declared andon_code_prefixes must become the server's own C_D"
    );
    assert_eq!(
        map.get("ggen-lsp").map(|v| v.as_slice()),
        Some(
            [
                "WASM4PM-".to_string(),
                "ANTI-LLM-".to_string(),
                "GGEN-".to_string()
            ]
            .as_slice()
        ),
        "server without the key must inherit the static defaults"
    );
}

#[test]
fn manifest_key_joins_the_workspace_union() {
    let config = config_from_manifest();
    let all = config.all_andon_prefixes();

    assert!(
        all.contains(&"CCC-"),
        "declared prefix must join the workspace-wide union"
    );
    assert!(
        all.contains(&"WASM4PM-"),
        "default prefixes of servers without the key must also join the union"
    );
    assert_eq!(all.len(), 4, "union is deduplicated across servers");
}

#[test]
fn from_config_gives_every_server_its_prefix_set() {
    let ctx = MergeContext::from_config(&config_from_manifest());

    assert_eq!(
        ctx.prefixes_for_server("config-lsp"),
        ["CCC-"],
        "configured server routes through its own C_D"
    );
    assert_eq!(
        ctx.prefixes_for_server("unknown-server"),
        ["CCC-", "WASM4PM-", "ANTI-LLM-", "GGEN-"],
        "unknown server falls back to the workspace baseline, which here is the \
         full deduplicated union including the declared CCC- prefix"
    );
    assert_eq!(ctx.andon_prefixes_count(), 4);
}

// ── 2. Registered prefix: CCC-* from config-lsp → attribution + blocking ────

#[test]
fn registered_ccc_code_attributes_per_server() {
    let ctx = MergeContext::from_config(&config_from_manifest());

    let route = ctx.attribute_andon("CCC-TEST-001", Some("config-lsp"));
    assert_eq!(
        route,
        AndonRoute::PerServer {
            server_id: "config-lsp".to_string()
        },
        "CCC-* from config-lsp must attribute to config-lsp's own C_D, not the union"
    );
    assert!(ctx.is_andon_for_server("CCC-TEST-001", Some("config-lsp")));
}

#[test]
fn registered_ccc_error_blocks_and_sorts_first() {
    let ctx = MergeContext::from_config(&config_from_manifest());
    let result = ctx.merge(vec![(
        ChildTier::Secondary,
        vec![
            entry(
                "file:///proj/settings.json",
                3,
                "CCC-BLOCK-001",
                1,
                Some("config-lsp"),
            ),
            entry(
                "file:///proj/settings.json",
                7,
                "ORDINARY-1",
                1,
                Some("config-lsp"),
            ),
        ],
    )]);

    assert!(
        result.has_andon_block,
        "CCC- Error (severity 1) from a configured server must raise has_andon_block"
    );
    // Documented divergence: MergeResult::andon_codes() filters with the
    // STATIC law prefixes (WASM4PM-/ANTI-LLM-/GGEN-), not the configured C_D.
    // A configured CCC- Error raises has_andon_block and sorts first, but is
    // not surfaced through andon_codes().
    assert!(
        result.andon_codes().is_empty(),
        "andon_codes() is static-law-only; configured CCC- codes surface via \
         has_andon_block and ordering, not this accessor"
    );
    assert_eq!(
        result.diagnostics[0].code, "CCC-BLOCK-001",
        "registered andon Error must sort before ordinary Errors"
    );
}

// ── 3. Unregistered prefix → documented NotAndon behavior ────────────────────

#[test]
fn unregistered_prefix_is_not_andon_and_does_not_block() {
    let ctx = MergeContext::from_config(&config_from_manifest());
    let route = ctx.attribute_andon("ZOO-UNKNOWN-9", Some("config-lsp"));

    assert_eq!(
        route,
        AndonRoute::NotAndon,
        "a code whose prefix no registered C_D declares is NotAndon for a configured server"
    );
    assert!(!ctx.is_andon_for_server("ZOO-UNKNOWN-9", Some("config-lsp")));

    let result = ctx.merge(vec![(
        ChildTier::Secondary,
        vec![entry(
            "file:///proj/x.json",
            0,
            "ZOO-UNKNOWN-9",
            1,
            Some("config-lsp"),
        )],
    )]);
    assert!(
        !result.has_andon_block,
        "NotAndon Error must not raise has_andon_block"
    );
}

#[test]
fn ccc_code_from_another_configured_server_does_not_classify_as_ccc() {
    // L7 speciation: ggen-lsp has its own C_D (the defaults); CCC- is not in
    // it. The union (which does contain CCC-) must NOT be borrowed.
    let ctx = MergeContext::from_config(&config_from_manifest());
    let route = ctx.attribute_andon("CCC-TEST-001", Some("ggen-lsp"));
    assert_eq!(
        route,
        AndonRoute::NotAndon,
        "CCC- must not leak via the workspace union to another configured server"
    );
    assert!(!ctx.is_andon_for_server("CCC-TEST-001", Some("ggen-lsp")));
}

// ── 4. ANDON law preservation (LSPMAX-ANDON-PUSH-MISSING) ────────────────────
//
// config-lsp's ANDON constraint (claude-code-config-lsp backend.rs
// andon_diagnostic_tests): findings must never emit ERROR severity, because
// lsp-max's client publish_diagnostics injects LSPMAX-ANDON-PUSH-MISSING on
// any ERROR whose source != "lsp-max-andon" (src/service/client/lsp_methods.rs).
// These tests pin the compositor side: the injected diagnostic must survive
// the compositor merge and must NOT be classified as a declared andon code
// (attribution stays honest).

/// Reconstruct the exact diagnostic lsp-max's client injects (lsp_methods.rs).
fn injected_andon_push_missing() -> Diagnostic {
    Diagnostic {
        range: Default::default(),
        severity: Some(DiagnosticSeverity::ERROR),
        code: Some(NumberOrString::String(
            "LSPMAX-ANDON-PUSH-MISSING".to_string(),
        )),
        source: Some("lsp-max-andon".to_string()),
        message: "A blocking diagnostic was emitted without a corresponding ANDON push event."
            .to_string(),
        related_information: None,
        tags: None,
        code_description: None,
        data: None,
    }
}

#[test]
fn injected_andon_push_missing_survives_compositor_merge() {
    let config = config_from_manifest();
    let prefixes = config
        .all_andon_prefixes()
        .into_iter()
        .map(|s| s.to_string())
        .collect::<Vec<_>>();

    // Real downstream shape: child contributions plus the client-injected
    // diagnostic arriving in the same publish batch.
    let contributions = vec![
        (
            "config-lsp".to_string(),
            vec![lsp_diag(
                "CCC-TEST-001",
                DiagnosticSeverity::WARNING,
                Some("claude-code-config-lsp"),
            )],
        ),
        (
            "lsp-max-client".to_string(),
            vec![injected_andon_push_missing()],
        ),
    ];

    let merged = merge_for_upstream(&contributions, &prefixes);
    assert_eq!(
        codes(&merged),
        vec!["CCC-TEST-001", "LSPMAX-ANDON-PUSH-MISSING"],
        "the injected ANDON-law diagnostic must be preserved through the merge"
    );
}

#[test]
fn injected_diagnostic_wins_same_key_ties_by_error_severity() {
    let config = config_from_manifest();
    let prefixes = config
        .all_andon_prefixes()
        .into_iter()
        .map(|s| s.to_string())
        .collect::<Vec<_>>();

    // Same (range, no-code) key as the injected diagnostic: the injected
    // Error must not be replaced by a same-key Warning.
    let child_warning_no_code = Diagnostic {
        code: None,
        severity: Some(DiagnosticSeverity::WARNING),
        ..injected_andon_push_missing()
    };
    let contributions = vec![
        (
            "lsp-max-client".to_string(),
            vec![injected_andon_push_missing()],
        ),
        ("config-lsp".to_string(), vec![child_warning_no_code]),
    ];

    let merged = merge_for_upstream(&contributions, &prefixes);
    assert_eq!(
        codes(&merged)[0],
        "LSPMAX-ANDON-PUSH-MISSING",
        "the injected Error must win the same-key tie by severity"
    );
}

#[test]
fn injected_code_is_not_a_declared_andon_code() {
    let ctx = MergeContext::from_config(&config_from_manifest());
    assert!(
        !ctx.is_andon_for_server("LSPMAX-ANDON-PUSH-MISSING", Some("config-lsp")),
        "LSPMAX- is not a declared prefix; the injected diagnostic must not \
         classify as andon under config-lsp's C_D"
    );
    assert!(
        !ctx.is_andon_for_server("LSPMAX-ANDON-PUSH-MISSING", None),
        "nor under the workspace union"
    );
}

#[test]
fn error_finding_from_config_lsp_would_trigger_injection_contract() {
    // Falsifier for config-lsp's own severity pin: IF a finding ever emits
    // ERROR severity (source != "lsp-max-andon"), the client injects
    // LSPMAX-ANDON-PUSH-MISSING. Run that hypothetical through the
    // compositor's real merge and pin the compositor's behavior: the injected
    // diagnostic is preserved AND does not mask the originating finding.
    let config = config_from_manifest();
    let prefixes = config
        .all_andon_prefixes()
        .into_iter()
        .map(|s| s.to_string())
        .collect::<Vec<_>>();

    let contributions = vec![(
        "config-lsp".to_string(),
        vec![
            lsp_diag(
                "CCC-VIOLATION",
                DiagnosticSeverity::ERROR,
                Some("claude-code-config-lsp"),
            ),
            injected_andon_push_missing(),
        ],
    )];

    let merged = merge_for_upstream(&contributions, &prefixes);
    assert_eq!(
        codes(&merged),
        vec!["CCC-VIOLATION", "LSPMAX-ANDON-PUSH-MISSING"],
        "injection contract downstream: both the violating finding and the \
         injected diagnostic must survive the merge"
    );
}
