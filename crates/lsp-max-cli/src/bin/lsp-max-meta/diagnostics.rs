use lsp_max::lsp_types::{Diagnostic, DiagnosticSeverity, NumberOrString, Position, Range};

use crate::scanner::{Violation, ViolationSeverity};

/// Convert a scanner `Violation` into an LSP `Diagnostic`.
pub fn to_lsp_diagnostic(v: &Violation) -> Diagnostic {
    let severity = match v.severity {
        ViolationSeverity::Error => DiagnosticSeverity::ERROR,
        ViolationSeverity::Warning => DiagnosticSeverity::WARNING,
        ViolationSeverity::Information => DiagnosticSeverity::INFORMATION,
        ViolationSeverity::Hint => DiagnosticSeverity::HINT,
    };
    Diagnostic {
        range: Range {
            start: Position {
                line: v.line,
                character: v.col_start,
            },
            end: Position {
                line: v.line,
                character: v.col_end,
            },
        },
        severity: Some(severity),
        code: Some(NumberOrString::String(v.code.clone())),
        source: Some("lsp-max-meta".into()),
        message: v.message.clone(),
        ..Default::default()
    }
}
