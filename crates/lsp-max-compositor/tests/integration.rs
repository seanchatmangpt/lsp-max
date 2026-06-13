// Integration tests for lsp-max-compositor: fanout dispatch, routing, merge, law codes.

use lsp_max_compositor::compositor_client::CompositorClient;
use lsp_max_compositor::config::{CompositorConfig, ServerEntry};
use lsp_max_compositor::connections::ChildConnections;
use lsp_max_compositor::diagnostic_buffer::DiagnosticBuffer;
use lsp_max_compositor::fanout::{dispatch_strategy, servers_for_uri, DispatchStrategy};
use lsp_max_compositor::merge::{
    is_refused_by_law, is_refused_by_law_with_prefixes, merge_diagnostics, DiagnosticEntry,
};
use lsp_max_compositor::{ChildServer, ChildTier, ExtensionRouter, MergeContext, MergeResult};

// ── 1. dispatch_strategy classification ──────────────────────────────────────

#[test]
fn dispatch_strategy_first_success() {
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
}

#[test]
fn dispatch_strategy_fan_all() {
    assert_eq!(
        dispatch_strategy("textDocument/publishDiagnostics"),
        DispatchStrategy::FanAll
    );
    assert_eq!(
        dispatch_strategy("textDocument/diagnostic"),
        DispatchStrategy::FanAll
    );
}

#[test]
fn dispatch_strategy_notify() {
    assert_eq!(
        dispatch_strategy("textDocument/didOpen"),
        DispatchStrategy::Notify
    );
    assert_eq!(
        dispatch_strategy("textDocument/didChange"),
        DispatchStrategy::Notify
    );
    assert_eq!(
        dispatch_strategy("textDocument/didClose"),
        DispatchStrategy::Notify
    );
    assert_eq!(
        dispatch_strategy("textDocument/didSave"),
        DispatchStrategy::Notify
    );
}

#[test]
fn dispatch_strategy_primary_only_fallback() {
    assert_eq!(
        dispatch_strategy("textDocument/formatting"),
        DispatchStrategy::PrimaryOnly
    );
    assert_eq!(
        dispatch_strategy("workspace/symbol"),
        DispatchStrategy::PrimaryOnly
    );
    assert_eq!(
        dispatch_strategy("unknown/method"),
        DispatchStrategy::PrimaryOnly
    );
}

// ── 2. servers_for_uri ordering ───────────────────────────────────────────────

#[test]
fn servers_for_uri_primary_first() {
    let router = ExtensionRouter::new();
    router.register(
        "rs",
        ChildServer {
            id: "diag-server".into(),
            tier: ChildTier::DiagnosticsOnly,
            extensions: vec!["rs".into()],
        },
    );
    router.register(
        "rs",
        ChildServer {
            id: "primary-server".into(),
            tier: ChildTier::Primary,
            extensions: vec!["rs".into()],
        },
    );
    router.register(
        "rs",
        ChildServer {
            id: "secondary-server".into(),
            tier: ChildTier::Secondary,
            extensions: vec!["rs".into()],
        },
    );

    let result = servers_for_uri(&router, "file:///workspace/main.rs");
    assert_eq!(result.len(), 3);
    assert_eq!(result[0].id, "primary-server");
    assert_eq!(result[1].id, "secondary-server");
    assert_eq!(result[2].id, "diag-server");
}

#[test]
fn servers_for_uri_no_match_returns_empty() {
    let router = ExtensionRouter::new();
    router.register(
        "ts",
        ChildServer {
            id: "ts-server".into(),
            tier: ChildTier::Primary,
            extensions: vec!["ts".into()],
        },
    );

    let result = servers_for_uri(&router, "file:///workspace/main.rs");
    assert!(result.is_empty());
}

// ── 3. merge_diagnostics tier priority ───────────────────────────────────────

fn make_entry(
    uri: &str,
    line: u32,
    character: u32,
    code: &str,
    message: &str,
    severity: u8,
    tier: ChildTier,
) -> DiagnosticEntry {
    DiagnosticEntry {
        uri: uri.into(),
        line,
        character,
        severity,
        code: code.into(),
        message: message.into(),
        source_tier: tier,
    }
}

#[test]
fn merge_primary_wins_over_secondary_same_key() {
    let primary_entry = make_entry(
        "file:///foo.rs",
        10,
        5,
        "RUST-001",
        "primary message",
        2,
        ChildTier::Primary,
    );
    let secondary_entry = make_entry(
        "file:///foo.rs",
        10,
        5,
        "RUST-001",
        "secondary message",
        2,
        ChildTier::Secondary,
    );

    let result = merge_diagnostics(
        vec![
            (ChildTier::Secondary, vec![secondary_entry]),
            (ChildTier::Primary, vec![primary_entry]),
        ],
        None,
    );

    assert_eq!(result.len(), 1);
    assert_eq!(result[0].message, "primary message");
    assert!(matches!(result[0].source_tier, ChildTier::Primary));
}

#[test]
fn merge_distinct_keys_all_included() {
    let e1 = make_entry(
        "file:///foo.rs",
        1,
        0,
        "ERR-A",
        "error a",
        1,
        ChildTier::Primary,
    );
    let e2 = make_entry(
        "file:///foo.rs",
        2,
        0,
        "ERR-B",
        "error b",
        2,
        ChildTier::Secondary,
    );

    let result = merge_diagnostics(
        vec![
            (ChildTier::Primary, vec![e1]),
            (ChildTier::Secondary, vec![e2]),
        ],
        None,
    );

    assert_eq!(result.len(), 2);
}

#[test]
fn merge_refused_by_law_sorted_first() {
    let normal = make_entry(
        "file:///foo.rs",
        5,
        0,
        "RUST-001",
        "normal",
        1,
        ChildTier::Primary,
    );
    let law_violation = make_entry(
        "file:///foo.rs",
        10,
        0,
        "WASM4PM-CROWN-001",
        "law violation",
        1,
        ChildTier::DiagnosticsOnly,
    );

    let result = merge_diagnostics(
        vec![
            (ChildTier::Primary, vec![normal]),
            (ChildTier::DiagnosticsOnly, vec![law_violation]),
        ],
        None,
    );

    assert_eq!(result.len(), 2);
    // REFUSED_BY_LAW error must sort first
    assert_eq!(result[0].code, "WASM4PM-CROWN-001");
}

// ── 3b. REFUSED_BY_LAW severity inversion regression ─────────────────────────

#[test]
fn merge_refused_by_law_severity_wins_over_tier() {
    // Primary tier emits a Warning for a law code.
    let primary_warning = make_entry(
        "file:///foo.rs",
        10,
        5,
        "WASM4PM-CROWN-001",
        "primary warning",
        2, // Warning
        ChildTier::Primary,
    );
    // DiagnosticsOnly tier emits an Error for the same (uri, line, character, code).
    let diag_only_error = make_entry(
        "file:///foo.rs",
        10,
        5,
        "WASM4PM-CROWN-001",
        "diagnostics-only error",
        1, // Error
        ChildTier::DiagnosticsOnly,
    );

    let result = merge_diagnostics(
        vec![
            (ChildTier::Primary, vec![primary_warning]),
            (ChildTier::DiagnosticsOnly, vec![diag_only_error]),
        ],
        None,
    );

    assert_eq!(result.len(), 1);
    // Error (severity=1) must win over Warning (severity=2) regardless of tier.
    assert_eq!(
        result[0].severity, 1,
        "Error must not be shadowed by a Warning from a higher tier"
    );
    assert!(matches!(result[0].source_tier, ChildTier::DiagnosticsOnly));
}

// ── 4. is_refused_by_law coverage ────────────────────────────────────────────

#[test]
fn is_refused_by_law_true_for_law_prefixes() {
    assert!(is_refused_by_law("WASM4PM-CROWN-001"));
    assert!(is_refused_by_law("WASM4PM-CHEAT-C001"));
    assert!(is_refused_by_law("ANTI-LLM-CHEAT-C001"));
    assert!(is_refused_by_law("GGEN-TPL-001"));
    assert!(is_refused_by_law("GGEN-YIELD-003"));
    assert!(is_refused_by_law("GGEN-SRC-002"));
}

#[test]
fn is_refused_by_law_false_for_non_law_codes() {
    assert!(!is_refused_by_law("RUST-001"));
    assert!(!is_refused_by_law("E0001"));
    assert!(!is_refused_by_law(""));
    assert!(!is_refused_by_law("clippy::needless_return"));
}

// ── 5. config-driven ANDON prefix path ───────────────────────────────────────

#[test]
fn merge_respected_by_law_with_config_prefixes() {
    // "CUSTOM-LAW-001" is NOT refused by the static fallback.
    assert!(!is_refused_by_law("CUSTOM-LAW-001"));
    // But it IS refused when the config supplies the prefix.
    assert!(is_refused_by_law_with_prefixes(
        "CUSTOM-LAW-001",
        &["CUSTOM-LAW-"]
    ));

    let normal = make_entry(
        "file:///foo.rs",
        5,
        0,
        "RUST-001",
        "normal",
        1,
        ChildTier::Primary,
    );
    let custom_law = make_entry(
        "file:///foo.rs",
        10,
        0,
        "CUSTOM-LAW-001",
        "custom law violation",
        1,
        ChildTier::DiagnosticsOnly,
    );

    let prefixes: &[&str] = &["CUSTOM-LAW-"];
    let result = merge_diagnostics(
        vec![
            (ChildTier::Primary, vec![normal]),
            (ChildTier::DiagnosticsOnly, vec![custom_law]),
        ],
        Some(prefixes),
    );

    assert_eq!(result.len(), 2);
    // CUSTOM-LAW-001 must sort first when config prefixes are active.
    assert_eq!(result[0].code, "CUSTOM-LAW-001");
}

// ── 6a. ChildConnections tracking ────────────────────────────────────────────

#[test]
fn child_connections_tracks_notifications() {
    let connections = ChildConnections::new();
    connections.record_notification("wasm4pm-lsp", "file:///foo.ocel.json");
    let servers = connections.notified_servers("file:///foo.ocel.json");
    assert!(
        servers.contains(&"wasm4pm-lsp".to_string()),
        "expected wasm4pm-lsp in notified servers, got: {:?}",
        servers
    );
}

#[test]
fn child_connections_no_duplicate_queries_return_empty() {
    let connections = ChildConnections::new();
    let servers = connections.notified_servers("file:///never-notified.rs");
    assert!(servers.is_empty());
}

#[test]
fn child_connections_uris_for_server() {
    let connections = ChildConnections::new();
    connections.record_notification("rust-analyzer", "file:///a.rs");
    connections.record_notification("rust-analyzer", "file:///b.rs");
    let uris = connections.uris_for_server("rust-analyzer");
    assert_eq!(uris.len(), 2);
    assert!(uris.contains(&"file:///a.rs".to_string()));
    assert!(uris.contains(&"file:///b.rs".to_string()));
}

// ── 6b. MergeContext from config ──────────────────────────────────────────────

#[test]
fn merge_context_from_config_uses_configured_prefixes() {
    let config = CompositorConfig {
        server: vec![ServerEntry {
            id: "custom-law-server".into(),
            primary_extensions: vec!["rs".into()],
            secondary_extensions: vec![],
            priority: "high".into(),
            andon_code_prefixes: Some(vec!["CUSTOM-LAW-".into()]),
        }],
    };

    let context = MergeContext::from_config(&config);
    assert_eq!(context.andon_prefixes_count(), 1);

    let entry = make_entry(
        "file:///foo.rs",
        1,
        0,
        "CUSTOM-LAW-001",
        "custom law violation",
        1,
        ChildTier::DiagnosticsOnly,
    );

    let result: MergeResult = context.merge(vec![(ChildTier::DiagnosticsOnly, vec![entry])]);
    assert_eq!(result.diagnostics.len(), 1);
    // CUSTOM-LAW-001 with severity=1 sorts first (law error)
    assert_eq!(result.diagnostics[0].code, "CUSTOM-LAW-001");
    assert!(
        result.has_andon_block,
        "CUSTOM-LAW-001 severity 1 should set has_andon_block"
    );
}

// ── 6c. MergeResult has_andon_block false for non-error ──────────────────────

#[test]
fn merge_result_has_andon_block_false_for_non_error() {
    let config = CompositorConfig {
        server: vec![ServerEntry {
            id: "custom-law-server".into(),
            primary_extensions: vec!["rs".into()],
            secondary_extensions: vec![],
            priority: "high".into(),
            andon_code_prefixes: Some(vec!["CUSTOM-LAW-".into()]),
        }],
    };

    let context = MergeContext::from_config(&config);

    let entry = make_entry(
        "file:///foo.rs",
        1,
        0,
        "CUSTOM-LAW-001",
        "custom law warning",
        2, // Warning — not an Error
        ChildTier::DiagnosticsOnly,
    );

    let result: MergeResult = context.merge(vec![(ChildTier::DiagnosticsOnly, vec![entry])]);
    assert_eq!(result.diagnostics.len(), 1);
    assert!(
        !result.has_andon_block,
        "severity=2 (Warning) must not set has_andon_block"
    );
}

// ── 7. DiagnosticBuffer ───────────────────────────────────────────────────────

fn make_merge_ctx_with_legacy_prefixes() -> MergeContext {
    MergeContext::new(vec![
        "WASM4PM-".to_string(),
        "ANTI-LLM-".to_string(),
        "GGEN-".to_string(),
    ])
}

#[test]
fn diagnostic_buffer_deposit_and_flush() {
    use std::sync::Arc;
    let ctx = make_merge_ctx_with_legacy_prefixes();
    let buffer = DiagnosticBuffer::new(Arc::new(ctx));

    buffer.deposit(
        "file:///foo.rs",
        "anti-llm-cheat",
        ChildTier::DiagnosticsOnly,
        vec![make_entry(
            "file:///foo.rs",
            1,
            0,
            "ANTI-LLM-CHEAT-C001",
            "cheat detected",
            1,
            ChildTier::DiagnosticsOnly,
        )],
    );
    buffer.deposit(
        "file:///foo.rs",
        "rust-analyzer",
        ChildTier::Primary,
        vec![make_entry(
            "file:///foo.rs",
            5,
            0,
            "E0308",
            "type mismatch",
            1,
            ChildTier::Primary,
        )],
    );

    let result = buffer.flush("file:///foo.rs");
    assert_eq!(result.diagnostics.len(), 2);
    assert!(
        result.has_andon_block,
        "ANTI-LLM-CHEAT-C001 severity 1 must set has_andon_block"
    );
}

#[test]
fn diagnostic_buffer_deposit_replaces_previous_from_same_server() {
    use std::sync::Arc;
    let ctx = make_merge_ctx_with_legacy_prefixes();
    let buffer = DiagnosticBuffer::new(Arc::new(ctx));

    buffer.deposit(
        "file:///foo.rs",
        "server-a",
        ChildTier::Primary,
        vec![make_entry(
            "file:///foo.rs",
            1,
            0,
            "X001",
            "first",
            1,
            ChildTier::Primary,
        )],
    );
    buffer.deposit(
        "file:///foo.rs",
        "server-a",
        ChildTier::Primary,
        vec![make_entry(
            "file:///foo.rs",
            1,
            0,
            "X002",
            "second",
            1,
            ChildTier::Primary,
        )],
    );

    let result = buffer.flush("file:///foo.rs");
    assert_eq!(
        result.diagnostics.len(),
        1,
        "second deposit must replace first"
    );
    assert_eq!(result.diagnostics[0].code, "X002");
}

#[test]
fn diagnostic_buffer_clear_uri_empties_buffer() {
    use std::sync::Arc;
    let ctx = make_merge_ctx_with_legacy_prefixes();
    let buffer = DiagnosticBuffer::new(Arc::new(ctx));

    buffer.deposit(
        "file:///foo.rs",
        "server-a",
        ChildTier::Primary,
        vec![make_entry(
            "file:///foo.rs",
            1,
            0,
            "X001",
            "msg",
            1,
            ChildTier::Primary,
        )],
    );
    assert_eq!(buffer.buffered_uri_count(), 1);

    buffer.clear_uri("file:///foo.rs");
    assert_eq!(buffer.buffered_uri_count(), 0);
}

// ── 8. did_close eviction ─────────────────────────────────────────────────────

#[test]
fn server_clears_buffer_on_did_close() {
    use std::sync::Arc;
    let ctx = make_merge_ctx_with_legacy_prefixes();
    let buffer = DiagnosticBuffer::new(Arc::new(ctx));

    buffer.deposit(
        "file:///foo.rs",
        "server-a",
        ChildTier::Primary,
        vec![make_entry(
            "file:///foo.rs",
            1,
            0,
            "X001",
            "msg",
            1,
            ChildTier::Primary,
        )],
    );
    assert_eq!(buffer.buffered_uri_count(), 1);

    buffer.clear_uri("file:///foo.rs");
    assert_eq!(buffer.buffered_uri_count(), 0);

    let result = buffer.flush("file:///foo.rs");
    assert_eq!(result.diagnostics.len(), 0);
    assert!(!result.has_andon_block);
}

#[test]
fn flush_uri_returns_merge_result_with_andon_block() {
    use std::sync::Arc;
    let ctx = make_merge_ctx_with_legacy_prefixes();
    let buffer = DiagnosticBuffer::new(Arc::new(ctx));

    buffer.deposit(
        "file:///foo.rs",
        "anti-llm-cheat",
        ChildTier::DiagnosticsOnly,
        vec![make_entry(
            "file:///foo.rs",
            1,
            0,
            "ANTI-LLM-CHEAT-C001",
            "cheat detected",
            1,
            ChildTier::DiagnosticsOnly,
        )],
    );

    let result = buffer.flush("file:///foo.rs");
    assert!(
        result.has_andon_block,
        "ANTI-LLM-CHEAT-C001 severity 1 must set has_andon_block"
    );
    assert!(
        result.andon_codes().contains(&"ANTI-LLM-CHEAT-C001"),
        "andon_codes() must include ANTI-LLM-CHEAT-C001, got: {:?}",
        result.andon_codes()
    );
}

// ── 9. CompositorClient deposits on publish_diagnostics ──────────────────────

#[tokio::test]
async fn compositor_client_deposits_on_publish_diagnostics() {
    use lsp_max::lsp_types::{
        Diagnostic, DiagnosticSeverity, NumberOrString, Position, PublishDiagnosticsParams, Range,
        Uri,
    };
    use lsp_max_client::LanguageClient;
    use std::str::FromStr;
    use std::sync::Arc;

    let ctx = make_merge_ctx_with_legacy_prefixes();
    let buffer = Arc::new(DiagnosticBuffer::new(Arc::new(ctx)));

    let client = CompositorClient::new(
        "test-server".to_string(),
        ChildTier::DiagnosticsOnly,
        Arc::clone(&buffer),
    );

    let uri = Uri::from_str("file:///test.ocel.json").unwrap();
    let params = PublishDiagnosticsParams {
        uri: uri.clone(),
        diagnostics: vec![Diagnostic {
            range: Range {
                start: Position {
                    line: 0,
                    character: 0,
                },
                end: Position {
                    line: 0,
                    character: 10,
                },
            },
            severity: Some(DiagnosticSeverity::ERROR),
            code: Some(NumberOrString::String("WASM4PM-CROWN-001".to_string())),
            message: "missing output_hash".to_string(),
            ..Default::default()
        }],
        version: None,
    };

    client.publish_diagnostics(params).await;

    assert_eq!(buffer.buffered_uri_count(), 1);

    let result = buffer.flush("file:///test.ocel.json");
    assert!(
        result.has_andon_block,
        "WASM4PM-CROWN-001 severity ERROR must set has_andon_block"
    );
}
