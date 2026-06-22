//! Read-only `max/phaseShift` protocol surface.
//!
//! Water expands roughly 1,700x when it crosses the boiling point into steam.
//! This surface projects that picture onto the law-state runtime: the **boiling
//! point is the admission threshold** and the **1,700x expansion is the
//! autonomic-mesh fan-out** an admitted observation receives. `max/phaseShift`
//! resolves a hypothetical world-state into a bounded conformance *phase* and the
//! mesh amplification that phase carries.
//!
//! This surface is **read-only**. It reports a phase and an expansion factor; it
//! mutates nothing and propagates nothing on its own. The expansion factor is the
//! *intended* mesh fan-out for an admitted observation, not an executed action —
//! any actual propagation routes through the mesh's own hook/action chain, never
//! through this observation surface.
//!
//! All status fields carry **bounded statuses only** (`ADMITTED`, `PARTIAL`,
//! `UNKNOWN`, `REFUSED`, `BLOCKED`). The three-state law holds: the `UNKNOWN`
//! phase (`Unsettled`) is never coerced into `PARTIAL` (`Liquid`) or `ADMITTED`
//! (`Vapor`). The precedence mirrors [`crate::repair::simulate_admission`]:
//! BLOCKED > REFUSED > UNKNOWN > the boiling-point comparison.

use crate::diagnostics::MaxDiagnostic;
use lsp_types_max::{Diagnostic, DiagnosticSeverity, NumberOrString};
use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Method name constant (read-only surface — the LSP never mutates files)
// ---------------------------------------------------------------------------

/// max/phaseShift — Resolve a world-state into a bounded conformance phase and
/// its autonomic-mesh expansion factor. Read-only: returns a
/// [`PhaseShiftResultMsg`]; it neither writes state nor performs the fan-out.
pub const METHOD_PHASE_SHIFT: &str = "max/phaseShift";

/// Volumetric expansion of water into steam at standard pressure (~1,700x); the
/// autonomic-mesh amplification factor for an admitted (`Vapor`) observation.
pub const STEAM_EXPANSION_FACTOR: u32 = 1700;

// ---------------------------------------------------------------------------
// Bounded status constants (each maps to one matter-state phase label)
// ---------------------------------------------------------------------------

/// Bounded status for the `Vapor` phase: crossed the boiling point.
pub const STATUS_ADMITTED: &str = "ADMITTED";
/// Bounded status for the `Liquid` phase: flowing below the boiling point.
pub const STATUS_PARTIAL: &str = "PARTIAL";
/// Bounded status for the `Unsettled` phase: measurement undetermined.
pub const STATUS_UNKNOWN: &str = "UNKNOWN";
/// Bounded status for the `Decomposed` phase: explicit refusal.
pub const STATUS_REFUSED: &str = "REFUSED";
/// Bounded status for the `Frozen` phase: an ANDON signal is active.
pub const STATUS_BLOCKED: &str = "BLOCKED";

// ---------------------------------------------------------------------------
// PHASE-* diagnostic family
// ---------------------------------------------------------------------------

/// Emitted for the `Frozen` phase (status `BLOCKED`): an ANDON signal is active
/// and no observation propagates until it clears.
pub const PHASE_FROZEN: &str = "PHASE-FROZEN";

/// Emitted for the `Decomposed` phase (status `REFUSED`): an explicit refusal;
/// nothing propagates.
pub const PHASE_DECOMPOSED: &str = "PHASE-DECOMPOSED";

/// Emitted for the `Unsettled` phase (status `UNKNOWN`): the measurement is
/// undetermined. Information severity — `UNKNOWN` is never raised to the
/// refused/blocked (Error) polarity.
pub const PHASE_UNSETTLED: &str = "PHASE-UNSETTLED";

// ---------------------------------------------------------------------------
// Request / result params (serde)
// ---------------------------------------------------------------------------

/// Parameters for `max/phaseShift`: a hypothetical world-state to resolve.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhaseShiftParams {
    /// Whether an ANDON signal is active (the highest-precedence floor).
    pub andon_active: bool,
    /// Whether the observation is explicitly refused by law.
    pub refused: bool,
    /// Whether the measurement is undetermined (forces `UNKNOWN`).
    pub unknown: bool,
    /// Conformance measurement in `[0.0, 1.0]` (the "temperature").
    pub conformance: f64,
    /// The admission threshold in `[0.0, 1.0]` (the "boiling point").
    pub boiling_point: f64,
}

/// Result of `max/phaseShift`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhaseShiftResultMsg {
    /// Bounded status string for the resolved phase.
    pub status: String,
    /// Matter-state label of the phase: `Frozen`, `Liquid`, `Vapor`,
    /// `Unsettled`, or `Decomposed`.
    pub phase_label: String,
    /// Autonomic-mesh amplification factor for the resolved phase.
    pub expansion_factor: u32,
    /// The conformance measurement that drove the resolution.
    pub conformance: f64,
    /// The admission threshold (boiling point) compared against.
    pub boiling_point: f64,
    /// Whether the measurement crossed the boiling point into `Vapor`.
    pub crossed_boiling_point: bool,
    /// Human-readable summary (one line, no victory language).
    pub summary: String,
}

/// Resolve a world-state into its bounded phase and mesh expansion factor.
///
/// Read-only and pure: a function of the supplied params; it observes no file and
/// changes no state. Precedence (three-state law preserved):
/// BLOCKED > REFUSED > UNKNOWN > boiling-point comparison.
pub fn phase_shift(p: &PhaseShiftParams) -> PhaseShiftResultMsg {
    let (status, label) = if p.andon_active {
        (STATUS_BLOCKED, "Frozen")
    } else if p.refused {
        (STATUS_REFUSED, "Decomposed")
    } else if p.unknown {
        (STATUS_UNKNOWN, "Unsettled")
    } else if p.conformance >= p.boiling_point {
        (STATUS_ADMITTED, "Vapor")
    } else {
        (STATUS_PARTIAL, "Liquid")
    };

    let expansion_factor = match status {
        STATUS_ADMITTED => STEAM_EXPANSION_FACTOR,
        STATUS_PARTIAL => 1,
        _ => 0,
    };

    let summary = match status {
        STATUS_BLOCKED => {
            "ANDON active; phase Frozen — status BLOCKED, no observation propagates".to_string()
        }
        STATUS_REFUSED => {
            "explicit refusal; phase Decomposed — status REFUSED, no observation propagates"
                .to_string()
        }
        STATUS_UNKNOWN => "measurement undetermined; phase Unsettled — status UNKNOWN, never \
                           coerced to PARTIAL or ADMITTED"
            .to_string(),
        STATUS_ADMITTED => format!(
            "conformance {:.3} at or above boiling point {:.3}; phase Vapor — status ADMITTED, \
             mesh expansion {}x",
            p.conformance, p.boiling_point, STEAM_EXPANSION_FACTOR
        ),
        _ => format!(
            "conformance {:.3} below boiling point {:.3}; phase Liquid — status PARTIAL",
            p.conformance, p.boiling_point
        ),
    };

    PhaseShiftResultMsg {
        status: status.to_string(),
        phase_label: label.to_string(),
        expansion_factor,
        conformance: p.conformance,
        boiling_point: p.boiling_point,
        crossed_boiling_point: status == STATUS_ADMITTED,
        summary,
    }
}

/// Build a single `PHASE-*` diagnostic carrying its family code on `lsp.code`.
fn phase_diagnostic(code: &str, severity: DiagnosticSeverity, message: String) -> MaxDiagnostic {
    let lsp = Diagnostic {
        severity: Some(severity),
        code: Some(NumberOrString::String(code.to_string())),
        source: Some("lsp-max:phase".to_string()),
        message,
        ..Default::default()
    };
    MaxDiagnostic {
        lsp,
        diagnostic_id: code.to_string(),
        law_id: code.to_string(),
        law_axis: crate::conformance::LawAxis::Domain,
        violated_axes: vec![crate::conformance::LawAxis::Domain.to_string()],
        ..Default::default()
    }
}

/// Map a resolved phase status to zero or more `PHASE-*` diagnostics, three-state
/// law preserved.
///
/// - `BLOCKED` (`Frozen`) -> `PHASE-FROZEN` (Error).
/// - `REFUSED` (`Decomposed`) -> `PHASE-DECOMPOSED` (Error).
/// - `UNKNOWN` (`Unsettled`) -> `PHASE-UNSETTLED` (Information). Never raised to
///   Error; `UNKNOWN` stays `UNKNOWN`.
/// - `PARTIAL` (`Liquid`) and `ADMITTED` (`Vapor`) -> no diagnostics; the status
///   itself carries the outcome.
pub fn diagnostics_for_phase(status: &str) -> Vec<MaxDiagnostic> {
    match status {
        STATUS_BLOCKED => vec![phase_diagnostic(
            PHASE_FROZEN,
            DiagnosticSeverity::ERROR,
            "ANDON active; phase Frozen — status BLOCKED, resolve the gate before any flow"
                .to_string(),
        )],
        STATUS_REFUSED => vec![phase_diagnostic(
            PHASE_DECOMPOSED,
            DiagnosticSeverity::ERROR,
            "explicit refusal; phase Decomposed — status REFUSED".to_string(),
        )],
        STATUS_UNKNOWN => vec![phase_diagnostic(
            PHASE_UNSETTLED,
            DiagnosticSeverity::INFORMATION,
            "measurement undetermined; phase Unsettled — status UNKNOWN, not coerced to PARTIAL \
             or ADMITTED"
                .to_string(),
        )],
        _ => Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn params(andon: bool, refused: bool, unknown: bool, c: f64, bp: f64) -> PhaseShiftParams {
        PhaseShiftParams {
            andon_active: andon,
            refused,
            unknown,
            conformance: c,
            boiling_point: bp,
        }
    }

    fn codes(diags: &[MaxDiagnostic]) -> Vec<&str> {
        diags
            .iter()
            .filter_map(|d| match &d.lsp.code {
                Some(NumberOrString::String(s)) => Some(s.as_str()),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn precedence_blocked_over_refused_over_unknown_over_boiling() {
        assert_eq!(
            phase_shift(&params(true, true, true, 0.9, 0.5)).status,
            STATUS_BLOCKED
        );
        assert_eq!(
            phase_shift(&params(false, true, true, 0.9, 0.5)).status,
            STATUS_REFUSED
        );
        assert_eq!(
            phase_shift(&params(false, false, true, 0.9, 0.5)).status,
            STATUS_UNKNOWN
        );
        assert_eq!(
            phase_shift(&params(false, false, false, 0.9, 0.5)).status,
            STATUS_ADMITTED
        );
        assert_eq!(
            phase_shift(&params(false, false, false, 0.4, 0.5)).status,
            STATUS_PARTIAL
        );
    }

    #[test]
    fn vapor_carries_the_steam_expansion_factor() {
        let admitted = phase_shift(&params(false, false, false, 0.8, 0.7));
        assert_eq!(admitted.expansion_factor, STEAM_EXPANSION_FACTOR);
        assert!(admitted.crossed_boiling_point);
        assert_eq!(admitted.phase_label, "Vapor");

        let liquid = phase_shift(&params(false, false, false, 0.5, 0.7));
        assert_eq!(liquid.expansion_factor, 1);
        assert!(!liquid.crossed_boiling_point);
    }

    #[test]
    fn unknown_stays_unknown_and_is_not_coerced() {
        let r = phase_shift(&params(false, false, true, 1.0, 0.0));
        assert_eq!(r.status, STATUS_UNKNOWN);
        assert_eq!(r.expansion_factor, 0);
        let diags = diagnostics_for_phase(&r.status);
        assert_eq!(codes(&diags), vec![PHASE_UNSETTLED]);
        // UNKNOWN never surfaces a refused/blocked (Error) polarity diagnostic.
        assert!(diags
            .iter()
            .all(|d| d.lsp.severity != Some(DiagnosticSeverity::ERROR)));
    }

    #[test]
    fn frozen_and_decomposed_are_error_polarity() {
        assert_eq!(
            diagnostics_for_phase(STATUS_BLOCKED)[0].lsp.severity,
            Some(DiagnosticSeverity::ERROR)
        );
        assert_eq!(
            diagnostics_for_phase(STATUS_REFUSED)[0].lsp.severity,
            Some(DiagnosticSeverity::ERROR)
        );
    }

    #[test]
    fn admitted_and_partial_emit_no_diagnostics() {
        assert!(diagnostics_for_phase(STATUS_ADMITTED).is_empty());
        assert!(diagnostics_for_phase(STATUS_PARTIAL).is_empty());
    }
}
