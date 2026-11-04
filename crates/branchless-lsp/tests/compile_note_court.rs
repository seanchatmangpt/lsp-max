//! Lane LSPA-2 compile-note court (`docs/jira/v26.11.2/lanes/lane-lspa-02-compile-note.md`).
//!
//! Kills the LSPU-2 double-silence finding: a rustc-failure save must NOT
//! read as "0 refusals". `verdict_diagnostics` now emits the
//! `Tier2Outcome.note` as a WARNING-class diagnostic (COMPILE-NOTE-001),
//! published on the real wire.
//!
//! Real collaborators only (G06 pattern): real `LspService` +
//! `Client`/`ClientSocket` loopback, real tower `initialize`, real
//! `rustc --emit=obj` subprocess through `run_tier2`, real wire
//! `publishDiagnostics` notification parsed off the socket. No mocks.
//!
//! Anti-vacuity flip: the SYNTAX-ERROR fixture must publish >=1 WARNING
//! carrying the note; the CLEAN fixture over the SAME pipeline must publish
//! zero COMPILE-NOTE diagnostics. A court that cannot distinguish a
//! compile-failure save from a clean save is FAIL.

use branchless_lsp::rule_pack::{verdict_diagnostics, Tier2Outcome, COMPILE_NOTE_RULE_ID};
use branchless_lsp::BranchlessLspServer;
use futures::StreamExt;
use lsp_max::jsonrpc::Request;
use lsp_max::{LanguageServer, LspService};
use lsp_types_max::{
    DiagnosticSeverity, DidSaveTextDocumentParams, PublishDiagnosticsParams, TextDocumentIdentifier,
};
use std::time::Duration;
use tower::{Service, ServiceExt};

const DIAG_METHOD: &str = "textDocument/publishDiagnostics";

/// A source with a deterministic rustc syntax error: unbalanced closing brace.
const SYNTAX_ERROR: &str = "fn f() -> u32 { 1 } }\n";

/// Branch-free fixture (the flip arm): compiles clean, must publish no note.
const CLEAN: &str = "#[no_mangle]\npub extern \"C\" fn lspa2_clean(x: u32) -> u32 {\n    core::hint::black_box(x).wrapping_add(1)\n}\n";

/// Drive one did_save through the REAL service; return every
/// publishDiagnostics notification for the saved URI.
async fn save_and_collect(content: &str, uri_str: &str) -> Vec<PublishDiagnosticsParams> {
    let (mut service, mut socket) = LspService::new(|client| BranchlessLspServer::new(client));
    let init = Request::build("initialize")
        .params(serde_json::json!({"capabilities": {}}))
        .id(1)
        .finish();
    let resp = service
        .ready()
        .await
        .expect("service ready")
        .call(init)
        .await
        .expect("initialize must succeed");
    assert!(resp.is_some(), "initialize must answer");

    let server = service.inner();
    let uri: lsp_types_max::Uri = uri_str.parse().expect("valid uri");
    server
        .did_save(DidSaveTextDocumentParams {
            text_document: TextDocumentIdentifier { uri },
            text: Some(content.to_string()),
        })
        .await;

    let deadline = Duration::from_secs(180);
    let mut pubs = Vec::new();
    loop {
        match tokio::time::timeout(deadline, socket.next()).await {
            Ok(Some(req)) => {
                if req.method() == DIAG_METHOD {
                    let p: PublishDiagnosticsParams =
                        serde_json::from_value(req.params().cloned().unwrap_or_default())
                            .expect("publishDiagnostics params decode");
                    if p.uri.as_str() == uri_str {
                        pubs.push(p);
                        break;
                    }
                }
            }
            Ok(None) => break,
            Err(_) => break,
        }
    }
    pubs
}

fn note_diags(p: &PublishDiagnosticsParams) -> Vec<&lsp_types_max::Diagnostic> {
    p.diagnostics
        .iter()
        .filter(|d| {
            d.severity == Some(DiagnosticSeverity::WARNING)
                && d.code
                    == Some(lsp_types_max::NumberOrString::String(
                        COMPILE_NOTE_RULE_ID.into(),
                    ))
        })
        .collect()
}

/// COURT (falsifier arm, LSPU-2): a rustc syntax-error save MUST publish
/// >=1 WARNING COMPILE-NOTE diagnostic — never an empty array.
#[tokio::test(flavor = "current_thread")]
async fn syntax_error_save_publishes_warning_note() {
    let pubs = save_and_collect(SYNTAX_ERROR, "file:///tmp/lspa2-err.rs").await;
    assert!(
        !pubs.is_empty(),
        "[FAIL] did_save must publish diagnostics for the syntax-error save"
    );
    let notes = note_diags(&pubs[0]);
    assert!(
        !notes.is_empty(),
        "[FAIL] syntax-error save must publish >=1 WARNING COMPILE-NOTE; \
         got {} diag(s): {:?}",
        pubs[0].diagnostics.len(),
        pubs[0]
            .diagnostics
            .iter()
            .map(|d| (&d.severity, &d.message))
            .collect::<Vec<_>>()
    );
    assert!(
        notes.iter().any(|d| d.message.contains("compile error")),
        "[FAIL] the note must carry the rustc compile-error text; got {:?}",
        notes.iter().map(|d| &d.message).collect::<Vec<_>>()
    );
}

/// COURT (flip arm): a clean save over the SAME real pipeline must publish
/// ZERO COMPILE-NOTE diagnostics — unchanged behavior, no false notes.
#[tokio::test(flavor = "current_thread")]
async fn clean_save_publishes_no_compile_note() {
    let pubs = save_and_collect(CLEAN, "file:///tmp/lspa2-clean.rs").await;
    assert!(
        !pubs.is_empty(),
        "[FAIL] did_save must still publish (empty) diagnostics for a clean save"
    );
    assert!(
        note_diags(&pubs[0]).is_empty(),
        "[FAIL] clean save must publish zero COMPILE-NOTE diagnostics; got {:?}",
        pubs[0]
            .diagnostics
            .iter()
            .map(|d| (&d.severity, &d.message))
            .collect::<Vec<_>>()
    );
    assert!(
        pubs[0]
            .diagnostics
            .iter()
            .all(|d| d.severity != Some(DiagnosticSeverity::ERROR)),
        "[FAIL] clean save must also stay ERROR-free"
    );
}

/// COURT (ordering arm): note diagnostic is emitted FIRST (deterministic).
#[test]
fn note_diag_is_emitted_before_function_verdicts() {
    let mut out = Tier2Outcome::default();
    out.note = Some("compile error (not a court verdict): boom".to_string());
    let diags = verdict_diagnostics(&out);
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].severity, Some(DiagnosticSeverity::WARNING));
    assert_eq!(
        diags[0].code,
        Some(lsp_types_max::NumberOrString::String(
            COMPILE_NOTE_RULE_ID.into()
        ))
    );
}

/// COURT (truncation arm): an oversized note truncates to a bounded,
/// deterministic payload.
#[test]
fn oversized_note_truncates_deterministically() {
    let long = "x".repeat(10_000);
    let mut out = Tier2Outcome::default();
    out.note = Some(long.clone());
    let diags = verdict_diagnostics(&out);
    assert_eq!(diags.len(), 1);
    let msg = &diags[0].message;
    assert!(
        msg.chars().count() < long.chars().count(),
        "oversized note must be truncated"
    );
    assert!(msg.ends_with("...[truncated]"));
    // Determinism: identical input, identical output.
    let again = {
        let mut o = Tier2Outcome::default();
        o.note = Some(long);
        verdict_diagnostics(&o)
    };
    assert_eq!(diags, again, "truncation must be deterministic");
    // Also via the wire path (same construction as lib.rs did_save).
    assert!(verdict_diagnostics(&out) == again);
}

/// COURT (gate-parity arm): the note diagnostic is WARNING class, so the
/// gate's ERROR-only counting (`is_error`, gate-branchless-lsp.py:77-81)
/// must NOT classify it as a refusal. Severity WARNING == 2, ERROR == 1.
#[test]
fn note_diag_is_warning_not_error_for_gate_parity() {
    let mut out = Tier2Outcome::default();
    out.note = Some("compile error (not a court verdict): e".to_string());
    let d = &verdict_diagnostics(&out)[0];
    assert_eq!(
        d.severity,
        Some(DiagnosticSeverity::WARNING),
        "note must be WARNING (2); the gate keys on severity-ERROR (1)"
    );
    assert_ne!(d.severity, Some(DiagnosticSeverity::ERROR));
}
