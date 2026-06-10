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
pub mod hooks;
pub mod policy;

// Re-export all types so they are visible at the crate root level exactly as before.

pub use conformance::{ConformanceGrade, ConformanceVector, LawAxis};

pub use diagnostics::{
    DocRoute, MaxCodeAction, MaxDiagnostic, Precondition, ReceiptPlan, RepairAction, Repairability,
    RollbackPlan, SnapshotId, Terminality, TransitionAttempt, ValidationPlan,
};

pub use hooks::{
    AdmissionDecision, AdmissionResult, AutonomicLoopStatus, ChainDescriptor, HookDescriptor,
    HookEvent, HookGraphNode, LawfulTransitionResult, ManifoldSnapshot, PropagationResult,
    RefusalResult, ReleaseActuationResult, ReplayResult,
};

pub use custom_methods::{
    METHOD_ADMISSION, METHOD_AUTONOMIC_LOOP, METHOD_CHAIN, METHOD_HOOK, METHOD_HOOK_GRAPH,
    METHOD_LAWFUL_TRANSITION, METHOD_LSIF_EXPORT, METHOD_MANIFOLD_SNAPSHOT, METHOD_PROPAGATE,
    METHOD_REFUSAL, METHOD_RELEASE_ACTUATION, METHOD_REPLAY,
};

pub use policy::PolicyState;

pub use core::{
    AnalysisBundle, CapabilityGap, GateId, InstanceId, LspStateModel, MaxCapabilityVector, Receipt,
    ReceiptObligation,
};

impl lsp_types_max::request::Request for lsp_3_18::FoldingRangeRefreshRequest {
    type Params = ();
    type Result = ();
    const METHOD: &'static str = "workspace/foldingRange/refresh";
}

impl lsp_types_max::request::Request for lsp_3_18::TextDocumentContentRefreshRequest {
    type Params = lsp_3_18::TextDocumentContentRefreshParams;
    type Result = ();
    const METHOD: &'static str = "workspace/textDocumentContent/refresh";
}
