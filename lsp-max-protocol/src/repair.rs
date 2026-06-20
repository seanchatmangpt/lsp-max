//! Read-only `max/repairPlan` and `max/whatIf` protocol surface.
//!
//! A diagnostic tells an agent *that* a law axis is unsatisfied. This module
//! projects each known diagnostic family onto a structured, bounded **repair
//! plan**: an ordered list of [`RepairStep`]s an agent or CI runner can act on.
//! It also exposes a [`simulate_admission`] *what-if* gate that answers, without
//! touching any file, what admission verdict a hypothetical world-state would
//! produce.
//!
//! This surface is **read-only**. It emits plans and intents; it never mutates
//! files. A [`RepairPlanResult`] is guidance, not an applied change — any actual
//! mutation must route through the `CodeAction -> clap-noun-verb -> Receipt`
//! chain, never through this observation surface. A `verb` named on a step is a
//! *suggested* `clap-noun-verb` actuation, not an executed one.
//!
//! All status fields carry **bounded statuses only** (`ADMITTED`, `PARTIAL`,
//! `UNKNOWN`, `REFUSED`, `BLOCKED`). The three-state law holds throughout:
//! `UNKNOWN` is never coerced into `ADMITTED` or `REFUSED`. An unrecognized
//! diagnostic id yields an `UNKNOWN` plan with **no** fabricated steps, and a
//! what-if with any unknown axis yields `UNKNOWN` regardless of how many axes
//! are admitted.

use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Method name constants (read-only surface — the LSP never mutates files)
// ---------------------------------------------------------------------------

/// max/repairPlan — Synthesize a bounded, ordered repair plan for a diagnostic
/// id. Read-only: returns a [`RepairPlanResult`]; never applies the repair.
pub const METHOD_REPAIR_PLAN: &str = "max/repairPlan";

/// max/whatIf — Simulate the admission verdict for a hypothetical world-state.
/// Read-only: returns a [`WhatIfResult`]; observes nothing on disk and changes
/// no state.
pub const METHOD_WHAT_IF: &str = "max/whatIf";

// ---------------------------------------------------------------------------
// Bounded status constants
// ---------------------------------------------------------------------------

/// Bounded status: the plan/axis cleared the admission bar.
pub const STATUS_ADMITTED: &str = "ADMITTED";
/// Bounded status: a partial path exists but does not clear the bar on its own.
pub const STATUS_PARTIAL: &str = "PARTIAL";
/// Bounded status: admissibility cannot be determined from what is known.
pub const STATUS_UNKNOWN: &str = "UNKNOWN";
/// Bounded status: an explicit refusal by law.
pub const STATUS_REFUSED: &str = "REFUSED";
/// Bounded status: an active ANDON signal blocks progress until it clears.
pub const STATUS_BLOCKED: &str = "BLOCKED";

// ---------------------------------------------------------------------------
// max/repairPlan params / result
// ---------------------------------------------------------------------------

/// One ordered step in a repair plan. A step describes an intent, never an
/// applied mutation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepairStep {
    /// 1-based position of this step within the plan.
    pub order: usize,
    /// The action an agent or CI runner should take.
    pub action: String,
    /// Why this step is part of the bounded path toward admission.
    pub rationale: String,
    /// Suggested `clap-noun-verb` actuation, when one applies. This is a
    /// suggestion only; the LSP surface never invokes it.
    pub verb: Option<String>,
}

/// Parameters for `max/repairPlan`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepairPlanParams {
    /// The diagnostic id (or law id) to synthesize a plan for.
    pub diagnostic_id: String,
}

/// Result of `max/repairPlan`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepairPlanResult {
    /// Echo of the requested diagnostic id.
    pub diagnostic_id: String,
    /// Bounded status string (`ADMITTED`, `PARTIAL`, `UNKNOWN`, `REFUSED`,
    /// `BLOCKED`).
    pub status: String,
    /// Ordered, bounded repair steps. Empty when the id is unrecognized; the
    /// surface never fabricates steps for an `UNKNOWN` plan.
    pub steps: Vec<RepairStep>,
    /// Repairability of the diagnostic, mirroring
    /// [`crate::diagnostics::Repairability`] as a bounded string.
    pub repairability: String,
    /// Human-readable summary of the plan's intent.
    pub summary: String,
}

impl RepairPlanResult {
    /// The `UNKNOWN` plan for an unrecognized diagnostic id: empty steps, no
    /// fabrication, and a repairability of `Unknown`. This is the negative
    /// control — an absent catalog entry must never present as `ADMITTED` or
    /// `REFUSED`.
    fn unknown(diagnostic_id: &str) -> Self {
        Self {
            diagnostic_id: diagnostic_id.to_string(),
            status: STATUS_UNKNOWN.to_string(),
            steps: Vec::new(),
            repairability: "Unknown".to_string(),
            summary: format!(
                "no repair catalog entry for `{diagnostic_id}`; status UNKNOWN — \
                 admissibility cannot be determined and is not coerced to REFUSED or ADMITTED"
            ),
        }
    }
}

/// Build an ordered list of [`RepairStep`]s from `(action, rationale, verb)`
/// triples, assigning 1-based `order` positions.
fn steps_from(raw: &[(&str, &str, Option<&str>)]) -> Vec<RepairStep> {
    raw.iter()
        .enumerate()
        .map(|(i, (action, rationale, verb))| RepairStep {
            order: i + 1,
            action: (*action).to_string(),
            rationale: (*rationale).to_string(),
            verb: verb.map(str::to_string),
        })
        .collect()
}

/// Synthesize a bounded repair plan for a diagnostic id.
///
/// Read-only: this consults a small static catalog and reports a plan; it does
/// not read or change any file.
///
/// Status handling, three-state law preserved:
/// - An ANDON-gated `WASM4PM-*` code -> `BLOCKED`, with steps that route the
///   agent to resolve the gate (`gate check`). The ANDON must clear before work
///   proceeds; the plan reflects that floor rather than claiming admission.
/// - `TPOT2-NONCONVERGENCE` -> `PARTIAL`: a result exists but is below the bar;
///   the steps widen the search (more generations / larger population).
/// - `TPOT2-EMPTY-POOL` -> `REFUSED`: with no breeds there is nothing to admit;
///   the steps direct the agent to repopulate / check the catalog.
/// - Any unrecognized id -> `UNKNOWN` with an empty plan (negative control).
pub fn repair_plan_for(diagnostic_id: &str) -> RepairPlanResult {
    match diagnostic_id {
        // ANDON-gated process-mining family. The gate is a floor: while it is
        // set, no shell-side action proceeds, so the bounded status is BLOCKED.
        "WASM4PM-ANDON" | "WASM4PM-GATE" | "WASM4PM-GATE-BLOCKED" => RepairPlanResult {
            diagnostic_id: diagnostic_id.to_string(),
            status: STATUS_BLOCKED.to_string(),
            steps: steps_from(&[
                (
                    "Run the gate check to read the current ANDON state.",
                    "The ANDON signal is a one-byte gate; the check reports whether it is set \
                     before any further shell-side action is attempted.",
                    Some("gate check"),
                ),
                (
                    "Resolve every active WASM4PM-* and GGEN-* Error-severity diagnostic.",
                    "Lambda_CD blocks while any governed Error is present in the diagnostic \
                     context; the gate clears only when that set drains.",
                    None,
                ),
                (
                    "Re-run the gate check and confirm it reports clear before proceeding.",
                    "Re-reading the gate confirms the BLOCKED floor has lifted; status remains \
                     BLOCKED until the gate is observed clear.",
                    Some("gate check"),
                ),
            ]),
            repairability: "Repairable".to_string(),
            summary: "ANDON gate is active; status BLOCKED — resolve governed diagnostics and \
                      confirm the gate clears before any build, test, or release action"
                .to_string(),
        },

        // Search produced a below-threshold result: a partial path exists.
        "TPOT2-NONCONVERGENCE" => RepairPlanResult {
            diagnostic_id: diagnostic_id.to_string(),
            status: STATUS_PARTIAL.to_string(),
            steps: steps_from(&[
                (
                    "Increase the number of generations the breed search evolves.",
                    "More generations give the optimizer additional rounds to climb toward the \
                     admission threshold the previous run fell short of.",
                    None,
                ),
                (
                    "Increase the population size per generation.",
                    "A larger population widens the explored breed space, reducing the chance \
                     the search stalls in a below-threshold local optimum.",
                    None,
                ),
                (
                    "Re-run the search and compare best observed fitness against the threshold.",
                    "The status stays PARTIAL until a re-run clears the bar; the comparison is \
                     what distinguishes PARTIAL from ADMITTED.",
                    None,
                ),
            ]),
            repairability: "Repairable".to_string(),
            summary: "breed search ended below the admission threshold; status PARTIAL — widen \
                      the search (more generations / larger population) and re-run"
                .to_string(),
        },

        // Empty breed pool: an explicit refusal — nothing exists to admit.
        "TPOT2-EMPTY-POOL" => RepairPlanResult {
            diagnostic_id: diagnostic_id.to_string(),
            status: STATUS_REFUSED.to_string(),
            steps: steps_from(&[
                (
                    "Inspect the breed catalog to confirm whether any breeds are registered.",
                    "An empty pool is a REFUSED outcome; the first step is to verify the catalog \
                     is the cause rather than a filter excluding every entry.",
                    None,
                ),
                (
                    "Repopulate the breed pool with at least one candidate before searching.",
                    "With no breeds there is nothing to evaluate; admission is impossible until \
                     the pool is non-empty.",
                    None,
                ),
            ]),
            repairability: "Repairable".to_string(),
            summary: "breed pool is empty; status REFUSED — check the catalog and repopulate the \
                      pool before a search can yield any admissible breed"
                .to_string(),
        },

        // Negative control: anything outside the catalog stays UNKNOWN.
        _ => RepairPlanResult::unknown(diagnostic_id),
    }
}

// ---------------------------------------------------------------------------
// max/whatIf params / result
// ---------------------------------------------------------------------------

/// Parameters for `max/whatIf`: a hypothetical world-state to evaluate.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WhatIfParams {
    /// Whether an ANDON signal is active in the hypothetical state.
    pub andon_active: bool,
    /// Count of law axes whose admissibility is unknown.
    pub unknown_axes: usize,
    /// Count of law axes explicitly refused.
    pub refused_axes: usize,
}

/// Result of `max/whatIf`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WhatIfResult {
    /// Bounded admission verdict (`BLOCKED`, `REFUSED`, `UNKNOWN`, `ADMITTED`).
    pub admission: String,
    /// Why the verdict was reached, naming the governing precedence.
    pub rationale: String,
}

/// Simulate the admission verdict for a hypothetical world-state.
///
/// Read-only: this is a pure function over the supplied counts; it observes no
/// file and changes no state.
///
/// Strict precedence, three-state law preserved (`UNKNOWN` never collapses):
/// 1. `andon_active` -> `BLOCKED`. An active ANDON is a hard floor.
/// 2. `refused_axes > 0` -> `REFUSED`. An explicit refusal dominates remaining
///    polarities.
/// 3. `unknown_axes > 0` -> `UNKNOWN`. An undetermined axis is reported as
///    `UNKNOWN`; it is never coerced to `ADMITTED` (or to `REFUSED`) merely
///    because other axes are admitted.
/// 4. Otherwise -> `ADMITTED`.
pub fn simulate_admission(p: &WhatIfParams) -> WhatIfResult {
    if p.andon_active {
        return WhatIfResult {
            admission: STATUS_BLOCKED.to_string(),
            rationale: "an ANDON signal is active; status BLOCKED — it is the highest-precedence \
                        floor and no admission verdict is reached while it is set"
                .to_string(),
        };
    }
    if p.refused_axes > 0 {
        return WhatIfResult {
            admission: STATUS_REFUSED.to_string(),
            rationale: format!(
                "{} law axis/axes are explicitly refused; status REFUSED — refusal dominates \
                 any unknown or admitted axes",
                p.refused_axes
            ),
        };
    }
    if p.unknown_axes > 0 {
        return WhatIfResult {
            admission: STATUS_UNKNOWN.to_string(),
            rationale: format!(
                "{} law axis/axes are undetermined; status UNKNOWN — this is never coerced to \
                 ADMITTED or REFUSED, even when every other axis is admitted",
                p.unknown_axes
            ),
        };
    }
    WhatIfResult {
        admission: STATUS_ADMITTED.to_string(),
        rationale: "no ANDON signal, no refused axes, and no unknown axes; status ADMITTED"
            .to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A known code yields an ordered, non-empty plan with a bounded status.
    #[test]
    fn known_code_yields_ordered_bounded_plan() {
        let plan = repair_plan_for("TPOT2-NONCONVERGENCE");
        assert!(!plan.steps.is_empty(), "a known code must produce steps");
        assert_eq!(plan.status, STATUS_PARTIAL);
        // Steps carry 1-based, contiguous, ascending order indices.
        for (i, step) in plan.steps.iter().enumerate() {
            assert_eq!(step.order, i + 1, "step order must be 1-based contiguous");
        }
        // Status is drawn from the bounded set, never an ad-hoc string.
        assert!([
            STATUS_ADMITTED,
            STATUS_PARTIAL,
            STATUS_UNKNOWN,
            STATUS_REFUSED,
            STATUS_BLOCKED,
        ]
        .contains(&plan.status.as_str()));
    }

    /// The ANDON-gated family is BLOCKED and suggests the gate-check verb.
    #[test]
    fn andon_code_is_blocked_and_routes_to_gate() {
        let plan = repair_plan_for("WASM4PM-ANDON");
        assert_eq!(plan.status, STATUS_BLOCKED);
        assert!(!plan.steps.is_empty());
        assert!(
            plan.steps
                .iter()
                .any(|s| s.verb.as_deref() == Some("gate check")),
            "an ANDON plan must route the agent to `gate check`"
        );
    }

    /// The empty-pool family is an explicit REFUSED with a non-empty plan.
    #[test]
    fn empty_pool_code_is_refused() {
        let plan = repair_plan_for("TPOT2-EMPTY-POOL");
        assert_eq!(plan.status, STATUS_REFUSED);
        assert!(!plan.steps.is_empty());
    }

    /// NEGATIVE CONTROL: an unrecognized code yields UNKNOWN with no fabricated
    /// steps, and never collapses to REFUSED or ADMITTED.
    #[test]
    fn unknown_code_yields_unknown_and_empty_steps() {
        let plan = repair_plan_for("NOT-A-REAL-DIAGNOSTIC-CODE");
        assert_eq!(plan.status, STATUS_UNKNOWN);
        assert!(
            plan.steps.is_empty(),
            "an unrecognized code must not fabricate any repair step"
        );
        assert_ne!(plan.status, STATUS_REFUSED);
        assert_ne!(plan.status, STATUS_ADMITTED);
        assert_eq!(plan.repairability, "Unknown");
    }

    /// What-if with an unknown axis stays UNKNOWN — not ADMITTED, not REFUSED —
    /// even when no axis is refused.
    #[test]
    fn what_if_unknown_axis_stays_unknown() {
        let r = simulate_admission(&WhatIfParams {
            andon_active: false,
            unknown_axes: 1,
            refused_axes: 0,
        });
        assert_eq!(r.admission, STATUS_UNKNOWN);
        assert_ne!(r.admission, STATUS_ADMITTED);
        assert_ne!(r.admission, STATUS_REFUSED);
    }

    /// What-if precedence: BLOCKED > REFUSED > UNKNOWN > ADMITTED.
    #[test]
    fn what_if_precedence_holds() {
        // ANDON dominates everything else, including refused and unknown axes.
        let blocked = simulate_admission(&WhatIfParams {
            andon_active: true,
            unknown_axes: 3,
            refused_axes: 3,
        });
        assert_eq!(blocked.admission, STATUS_BLOCKED);

        // With no ANDON, a refusal dominates unknown axes.
        let refused = simulate_admission(&WhatIfParams {
            andon_active: false,
            unknown_axes: 3,
            refused_axes: 1,
        });
        assert_eq!(refused.admission, STATUS_REFUSED);

        // With no ANDON and no refusal, an unknown axis dominates admission.
        let unknown = simulate_admission(&WhatIfParams {
            andon_active: false,
            unknown_axes: 1,
            refused_axes: 0,
        });
        assert_eq!(unknown.admission, STATUS_UNKNOWN);

        // Only a fully clean state admits.
        let admitted = simulate_admission(&WhatIfParams {
            andon_active: false,
            unknown_axes: 0,
            refused_axes: 0,
        });
        assert_eq!(admitted.admission, STATUS_ADMITTED);
    }
}
