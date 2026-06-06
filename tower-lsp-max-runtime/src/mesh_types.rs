pub use tower_lsp_max_protocol::{HookEvent, InstanceId, MaxDiagnostic, PolicyState, Receipt};

#[derive(Debug, Clone)]
pub enum MeshAction {
    TransitionPolicyState {
        instance_id: InstanceId,
        new_state: PolicyState,
    },
    ClearDiagnostic {
        instance_id: InstanceId,
        diagnostic_id: String,
    },
    AddDiagnostic {
        instance_id: InstanceId,
        diagnostic: Box<MaxDiagnostic>,
    },
    EmitReceipt {
        instance_id: InstanceId,
        receipt: Receipt,
    },
    ExecuteBoundedAction {
        instance_id: InstanceId,
        action_id: String,
        description: String,
    },
    ResetInstance {
        instance_id: InstanceId,
    },
}

impl std::fmt::Display for MeshAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MeshAction::TransitionPolicyState {
                instance_id,
                new_state,
            } => {
                write!(f, "TransitionPolicyState({}, {:?})", instance_id, new_state)
            }
            MeshAction::ClearDiagnostic {
                instance_id,
                diagnostic_id,
            } => {
                write!(f, "ClearDiagnostic({}, {})", instance_id, diagnostic_id)
            }
            MeshAction::AddDiagnostic { instance_id, .. } => {
                write!(f, "AddDiagnostic({})", instance_id)
            }
            MeshAction::EmitReceipt {
                instance_id,
                receipt,
            } => {
                write!(f, "EmitReceipt({}, {})", instance_id, receipt.receipt_id)
            }
            MeshAction::ExecuteBoundedAction {
                instance_id,
                action_id,
                ..
            } => {
                write!(f, "ExecuteBoundedAction({}, {})", instance_id, action_id)
            }
            MeshAction::ResetInstance { instance_id } => {
                write!(f, "ResetInstance({})", instance_id)
            }
        }
    }
}

pub trait Hook: Send + Sync {
    fn name(&self) -> &str;
    fn trigger(&self, event: &HookEvent) -> Vec<MeshAction>;
}

/// Lifecycle phase of an LSP instance.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum LspPhase {
    Uninitialized,
    Initializing,
    Initialized,
    ShutDown,
    Exited,
}

impl std::fmt::Display for LspPhase {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LspPhase::Uninitialized => write!(f, "Uninitialized"),
            LspPhase::Initializing => write!(f, "Initializing"),
            LspPhase::Initialized => write!(f, "Initialized"),
            LspPhase::ShutDown => write!(f, "ShutDown"),
            LspPhase::Exited => write!(f, "Exited"),
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct LspInstance {
    pub id: String,
    pub phase: LspPhase,
    pub diagnostics: Vec<MaxDiagnostic>,
    pub receipts: Vec<Receipt>,
    pub policy_state: Option<PolicyState>,
    #[serde(skip)]
    cached_score: std::cell::Cell<Option<f64>>,
}

impl LspInstance {
    pub fn new(id: &str) -> Self {
        Self {
            id: id.to_string(),
            phase: LspPhase::Uninitialized,
            diagnostics: Vec::new(),
            receipts: Vec::new(),
            policy_state: None,
            cached_score: std::cell::Cell::new(None),
        }
    }

    #[inline]
    pub fn invalidate_score_cache(&mut self) {
        self.cached_score.set(None);
    }

    pub fn add_diagnostic(&mut self, diag: MaxDiagnostic) {
        self.diagnostics.push(diag);
        self.invalidate_score_cache();
    }

    pub fn remove_diagnostic(&mut self, diagnostic_id: &str) -> usize {
        let before = self.diagnostics.len();
        self.diagnostics
            .retain(|d| d.diagnostic_id != diagnostic_id);
        let removed = before - self.diagnostics.len();
        if removed > 0 {
            self.invalidate_score_cache();
        }
        removed
    }

    pub fn conformance_score(&self) -> f64 {
        if let Some(score) = self.cached_score.get() {
            return score;
        }
        let mut penalty: f64 = 0.0;
        for diag in &self.diagnostics {
            let p = match diag.lsp.severity {
                Some(lsp_types_max::DiagnosticSeverity::ERROR) => 30.0,
                Some(lsp_types_max::DiagnosticSeverity::WARNING) => 20.0,
                Some(lsp_types_max::DiagnosticSeverity::INFORMATION) => 10.0,
                Some(lsp_types_max::DiagnosticSeverity::HINT) => 5.0,
                Some(_) => 30.0,
                None => 30.0,
            };
            penalty += p;
        }
        let score = f64::max(100.0 - penalty, 0.0);
        self.cached_score.set(Some(score));
        score
    }

    pub fn conformance_grade(&self) -> ConformanceGrade {
        ConformanceGrade::from_score(self.conformance_score())
    }
}

impl Default for LspInstance {
    fn default() -> Self {
        Self {
            id: String::new(),
            phase: LspPhase::Uninitialized,
            diagnostics: Vec::new(),
            receipts: Vec::new(),
            policy_state: None,
            cached_score: std::cell::Cell::new(None),
        }
    }
}

/// Coarse quality bucket derived from a conformance score.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum ConformanceGrade {
    Perfect,
    Good,
    Degraded,
    Critical,
}

impl ConformanceGrade {
    pub fn from_score(score: f64) -> Self {
        if score >= 100.0 {
            ConformanceGrade::Perfect
        } else if score >= 75.0 {
            ConformanceGrade::Good
        } else if score >= 50.0 {
            ConformanceGrade::Degraded
        } else {
            ConformanceGrade::Critical
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            ConformanceGrade::Perfect => "perfect",
            ConformanceGrade::Good => "good",
            ConformanceGrade::Degraded => "degraded",
            ConformanceGrade::Critical => "critical",
        }
    }
}

impl std::fmt::Display for ConformanceGrade {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct AutonomicMeshState {
    pub instances: std::collections::HashMap<String, LspInstance>,
    pub event_log: Vec<HookEvent>,
    pub executed_bounded_actions: Vec<String>,
    #[serde(flatten)]
    pub extra: std::collections::HashMap<String, serde_json::Value>,
}

impl std::fmt::Display for AutonomicMeshState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "AutonomicMeshState {{ instances: {}, event_log: {} }}",
            self.instances.len(),
            self.event_log.len()
        )
    }
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct ConformanceDeltaEntry {
    pub seq: u64,
    pub instance_id: String,
    pub old_score: f64,
    pub new_score: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MaxMethod {
    Snapshot,
    ConformanceVector,
    ClearDiagnostic,
    ExplainDiagnostic,
    RepairPlan,
    ApplyRepairTransaction,
    ExportAnalysisBundle,
    RunGate,
    Receipt,
    Hook,
    HookGraph,
    Chain,
    Propagate,
    AutonomicLoop,
    ManifoldSnapshot,
    LawfulTransition,
    Admission,
    Refusal,
    Replay,
    ReleaseActuation,
    InstanceList,
    DumpState,
    RestoreState,
    Reset,
    ConformanceDelta,
    VerifyLedger,
    LedgerReport,
}

impl MaxMethod {
    pub fn as_str(self) -> &'static str {
        match self {
            MaxMethod::Snapshot => "max/snapshot",
            MaxMethod::ConformanceVector => "max/conformanceVector",
            MaxMethod::ClearDiagnostic => "max/clearDiagnostic",
            MaxMethod::ExplainDiagnostic => "max/explainDiagnostic",
            MaxMethod::RepairPlan => "max/repairPlan",
            MaxMethod::ApplyRepairTransaction => "max/applyRepairTransaction",
            MaxMethod::ExportAnalysisBundle => "max/exportAnalysisBundle",
            MaxMethod::RunGate => "max/runGate",
            MaxMethod::Receipt => "max/receipt",
            MaxMethod::Hook => "max/hook",
            MaxMethod::HookGraph => "max/hookGraph",
            MaxMethod::Chain => "max/chain",
            MaxMethod::Propagate => "max/propagate",
            MaxMethod::AutonomicLoop => "max/autonomicLoop",
            MaxMethod::ManifoldSnapshot => "max/manifoldSnapshot",
            MaxMethod::LawfulTransition => "max/lawfulTransition",
            MaxMethod::Admission => "max/admission",
            MaxMethod::Refusal => "max/refusal",
            MaxMethod::Replay => "max/replay",
            MaxMethod::ReleaseActuation => "max/releaseActuation",
            MaxMethod::InstanceList => "max/instanceList",
            MaxMethod::DumpState => "max/dumpState",
            MaxMethod::RestoreState => "max/restoreState",
            MaxMethod::Reset => "max/reset",
            MaxMethod::ConformanceDelta => "max/conformanceDelta",
            MaxMethod::VerifyLedger => "max/verifyLedger",
            MaxMethod::LedgerReport => "max/ledgerReport",
        }
    }
}

impl<'a> TryFrom<&'a str> for MaxMethod {
    type Error = ();

    fn try_from(s: &'a str) -> Result<Self, Self::Error> {
        match s {
            "max/snapshot" => Ok(MaxMethod::Snapshot),
            "max/conformanceVector" => Ok(MaxMethod::ConformanceVector),
            "max/clearDiagnostic" => Ok(MaxMethod::ClearDiagnostic),
            "max/explainDiagnostic" => Ok(MaxMethod::ExplainDiagnostic),
            "max/repairPlan" => Ok(MaxMethod::RepairPlan),
            "max/applyRepairTransaction" => Ok(MaxMethod::ApplyRepairTransaction),
            "max/exportAnalysisBundle" => Ok(MaxMethod::ExportAnalysisBundle),
            "max/runGate" => Ok(MaxMethod::RunGate),
            "max/receipt" => Ok(MaxMethod::Receipt),
            "max/hook" => Ok(MaxMethod::Hook),
            "max/hookGraph" => Ok(MaxMethod::HookGraph),
            "max/chain" => Ok(MaxMethod::Chain),
            "max/propagate" => Ok(MaxMethod::Propagate),
            "max/autonomicLoop" => Ok(MaxMethod::AutonomicLoop),
            "max/manifoldSnapshot" => Ok(MaxMethod::ManifoldSnapshot),
            "max/lawfulTransition" => Ok(MaxMethod::LawfulTransition),
            "max/admission" => Ok(MaxMethod::Admission),
            "max/refusal" => Ok(MaxMethod::Refusal),
            "max/replay" => Ok(MaxMethod::Replay),
            "max/releaseActuation" => Ok(MaxMethod::ReleaseActuation),
            "max/instanceList" => Ok(MaxMethod::InstanceList),
            "max/dumpState" => Ok(MaxMethod::DumpState),
            "max/restoreState" => Ok(MaxMethod::RestoreState),
            "max/reset" => Ok(MaxMethod::Reset),
            "max/conformanceDelta" => Ok(MaxMethod::ConformanceDelta),
            "max/verifyLedger" => Ok(MaxMethod::VerifyLedger),
            "max/ledgerReport" => Ok(MaxMethod::LedgerReport),
            _ => Err(()),
        }
    }
}
