use crate::{GateId, LawAxis, ReceiptObligation};
use lsp_types_max::{CodeAction, Diagnostic};
use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Repairability / Terminality
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub enum Repairability {
    Repairable,
    NotRepairable,
    #[default]
    Unknown,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub enum Terminality {
    Terminal,
    #[default]
    NonTerminal,
}

impl std::fmt::Display for Terminality {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransitionAttempt {
    pub from_state: String,
    pub to_state: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocRoute {
    pub path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepairAction {
    pub action_id: String,
    pub description: String,
}

// ---------------------------------------------------------------------------
// MaxDiagnostic — extended with doctrine fields
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MaxDiagnostic {
    pub lsp: Diagnostic,
    pub diagnostic_id: String,
    pub law_id: String,
    pub attempted_transition: Option<TransitionAttempt>,
    pub violated_axes: Vec<String>,
    pub doc_routes: Vec<DocRoute>,
    pub repair_actions: Vec<RepairAction>,
    pub verification_gates: Vec<GateId>,
    pub receipt_obligation: Option<ReceiptObligation>,

    // Doctrine extensions (serde(default) preserves backward compatibility)
    #[serde(default)]
    pub law_axis: LawAxis,
    #[serde(default)]
    pub violated_invariant: String,
    #[serde(default)]
    pub observed_state: serde_json::Value,
    #[serde(default)]
    pub expected_state: serde_json::Value,
    #[serde(default)]
    pub repairability: Repairability,
    #[serde(default)]
    pub terminality: Terminality,
}

impl Default for MaxDiagnostic {
    fn default() -> Self {
        Self {
            lsp: Diagnostic::default(),
            diagnostic_id: String::new(),
            law_id: String::new(),
            attempted_transition: None,
            violated_axes: Vec::new(),
            doc_routes: Vec::new(),
            repair_actions: Vec::new(),
            verification_gates: Vec::new(),
            receipt_obligation: None,
            law_axis: LawAxis::default(),
            violated_invariant: String::new(),
            observed_state: serde_json::Value::Null,
            expected_state: serde_json::Value::Null,
            repairability: Repairability::default(),
            terminality: Terminality::default(),
        }
    }
}

impl MaxDiagnostic {
    /// Projects the `MaxDiagnostic` down into a standard `lsp_types_max::Diagnostic`.
    pub fn into_lsp(self) -> Diagnostic {
        let mut d = self.lsp.clone();
        if d.data.is_none() {
            if let Ok(data) = serde_json::to_value(self) {
                d.data = Some(data);
            }
        }
        d
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Precondition {
    pub condition: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationPlan {
    pub gates: Vec<GateId>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RollbackPlan {
    pub strategy: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReceiptPlan {
    pub expected_receipts: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MaxCodeAction {
    pub action: CodeAction,
    pub preconditions: Vec<Precondition>,
    pub validation_plan: ValidationPlan,
    pub rollback_plan: RollbackPlan,
    pub receipt_plan: ReceiptPlan,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotId(pub String);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn max_diagnostic_default_is_valid() {
        let d = MaxDiagnostic::default();
        assert!(d.diagnostic_id.is_empty());
        assert!(d.violated_axes.is_empty());
        assert!(d.repair_actions.is_empty());
    }

    #[test]
    fn max_diagnostic_into_lsp_preserves_data() {
        let d = MaxDiagnostic {
            diagnostic_id: "diag-1".to_string(),
            law_id: "LSP-001".to_string(),
            ..MaxDiagnostic::default()
        };
        let lsp = d.into_lsp();
        assert!(lsp.data.is_some());
    }
}
