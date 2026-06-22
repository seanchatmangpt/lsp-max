//! Protocol vocabulary and conformance types for lsp-max.
//!
//! Re-exports the generated LSP 3.18 meta-model (`lsp_3_18`), defines `LawAxis`
//! for typed law identifiers, and houses `ConformanceVector` and capability
//! negotiation helpers consumed by the runtime and CLI crates.

pub mod lsp_3_18;
pub use lsp_3_18 as generated_3_18;

pub mod conformance;
pub mod core;
pub mod custom_methods;
pub mod diagnostics;
pub mod explain;
pub mod hooks;
pub mod phase;
pub mod pipeline;
pub mod policy;
pub mod repair;
pub mod intent;
pub mod stream;

// Re-export all types so they are visible at the crate root level exactly as before.

pub use conformance::{ConformanceGrade, ConformanceVector, LawAxis};

pub use diagnostics::{
    DocRoute, MaxCodeAction, MaxDiagnostic, Precondition, ReceiptPlan, RepairAction, Repairability,
    RollbackPlan, SnapshotId, Terminality, TransitionAttempt, ValidationPlan,
};

pub use explain::{
    explain_code, explain_method_status, ExplainDiagnosticParams, ExplainDiagnosticResult,
    ExplainPosition, ExplainReceiptParams, ExplainReceiptResult, ExplainStatusParams,
    ExplainStatusResult, LawAxisTrace, EXPLAIN_DIAGNOSTIC, EXPLAIN_RECEIPT, EXPLAIN_STATUS,
};

pub use hooks::{
    AdmissionDecision, AdmissionResult, AutonomicLoopStatus, ChainDescriptor, HookDescriptor,
    HookEvent, HookGraphNode, LawfulTransitionResult, ManifoldSnapshot, PropagationResult,
    RefusalResult, ReleaseActuationResult, ReplayResult,
};

pub use custom_methods::{
    MaxRulePackDiff,
    MaxRulePackStatus,
    MaxRulePacks,
    MaxWorkspaceConformance,
    RulePackDescriptor,
    RulePackDiffEntry,
    RulePackStatusResult,
    METHOD_ADMISSION,
    METHOD_AUTONOMIC_LOOP,
    METHOD_CHAIN,
    METHOD_HOOK,
    METHOD_HOOK_GRAPH,
    METHOD_LAWFUL_TRANSITION,
    METHOD_LSIF_EXPORT,
    METHOD_MANIFOLD_SNAPSHOT,
    METHOD_PROPAGATE,
    METHOD_REFUSAL,
    METHOD_RELEASE_ACTUATION,
    METHOD_REPLAY,
    // Rule-pack protocol methods
    METHOD_RULE_PACKS,
    METHOD_RULE_PACK_DIFF,
    METHOD_RULE_PACK_STATUS,
    METHOD_WORKSPACE_CONFORMANCE,
};

pub use policy::PolicyState;

pub use intent::{
    IntentDeclareParams, IntentDeclareResult, IntentKind, IntentListParams, IntentListResult,
    IntentOutcome, IntentRegistry, IntentRevokeParams, IntentRevokeResult, IntentSummary,
    IntentValidateParams, IntentValidateResult, INTENT_DECLARE, INTENT_LIST, INTENT_REVOKE,
    INTENT_VALIDATE,
};

pub use core::{
    AnalysisBundle, CapabilityGap, GateId, InstanceId, LspStateModel, MaxCapabilityVector, Receipt,
    ReceiptObligation,
};

pub use stream::{
    StreamBus, StreamEvent, StreamEventKind, StreamSubscribeParams, StreamSubscribeResult,
    StreamUnsubscribeParams, StreamUnsubscribeResult, STREAM_EVENT, STREAM_SUBSCRIBE,
    STREAM_UNSUBSCRIBE,
};

// LspRequest is implemented for FoldingRangeRefreshRequest and
// TextDocumentContentRefreshRequest directly in lsp_3_18.rs.
// The impls are available via `lsp_3_18::LspRequest`.
