use lsp_max::lsp_types::{Diagnostic, DiagnosticSeverity, Position, Range};
use serde::{Deserialize, Serialize};

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

        Diagnostic {
            range: Range::new(start_pos, end_pos),
            severity: Some(severity),
            code: Some(lsp_max::lsp_types::NumberOrString::String(
                self.code.clone(),
            )),
            source: Some("anti-llm-cheat-lsp".to_string()),
            message: format!(
                "{}\nForbidden Implication: {}\nRequired Correction: {}\nRequired Next Proof: {}",
                self.message,
                self.forbidden_implication,
                self.required_correction,
                self.required_next_proof
            ),
            ..Default::default()
        }
    }

    pub fn to_andon_event(&self) -> lsp_max::andon::andon::AndonEvent {
        lsp_max::andon::andon::AndonEvent {
            id: uuid::Uuid::new_v4().to_string(),
            severity: if self.blocking {
                lsp_max::andon::core::Severity::Stop
            } else {
                lsp_max::andon::core::Severity::Warning
            },
            code: self.code.clone(),
            title: self.category.clone(),
            message: self.message.clone(),
            invariant_id: Some(self.code.clone()),
            observed_state: None,
            expected_state: None,
            blocking: self.blocking,
            requires_ack: self.blocking,
            admission_allowed: !self.blocking,
            next_lawful_step: Some(self.required_correction.clone()),
            required_command: Some(self.required_next_proof.clone()),
            evidence_uri: None,
            virtual_doc_uri: Some("anti-llm://failset".to_string()),
            receipt_required: self.blocking,
        }
    }
}
