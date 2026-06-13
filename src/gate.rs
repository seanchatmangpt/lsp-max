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
