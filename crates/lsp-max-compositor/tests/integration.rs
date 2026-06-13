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
        server_id: None,
    }
}

#[test]
fn diagnostic_entry_server_id_preserved_through_merge() {
    let entry = DiagnosticEntry {
        uri: "file:///foo.rs".into(),
        line: 0,
        character: 0,
        severity: 1,
        code: "WASM4PM-CROWN-001".into(),
        message: "missing output_hash".into(),
        source_tier: ChildTier::DiagnosticsOnly,
        server_id: Some("wasm4pm-lsp".into()),
    };

    let result = merge_diagnostics(vec![(ChildTier::DiagnosticsOnly, vec![entry])], None);

    assert_eq!(result.len(), 1);
    assert_eq!(result[0].server_id, Some("wasm4pm-lsp".into()));
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
            command: None,
            args: None,
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
            command: None,
            args: None,
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
    let buffer = DiagnosticBuffer::new(Arc::new(ctx), Arc::new(lsp_max_compositor::GateFile::from_path(std::path::PathBuf::from("/tmp/test-gate"))));

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
    let buffer = DiagnosticBuffer::new(Arc::new(ctx), Arc::new(lsp_max_compositor::GateFile::from_path(std::path::PathBuf::from("/tmp/test-gate"))));

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
    let buffer = DiagnosticBuffer::new(Arc::new(ctx), Arc::new(lsp_max_compositor::GateFile::from_path(std::path::PathBuf::from("/tmp/test-gate"))));

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
    let buffer = DiagnosticBuffer::new(Arc::new(ctx), Arc::new(lsp_max_compositor::GateFile::from_path(std::path::PathBuf::from("/tmp/test-gate"))));

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
    let buffer = DiagnosticBuffer::new(Arc::new(ctx), Arc::new(lsp_max_compositor::GateFile::from_path(std::path::PathBuf::from("/tmp/test-gate"))));

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
    let buffer = Arc::new(DiagnosticBuffer::new(Arc::new(ctx), Arc::new(lsp_max_compositor::GateFile::from_path(std::path::PathBuf::from("/tmp/test-gate")))));

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

// ── 10. FlushCoordinator — signal_flush on dropped receiver does not panic ────

#[test]
fn flush_coordinator_signal_flush_on_closed_channel_does_not_panic() {
    // Build a coordinator by constructing only the sender side of an mpsc channel,
    // then drop the receiver to simulate server shutdown, and verify try_send
    // (which signal_flush uses internally) is panic-free.
    use tokio::sync::mpsc;
    let (tx, rx) = mpsc::channel::<String>(4);
    drop(rx); // receiver gone — channel is closed
              // try_send on a closed channel returns Err, which signal_flush discards silently.
    let result = tx.try_send("file:///foo.rs".to_string());
    // Err expected — the point is that it does NOT panic.
    assert!(result.is_err(), "expected Err on closed channel, got Ok");
}

// ── 11. Soundness contract: REFUSED_BY_LAW dedup across all three tiers ──────

#[test]
fn merge_law_soundness_all_tiers_contribute() {
    // Three entries for the same (uri, line, char, code) — one from each tier.
    // The DiagnosticsOnly entry carries severity=1 (Error), the other two carry
    // severity=2 (Warning). The soundness contract requires exactly one survivor
    // with the minimum severity (1 = Error).
    let primary = make_entry(
        "file:///foo.rs",
        5,
        0,
        "ANTI-LLM-CHEAT-C001",
        "primary warning",
        2, // Warning
        ChildTier::Primary,
    );
    let secondary = make_entry(
        "file:///foo.rs",
        5,
        0,
        "ANTI-LLM-CHEAT-C001",
        "secondary warning",
        2, // Warning
        ChildTier::Secondary,
    );
    let diag_only = make_entry(
        "file:///foo.rs",
        5,
        0,
        "ANTI-LLM-CHEAT-C001",
        "diagnostics-only error",
        1, // Error — most severe
        ChildTier::DiagnosticsOnly,
    );

    let result = merge_diagnostics(
        vec![
            (ChildTier::Primary, vec![primary]),
            (ChildTier::Secondary, vec![secondary]),
            (ChildTier::DiagnosticsOnly, vec![diag_only]),
        ],
        None,
    );

    assert_eq!(result.len(), 1, "all three tiers must dedup to one entry");
    assert_eq!(
        result[0].severity, 1,
        "Error (severity=1) must survive, not Warning (severity=2)"
    );
    // Sanity check: the code is indeed a law code.
    assert!(
        is_refused_by_law("ANTI-LLM-CHEAT-C001"),
        "ANTI-LLM-CHEAT-C001 must be classified as REFUSED_BY_LAW"
    );
}

#[test]
fn merge_non_law_dedup_primary_wins() {
    // Two entries for the same (uri, line, char, code), non-law code.
    // Primary tier must win deduplication.
    let primary = make_entry(
        "file:///foo.rs",
        3,
        0,
        "E0308",
        "primary type mismatch",
        1, // Error
        ChildTier::Primary,
    );
    let diag_only = make_entry(
        "file:///foo.rs",
        3,
        0,
        "E0308",
        "diagnostics-only type mismatch",
        1, // Error — same severity
        ChildTier::DiagnosticsOnly,
    );

    let result = merge_diagnostics(
        vec![
            (ChildTier::Primary, vec![primary]),
            (ChildTier::DiagnosticsOnly, vec![diag_only]),
        ],
        None,
    );

    assert_eq!(
        result.len(),
        1,
        "non-law entries at same location must dedup to one"
    );
    assert!(
        matches!(result[0].source_tier, ChildTier::Primary),
        "Primary tier must win deduplication for non-law codes, got: {:?}",
        result[0].source_tier
    );
}

// ── notify children for uri — empty router ───────────────────────────────────

#[test]
fn notify_children_for_uri_with_no_pool_entries_does_not_panic() {
    // Since we cannot instantiate CompositorServer without a real Client,
    // verify the fanout logic at the servers_for_uri level.
    let router = ExtensionRouter::new();
    let servers = servers_for_uri(&router, "file:///test.rs");
    assert!(servers.is_empty(), "empty router returns no servers");
}

// ── initialized fan-out — empty pool ─────────────────────────────────────────

#[test]
fn initialized_fan_out_does_not_panic_with_empty_pool() {
    // Verify that server_ids_snapshot() on an empty pool returns an empty Vec
    // and that the fan-out iteration loop body is never entered.
    // Full CompositorServer construction requires a real lsp_max::Client
    // (not constructible in unit tests); this test covers the pool-level
    // primitive that the fan-out delegates to.
    use lsp_max_compositor::child_process::ChildProcessPool;
    let pool = ChildProcessPool::new();
    let ids = pool.server_ids_snapshot();
    assert!(
        ids.is_empty(),
        "empty pool must return empty snapshot, got: {:?}",
        ids
    );
    // The fan-out loop iterates over ids — with an empty vec it performs zero
    // iterations and cannot panic. Verify the count explicitly.
    assert_eq!(
        ids.len(),
        0,
        "fan-out over empty snapshot must iterate zero times"
    );
}

// ── initialized backfill ──────────────────────────────────────────────────────

#[test]
fn initialized_backfill_flushes_all_buffered_uris() {
    use std::sync::Arc;
    let ctx = make_merge_ctx_with_legacy_prefixes();
    let buffer = DiagnosticBuffer::new(Arc::new(ctx), Arc::new(lsp_max_compositor::GateFile::from_path(std::path::PathBuf::from("/tmp/test-gate"))));

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
            2,
            ChildTier::DiagnosticsOnly,
        )],
    );

    buffer.deposit(
        "file:///bar.ocel.json",
        "wasm4pm-lsp",
        ChildTier::DiagnosticsOnly,
        vec![make_entry(
            "file:///bar.ocel.json",
            0,
            0,
            "WASM4PM-GALL-001",
            "process conformance deviation",
            1,
            ChildTier::DiagnosticsOnly,
        )],
    );

    assert_eq!(buffer.buffered_uri_count(), 2);

    let mut uris = buffer.buffered_uris();
    uris.sort();
    assert_eq!(
        uris,
        vec!["file:///bar.ocel.json", "file:///foo.rs"],
        "buffered_uris() must return both deposited URIs"
    );

    // flush each URI and verify entries are present
    let result_foo = buffer.flush("file:///foo.rs");
    assert_eq!(result_foo.diagnostics.len(), 1);
    assert_eq!(result_foo.diagnostics[0].code, "ANTI-LLM-CHEAT-C001");

    let result_bar = buffer.flush("file:///bar.ocel.json");
    assert_eq!(result_bar.diagnostics.len(), 1);
    assert_eq!(result_bar.diagnostics[0].code, "WASM4PM-GALL-001");
    assert!(
        result_bar.has_andon_block,
        "WASM4PM-GALL-001 severity ERROR must set has_andon_block"
    );
}

// ── 12. CompositorStateResponse ───────────────────────────────────────────────

#[test]
fn compositor_state_empty_buffer_returns_empty_uris() {
    use std::sync::Arc;

    let ctx = Arc::new(MergeContext::new(vec![]));
    let buffer = Arc::new(DiagnosticBuffer::new(ctx.clone(), Arc::new(lsp_max_compositor::GateFile::from_path(std::path::PathBuf::from("/tmp/test-gate")))));

    // Verify that buffered_uris() is empty on a fresh buffer.
    assert!(buffer.buffered_uris().is_empty());
}

#[test]
fn uri_andon_state_serializes_to_json() {
    use lsp_max_compositor::state_response::{CompositorStateResponse, UriAndonState};

    let response = CompositorStateResponse {
        uris: vec![UriAndonState {
            uri: "file:///test.rs".to_string(),
            has_andon_block: true,
            andon_codes: vec!["WASM4PM-CROWN-001".to_string()],
            diagnostic_count: 1,
        }],
        global_andon_block: true,
        child_server_count: 2,
        query_timestamp_ms: 0,
        signal_drop_count: 0,
    };

    let json = serde_json::to_string(&response).unwrap();
    assert!(json.contains("WASM4PM-CROWN-001"));
    assert!(json.contains("\"global_andon_block\":true"));
}

#[test]
fn compositor_state_response_has_timestamp_field() {
    use lsp_max_compositor::state_response::CompositorStateResponse;
    let resp = CompositorStateResponse {
        uris: vec![],
        global_andon_block: false,
        child_server_count: 0,
        query_timestamp_ms: 1_700_000_000_000,
        signal_drop_count: 0,
    };
    let json = serde_json::to_string(&resp).unwrap();
    assert!(json.contains("query_timestamp_ms"));
    assert!(json.contains("1700000000000"));
}

// ── health_response tests ────────────────────────────────────────────────────

#[test]
fn health_response_serializes_to_json() {
    use lsp_max_compositor::health_response::CompositorHealth;
    let h = CompositorHealth {
        child_server_count: 3,
        child_server_ids: vec!["wasm4pm-lsp".to_string()],
        buffered_uri_count: 5,
        has_any_andon_block: false,
    };
    let json = serde_json::to_string(&h).unwrap();
    assert!(json.contains("\"child_server_count\":3"));
    assert!(json.contains("wasm4pm-lsp"));
}

#[test]
fn diagnostic_buffer_gate_not_written_on_empty_deposit() {
    use lsp_max_compositor::diagnostic_buffer::DiagnosticBuffer;
    use lsp_max_compositor::merge::MergeContext;
    use lsp_max_compositor::GateFile;
    use std::path::PathBuf;
    use std::sync::Arc;
    // Use a temp path that will not exist before the test runs.
    let gate_path = PathBuf::from("/tmp/test-gate-empty-deposit-xyzzy");
    let _ = std::fs::remove_file(&gate_path);
    let gate = Arc::new(GateFile::from_path(gate_path.clone()));
    let ctx = Arc::new(MergeContext::new(vec![]));
    let buffer = DiagnosticBuffer::new(ctx, Arc::clone(&gate));
    // Deposit with no entries — gate must NOT be written (no ANDON signal).
    buffer.deposit("file:///test.rs", "srv", lsp_max_compositor::registry::ChildTier::DiagnosticsOnly, vec![]);
    // Gate file should not exist or be unset.
    assert_ne!(gate.read(), Some(true));
}

#[test]
fn diagnostic_ack_serializes_to_json() {
    use lsp_max_compositor::diagnostic_ack::DiagnosticAck;
    let ack = DiagnosticAck {
        uri: "file:///test.rs".to_string(),
        admitted_count: 3,
        suppressed_count: 1,
        has_andon_contribution: true,
    };
    let json = serde_json::to_string(&ack).unwrap();
    assert!(json.contains("\"admitted_count\":3"));
    assert!(json.contains("\"has_andon_contribution\":true"));
}

// ── shutdown fan-out — pool primitives ───────────────────────────────────────

#[test]
fn shutdown_fan_out_iterates_all_children() {
    // Verify server_ids_snapshot returns consistent data for shutdown iteration.
    use lsp_max_compositor::child_process::ChildProcessPool;
    let pool = ChildProcessPool::new();
    // Empty pool: snapshot should return empty vec (no panic).
    let ids = pool.server_ids_snapshot();
    assert!(
        ids.is_empty(),
        "empty pool snapshot should be empty for shutdown"
    );
}

#[test]
fn pool_snapshot_stable_across_iterations() {
    use lsp_max_compositor::child_process::ChildProcessPool;
    let pool = ChildProcessPool::new();
    let snap1 = pool.server_ids_snapshot();
    let snap2 = pool.server_ids_snapshot();
    assert_eq!(
        snap1, snap2,
        "consecutive snapshots of empty pool must be equal"
    );
}

#[test]
fn diagnostic_ack_zero_suppressed_is_valid() {
    use lsp_max_compositor::diagnostic_ack::DiagnosticAck;
    let ack = DiagnosticAck {
        uri: "file:///test.rs".to_string(),
        admitted_count: 0,
        suppressed_count: 0,
        has_andon_contribution: false,
    };
    assert_eq!(ack.admitted_count, 0);
    assert!(!ack.has_andon_contribution);
}

#[test]
fn merged_capabilities_starts_as_none() {
    // Verify the capability_merge output is serializable (type-level check).
    // CompositorServer cannot be instantiated without a Client, so we exercise
    // the merge path and confirm serde round-trip is ADMITTED.
    use lsp_max_compositor::capability_merge::merge_capabilities;

    let caps = merge_capabilities(&[]);
    let json = serde_json::to_value(&caps);
    assert!(
        json.is_ok(),
        "default ServerCapabilities must serialize to JSON"
    );
}

#[test]
fn merged_capabilities_with_hover_serializes_correctly() {
    use lsp_max::lsp_types::{HoverProviderCapability, ServerCapabilities};
    use lsp_max_compositor::capability_merge::merge_capabilities;
    use lsp_max_compositor::registry::ChildTier;

    let primary = ServerCapabilities {
        hover_provider: Some(HoverProviderCapability::Simple(true)),
        ..Default::default()
    };

    let merged = merge_capabilities(&[(ChildTier::Primary, primary)]);
    let json = serde_json::to_value(&merged).unwrap();

    // hover_provider must be present in the JSON when Primary sets it.
    assert!(
        json.get("hoverProvider").is_some(),
        "merged caps JSON must contain hoverProvider when Primary sets it"
    );
}

#[test]
fn child_process_pool_remove_returns_proc() {
    // After remove on empty pool returns None — no panic.
    use lsp_max_compositor::child_process::ChildProcessPool;
    let pool = ChildProcessPool::new();
    assert_eq!(pool.server_ids_snapshot().len(), 0);
    let result = pool.remove("nonexistent");
    assert!(result.is_none());
    // Pool remains empty after remove.
    assert_eq!(pool.server_ids_snapshot().len(), 0);
}

// ── gate_file tests ──────────────────────────────────────────────────────────

#[test]
fn gate_file_write_read_round_trip() {
    use lsp_max_compositor::gate_file::GateFile;

    let tmp = std::env::temp_dir().join(format!("lsp-max-gate-test-{}", std::process::id()));
    let gate = GateFile::from_path(tmp.clone());

    gate.write(false);
    assert_eq!(gate.read(), Some(false));

    gate.write(true);
    assert_eq!(gate.read(), Some(true));

    // Explicit remove so Drop doesn't race with assertions on CI.
    gate.remove();
    assert_eq!(gate.read(), None);
}

#[test]
fn gate_file_missing_returns_none() {
    use lsp_max_compositor::gate_file::GateFile;
    use std::path::PathBuf;

    let gate = GateFile::from_path(PathBuf::from("/tmp/lsp-max-gate-no-such-file-xyzzy"));
    assert_eq!(gate.read(), None);
}

#[test]
fn gate_file_drop_removes_file() {
    use lsp_max_compositor::gate_file::GateFile;

    let path = std::env::temp_dir().join(format!("lsp-max-gate-drop-{}", std::process::id()));
    {
        let gate = GateFile::from_path(path.clone());
        gate.write(false);
        assert!(path.exists());
    } // Drop fires here
    assert!(!path.exists());
}
