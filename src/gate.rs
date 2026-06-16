//! Validation gate logic for verifying server authorization, state checks, and compliance keys.

// ---------------------------------------------------------------------------
// Composable gate algebra (BreedGate pattern from dteam)
// ---------------------------------------------------------------------------

/// A named, composable law gate — a single intuitionistic proof obligation.
///
/// Each `LawGate` is a predicate over the server registry. `accept_gates`
/// is their conjunction: a proof-tree root that is `true` iff every obligation
/// is satisfied. Function-pointer representation makes gates stack-allocable,
/// inspectable by name, and zero-heap.
#[derive(Clone, Copy)]
pub struct LawGate {
    /// Human-readable name for this gate obligation (stable identifier).
    pub name: &'static str,
    /// The predicate to evaluate against the current server registry.
    pub check: fn(&crate::ServerRegistry) -> bool,
}

impl std::fmt::Debug for LawGate {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LawGate").field("name", &self.name).finish()
    }
}

impl LawGate {
    /// Evaluate this gate against `registry`.
    pub fn eval(&self, registry: &crate::ServerRegistry) -> bool {
        (self.check)(registry)
    }
}

/// Returns `true` iff every gate in `gates` passes against `registry`.
///
/// Short-circuits on the first failing gate. The name of the failing gate
/// can be found by iterating manually when diagnostics are needed.
pub fn accept_gates(registry: &crate::ServerRegistry, gates: &[LawGate]) -> bool {
    gates.iter().all(|g| g.eval(registry))
}

/// Default gate pack for law-state transition validation.
///
/// These are the baseline proof obligations every server must satisfy before
/// a law-state transition is admitted. Additional gates can be composed by
/// concatenating slices.
pub const DEFAULT_GATES: &[LawGate] = &[
    LawGate {
        name: "not-uninitialized",
        check: |r| r.current_state != crate::service::State::Uninitialized,
    },
    LawGate {
        name: "receipt-present",
        check: |r| !r.receipts.is_empty(),
    },
];

// ---------------------------------------------------------------------------
// Legacy per-ID gate dispatch
// ---------------------------------------------------------------------------

/// Evaluates a security gate check given its unique identifier, the current server state,
/// and the workspace root path.
///
/// Returns `true` if the gate permits transition, `false` otherwise.
pub fn run_gate_logic(
    gate_id: &str,
    current_state: crate::service::State,
    root_path: std::path::PathBuf,
) -> bool {
    match gate_id {
        "some-gate" => true,
        "gate-state-check" => current_state != crate::service::State::Uninitialized,
        "gate-receipt-check" | "gate-auth-check" => {
            let spec = crate::diagnostics::law_table::law_table()
                .iter()
                .find(|s| s.gate_id == gate_id);
            let Some(spec) = spec else { return false };
            let path = root_path.join(spec.receipt_file);
            if path.exists() {
                std::fs::read_to_string(&path)
                    .map(|c| c.trim() == spec.receipt_token)
                    .unwrap_or(false)
            } else {
                false
            }
        }
        "gate-powl-conformance" => {
            // POWL conformance gate — admitted when a receipt exists for the declared model.
            // Full wasm4pm integration routes through control_plane::powl_conformance.
            let spec = crate::diagnostics::law_table::law_table()
                .iter()
                .find(|s| s.gate_id == gate_id);
            match spec {
                Some(spec) => root_path.join(spec.receipt_file).exists(),
                None => false, // No law table entry — UNKNOWN, conservative refusal
            }
        }
        _ => {
            let output = std::process::Command::new("cargo")
                .arg("check")
                .current_dir(root_path)
                .output();
            match output {
                Ok(out) => {
                    if !out.status.success() {
                        eprintln!("cargo check failed!");
                        eprintln!("stdout: {}", String::from_utf8_lossy(&out.stdout));
                        eprintln!("stderr: {}", String::from_utf8_lossy(&out.stderr));
                    }
                    out.status.success()
                }
                Err(e) => {
                    eprintln!("failed to execute cargo check: {:?}", e);
                    false
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::service::State;
    use std::collections::HashMap;

    fn minimal_registry(state: State) -> crate::ServerRegistry {
        crate::ServerRegistry {
            client_capabilities: None,
            server_capabilities: None,
            diagnostics: HashMap::new(),
            repair_plans: HashMap::new(),
            gates: HashMap::new(),
            receipts: HashMap::new(),
            snapshots: HashMap::new(),
            cleared_diagnostics: std::collections::HashSet::new(),
            current_state: state,
            document_versions: HashMap::new(),
            root_path: std::path::PathBuf::from("/tmp"),
            action_seq: 0,
            conformance_delta_log: std::collections::VecDeque::new(),
        }
    }

    #[test]
    fn law_gate_eval_returns_true_when_predicate_passes() {
        let gate = LawGate { name: "always-true", check: |_| true };
        let reg = minimal_registry(State::Initialized);
        assert!(gate.eval(&reg));
    }

    #[test]
    fn law_gate_eval_returns_false_when_predicate_fails() {
        let gate = LawGate { name: "always-false", check: |_| false };
        let reg = minimal_registry(State::Initialized);
        assert!(!gate.eval(&reg));
    }

    #[test]
    fn law_gate_debug_shows_name() {
        let gate = LawGate { name: "my-gate", check: |_| true };
        assert!(format!("{gate:?}").contains("my-gate"));
    }

    #[test]
    fn accept_gates_empty_slice_always_passes() {
        let reg = minimal_registry(State::Uninitialized);
        assert!(accept_gates(&reg, &[]));
    }

    #[test]
    fn accept_gates_all_pass() {
        let gates = [
            LawGate { name: "g1", check: |_| true },
            LawGate { name: "g2", check: |_| true },
        ];
        let reg = minimal_registry(State::Initialized);
        assert!(accept_gates(&reg, &gates));
    }

    #[test]
    fn accept_gates_fails_on_first_false_gate() {
        let gates = [
            LawGate { name: "fail", check: |_| false },
            LawGate { name: "pass", check: |_| true },
        ];
        let reg = minimal_registry(State::Initialized);
        assert!(!accept_gates(&reg, &gates));
    }

    #[test]
    fn default_gates_block_uninitialized_state() {
        let reg = minimal_registry(State::Uninitialized);
        assert!(!accept_gates(&reg, DEFAULT_GATES));
    }

    #[test]
    fn run_gate_logic_some_gate_always_returns_true() {
        let result = run_gate_logic(
            "some-gate",
            State::Uninitialized,
            std::path::PathBuf::from("/tmp"),
        );
        assert!(result);
    }

    #[test]
    fn run_gate_logic_state_check_passes_when_initialized() {
        let result = run_gate_logic(
            "gate-state-check",
            State::Initialized,
            std::path::PathBuf::from("/tmp"),
        );
        assert!(result);
    }

    #[test]
    fn run_gate_logic_state_check_fails_when_uninitialized() {
        let result = run_gate_logic(
            "gate-state-check",
            State::Uninitialized,
            std::path::PathBuf::from("/tmp"),
        );
        assert!(!result);
    }
}
