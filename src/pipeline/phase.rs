//! Phase-shift model: conformance state as a thermodynamic phase transition.
//!
//! Water expands roughly 1,700x in volume when it crosses the boiling point into
//! steam. This module borrows that picture for the law-state runtime: the
//! **boiling point is the admission threshold**, and the **1,700x expansion is
//! the autonomic-mesh fan-out** applied to an observation once it is admitted —
//! a single admitted observation propagates as many bounded intents across the
//! layer-5 mesh. The phase is the bounded admission state of that observation.
//!
//! This is a *read-only* projection: it reports a phase and an amplification
//! factor; it mutates nothing. It is distinct from the runtime's `LspPhase`,
//! which tracks the LSP protocol lifecycle (`Uninitialized -> ... -> Exited`).
//!
//! Bounded statuses only, three-state law preserved. The precedence matches
//! [`crate::pipeline`]'s admission semantics: an active ANDON (`Frozen`/BLOCKED)
//! dominates; an explicit refusal (`Decomposed`/REFUSED) is next; an
//! undetermined measurement (`Unsettled`/UNKNOWN) is next and is **never**
//! coerced into `Liquid`/PARTIAL or `Vapor`/ADMITTED; only a settled measurement
//! at or above the boiling point reaches `Vapor`/ADMITTED.

use serde::{Deserialize, Serialize};

/// Volumetric expansion of water into steam at standard pressure (~1,700x).
/// Used as the autonomic-mesh amplification factor for an admitted observation.
pub const STEAM_EXPANSION_FACTOR: u32 = 1700;

/// The bounded conformance phase of an observation, named for matter states.
///
/// Each variant maps to exactly one of the project's bounded admission statuses;
/// the mapping is total and the three-state law is preserved (`Unsettled` is its
/// own phase, never folded into `Liquid` or `Vapor`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConformancePhase {
    /// Solid / ice — an ANDON signal is active; no flow until it clears (BLOCKED).
    Frozen,
    /// Liquid / water — flowing below the admission boiling point (PARTIAL).
    Liquid,
    /// Gas / steam — crossed the boiling point; the mesh expands it (ADMITTED).
    Vapor,
    /// Phase indeterminate — the measurement is undetermined (UNKNOWN).
    Unsettled,
    /// Decomposed — an explicit refusal; no longer the same substance (REFUSED).
    Decomposed,
}

impl ConformancePhase {
    /// The bounded admission status string this phase maps to.
    pub fn as_status(self) -> &'static str {
        match self {
            Self::Frozen => "BLOCKED",
            Self::Liquid => "PARTIAL",
            Self::Vapor => "ADMITTED",
            Self::Unsettled => "UNKNOWN",
            Self::Decomposed => "REFUSED",
        }
    }

    /// The autonomic-mesh amplification factor for an observation in this phase.
    ///
    /// `Vapor` expands by [`STEAM_EXPANSION_FACTOR`] (the admitted observation
    /// fans out across the mesh); `Liquid` carries unit volume (it flows but does
    /// not expand); every non-flowing phase is `0` (nothing propagates).
    pub fn expansion_factor(self) -> u32 {
        match self {
            Self::Vapor => STEAM_EXPANSION_FACTOR,
            Self::Liquid => 1,
            Self::Frozen | Self::Unsettled | Self::Decomposed => 0,
        }
    }
}

impl std::fmt::Display for ConformancePhase {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_status())
    }
}

/// A hypothetical world-state to resolve into a [`ConformancePhase`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhaseInput {
    /// Whether an ANDON signal is active (the highest-precedence floor).
    pub andon_active: bool,
    /// Whether the observation is explicitly refused by law.
    pub refused: bool,
    /// Whether the measurement is undetermined (forces UNKNOWN).
    pub unknown: bool,
    /// Conformance measurement in `[0.0, 1.0]` (the "temperature").
    pub conformance: f64,
    /// The admission threshold in `[0.0, 1.0]` (the "boiling point").
    pub boiling_point: f64,
}

/// Resolve a world-state into its bounded [`ConformancePhase`].
///
/// Precedence, three-state law preserved:
/// 1. `andon_active` -> `Frozen` (BLOCKED). A hard floor.
/// 2. `refused` -> `Decomposed` (REFUSED). Refusal dominates remaining polarity.
/// 3. `unknown` -> `Unsettled` (UNKNOWN). Never coerced to `Liquid` or `Vapor`.
/// 4. `conformance >= boiling_point` -> `Vapor` (ADMITTED), else `Liquid`
///    (PARTIAL).
pub fn phase_for(input: &PhaseInput) -> ConformancePhase {
    if input.andon_active {
        return ConformancePhase::Frozen;
    }
    if input.refused {
        return ConformancePhase::Decomposed;
    }
    if input.unknown {
        return ConformancePhase::Unsettled;
    }
    if input.conformance >= input.boiling_point {
        ConformancePhase::Vapor
    } else {
        ConformancePhase::Liquid
    }
}

/// A read-only report of a resolved phase shift.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhaseShiftReport {
    /// The bounded admission status of the resolved phase.
    pub phase: String,
    /// The autonomic-mesh amplification factor for the resolved phase.
    pub expansion_factor: u32,
    /// The conformance measurement that drove the resolution.
    pub conformance: f64,
    /// The admission threshold (boiling point) compared against.
    pub boiling_point: f64,
    /// Whether the measurement crossed the boiling point into `Vapor`.
    pub crossed_boiling_point: bool,
    /// One-line bounded summary (no victory language).
    pub summary: String,
}

/// Build a read-only [`PhaseShiftReport`] for a world-state.
pub fn phase_shift_report(input: &PhaseInput) -> PhaseShiftReport {
    let phase = phase_for(input);
    let crossed = phase == ConformancePhase::Vapor;
    let summary = match phase {
        ConformancePhase::Frozen => {
            "ANDON active; phase Frozen — status BLOCKED, no observation propagates".to_string()
        }
        ConformancePhase::Decomposed => {
            "explicit refusal; phase Decomposed — status REFUSED, no observation propagates"
                .to_string()
        }
        ConformancePhase::Unsettled => {
            "measurement undetermined; phase Unsettled — status UNKNOWN, never coerced to PARTIAL \
             or ADMITTED"
                .to_string()
        }
        ConformancePhase::Liquid => format!(
            "conformance {:.3} below boiling point {:.3}; phase Liquid — status PARTIAL",
            input.conformance, input.boiling_point
        ),
        ConformancePhase::Vapor => format!(
            "conformance {:.3} at or above boiling point {:.3}; phase Vapor — status ADMITTED, mesh \
             expansion {}x",
            input.conformance, input.boiling_point, STEAM_EXPANSION_FACTOR
        ),
    };
    PhaseShiftReport {
        phase: phase.as_status().to_string(),
        expansion_factor: phase.expansion_factor(),
        conformance: input.conformance,
        boiling_point: input.boiling_point,
        crossed_boiling_point: crossed,
        summary,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn input(andon: bool, refused: bool, unknown: bool, c: f64, bp: f64) -> PhaseInput {
        PhaseInput {
            andon_active: andon,
            refused,
            unknown,
            conformance: c,
            boiling_point: bp,
        }
    }

    #[test]
    fn precedence_blocked_over_refused_over_unknown() {
        // ANDON dominates even when refused and unknown are also set.
        assert_eq!(
            phase_for(&input(true, true, true, 0.9, 0.5)),
            ConformancePhase::Frozen
        );
        // Refusal dominates unknown and a high measurement.
        assert_eq!(
            phase_for(&input(false, true, true, 0.9, 0.5)),
            ConformancePhase::Decomposed
        );
        // Unknown dominates a high measurement.
        assert_eq!(
            phase_for(&input(false, false, true, 0.9, 0.5)),
            ConformancePhase::Unsettled
        );
    }

    #[test]
    fn unknown_never_collapses_into_liquid_or_vapor() {
        let p = phase_for(&input(false, false, true, 1.0, 0.0));
        assert_eq!(p, ConformancePhase::Unsettled);
        assert_ne!(p, ConformancePhase::Liquid);
        assert_ne!(p, ConformancePhase::Vapor);
        assert_eq!(p.as_status(), "UNKNOWN");
    }

    #[test]
    fn boiling_point_boundary_is_admission() {
        // At the boiling point exactly -> Vapor; just below -> Liquid.
        assert_eq!(
            phase_for(&input(false, false, false, 0.7, 0.7)),
            ConformancePhase::Vapor
        );
        assert_eq!(
            phase_for(&input(false, false, false, 0.699, 0.7)),
            ConformancePhase::Liquid
        );
    }

    #[test]
    fn vapor_expands_seventeen_hundred_x() {
        assert_eq!(ConformancePhase::Vapor.expansion_factor(), 1700);
        assert_eq!(ConformancePhase::Liquid.expansion_factor(), 1);
        assert_eq!(ConformancePhase::Frozen.expansion_factor(), 0);
        assert_eq!(ConformancePhase::Unsettled.expansion_factor(), 0);
        assert_eq!(ConformancePhase::Decomposed.expansion_factor(), 0);
    }

    #[test]
    fn report_marks_crossing_only_for_vapor() {
        let admitted = phase_shift_report(&input(false, false, false, 0.8, 0.7));
        assert!(admitted.crossed_boiling_point);
        assert_eq!(admitted.phase, "ADMITTED");
        assert_eq!(admitted.expansion_factor, 1700);

        let partial = phase_shift_report(&input(false, false, false, 0.5, 0.7));
        assert!(!partial.crossed_boiling_point);
        assert_eq!(partial.phase, "PARTIAL");
    }

    #[test]
    fn all_phases_map_to_bounded_statuses() {
        for p in [
            ConformancePhase::Frozen,
            ConformancePhase::Liquid,
            ConformancePhase::Vapor,
            ConformancePhase::Unsettled,
            ConformancePhase::Decomposed,
        ] {
            assert!(
                ["BLOCKED", "PARTIAL", "ADMITTED", "UNKNOWN", "REFUSED"].contains(&p.as_status()),
                "{p:?} mapped to a non-bounded status"
            );
        }
    }
}
