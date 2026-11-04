//! M10 residual 1: relayed diagnostic ranges collapsed to their start point —
//! DiagnosticEntry carried only (line, character); the range end was dropped in
//! compositor_client.rs's entry mapping and re-materialized as a point range in
//! server.rs's Diagnostic conversion. These tests drive the real relay path
//! (CompositorClient → DiagnosticBuffer → MergeContext) with multi-character and
//! zero-width spans and assert the end position arrives at the merged output.

use std::sync::Arc;

use lsp_max::client::LanguageClient;
use lsp_max::lsp_types::{
    DiagnosticSeverity, NumberOrString, Position, PublishDiagnosticsParams, Range, Uri,
};
use lsp_max_compositor::merge::{merge_diagnostics, DiagnosticEntry, MergeContext};
use lsp_max_compositor::{ChildTier, CompositorClient, DiagnosticBuffer, GateFile};

fn diag(range: Range, code: &str, message: &str) -> lsp_max::lsp_types::Diagnostic {
    lsp_max::lsp_types::Diagnostic {
        range,
        severity: Some(DiagnosticSeverity::WARNING),
        code: Some(NumberOrString::String(code.to_string())),
        source: Some("claude-code-config-lsp".to_string()),
        message: message.to_string(),
        ..Default::default()
    }
}

fn params(uri: &Uri, diagnostics: Vec<lsp_max::lsp_types::Diagnostic>) -> PublishDiagnosticsParams {
    PublishDiagnosticsParams {
        uri: uri.clone(),
        diagnostics,
        version: None,
    }
}

#[tokio::test]
async fn multi_char_range_end_survives_relay_and_merge() {
    let ctx = Arc::new(MergeContext::new(vec!["CCC-".to_string()]));
    let gate = Arc::new(GateFile::for_workspace());
    let buffer = Arc::new(DiagnosticBuffer::new(ctx.clone(), gate));
    let client = CompositorClient::new(
        "ccc-child".to_string(),
        ChildTier::DiagnosticsOnly,
        buffer.clone(),
    );

    // The settings-enum span M10 witnessed on the wire: `{"model": "gpt-4"}` —
    // child emitted 0:1 → 0:8 (the quoted key span); the relay collapsed it to 0:1.
    let range = Range {
        start: Position {
            line: 0,
            character: 1,
        },
        end: Position {
            line: 0,
            character: 8,
        },
    };
    let uri: Uri = "file:///ws/settings.json".parse().unwrap();
    client
        .publish_diagnostics(params(
            &uri,
            vec![diag(range, "CCC-JSON-004", "Invalid model 'gpt-4'")],
        ))
        .await;

    let merged = buffer.flush("file:///ws/settings.json");

    assert_eq!(merged.diagnostics.len(), 1, "one diagnostic relayed");
    let d = &merged.diagnostics[0];
    assert_eq!((d.line, d.character), (0, 1), "start preserved");
    assert_eq!(
        (d.end_line, d.end_character),
        (0, 8),
        "end position must survive child → compositor → merge exactly as the child emitted it"
    );
}

#[tokio::test]
async fn zero_range_diagnostic_stays_zero_range() {
    let ctx = Arc::new(MergeContext::new(vec!["CCC-".to_string()]));
    let gate = Arc::new(GateFile::for_workspace());
    let buffer = Arc::new(DiagnosticBuffer::new(ctx.clone(), gate));
    let client = CompositorClient::new(
        "ccc-child".to_string(),
        ChildTier::DiagnosticsOnly,
        buffer.clone(),
    );

    let point = Range {
        start: Position {
            line: 3,
            character: 7,
        },
        end: Position {
            line: 3,
            character: 7,
        },
    };
    let uri: Uri = "file:///ws/settings.json".parse().unwrap();
    client
        .publish_diagnostics(params(&uri, vec![diag(point, "CCC-MD-006", "missing H1")]))
        .await;

    let merged = buffer.flush("file:///ws/settings.json");

    assert_eq!(merged.diagnostics.len(), 1);
    let d = &merged.diagnostics[0];
    assert_eq!(
        (d.line, d.character, d.end_line, d.end_character),
        (3, 7, 3, 7),
        "zero-range diagnostics must remain zero-range after relay"
    );
}

#[tokio::test]
async fn multiline_range_end_survives_relay() {
    let ctx = Arc::new(MergeContext::new(vec!["CCC-".to_string()]));
    let gate = Arc::new(GateFile::for_workspace());
    let buffer = Arc::new(DiagnosticBuffer::new(ctx.clone(), gate));
    let client = CompositorClient::new(
        "ccc-child".to_string(),
        ChildTier::DiagnosticsOnly,
        buffer.clone(),
    );

    let range = Range {
        start: Position {
            line: 2,
            character: 4,
        },
        end: Position {
            line: 5,
            character: 11,
        },
    };
    let uri: Uri = "file:///ws/CLAUDE.md".parse().unwrap();
    client
        .publish_diagnostics(params(
            &uri,
            vec![diag(range, "CCC-MD-001", "block spans lines")],
        ))
        .await;

    let merged = buffer.flush("file:///ws/CLAUDE.md");

    let d = &merged.diagnostics[0];
    assert_eq!(
        (d.end_line, d.end_character),
        (5, 11),
        "cross-line end positions are preserved, not flattened"
    );
}

/// Dedup key stays (uri, line, character, code): when two servers emit at the same
/// start with different ends, the dedup WINNER's own end must be kept — the merge
/// must not overwrite or widen the winner's span with the loser's.
#[test]
fn dedup_winner_keeps_its_own_end_position() {
    use lsp_max_compositor::merge::DiagnosticEntry;

    let mk = |tier: ChildTier, end_char: u32| DiagnosticEntry {
        uri: "file:///x.rs".into(),
        line: 0,
        character: 1,
        end_line: 0,
        end_character: end_char,
        severity: 2,
        code: "CCC-JSON-004".into(),
        message: "same start, different ends".into(),
        source_tier: tier,
        server_id: Some("child".into()),
    };

    let result = merge_diagnostics(
        vec![
            (ChildTier::Secondary, vec![mk(ChildTier::Secondary, 8)]),
            (
                ChildTier::DiagnosticsOnly,
                vec![mk(ChildTier::DiagnosticsOnly, 99)],
            ),
        ],
        Some(&["CCC-"]),
    );

    assert_eq!(result.len(), 1, "same (uri,start,code) dedups to one");
    assert_eq!(
        (result[0].end_line, result[0].end_character),
        (0, 8),
        "CCC- is a law prefix here: equal severity, Secondary outranks DiagnosticsOnly, \
         so the Secondary entry's own end (8) stands — no widening, no point-collapse"
    );
}
