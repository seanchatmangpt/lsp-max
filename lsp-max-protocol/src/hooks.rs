use crate::{
    ConformanceVector, InstanceId, LawAxis, MaxDiagnostic, PolicyState, Receipt, RepairAction,
    SnapshotId,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct HookDescriptor {
    pub hook_id: String,
    pub name: String,
    pub description: String,
    pub axes: Vec<LawAxis>,
    pub trigger_law: LawAxis,
    pub input_type: String,
    pub output_type: String,
    pub failure_mode: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct HookGraphNode {
    pub node_id: String,
    pub hook: HookDescriptor,
    pub predecessors: Vec<String>,
    pub successors: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ChainDescriptor {
    pub chain_id: String,
    pub nodes: Vec<HookGraphNode>,
    pub law_axis: LawAxis,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PropagationResult {
    pub propagation_id: String,
    pub affected_nodes: Vec<String>,
    pub receipts: Vec<Receipt>,
    pub success: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AutonomicLoopStatus {
    pub loop_id: String,
    pub active: bool,
    pub iteration_count: u64,
    pub last_receipt: Option<Receipt>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManifoldSnapshot {
    pub snapshot_id: SnapshotId,
    pub conformance: ConformanceVector,
    pub hooks: Vec<HookDescriptor>,
    pub chains: Vec<ChainDescriptor>,
    pub receipts: Vec<Receipt>,
}

impl Default for ManifoldSnapshot {
    fn default() -> Self {
        Self {
            snapshot_id: SnapshotId(String::new()),
            conformance: ConformanceVector::default(),
            hooks: Vec::new(),
            chains: Vec::new(),
            receipts: Vec::new(),
        }
    }
}

/// Admission decision — must be Admitted, Refused, or Unknown. Never a bool.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub enum AdmissionDecision {
    Admitted,
    Refused,
    #[default]
    Unknown,
}

impl From<bool> for AdmissionDecision {
    fn from(b: bool) -> Self {
        if b {
            AdmissionDecision::Admitted
        } else {
            AdmissionDecision::Refused
        }
    }
}

impl From<AdmissionDecision> for bool {
    fn from(d: AdmissionDecision) -> bool {
        matches!(d, AdmissionDecision::Admitted)
    }
}

impl std::fmt::Display for AdmissionDecision {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AdmissionResult {
    pub decision: AdmissionDecision,
    pub law_axis: LawAxis,
    pub rationale: String,
    pub receipt: Option<Receipt>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RefusalResult {
    pub law_axis: LawAxis,
    pub rationale: String,
    pub receipt: Receipt,
    pub repair_actions: Vec<RepairAction>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct LawfulTransitionResult {
    pub from_phase: String,
    pub to_phase: String,
    pub lawful: bool,
    pub violated_laws: Vec<LawAxis>,
    pub receipt: Option<Receipt>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ReplayResult {
    pub replay_id: String,
    pub events_replayed: u64,
    pub conformance: ConformanceVector,
    pub receipts: Vec<Receipt>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ReleaseActuationResult {
    pub released: bool,
    pub conformance: ConformanceVector,
    pub blocking_axes: Vec<LawAxis>,
    pub receipt: Option<Receipt>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum HookEvent {
    StateTransition {
        instance_id: InstanceId,
        from_phase: String,
        to_phase: String,
    },
    DiagnosticEmitted {
        instance_id: InstanceId,
        diagnostic: Box<MaxDiagnostic>,
    },
    DiagnosticCleared {
        instance_id: InstanceId,
        diagnostic_id: String,
    },
    ReceiptEmitted {
        instance_id: InstanceId,
        receipt: Receipt,
    },
    PolicyStateChanged {
        instance_id: InstanceId,
        from_state: PolicyState,
        to_state: PolicyState,
    },
    BoundedActionExecuted {
        instance_id: InstanceId,
        action_id: String,
        description: String,
    },
    InstanceReset {
        instance_id: InstanceId,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn admission_decision_from_bool() {
        assert_eq!(AdmissionDecision::from(true), AdmissionDecision::Admitted);
        assert_eq!(AdmissionDecision::from(false), AdmissionDecision::Refused);
    }

    #[test]
    fn admission_decision_into_bool() {
        assert!(bool::from(AdmissionDecision::Admitted));
        assert!(!bool::from(AdmissionDecision::Refused));
        assert!(!bool::from(AdmissionDecision::Unknown));
    }
}
