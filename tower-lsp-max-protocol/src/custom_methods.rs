use crate::*;
use lsp_types::request::Request;

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

// Request implementations

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
