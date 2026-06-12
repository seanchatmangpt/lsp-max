use crate::*;
use lsp_types_max::request::Request;

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

/// max/lsif — Stream the current registry state as an exhaustive LSIF NDJSON graph.
pub const METHOD_LSIF_EXPORT: &str = "max/lsif";

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

// ── Rule-pack protocol methods (ERRC Innovation 2 + 4) ────────────────────

/// max/rulePacks — list all active rule packs with their metadata and
/// dependency graph.
pub const METHOD_RULE_PACKS: &str = "max/rulePacks";

/// max/rulePackStatus — return the conformance status contributed by a single
/// rule pack: which of its rules fired, on which files, and the resulting
/// per-axis verdict.
pub const METHOD_RULE_PACK_STATUS: &str = "max/rulePackStatus";

/// max/rulePackDiff — compare two snapshots (by seq number) and return which
/// rule-pack findings were added, removed, or unchanged.
pub const METHOD_RULE_PACK_DIFF: &str = "max/rulePackDiff";

/// max/workspaceConformance — return the workspace-level ConformanceVector:
/// the aggregate of all per-file vectors across all open documents.
/// Refused axes propagate from any file; axes with no coverage remain Unknown.
pub const METHOD_WORKSPACE_CONFORMANCE: &str = "max/workspaceConformance";

/// Rule-pack descriptor returned by `max/rulePacks`.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RulePackDescriptor {
    /// Pack identifier.
    pub id: String,
    /// Pack semantic version.
    pub version: String,
    /// Rule IDs contained in this pack.
    pub rule_ids: Vec<String>,
    /// Packs this pack depends on.
    pub depends_on: Vec<String>,
    /// Number of rules that fired in the last workspace scan.
    pub active_rule_count: usize,
}

/// Per-pack status returned by `max/rulePackStatus`.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RulePackStatusResult {
    /// Pack identifier.
    pub pack_id: String,
    /// Rules that fired, grouped by file URI.
    pub findings_by_uri: std::collections::HashMap<String, Vec<String>>,
    /// The ConformanceVector contribution from this pack alone.
    pub conformance: ConformanceVector,
}

/// Diff entry returned by `max/rulePackDiff`.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RulePackDiffEntry {
    /// Rule ID of the changed finding.
    pub rule_id: String,
    /// File URI.
    pub uri: String,
    /// Line number (0-based).
    pub line: u32,
    /// `"added"`, `"removed"`, or `"unchanged"`.
    pub change: String,
}

pub enum MaxRulePacks {}
impl Request for MaxRulePacks {
    type Params = ();
    type Result = Vec<RulePackDescriptor>;
    const METHOD: &'static str = "max/rulePacks";
}

pub enum MaxRulePackStatus {}
impl Request for MaxRulePackStatus {
    type Params = String; // pack_id
    type Result = RulePackStatusResult;
    const METHOD: &'static str = "max/rulePackStatus";
}

pub enum MaxRulePackDiff {}
impl Request for MaxRulePackDiff {
    type Params = (u64, u64); // (seq_before, seq_after)
    type Result = Vec<RulePackDiffEntry>;
    const METHOD: &'static str = "max/rulePackDiff";
}

pub enum MaxWorkspaceConformance {}
impl Request for MaxWorkspaceConformance {
    type Params = ();
    type Result = ConformanceVector;
    const METHOD: &'static str = "max/workspaceConformance";
}
