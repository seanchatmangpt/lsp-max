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
    AutonomicMeshState, ConformanceDeltaEntry, ConformanceGrade, FailureMode, Hook, HookDescriptor,
    HookEvent, InstanceId, LspInstance, LspPhase, MaxDiagnostic, MaxMethod, MeshAction,
    PolicyState, Receipt,
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

#[cfg(test)]
mod hook_descriptor_tests {
    use super::*;

    #[test]
    fn all_hooks_have_non_empty_descriptors() {
        let mut mesh = AutonomicMesh::new();
        mesh.register_hook(Box::new(mesh_hooks::IntakeDiagnosticHook));
        mesh.register_hook(Box::new(mesh_hooks::IntakeClearHook));
        mesh.register_hook(Box::new(mesh_hooks::CustomerRequestClassifierHook::new()));
        mesh.register_hook(Box::new(mesh_hooks::PolicyEvaluationHook::new()));
        mesh.register_hook(Box::new(mesh_hooks::ReceiptRoutingHook::new()));

        let descriptors = mesh.hook_descriptors();
        assert_eq!(descriptors.len(), 5, "expected exactly 5 registered hooks");

        for d in &descriptors {
            assert!(!d.name.is_empty(), "hook name must not be empty");
            assert!(
                !d.input_type.is_empty(),
                "input_type must not be empty for {}",
                d.name
            );
            assert!(
                !d.output_type.is_empty(),
                "output_type must not be empty for {}",
                d.name
            );
            assert!(
                !d.trigger_law.is_empty(),
                "trigger_law must not be empty for {}",
                d.name
            );
        }
    }

    #[test]
    fn hook_names_match_struct_names() {
        let hooks: Vec<Box<dyn mesh_types::Hook>> = vec![
            Box::new(mesh_hooks::IntakeDiagnosticHook),
            Box::new(mesh_hooks::IntakeClearHook),
            Box::new(mesh_hooks::CustomerRequestClassifierHook::new()),
            Box::new(mesh_hooks::PolicyEvaluationHook::new()),
            Box::new(mesh_hooks::ReceiptRoutingHook::new()),
        ];
        let expected_names = [
            "IntakeDiagnosticHook",
            "IntakeClearHook",
            "CustomerRequestClassifierHook",
            "PolicyEvaluationHook",
            "ReceiptRoutingHook",
        ];
        for (hook, expected) in hooks.iter().zip(expected_names.iter()) {
            assert_eq!(hook.name(), *expected);
            assert_eq!(hook.descriptor().name, *expected);
        }
    }
}
