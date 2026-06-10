/// Shared helpers for autonomic_mesh tests.
use lsp_types_max as lsp_types;
use tower_lsp_max_runtime::MaxDiagnostic;

pub fn make_test_diagnostic(id: &str, law_id: &str) -> MaxDiagnostic {
    MaxDiagnostic {
        lsp: lsp_types::Diagnostic {
            range: lsp_types::Range::default(),
            severity: Some(lsp_types::DiagnosticSeverity::ERROR),
            code: None,
            code_description: None,
            source: Some("test".to_string()),
            message: format!("Test diagnostic {}", id),
            related_information: None,
            tags: None,
            data: None,
        },
        diagnostic_id: id.to_string(),
        law_id: law_id.to_string(),
        attempted_transition: None,
        violated_axes: vec![],
        doc_routes: vec![],
        repair_actions: vec![],
        verification_gates: vec![],
        receipt_obligation: None,
        ..Default::default()
    }
}
