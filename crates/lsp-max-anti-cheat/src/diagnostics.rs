use lsp_max::lsp_types::{Diagnostic, DiagnosticSeverity, Position, Range};
use serde::{Deserialize, Serialize};

/// Anti-LLM diagnostic with optional Oracle class association and confidence score.
/// Integrates with wasm4pm process mining (Oracle classes A8-A12).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AntiLlmDiagnostic {
    pub code: String,
    pub category: String,
    pub file_path: String,
    pub line: usize,
    pub column: usize,
    pub message: String,
    pub forbidden_implication: String,
    pub blocking: bool,
    pub required_correction: String,
    pub required_next_proof: String,
    /// Oracle class (A8-A12) if detected by wasm4pm conformance analysis.
    pub oracle_class: Option<String>,
    /// Confidence score [0.0, 1.0] from wasm4pm Oracle inference.
    pub confidence: Option<f64>,
}

impl AntiLlmDiagnostic {
    pub fn to_lsp(&self) -> Diagnostic {
        let start_pos = Position::new(
            (self.line.saturating_sub(1)) as u32,
            (self.column.saturating_sub(1)) as u32,
        );
        let end_pos = Position::new(
            (self.line.saturating_sub(1)) as u32,
            (self.column.saturating_sub(1) + 10) as u32,
        );

        let severity = if self.blocking {
            DiagnosticSeverity::ERROR
        } else {
            DiagnosticSeverity::WARNING
        };

        let mut message = format!(
            "{}\nForbidden Implication: {}\nRequired Correction: {}\nRequired Next Proof: {}",
            self.message,
            self.forbidden_implication,
            self.required_correction,
            self.required_next_proof
        );

        if let Some(oracle) = &self.oracle_class {
            message.push_str(&format!("\nOracle Class: {}", oracle));
        }
        if let Some(conf) = self.confidence {
            message.push_str(&format!("\nConfidence: {:.2}%", conf * 100.0));
        }

        Diagnostic {
            range: Range::new(start_pos, end_pos),
            severity: Some(severity),
            code: Some(lsp_max::lsp_types::NumberOrString::String(
                self.code.clone(),
            )),
            source: Some("anti-llm-cheat-lsp".to_string()),
            message,
            ..Default::default()
        }
    }
}
