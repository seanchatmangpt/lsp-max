//! Runtime utilities for tower-lsp-max servers.
//!
//! Provides SHA-256 hashing, the `ConformanceVector` (Admitted/Refused/Unknown
//! tallies), and the `MaxServer` wrapper that wires a `LanguageServer` impl into
//! the five-layer AMI execution model used by tower-lsp-max.

pub mod sha256;
pub mod typestate;
pub mod mesh_types;
pub mod mesh_hooks;
pub mod mesh;
pub mod ledger;
pub mod rpc;

pub use sha256::{sha256, validate_and_reconstruct_chain_checked};
pub use typestate::{
    DeterministicSnapshot, Law, AccessAdmissionLaw, Phase, Uninitialized,
    Initializing, Initialized, ShutDown, Exited, Data, EmptyData,
    InitializingData, InitializedData, Machine, ChainError, TypestateKernel
};
pub use mesh_types::{
    MeshAction, Hook, LspPhase, LspInstance, ConformanceGrade,
    AutonomicMeshState, ConformanceDeltaEntry, MaxMethod,
    HookEvent, InstanceId, MaxDiagnostic, PolicyState, Receipt
};
pub use mesh_hooks::{
    IntakeDiagnosticHook, IntakeClearHook, CustomerRequestClassifierHook,
    PolicyEvaluationHook, ReceiptRoutingHook
};
pub use mesh::{AutonomicMesh, MaxMesh, build_conformance_vector};

pub mod control_plane;
pub use control_plane::replay;
pub use control_plane::views;
