//! Runtime utilities for tower-lsp-max servers.
//!
//! Provides SHA-256 hashing, the `ConformanceVector` (Admitted/Refused/Unknown
//! tallies), and the `MaxServer` wrapper that wires a `LanguageServer` impl into
//! the five-layer AMI execution model used by tower-lsp-max.

pub mod ledger;
pub mod mesh;
pub mod mesh_hooks;
pub mod mesh_types;
pub mod rpc;
pub mod sha256;
pub mod typestate;

pub use mesh::{build_conformance_vector, AutonomicMesh, MaxMesh};
pub use mesh_hooks::{
    CustomerRequestClassifierHook, IntakeClearHook, IntakeDiagnosticHook, PolicyEvaluationHook,
    ReceiptRoutingHook,
};
pub use mesh_types::{
    AutonomicMeshState, ConformanceDeltaEntry, ConformanceGrade, Hook, HookEvent, InstanceId,
    LspInstance, LspPhase, MaxDiagnostic, MaxMethod, MeshAction, PolicyState, Receipt,
};
pub use sha256::{sha256, validate_and_reconstruct_chain_checked};
pub use typestate::{
    AccessAdmissionLaw, ChainError, Data, DeterministicSnapshot, EmptyData, Exited, Initialized,
    InitializedData, Initializing, InitializingData, Law, Machine, Phase, ShutDown,
    TypestateKernel, Uninitialized,
};

pub mod control_plane;
pub use control_plane::replay;
pub use control_plane::views;
