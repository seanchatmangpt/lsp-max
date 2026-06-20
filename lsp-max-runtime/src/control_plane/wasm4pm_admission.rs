//! Admitters that bridge lsp-max's relation graph to wasm4pm witness types.
//!
//! The wasm4pm_compat modules `admission`, `evidence`, `ocel`, `receipt`,
//! `state`, and `witness` are absent from the current stub build.  Both
//! admitter structs are preserved as public types; their `admit` bodies
//! panic with a BLOCKED status so the gap is visible at call time.

use crate::control_plane::admission::AdmittedRelationGraph;
use crate::control_plane::receipts::to_hex;

/// Admitter that bridges lsp-max's AdmittedRelationGraph to
/// Wasm4pm's Ocel20 witness validation and formatting laws.
///
/// BLOCKED: wasm4pm_compat not available in stub build.
pub struct Ocel20GraphAdmitter;

impl Ocel20GraphAdmitter {
    /// BLOCKED: wasm4pm_compat unavailable in stub build.
    pub fn admit(_raw: AdmittedRelationGraph) -> ! {
        panic!("BLOCKED: wasm4pm_compat::admission/ocel unavailable in stub build — Ocel20GraphAdmitter::admit not reachable");
    }
}

/// Admitter that bridges lsp-max's AdmittedRelationGraph to
/// Wasm4pm's graduation bridge under Wasm4pmBridge witness.
///
/// BLOCKED: wasm4pm_compat not available in stub build.
pub struct Wasm4pmBridgeGraphAdmitter;

impl Wasm4pmBridgeGraphAdmitter {
    /// BLOCKED: wasm4pm_compat unavailable in stub build.
    pub fn admit(_raw: AdmittedRelationGraph) -> ! {
        panic!("BLOCKED: wasm4pm_compat::admission/receipt unavailable in stub build — Wasm4pmBridgeGraphAdmitter::admit not reachable");
    }
}

// Silence the unused-import warning on `to_hex`; it was used by the real impls
// and will be used again when wasm4pm_compat is available.
const _: () = {
    let _ = to_hex as fn(&[u8]) -> String;
};
