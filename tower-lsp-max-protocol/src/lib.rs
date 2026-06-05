//! Protocol vocabulary and conformance types for tower-lsp-max.
//!
//! Re-exports the generated LSP 3.18 meta-model (`lsp_3_18`), defines `LawAxis`
//! for typed law identifiers, and houses `ConformanceVector` and capability
//! negotiation helpers consumed by the runtime and CLI crates.

pub mod lsp_3_18;
pub use lsp_3_18 as generated_3_18;

use lsp_types::{ClientCapabilities, CodeAction, Diagnostic, ServerCapabilities};
use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// InstanceId — newtype for LSP instance identifiers
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct InstanceId(pub String);

impl From<String> for InstanceId {
    fn from(s: String) -> Self {
        InstanceId(s)
    }
}

impl From<&str> for InstanceId {
    fn from(s: &str) -> Self {
        InstanceId(s.to_string())
    }
}

impl PartialEq<str> for InstanceId {
    fn eq(&self, other: &str) -> bool {
        self.0 == other
    }
}
impl PartialEq<InstanceId> for str {
    fn eq(&self, other: &InstanceId) -> bool {
        self == other.0
    }
}


impl std::fmt::Display for InstanceId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

// ---------------------------------------------------------------------------
// LawAxis — replaces ad-hoc string law_ids
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum LawAxis {
    Protocol,
    Type,
    Fixture,
    Documentation,
    Release,
    Hook,
    Repair,
    Receipt,
    Security,
    Autopoiesis,
    Domain,
    Custom(String),
}

impl Default for LawAxis {
    fn default() -> Self {
        LawAxis::Custom(String::new())
    }
}

impl LawAxis {
    pub fn all_named() -> &'static [LawAxis] {
        &[
            LawAxis::Protocol,
            LawAxis::Type,
            LawAxis::Fixture,
            LawAxis::Documentation,
            LawAxis::Release,
            LawAxis::Hook,
            LawAxis::Repair,
            LawAxis::Receipt,
            LawAxis::Security,
            LawAxis::Autopoiesis,
            LawAxis::Domain,
        ]
    }
}

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

// ---------------------------------------------------------------------------
// Core types
// ---------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------
// ConformanceVector — doctrine-correct: Admitted/Refused/Unknown are distinct
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConformanceVector {
    /// Law axes that have been admitted (evidence present and valid)
    pub admitted: Vec<LawAxis>,
    /// Law axes that have been explicitly refused (evidence present, violation confirmed)
    pub refused: Vec<LawAxis>,
    /// Law axes where admissibility cannot be determined (NEVER collapsed into admitted or refused)
    pub unknown: Vec<LawAxis>,
    /// Derived score: 100 * admitted / (admitted + refused + unknown), None if all unknown
    pub score: Option<f64>,
    /// Whether unknown axes block release actuation
    pub strict_mode: bool,
}

impl ConformanceVector {
    pub fn all_admitted(&self) -> bool {
        self.refused.is_empty() && self.unknown.is_empty()
    }

    pub fn admits_release(&self) -> bool {
        self.refused.is_empty() && (!self.strict_mode || self.unknown.is_empty())
    }
}

impl Default for ConformanceVector {
    fn default() -> Self {
        Self {
            admitted: Vec::new(),
            refused: Vec::new(),
            unknown: Vec::new(),
            score: None,
            strict_mode: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Receipt {
    pub receipt_id: String,
    pub hash: String,
    /// Hash of the immediately preceding receipt in the instance ledger.
    /// `None` for genesis (first) receipts only.  All subsequent receipts
    /// must set this to close the Merkle chain and make `verify_instance_ledger`
    /// meaningful for non-LSP_1 instances.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prev_receipt_hash: Option<String>,
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

// ---------------------------------------------------------------------------
// New protocol response structs (11 stubs)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct HookDescriptor {
    pub hook_id: String,
    pub name: String,
    pub description: String,
    pub axes: Vec<LawAxis>,
    /// The primary LawAxis that triggers this hook.
    pub trigger_law: LawAxis,
    /// Fully-qualified type name of the hook's input payload.
    pub input_type: String,
    /// Fully-qualified type name of the hook's output payload.
    pub output_type: String,
    /// Describes the failure mode if the hook cannot execute (e.g. "Refused", "Unknown", "Panic").
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
        if b { AdmissionDecision::Admitted } else { AdmissionDecision::Refused }
    }
}

impl From<AdmissionDecision> for bool {
    fn from(d: AdmissionDecision) -> bool {
        matches!(d, AdmissionDecision::Admitted)
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

// ---------------------------------------------------------------------------
// Method name constants for the 11 new max/* methods
// ---------------------------------------------------------------------------

/// max/hook — Register or query hook descriptors. Returns all hooks if no id given.
pub const METHOD_HOOK: &str = "max/hook";

/// max/hookGraph — Return the directed hook dependency graph rooted at an optional node.
pub const METHOD_HOOK_GRAPH: &str = "max/hookGraph";

/// max/chain — Return chain descriptors. Returns all chains if no chain_id given.
pub const METHOD_CHAIN: &str = "max/chain";

/// max/propagate — Propagate a signal through a chain or from a hook. Returns affected nodes and receipts.
pub const METHOD_PROPAGATE: &str = "max/propagate";

/// max/autonomicLoop — Query or trigger an autonomic (self-regulating) loop. Returns loop status.
pub const METHOD_AUTONOMIC_LOOP: &str = "max/autonomicLoop";

/// max/manifoldSnapshot — Return a full manifold snapshot: conformance, hooks, chains, receipts.
pub const METHOD_MANIFOLD_SNAPSHOT: &str = "max/manifoldSnapshot";

/// max/lawfulTransition — Assert a phase transition is lawful against all active LawAxes. Returns violated laws.
pub const METHOD_LAWFUL_TRANSITION: &str = "max/lawfulTransition";

/// max/admission — Admissibility gate. Returns Admitted/Refused/Unknown. Never collapses Unknown.
pub const METHOD_ADMISSION: &str = "max/admission";

/// max/refusal — Explicit refusal gate. Records a refused LawAxis with rationale and receipt.
pub const METHOD_REFUSAL: &str = "max/refusal";

/// max/replay — Replay an event log against a snapshot and return conformance evidence.
pub const METHOD_REPLAY: &str = "max/replay";

/// max/releaseActuation — Actuate a release if and only if the ConformanceVector admits it. Strict mode blocks on Unknown.
pub const METHOD_RELEASE_ACTUATION: &str = "max/releaseActuation";

// ---------------------------------------------------------------------------
// Custom LSP methods
// ---------------------------------------------------------------------------

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

    // New doctrine methods

    pub enum MaxHook {}
    impl Request for MaxHook {
        type Params = Option<String>; // optional hook_id; None = list all
        type Result = Vec<HookDescriptor>;
        const METHOD: &'static str = "max/hook";
    }

    pub enum MaxHookGraph {}
    impl Request for MaxHookGraph {
        type Params = Option<String>; // optional root node_id
        type Result = Vec<HookGraphNode>;
        const METHOD: &'static str = "max/hookGraph";
    }

    pub enum MaxChain {}
    impl Request for MaxChain {
        type Params = Option<String>; // optional chain_id
        type Result = Vec<ChainDescriptor>;
        const METHOD: &'static str = "max/chain";
    }

    pub enum MaxPropagate {}
    impl Request for MaxPropagate {
        type Params = String; // chain_id or hook_id to propagate from
        type Result = PropagationResult;
        const METHOD: &'static str = "max/propagate";
    }

    pub enum MaxAutonomicLoop {}
    impl Request for MaxAutonomicLoop {
        type Params = Option<String>; // optional loop_id
        type Result = AutonomicLoopStatus;
        const METHOD: &'static str = "max/autonomicLoop";
    }

    pub enum MaxManifoldSnapshot {}
    impl Request for MaxManifoldSnapshot {
        type Params = SnapshotId;
        type Result = ManifoldSnapshot;
        const METHOD: &'static str = "max/manifoldSnapshot";
    }

    pub enum MaxLawfulTransition {}
    impl Request for MaxLawfulTransition {
        type Params = TransitionAttempt;
        type Result = LawfulTransitionResult;
        const METHOD: &'static str = "max/lawfulTransition";
    }

    pub enum MaxAdmission {}
    impl Request for MaxAdmission {
        type Params = LawAxis;
        type Result = AdmissionResult;
        const METHOD: &'static str = "max/admission";
    }

    pub enum MaxRefusal {}
    impl Request for MaxRefusal {
        type Params = LawAxis;
        type Result = RefusalResult;
        const METHOD: &'static str = "max/refusal";
    }

    pub enum MaxReplay {}
    impl Request for MaxReplay {
        type Params = SnapshotId;
        type Result = ReplayResult;
        const METHOD: &'static str = "max/replay";
    }

    pub enum MaxReleaseActuation {}
    impl Request for MaxReleaseActuation {
        type Params = SnapshotId;
        type Result = ReleaseActuationResult;
        const METHOD: &'static str = "max/releaseActuation";
    }
}

// ---------------------------------------------------------------------------
// PolicyState
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum PolicyState {
    Operational,
    ClarificationRequested,
    RefundAuthorized,
}

impl std::str::FromStr for PolicyState {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Operational" => Ok(Self::Operational),
            "ClarificationRequested" => Ok(Self::ClarificationRequested),
            "RefundAuthorized" => Ok(Self::RefundAuthorized),
            other => Err(format!("Unknown policy state: {other}")),
        }
    }
}

#[cfg(test)]
mod policy_state_tests {
    use super::PolicyState;
    #[test]
    fn test_policy_state_from_str_roundtrip() {
        assert_eq!("Operational".parse::<PolicyState>(), Ok(PolicyState::Operational));
        assert_eq!(
            "ClarificationRequested".parse::<PolicyState>(),
            Ok(PolicyState::ClarificationRequested)
        );
        assert_eq!(
            "RefundAuthorized".parse::<PolicyState>(),
            Ok(PolicyState::RefundAuthorized)
        );
        assert!("Bogus".parse::<PolicyState>().is_err());
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LspStateModel {
    pub instance_id: InstanceId,
    pub phase: String, // e.g. "Uninitialized", "Initializing", "Initialized", etc.
    pub diagnostics: Vec<MaxDiagnostic>,
    pub receipts: Vec<Receipt>,
    pub policy_state: Option<PolicyState>,
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
}
