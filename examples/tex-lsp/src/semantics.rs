use tower_lsp_max::auto_lsp::AutoLspAdapter;
use tower_lsp_max::lsp_types_max::Diagnostic;
use tower_lsp_max::lsp_types_max::{DiagnosticSeverity, NumberOrString, Position, Range};

pub fn dispatch_semantic_rules(
    adapter: &AutoLspAdapter,
    uri: &tower_lsp_max::lsp_types_max::DocumentUri,
) -> Vec<Diagnostic> {
    let mut diagnostics = adapter.pull_diagnostics(uri);

    adapter.get_document(uri, |doc| {
        let text = doc.as_str();
        let lines: Vec<&str> = text.lines().collect();

        // 1. Lexical Pattern Refusals (Lazy Authoring)
        for (i, line) in lines.iter().enumerate() {
            let lower_line = line.to_lowercase();
            if lower_line.contains("in a full")
                || lower_line.contains("in a real")
                || lower_line.contains("todo!")
            {
                diagnostics.push(Diagnostic {
                    range: Range {
                        start: Position {
                            line: i as u32,
                            character: 0,
                        },
                        end: Position {
                            line: i as u32,
                            character: line.len() as u32,
                        },
                    },
                    severity: Some(DiagnosticSeverity::ERROR),
                    code: Some(NumberOrString::String("LAZY_AUTHORING".to_string())),
                    source: Some("tex-lsp".to_string()),
                    message: "Forbidden placeholder or stub detected in academic document."
                        .to_string(),
                    ..Default::default()
                });
            }
        }
    });

    diagnostics
}
