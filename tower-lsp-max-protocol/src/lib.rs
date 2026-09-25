pub mod lsp_3_18;
pub use lsp_3_18 as generated_3_18;

use lsp_types::{ClientCapabilities, CodeAction, Diagnostic, ServerCapabilities};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MaxCapabilityVector {
    pub client: ClientCapabilities,
    pub server: ServerCapabilities,
    pub negotiated: serde_json::Value,
    pub experimental: serde_json::Value,
    pub gaps: Vec<CapabilityGap>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilityGap {
    pub capability_path: String,
    pub reason: String,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GateId(pub String);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReceiptObligation {
    pub required_receipts: Vec<String>,
}

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
}

impl MaxDiagnostic {
    /// Projects the `MaxDiagnostic` down into a standard `lsp_types::Diagnostic`.
    pub fn into_lsp(self) -> Diagnostic {
        // Here we could enrich the standard diagnostic message or data field
        // with the max capabilities before sending it to a standard client.
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConformanceVector {
    pub score: f64,
    pub strict_mode: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Receipt {
    pub receipt_id: String,
    pub hash: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisBundle {
    pub snapshot_id: SnapshotId,
    pub capability_vector: MaxCapabilityVector,
    pub diagnostics: Vec<MaxDiagnostic>,
    pub actions: Vec<MaxCodeAction>,
    pub conformance_vector: ConformanceVector,
    pub receipts: Vec<Receipt>,
}

pub mod custom_methods {
    use super::*;
    use lsp_types::request::Request;

    pub enum MaxSnapshot {}
    impl Request for MaxSnapshot {
        type Params = ();
        type Result = SnapshotId;
        const METHOD: &'static str = "max/snapshot";
    }

    pub enum MaxConformanceVector {}
    impl Request for MaxConformanceVector {
        type Params = SnapshotId;
        type Result = ConformanceVector;
        const METHOD: &'static str = "max/conformanceVector";
    }

    pub enum MaxExplainDiagnostic {}
    impl Request for MaxExplainDiagnostic {
        type Params = String; // diagnostic_id
        type Result = MaxDiagnostic;
        const METHOD: &'static str = "max/explainDiagnostic";
    }

    pub enum MaxRepairPlan {}
    impl Request for MaxRepairPlan {
        type Params = String; // diagnostic_id or law_id
        type Result = Vec<MaxCodeAction>;
        const METHOD: &'static str = "max/repairPlan";
    }

    pub enum MaxApplyRepairTransaction {}
    impl Request for MaxApplyRepairTransaction {
        type Params = MaxCodeAction;
        type Result = Receipt;
        const METHOD: &'static str = "max/applyRepairTransaction";
    }

    pub enum MaxExportAnalysisBundle {}
    impl Request for MaxExportAnalysisBundle {
        type Params = SnapshotId;
        type Result = AnalysisBundle;
        const METHOD: &'static str = "max/exportAnalysisBundle";
    }

    pub enum MaxRunGate {}
    impl Request for MaxRunGate {
        type Params = GateId;
        type Result = bool;
        const METHOD: &'static str = "max/runGate";
    }

    pub enum MaxClearDiagnostic {}
    impl Request for MaxClearDiagnostic {
        type Params = String; // diagnostic_id
        type Result = ();
        const METHOD: &'static str = "max/clearDiagnostic";
    }

    pub enum MaxReceipt {}
    impl Request for MaxReceipt {
        type Params = String; // receipt_id
        type Result = Receipt;
        const METHOD: &'static str = "max/receipt";
    }
}
