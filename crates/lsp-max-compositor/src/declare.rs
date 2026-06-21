//! Declare constraint model for the compositor process.
//!
//! Van der Aalst's Declare formalism: declarative process constraints expressed
//! in Linear Temporal Logic (LTL). Each constraint specifies what MUST, SHOULD,
//! or CANNOT happen in a valid execution trace.
//!
//! Reference: W.M.P. van der Aalst et al., "Declarative workflows: Balancing between
//! flexibility and support" (CEUR-WS 2005). See also OCEL 2.0 spec §3.2.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

// ─────────────────────────────────────────────────────────────────────────────
// Constraint vocabulary
// ─────────────────────────────────────────────────────────────────────────────

/// A single Declare constraint — an LTL formula over activity names.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DeclareConstraint {
    /// `init(a)` — `a` must be the first event in every trace.
    Init(String),
    /// `end(a)` — `a` must be the last event in every trace.
    End(String),
    /// `response(a, b)` — whenever `a` occurs, `b` must eventually follow.
    Response(String, String),
    /// `precedence(a, b)` — `b` can only occur if `a` has already occurred.
    Precedence(String, String),
    /// `exactly_one(a)` — `a` must occur exactly once per trace.
    ExactlyOne(String),
    /// `not_co_existence(a, b)` — `a` and `b` cannot both occur in the same trace.
    NotCoExistence(String, String),
    /// `responded_existence(a, b)` — if `a` occurs, `b` must occur (before or after).
    RespondedExistence(String, String),
    /// `absence(a)` — `a` must NOT occur in any trace.
    Absence(String),
    /// `chain_response(a, b)` — `a` must be DIRECTLY followed by `b` (no interleaving).
    ChainResponse(String, String),
}

impl DeclareConstraint {
    fn label(&self) -> String {
        match self {
            Self::Init(a) => format!("init({a})"),
            Self::End(a) => format!("end({a})"),
            Self::Response(a, b) => format!("response({a}, {b})"),
            Self::Precedence(a, b) => format!("precedence({a}, {b})"),
            Self::ExactlyOne(a) => format!("exactly_one({a})"),
            Self::NotCoExistence(a, b) => format!("not_co_existence({a}, {b})"),
            Self::RespondedExistence(a, b) => format!("responded_existence({a}, {b})"),
            Self::Absence(a) => format!("absence({a})"),
            Self::ChainResponse(a, b) => format!("chain_response({a}, {b})"),
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Violation
// ─────────────────────────────────────────────────────────────────────────────

/// A constraint violation detected when a trace deviates from the model.
#[derive(Debug, Clone, Serialize)]
pub struct ConstraintViolation {
    /// The constraint that was violated (e.g. `"response(CompositorFlush, Admit)"`).
    pub constraint: String,
    /// The case identifier (e.g. URI) that produced the violating trace.
    pub case_id: String,
    /// Human-readable explanation of the violation.
    pub detail: String,
}

// ─────────────────────────────────────────────────────────────────────────────
// DeclareModel
// ─────────────────────────────────────────────────────────────────────────────

/// A Declare process model — a named set of LTL constraints.
///
/// Instances are constructed via the normative factory methods (`compositor()`,
/// `anti_llm_detection()`) to avoid accidental mutation of the governing model.
#[derive(Debug, Clone)]
pub struct DeclareModel {
    pub name: String,
    pub constraints: Vec<DeclareConstraint>,
}

impl DeclareModel {
    /// Normative process model for the compositor: fan-out → merge → admit.
    ///
    /// The `CompositorFlush` event is the root activity.  Every flush must
    /// conclude in either `CompositorFlushAdmitted` or `CompositorFlushBlocked`.
    /// A trace cannot be both admitted and blocked for the same URI case.
    pub fn compositor() -> Self {
        Self {
            name: "compositor-fan-out-merge-admit".to_string(),
            constraints: vec![
                DeclareConstraint::Init("CompositorFlush".to_string()),
                DeclareConstraint::Response(
                    "CompositorFlush".to_string(),
                    "CompositorFlushAdmitted".to_string(),
                ),
                DeclareConstraint::NotCoExistence(
                    "CompositorFlushAdmitted".to_string(),
                    "CompositorFlushBlocked".to_string(),
                ),
                DeclareConstraint::RespondedExistence(
                    "CompositorFlushBlocked".to_string(),
                    "AndonCodePresent".to_string(),
                ),
                // ANDON must not precede the flush that triggered it.
                DeclareConstraint::Precedence(
                    "CompositorFlush".to_string(),
                    "AndonCodePresent".to_string(),
                ),
            ],
        }
    }

    /// Normative process model for the anti-llm detection pipeline.
    ///
    /// Every detection session starts with a `ScanComplete` event.  Any
    /// `CheatDetected` event must co-occur with a `FailsetUpdated` event in
    /// the same case.  `ReceiptValidated` can only follow `ScanComplete`.
    pub fn anti_llm_detection() -> Self {
        Self {
            name: "anti-llm-detection-pipeline".to_string(),
            constraints: vec![
                DeclareConstraint::Init("ScanComplete".to_string()),
                DeclareConstraint::RespondedExistence(
                    "CheatDetected".to_string(),
                    "FailsetUpdated".to_string(),
                ),
                DeclareConstraint::Precedence(
                    "ScanComplete".to_string(),
                    "ReceiptValidated".to_string(),
                ),
                DeclareConstraint::ExactlyOne("ScanComplete".to_string()),
                // Victory language is permanently forbidden in detection output.
                DeclareConstraint::Absence("VictoryLanguageEmitted".to_string()),
                // NegativeControlExecuted must always follow a DetectionClaim.
                DeclareConstraint::ChainResponse(
                    "DetectionClaim".to_string(),
                    "NegativeControlExecuted".to_string(),
                ),
            ],
        }
    }

    /// Check traces against all constraints.  Returns every violation; an
    /// empty `Vec` means the log is conformant with this model.
    pub fn check(&self, events_by_case: &HashMap<String, Vec<String>>) -> Vec<ConstraintViolation> {
        let mut violations = Vec::new();
        for (case_id, trace) in events_by_case {
            for constraint in &self.constraints {
                if let Some(v) = check_constraint(constraint, case_id, trace) {
                    violations.push(v);
                }
            }
        }
        violations
    }

    /// Conformance score ∈ [0.0, 1.0]: fraction of constraints satisfied across
    /// all cases.  An empty log yields `1.0` (vacuously conformant).
    pub fn fitness(&self, events_by_case: &HashMap<String, Vec<String>>) -> f64 {
        if events_by_case.is_empty() || self.constraints.is_empty() {
            return 1.0;
        }
        let total = events_by_case.len() * self.constraints.len();
        let violated = self.check(events_by_case).len();
        let satisfied = total.saturating_sub(violated);
        satisfied as f64 / total as f64
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Constraint evaluation
// ─────────────────────────────────────────────────────────────────────────────

fn check_constraint(
    constraint: &DeclareConstraint,
    case_id: &str,
    trace: &[String],
) -> Option<ConstraintViolation> {
    let viol = |detail: String| ConstraintViolation {
        constraint: constraint.label(),
        case_id: case_id.to_string(),
        detail,
    };

    match constraint {
        DeclareConstraint::Init(a) => {
            if !trace.is_empty() && trace.first().map(|s| s.as_str()) != Some(a.as_str()) {
                Some(viol(format!(
                    "first activity is {:?}, expected {a}",
                    trace.first()
                )))
            } else {
                None
            }
        }
        DeclareConstraint::End(a) => {
            if !trace.is_empty() && trace.last().map(|s| s.as_str()) != Some(a.as_str()) {
                Some(viol(format!(
                    "last activity is {:?}, expected {a}",
                    trace.last()
                )))
            } else {
                None
            }
        }
        DeclareConstraint::Response(a, b) => {
            for (i, act) in trace.iter().enumerate() {
                if act == a && !trace[i + 1..].contains(b) {
                    return Some(viol(format!(
                        "{a} at position {i} has no subsequent {b}"
                    )));
                }
            }
            None
        }
        DeclareConstraint::Precedence(a, b) => {
            let mut a_seen = false;
            for act in trace {
                if act == a {
                    a_seen = true;
                }
                if act == b && !a_seen {
                    return Some(viol(format!("{b} occurred before {a}")));
                }
            }
            None
        }
        DeclareConstraint::ExactlyOne(a) => {
            let count = trace.iter().filter(|act| *act == a).count();
            if count != 1 && !trace.is_empty() {
                Some(viol(format!("{a} occurred {count} times (expected 1)")))
            } else {
                None
            }
        }
        DeclareConstraint::NotCoExistence(a, b) => {
            if trace.contains(a) && trace.contains(b) {
                Some(viol(format!(
                    "both {a} and {b} occurred in same trace"
                )))
            } else {
                None
            }
        }
        DeclareConstraint::RespondedExistence(a, b) => {
            if trace.contains(a) && !trace.contains(b) {
                Some(viol(format!("{a} occurred but {b} did not")))
            } else {
                None
            }
        }
        DeclareConstraint::Absence(a) => {
            if trace.contains(a) {
                Some(viol(format!("forbidden activity {a} occurred")))
            } else {
                None
            }
        }
        DeclareConstraint::ChainResponse(a, b) => {
            for (i, act) in trace.iter().enumerate() {
                if act == a {
                    let next = trace.get(i + 1).map(|s| s.as_str());
                    if next != Some(b.as_str()) {
                        return Some(viol(format!(
                            "{a} at {i} not directly followed by {b} (got {next:?})"
                        )));
                    }
                }
            }
            None
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Trace extraction from OCEL JSON events
// ─────────────────────────────────────────────────────────────────────────────

/// Extract per-case activity traces from raw OCEL 2.0 event JSON values.
///
/// Case identity is derived from `attributes.uri` → `attributes.case_id` →
/// falls back to `"_default"`.  Activity name comes from the `"type"` field.
pub fn extract_traces(events: &[Value]) -> HashMap<String, Vec<String>> {
    let mut traces: HashMap<String, Vec<String>> = HashMap::new();
    for ev in events {
        let event_type = ev
            .get("type")
            .and_then(|t| t.as_str())
            .unwrap_or("Unknown");
        let case_id = ev
            .get("attributes")
            .and_then(|a| {
                a.get("uri")
                    .or_else(|| a.get("case_id"))
                    .and_then(|v| v.as_str())
            })
            .unwrap_or("_default");
        traces
            .entry(case_id.to_string())
            .or_default()
            .push(event_type.to_string());
    }
    traces
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn response_constraint_fires_when_b_absent() {
        let model = DeclareModel {
            name: "test".to_string(),
            constraints: vec![DeclareConstraint::Response("A".to_string(), "B".to_string())],
        };
        let mut traces = HashMap::new();
        traces.insert("case1".to_string(), vec!["A".to_string(), "C".to_string()]);
        let violations = model.check(&traces);
        assert!(!violations.is_empty());
        assert!(violations[0].constraint.contains("response"));
    }

    #[test]
    fn not_co_existence_fires_when_both_present() {
        let model = DeclareModel {
            name: "test".to_string(),
            constraints: vec![DeclareConstraint::NotCoExistence(
                "Admit".to_string(),
                "Block".to_string(),
            )],
        };
        let mut traces = HashMap::new();
        traces.insert(
            "uri1".to_string(),
            vec!["Admit".to_string(), "Block".to_string()],
        );
        let violations = model.check(&traces);
        assert_eq!(violations.len(), 1);
    }

    #[test]
    fn conformant_trace_yields_no_violations() {
        let model = DeclareModel::compositor();
        // Minimal conformant trace: CompositorFlush → CompositorFlushAdmitted
        let mut traces = HashMap::new();
        traces.insert(
            "file:///tmp/a.rs".to_string(),
            vec![
                "CompositorFlush".to_string(),
                "CompositorFlushAdmitted".to_string(),
            ],
        );
        let violations = model.check(&traces);
        assert!(violations.is_empty(), "unexpected violations: {violations:?}");
    }

    #[test]
    fn fitness_perfect_for_empty_log() {
        let model = DeclareModel::compositor();
        let fitness = model.fitness(&HashMap::new());
        assert_eq!(fitness, 1.0);
    }
}
