//! Declare constraint model backed by wasm4pm's Declare type vocabulary.
//!
//! The `DeclareModel` struct wraps wasm4pm's `DeclareModel` and uses wasm4pm's
//! `DeclareTemplate` enum (18 constraint types) as the authoritative constraint
//! vocabulary.  No custom constraint type definitions are implemented here.
//!
//! Factory methods (`compositor()`, `anti_llm_detection()`) build normative
//! models.  Conformance checking interprets wasm4pm's `DeclareTemplate` variants
//! against per-case activity traces derived from OCEL JSON events.
//!
//! Reference: W.M.P. van der Aalst et al., "Declarative workflows: Balancing
//! between flexibility and support" (CEUR-WS 2005). OCEL 2.0 spec §3.2.

use serde::Serialize;
use serde_json::Value;
use std::collections::HashMap;

use wasm4pm::declare::DeclareModel as WasmDeclareModel;
pub use wasm4pm::declare::{
    ActivityName, Confidence, DeclareConstraint, DeclareTemplate, Support,
};

// ─────────────────────────────────────────────────────────────────────────────
// Violation
// ─────────────────────────────────────────────────────────────────────────────

/// A constraint violation detected when a trace deviates from the model.
#[derive(Debug, Clone, Serialize)]
pub struct ConstraintViolation {
    /// The constraint label (e.g. `"response(CompositorFlush, CompositorFlushAdmitted)"`).
    pub constraint: String,
    /// The case identifier (e.g. URI) that produced the violating trace.
    pub case_id: String,
    /// Human-readable explanation of the violation.
    pub detail: String,
}

// ─────────────────────────────────────────────────────────────────────────────
// DeclareModel
// ─────────────────────────────────────────────────────────────────────────────

/// A named Declare process model wrapping wasm4pm's `DeclareModel`.
///
/// The `name` field identifies the model instance.  The inner `WasmDeclareModel`
/// holds the wasm4pm-typed constraint list and activity set.
#[derive(Debug, Clone)]
pub struct DeclareModel {
    pub name: String,
    inner: WasmDeclareModel,
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn act(name: &str) -> ActivityName {
    ActivityName::from_str(name).expect("non-empty activity name")
}

fn wasm_constraint(template: DeclareTemplate, activities: Vec<ActivityName>) -> DeclareConstraint {
    DeclareConstraint {
        template,
        activities,
        // support/confidence are stored as f64; validate the range via the
        // newtypes (per wasm4pm's construction-site guidance) and store the value.
        support: Support::new(1.0).expect("support 1.0 is in [0,1]").value(),
        confidence: Confidence::new(1.0)
            .expect("confidence 1.0 is in [0,1]")
            .value(),
    }
}

fn build_model(name: &str, constraints: Vec<DeclareConstraint>) -> DeclareModel {
    let activities = constraints
        .iter()
        .flat_map(|c| c.activities.iter().cloned())
        .collect();
    DeclareModel {
        name: name.to_string(),
        inner: WasmDeclareModel {
            activities,
            constraints,
        },
    }
}

// ── Factory methods ───────────────────────────────────────────────────────────

impl DeclareModel {
    /// Normative process model for the compositor: fan-out → merge → admit.
    pub fn compositor() -> Self {
        build_model(
            "compositor-fan-out-merge-admit",
            vec![
                wasm_constraint(DeclareTemplate::Init, vec![act("CompositorFlush")]),
                wasm_constraint(
                    DeclareTemplate::Response,
                    vec![act("CompositorFlush"), act("CompositorFlushAdmitted")],
                ),
                wasm_constraint(
                    DeclareTemplate::NotCoExistence,
                    vec![act("CompositorFlushAdmitted"), act("CompositorFlushBlocked")],
                ),
                wasm_constraint(
                    DeclareTemplate::RespondedExistence,
                    vec![act("CompositorFlushBlocked"), act("AndonCodePresent")],
                ),
                wasm_constraint(
                    DeclareTemplate::Precedence,
                    vec![act("CompositorFlush"), act("AndonCodePresent")],
                ),
            ],
        )
    }

    /// Normative process model for the anti-llm detection pipeline.
    pub fn anti_llm_detection() -> Self {
        build_model(
            "anti-llm-detection-pipeline",
            vec![
                wasm_constraint(DeclareTemplate::Init, vec![act("ScanComplete")]),
                wasm_constraint(
                    DeclareTemplate::RespondedExistence,
                    vec![act("CheatDetected"), act("FailsetUpdated")],
                ),
                wasm_constraint(
                    DeclareTemplate::Precedence,
                    vec![act("ScanComplete"), act("ReceiptValidated")],
                ),
                wasm_constraint(
                    DeclareTemplate::ExactlyN { n: 1 },
                    vec![act("ScanComplete")],
                ),
                wasm_constraint(
                    DeclareTemplate::Absence { max: 0 },
                    vec![act("VictoryLanguageEmitted")],
                ),
                wasm_constraint(
                    DeclareTemplate::ChainResponse,
                    vec![act("DetectionClaim"), act("NegativeControlExecuted")],
                ),
            ],
        )
    }

    /// Check traces against all constraints.  Returns every violation; an
    /// empty `Vec` means the log is conformant with this model.
    pub fn check(&self, events_by_case: &HashMap<String, Vec<String>>) -> Vec<ConstraintViolation> {
        let mut violations = Vec::new();
        for (case_id, trace) in events_by_case {
            for constraint in &self.inner.constraints {
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
        if events_by_case.is_empty() || self.inner.constraints.is_empty() {
            return 1.0;
        }
        let total = events_by_case.len() * self.inner.constraints.len();
        let violated = self.check(events_by_case).len();
        let satisfied = total.saturating_sub(violated);
        satisfied as f64 / total as f64
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Constraint evaluation against wasm4pm DeclareTemplate variants
// ─────────────────────────────────────────────────────────────────────────────

fn constraint_label(c: &DeclareConstraint) -> String {
    let acts: Vec<&str> = c.activities.iter().map(|a| a.as_str()).collect();
    let a = acts.first().copied().unwrap_or("?");
    let b = acts.get(1).copied().unwrap_or("?");
    match &c.template {
        DeclareTemplate::Init => format!("init({a})"),
        DeclareTemplate::End => format!("end({a})"),
        DeclareTemplate::Response => format!("response({a}, {b})"),
        DeclareTemplate::AlternateResponse => format!("alternate_response({a}, {b})"),
        DeclareTemplate::ChainResponse => format!("chain_response({a}, {b})"),
        DeclareTemplate::Precedence => format!("precedence({a}, {b})"),
        DeclareTemplate::AlternatePrecedence => format!("alternate_precedence({a}, {b})"),
        DeclareTemplate::ChainPrecedence => format!("chain_precedence({a}, {b})"),
        DeclareTemplate::Existence { .. } => format!("existence({a})"),
        DeclareTemplate::Absence { .. } => format!("absence({a})"),
        DeclareTemplate::ExactlyN { .. } => format!("exactly_n({a})"),
        DeclareTemplate::NotCoExistence => format!("not_co_existence({a}, {b})"),
        DeclareTemplate::RespondedExistence => format!("responded_existence({a}, {b})"),
        DeclareTemplate::CoExistence => format!("co_existence({a}, {b})"),
        DeclareTemplate::Succession => format!("succession({a}, {b})"),
        DeclareTemplate::AlternateSuccession => format!("alternate_succession({a}, {b})"),
        DeclareTemplate::ChainSuccession => format!("chain_succession({a}, {b})"),
        DeclareTemplate::NotSuccession => format!("not_succession({a}, {b})"),
        DeclareTemplate::NotChainSuccession => format!("not_chain_succession({a}, {b})"),
    }
}

fn check_constraint(
    constraint: &DeclareConstraint,
    case_id: &str,
    trace: &[String],
) -> Option<ConstraintViolation> {
    let label = constraint_label(constraint);
    let acts: Vec<&str> = constraint.activities.iter().map(|a| a.as_str()).collect();
    let a = acts.first().copied().unwrap_or("");
    let b = acts.get(1).copied().unwrap_or("");

    let viol = |detail: String| ConstraintViolation {
        constraint: label.clone(),
        case_id: case_id.to_string(),
        detail,
    };

    match &constraint.template {
        DeclareTemplate::Init => {
            if !trace.is_empty() && trace.first().map(|s| s.as_str()) != Some(a) {
                Some(viol(format!(
                    "first activity is {:?}, expected {a}",
                    trace.first()
                )))
            } else {
                None
            }
        }
        DeclareTemplate::End => {
            if !trace.is_empty() && trace.last().map(|s| s.as_str()) != Some(a) {
                Some(viol(format!(
                    "last activity is {:?}, expected {a}",
                    trace.last()
                )))
            } else {
                None
            }
        }
        DeclareTemplate::Response | DeclareTemplate::AlternateResponse => {
            for (i, act) in trace.iter().enumerate() {
                if act.as_str() == a && !trace[i + 1..].iter().any(|x| x.as_str() == b) {
                    return Some(viol(format!(
                        "{a} at position {i} has no subsequent {b}"
                    )));
                }
            }
            None
        }
        DeclareTemplate::ChainResponse => {
            for (i, act) in trace.iter().enumerate() {
                if act.as_str() == a {
                    let next = trace.get(i + 1).map(|s| s.as_str());
                    if next != Some(b) {
                        return Some(viol(format!(
                            "{a} at {i} not directly followed by {b} (got {next:?})"
                        )));
                    }
                }
            }
            None
        }
        DeclareTemplate::Precedence | DeclareTemplate::AlternatePrecedence => {
            let mut a_seen = false;
            for act in trace {
                if act.as_str() == a {
                    a_seen = true;
                }
                if act.as_str() == b && !a_seen {
                    return Some(viol(format!("{b} occurred before {a}")));
                }
            }
            None
        }
        DeclareTemplate::ChainPrecedence => {
            for (i, act) in trace.iter().enumerate() {
                if act.as_str() == b {
                    let prev = i.checked_sub(1).and_then(|p| trace.get(p)).map(|s| s.as_str());
                    if prev != Some(a) {
                        return Some(viol(format!(
                            "{b} at {i} not directly preceded by {a} (got {prev:?})"
                        )));
                    }
                }
            }
            None
        }
        DeclareTemplate::ExactlyN { n } => {
            let count = trace.iter().filter(|act| act.as_str() == a).count();
            if count != *n as usize && !trace.is_empty() {
                Some(viol(format!("{a} occurred {count} times (expected {n})")))
            } else {
                None
            }
        }
        DeclareTemplate::Existence { min } => {
            let count = trace.iter().filter(|act| act.as_str() == a).count();
            if !trace.is_empty() && count < *min as usize {
                Some(viol(format!(
                    "{a} occurred {count} times (expected at least {min})"
                )))
            } else {
                None
            }
        }
        DeclareTemplate::Absence { max } => {
            let count = trace.iter().filter(|act| act.as_str() == a).count();
            if count > *max as usize {
                Some(viol(format!(
                    "{a} occurred {count} times (at most {max} allowed)"
                )))
            } else {
                None
            }
        }
        DeclareTemplate::NotCoExistence => {
            let has_a = trace.iter().any(|act| act.as_str() == a);
            let has_b = trace.iter().any(|act| act.as_str() == b);
            if has_a && has_b {
                Some(viol(format!("both {a} and {b} occurred in same trace")))
            } else {
                None
            }
        }
        DeclareTemplate::RespondedExistence => {
            let has_a = trace.iter().any(|act| act.as_str() == a);
            let has_b = trace.iter().any(|act| act.as_str() == b);
            if has_a && !has_b {
                Some(viol(format!("{a} occurred but {b} did not")))
            } else {
                None
            }
        }
        DeclareTemplate::CoExistence => {
            let has_a = trace.iter().any(|act| act.as_str() == a);
            let has_b = trace.iter().any(|act| act.as_str() == b);
            if has_a != has_b {
                Some(viol(format!(
                    "co-existence violated: {a} present={has_a}, {b} present={has_b}"
                )))
            } else {
                None
            }
        }
        DeclareTemplate::Succession => {
            // Both Response and Precedence must hold.
            let mut a_seen = false;
            for act in trace {
                if act.as_str() == a {
                    a_seen = true;
                }
                if act.as_str() == b && !a_seen {
                    return Some(viol(format!("succession: {b} occurred before {a}")));
                }
            }
            for (i, act) in trace.iter().enumerate() {
                if act.as_str() == a && !trace[i + 1..].iter().any(|x| x.as_str() == b) {
                    return Some(viol(format!(
                        "succession: {a} at {i} not followed by {b}"
                    )));
                }
            }
            None
        }
        DeclareTemplate::AlternateSuccession
        | DeclareTemplate::ChainSuccession
        | DeclareTemplate::NotSuccession
        | DeclareTemplate::NotChainSuccession => {
            // Advanced templates not used in normative models; no violation raised.
            None
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Trace extraction from OCEL JSON events
// ─────────────────────────────────────────────────────────────────────────────

/// Extract per-case activity traces from raw OCEL 2.0 event JSON values.
///
/// Case identity derives from `attributes.uri` → `attributes.case_id` →
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

    fn test_model(template: DeclareTemplate, activities: Vec<&str>) -> DeclareModel {
        build_model(
            "test",
            vec![wasm_constraint(template, activities.into_iter().map(act).collect())],
        )
    }

    #[test]
    fn response_constraint_fires_when_b_absent() {
        let model = test_model(DeclareTemplate::Response, vec!["A", "B"]);
        let mut traces = HashMap::new();
        traces.insert("case1".to_string(), vec!["A".to_string(), "C".to_string()]);
        let violations = model.check(&traces);
        assert!(!violations.is_empty());
        assert!(violations[0].constraint.contains("response"));
    }

    #[test]
    fn not_co_existence_fires_when_both_present() {
        let model = test_model(DeclareTemplate::NotCoExistence, vec!["Admit", "Block"]);
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
