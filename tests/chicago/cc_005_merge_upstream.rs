//! Chicago acceptance test for CC-005: merge_for_upstream publishDiagnostics
//! Status: CANDIDATE — implement merge_for_upstream to make this test pass.
//! Ticket: docs/jira/v26.6.30/CC-005-diagnostic-merge-claude-code.md

use chicago_tdd_tools::chicago_test;
use lsp_max::lsp_types::{Diagnostic, DiagnosticSeverity, NumberOrString, Position, Range};

fn diag(start: (u32, u32), end: (u32, u32), code: &str, severity: DiagnosticSeverity) -> Diagnostic {
    Diagnostic {
        range: Range {
            start: Position { line: start.0, character: start.1 },
            end: Position { line: end.0, character: end.1 },
        },
        severity: Some(severity),
        code: Some(NumberOrString::String(code.to_string())),
        message: format!("test diagnostic {code}"),
        ..Default::default()
    }
}

#[chicago_test(
    ticket      = "docs/jira/v26.6.30/CC-005-diagnostic-merge-claude-code.md",
    scaffold_fn = "lsp_max_compositor::merge::merge_for_upstream"
)]
fn identical_diagnostics_from_two_servers_deduplicated() {
    // Given: two servers emit the same diagnostic (same range + code)
    let d = diag((1, 0), (1, 10), "E001", DiagnosticSeverity::ERROR);
    let contributions = vec![
        ("rust-analyzer".to_string(), vec![d.clone()]),
        ("anti-llm".to_string(), vec![d.clone()]),
    ];
    // When: merge_for_upstream is called
    let merged = lsp_max_compositor::merge::merge_for_upstream(&contributions, &[]);
    // Then: only one diagnostic in the merged list
    assert_eq!(merged.len(), 1, "identical diagnostics should be deduplicated");
}

#[chicago_test(
    ticket      = "docs/jira/v26.6.30/CC-005-diagnostic-merge-claude-code.md",
    scaffold_fn = "lsp_max_compositor::merge::merge_for_upstream"
)]
fn refused_by_law_hint_survives_dedup_against_warning() {
    // Given: anti-llm emits ANTI-LLM-HOLLOW-002 at Hint; rust-analyzer emits Warning at same range
    let anti_llm_diag = diag((2, 0), (2, 5), "ANTI-LLM-HOLLOW-002", DiagnosticSeverity::HINT);
    let rust_diag = diag((2, 0), (2, 5), "ANTI-LLM-HOLLOW-002", DiagnosticSeverity::WARNING);
    let contributions = vec![
        ("anti-llm".to_string(), vec![anti_llm_diag]),
        ("rust-analyzer".to_string(), vec![rust_diag]),
    ];
    let andon_prefixes = vec!["ANTI-LLM-".to_string()];
    // When: merge_for_upstream is called with ANTI-LLM- as andon prefix
    let merged = lsp_max_compositor::merge::merge_for_upstream(&contributions, &andon_prefixes);
    // Then: exactly one diagnostic survives (REFUSED_BY_LAW always survives)
    assert_eq!(merged.len(), 1, "REFUSED_BY_LAW code should survive dedup");
}
