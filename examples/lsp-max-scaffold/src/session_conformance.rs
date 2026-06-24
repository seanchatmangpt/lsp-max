//! Process-Mined Session Conformance (PMSC)
//!
//! Extends RVD from per-diagnostic proof to per-session process conformance.
//! Every event in a scaffold session becomes an OCEL 2.0 event bound to
//! multiple object types simultaneously. Declare constraints encode the
//! required process laws; token replay detects violations (Oracle classes
//! A8–A12) that single-receipt verification cannot see.
//!
//! Van der Aalst insight applied here: RVD proves individual findings are
//! honest, but a dishonest *process* can produce individually-honest findings
//! that violate causal, temporal, or epistemic laws. PMSC closes this gap.

use serde::{Deserialize, Serialize};

// ==============================================================================
// OCEL 2.0 Object-Centric Event Model
// ==============================================================================

/// One event in the session log, bound to multiple object types simultaneously.
/// `seq` is a monotonic counter — no wall-clock dependency, so replay is
/// deterministic even across machines and timezones.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SessionEvent {
    pub seq: u64,
    pub activity: EventActivity,
    pub objects: EventObjects,
}

/// Activity label for a session event.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "kind")]
pub enum EventActivity {
    DocumentOpened,
    AnalysisRun {
        source_digest: String,
    },
    FindingProduced {
        code: String,
    },
    ReceiptProduced {
        chain_head: String,
    },
    ReceiptVerified {
        admitted: bool,
    },
    ChainVerified {
        intact: bool,
    },
    GateChecked {
        blocked: bool,
    },
    /// Records an axis state transition.  `from` and `to` are the bounded
    /// vocabulary strings ("Unknown", "Admitted", "Refused").
    AxisTransitioned {
        axis: String,
        from: String,
        to: String,
    },
}

/// Object bindings for an OCEL 2.0 event — multiple object types per event.
/// All fields are optional; an event binds only the types relevant to it.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct EventObjects {
    pub document: Option<String>,
    pub finding: Option<String>,
    pub receipt: Option<String>,
    pub ruleset: Option<String>,
    pub axis: Option<String>,
    pub gate: Option<bool>,
}

// ==============================================================================
// Session Event Log
// ==============================================================================

/// An OCEL 2.0 session event log — serializable for persistence and replay.
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct SessionLog {
    events: Vec<SessionEvent>,
    counter: u64,
}

impl SessionLog {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn append(&mut self, activity: EventActivity, objects: EventObjects) {
        self.events.push(SessionEvent {
            seq: self.counter,
            activity,
            objects,
        });
        self.counter += 1;
    }

    pub fn events(&self) -> &[SessionEvent] {
        &self.events
    }

    /// Content-addressed digest of all events in this log.
    /// Used for tamper detection: the log digest travels with the log, so any
    /// post-hoc edit to the event sequence changes the digest.
    pub fn digest(&self) -> String {
        let mut h = blake3::Hasher::new();
        h.update(b"lsp-max-pmsc/session-log/v1\n");
        for ev in &self.events {
            h.update(&ev.seq.to_le_bytes());
            let encoded = serde_json::to_string(&ev.activity).unwrap_or_default();
            h.update(encoded.as_bytes());
            h.update(b"\n");
        }
        h.finalize().to_hex().to_string()
    }
}

// ==============================================================================
// Declare Constraint Model
// ==============================================================================

/// Discriminant-only classification of event activities, used for Declare
/// pattern matching without binding payload data.
#[derive(Debug, Clone, PartialEq)]
pub enum EventKind {
    DocumentOpened,
    AnalysisRun,
    FindingProduced,
    ReceiptProduced,
    ReceiptVerified,
    ChainVerified,
    GateChecked,
    AxisTransitioned,
}

impl EventKind {
    fn matches(&self, activity: &EventActivity) -> bool {
        matches!(
            (self, activity),
            (EventKind::DocumentOpened, EventActivity::DocumentOpened)
                | (EventKind::AnalysisRun, EventActivity::AnalysisRun { .. })
                | (
                    EventKind::FindingProduced,
                    EventActivity::FindingProduced { .. }
                )
                | (
                    EventKind::ReceiptProduced,
                    EventActivity::ReceiptProduced { .. }
                )
                | (
                    EventKind::ReceiptVerified,
                    EventActivity::ReceiptVerified { .. }
                )
                | (
                    EventKind::ChainVerified,
                    EventActivity::ChainVerified { .. }
                )
                | (EventKind::GateChecked, EventActivity::GateChecked { .. })
                | (
                    EventKind::AxisTransitioned,
                    EventActivity::AxisTransitioned { .. }
                )
        )
    }
}

/// Activity pattern used in Declare constraint definitions.
#[derive(Debug, Clone, PartialEq)]
pub enum ActivityPattern {
    Exact(EventKind),
    Any,
}

impl ActivityPattern {
    fn matches(&self, activity: &EventActivity) -> bool {
        match self {
            ActivityPattern::Any => true,
            ActivityPattern::Exact(kind) => kind.matches(activity),
        }
    }
}

/// A Declare temporal constraint over the session event log.
///
/// Declare is a declarative process modelling language: each constraint
/// specifies a law that ALL traces must obey rather than a single permitted
/// trace.  Van der Aalst uses Declare when the permitted space is large and
/// forbidden patterns are easier to enumerate than allowed ones.
#[derive(Debug, Clone)]
pub enum DeclareConstraint {
    /// Every occurrence of A must be followed by at least one B later.
    Response {
        antecedent: ActivityPattern,
        consequent: ActivityPattern,
    },
    /// B can only occur if A occurred at some earlier position.
    Precedence {
        predecessor: ActivityPattern,
        successor: ActivityPattern,
    },
    /// A must never occur in any trace.
    Absence { activity: ActivityPattern },
    /// A and B must not both appear in the same trace.
    NotCoexistence {
        first: ActivityPattern,
        second: ActivityPattern,
    },
}

// ==============================================================================
// Oracle Class Violations (A8–A12)
// ==============================================================================

/// Oracle classification for audit anomalies — adapted from van der Aalst's
/// Oracle taxonomy for law-state LSP session analysis.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub enum OracleClass {
    /// A8: A `ChainVerified(intact)` event follows a `ReceiptVerified(refused)`
    /// event — a refused receipt contaminates the chain, so an intact verdict
    /// afterwards is suspect (possible post-hoc chain substitution).
    A8AuditTampering,
    /// A9: A `FindingProduced` event appears after a `ChainVerified(broken)`
    /// event — findings may not be appended to a broken chain.
    A9TemporalAnomaly,
    /// A10: A `ReceiptProduced` event has no prior `AnalysisRun` — receipts
    /// cannot legitimately exist without a preceding analysis pass.
    A10CausalViolation,
    /// A11: An `AxisTransitioned(Unknown→Admitted|Refused)` event has no
    /// prior `ReceiptVerified` evidence — UNKNOWN must not collapse without proof.
    A11UnknownCollapse,
    /// A12: `GateChecked(blocked)` repeats ≥ 5 times without resolution —
    /// the session is in a non-terminating blocked loop.
    A12CyclicDependency,
}

/// A single Oracle class hit in a session replay.
#[derive(Debug, Clone, Serialize)]
pub struct OracleClassHit {
    pub class: OracleClass,
    pub event_index: usize,
    pub description: String,
}

// ==============================================================================
// Declare Constraint Violation
// ==============================================================================

#[derive(Debug, Clone, Serialize)]
pub struct ConstraintViolation {
    pub constraint_name: String,
    pub event_index: usize,
    pub description: String,
}

// ==============================================================================
// Token Replay & Fitness
// ==============================================================================

/// Token replay result — van der Aalst fitness metric plus law violations.
#[derive(Debug, Serialize)]
pub struct ReplayResult {
    /// Fitness ∈ [0, 1]; 1.0 means full conformance with all constraints.
    /// Computed as 1 - (violations + oracle_hits) / max(event_count, 1).
    pub fitness: f64,
    pub violations: Vec<ConstraintViolation>,
    pub oracle_hits: Vec<OracleClassHit>,
    pub status: &'static str,
}

/// The scaffold's built-in Declare constraint model.
///
/// These three constraints encode the core causal laws of the RVD lifecycle:
/// findings produce receipts; receipts require prior analysis; verification
/// requires a receipt to verify.
pub fn scaffold_constraint_model() -> Vec<DeclareConstraint> {
    vec![
        DeclareConstraint::Response {
            antecedent: ActivityPattern::Exact(EventKind::FindingProduced),
            consequent: ActivityPattern::Exact(EventKind::ReceiptProduced),
        },
        DeclareConstraint::Precedence {
            predecessor: ActivityPattern::Exact(EventKind::AnalysisRun),
            successor: ActivityPattern::Exact(EventKind::ReceiptProduced),
        },
        DeclareConstraint::Precedence {
            predecessor: ActivityPattern::Exact(EventKind::ReceiptProduced),
            successor: ActivityPattern::Exact(EventKind::ReceiptVerified),
        },
    ]
}

/// Run van der Aalst token replay against the scaffold Declare model.
///
/// The fitness metric follows the standard formula:
///   fitness = 1 - (total_issues / event_count)
/// floored at 0.  This is the simplified version of the alignment-based
/// fitness from Process Mining: Data Science in Action, Chapter 7.
pub fn replay_session(log: &SessionLog) -> ReplayResult {
    let constraints = scaffold_constraint_model();
    let events = log.events();
    let mut violations = Vec::new();
    let mut oracle_hits = Vec::new();

    for constraint in &constraints {
        check_constraint(constraint, events, &mut violations);
    }

    check_a8_audit_tampering(events, &mut oracle_hits);
    check_a9_temporal_anomaly(events, &mut oracle_hits);
    check_a10_causal_violation(events, &mut oracle_hits);
    check_a11_unknown_collapse(events, &mut oracle_hits);
    check_a12_cyclic_dependency(events, &mut oracle_hits);

    let total_issues = violations.len() + oracle_hits.len();
    let fitness = if events.is_empty() {
        1.0
    } else {
        (1.0_f64 - total_issues as f64 / events.len() as f64).max(0.0)
    };

    let status = if violations.is_empty() && oracle_hits.is_empty() {
        "ADMITTED"
    } else if fitness >= 0.5 {
        "PARTIAL"
    } else {
        "REFUSED"
    };

    ReplayResult {
        fitness,
        violations,
        oracle_hits,
        status,
    }
}

// ==============================================================================
// Declare Constraint Evaluation
// ==============================================================================

fn check_constraint(
    constraint: &DeclareConstraint,
    events: &[SessionEvent],
    violations: &mut Vec<ConstraintViolation>,
) {
    match constraint {
        DeclareConstraint::Response {
            antecedent,
            consequent,
        } => {
            for (i, ev) in events.iter().enumerate() {
                if antecedent.matches(&ev.activity) {
                    let satisfied = events[i + 1..]
                        .iter()
                        .any(|e| consequent.matches(&e.activity));
                    if !satisfied {
                        violations.push(ConstraintViolation {
                            constraint_name: "Response".to_string(),
                            event_index: i,
                            description: format!(
                                "seq={} triggered Response but consequent never followed",
                                ev.seq
                            ),
                        });
                    }
                }
            }
        }
        DeclareConstraint::Precedence {
            predecessor,
            successor,
        } => {
            for (i, ev) in events.iter().enumerate() {
                if successor.matches(&ev.activity) {
                    let satisfied = events[..i].iter().any(|e| predecessor.matches(&e.activity));
                    if !satisfied {
                        violations.push(ConstraintViolation {
                            constraint_name: "Precedence".to_string(),
                            event_index: i,
                            description: format!(
                                "seq={} successor occurred without required predecessor",
                                ev.seq
                            ),
                        });
                    }
                }
            }
        }
        DeclareConstraint::Absence { activity } => {
            for (i, ev) in events.iter().enumerate() {
                if activity.matches(&ev.activity) {
                    violations.push(ConstraintViolation {
                        constraint_name: "Absence".to_string(),
                        event_index: i,
                        description: format!("seq={} forbidden activity occurred", ev.seq),
                    });
                }
            }
        }
        DeclareConstraint::NotCoexistence { first, second } => {
            let has_first = events.iter().any(|e| first.matches(&e.activity));
            let has_second = events.iter().any(|e| second.matches(&e.activity));
            if has_first && has_second {
                violations.push(ConstraintViolation {
                    constraint_name: "NotCoexistence".to_string(),
                    event_index: 0,
                    description: "mutually exclusive activities both occurred".to_string(),
                });
            }
        }
    }
}

// ==============================================================================
// Oracle Class Detectors
// ==============================================================================

fn check_a8_audit_tampering(events: &[SessionEvent], hits: &mut Vec<OracleClassHit>) {
    for (i, ev) in events.iter().enumerate() {
        if matches!(ev.activity, EventActivity::ChainVerified { intact: true }) {
            let prior_refused = events[..i].iter().any(|e| {
                matches!(
                    e.activity,
                    EventActivity::ReceiptVerified { admitted: false }
                )
            });
            if prior_refused {
                hits.push(OracleClassHit {
                    class: OracleClass::A8AuditTampering,
                    event_index: i,
                    description: format!(
                        "A8: ChainVerified(intact) at seq={} follows a refused receipt — tampered chain suspected",
                        ev.seq
                    ),
                });
            }
        }
    }
}

fn check_a9_temporal_anomaly(events: &[SessionEvent], hits: &mut Vec<OracleClassHit>) {
    let mut broken_at: Option<usize> = None;
    for (i, ev) in events.iter().enumerate() {
        if matches!(ev.activity, EventActivity::ChainVerified { intact: false }) {
            broken_at = Some(i);
        }
        if let Some(bi) = broken_at {
            if matches!(ev.activity, EventActivity::FindingProduced { .. }) && i > bi {
                hits.push(OracleClassHit {
                    class: OracleClass::A9TemporalAnomaly,
                    event_index: i,
                    description: format!(
                        "A9: FindingProduced at seq={} after chain broken at seq={}",
                        ev.seq, events[bi].seq
                    ),
                });
            }
        }
    }
}

fn check_a10_causal_violation(events: &[SessionEvent], hits: &mut Vec<OracleClassHit>) {
    for (i, ev) in events.iter().enumerate() {
        if matches!(ev.activity, EventActivity::ReceiptProduced { .. }) {
            let prior_analysis = events[..i]
                .iter()
                .any(|e| matches!(e.activity, EventActivity::AnalysisRun { .. }));
            if !prior_analysis {
                hits.push(OracleClassHit {
                    class: OracleClass::A10CausalViolation,
                    event_index: i,
                    description: format!(
                        "A10: ReceiptProduced at seq={} without prior AnalysisRun",
                        ev.seq
                    ),
                });
            }
        }
    }
}

fn check_a11_unknown_collapse(events: &[SessionEvent], hits: &mut Vec<OracleClassHit>) {
    for (i, ev) in events.iter().enumerate() {
        if let EventActivity::AxisTransitioned { from, to, .. } = &ev.activity {
            if from == "Unknown" && (to == "Admitted" || to == "Refused") {
                let has_evidence = events[..i]
                    .iter()
                    .rev()
                    .any(|e| matches!(e.activity, EventActivity::ReceiptVerified { .. }));
                if !has_evidence {
                    hits.push(OracleClassHit {
                        class: OracleClass::A11UnknownCollapse,
                        event_index: i,
                        description: format!(
                            "A11: Unknown collapsed to {to} at seq={} without ReceiptVerified evidence",
                            ev.seq
                        ),
                    });
                }
            }
        }
    }
}

fn check_a12_cyclic_dependency(events: &[SessionEvent], hits: &mut Vec<OracleClassHit>) {
    const CYCLE_THRESHOLD: usize = 5;
    let mut blocked_run = 0usize;
    for (i, ev) in events.iter().enumerate() {
        match &ev.activity {
            EventActivity::GateChecked { blocked: true } => {
                blocked_run += 1;
                if blocked_run == CYCLE_THRESHOLD {
                    hits.push(OracleClassHit {
                        class: OracleClass::A12CyclicDependency,
                        event_index: i,
                        description: format!(
                            "A12: GateChecked(blocked) repeated {blocked_run} times without resolution at seq={}",
                            ev.seq
                        ),
                    });
                }
            }
            EventActivity::GateChecked { blocked: false }
            | EventActivity::AxisTransitioned { .. } => {
                blocked_run = 0;
            }
            _ => {}
        }
    }
}

// ==============================================================================
// Tests
// ==============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn analysis_run() -> EventActivity {
        EventActivity::AnalysisRun {
            source_digest: "abc123".to_string(),
        }
    }

    fn finding() -> EventActivity {
        EventActivity::FindingProduced {
            code: "TEST-001".to_string(),
        }
    }

    fn receipt() -> EventActivity {
        EventActivity::ReceiptProduced {
            chain_head: "deadbeef".to_string(),
        }
    }

    fn verified_ok() -> EventActivity {
        EventActivity::ReceiptVerified { admitted: true }
    }

    fn append(log: &mut SessionLog, activity: EventActivity) {
        log.append(activity, EventObjects::default());
    }

    #[test]
    fn empty_log_is_fully_admitted() {
        let log = SessionLog::new();
        let r = replay_session(&log);
        assert_eq!(r.status, "ADMITTED");
        assert!((r.fitness - 1.0).abs() < f64::EPSILON);
        assert!(r.violations.is_empty());
        assert!(r.oracle_hits.is_empty());
    }

    #[test]
    fn honest_session_is_admitted() {
        let mut log = SessionLog::new();
        append(&mut log, EventActivity::DocumentOpened);
        append(&mut log, analysis_run());
        append(&mut log, finding());
        append(&mut log, receipt());
        append(&mut log, verified_ok());
        append(&mut log, EventActivity::ChainVerified { intact: true });
        let r = replay_session(&log);
        assert_eq!(r.status, "ADMITTED");
        assert!(r.violations.is_empty());
        assert!(r.oracle_hits.is_empty());
    }

    #[test]
    fn finding_without_subsequent_receipt_violates_response() {
        let mut log = SessionLog::new();
        append(&mut log, analysis_run());
        append(&mut log, finding()); // no ReceiptProduced follows
        let r = replay_session(&log);
        assert!(!r.violations.is_empty());
        assert!(r.violations.iter().any(|v| v.constraint_name == "Response"));
    }

    #[test]
    fn receipt_without_prior_analysis_violates_precedence_and_a10() {
        let mut log = SessionLog::new();
        append(&mut log, receipt()); // no AnalysisRun before
        let r = replay_session(&log);
        let has_precedence = r
            .violations
            .iter()
            .any(|v| v.constraint_name == "Precedence");
        let has_a10 = r
            .oracle_hits
            .iter()
            .any(|h| h.class == OracleClass::A10CausalViolation);
        assert!(has_precedence, "should detect Precedence violation");
        assert!(has_a10, "should detect A10 causal violation");
    }

    #[test]
    fn unknown_collapse_without_evidence_is_a11() {
        let mut log = SessionLog::new();
        // No ReceiptVerified before the transition
        append(
            &mut log,
            EventActivity::AxisTransitioned {
                axis: "Gate".to_string(),
                from: "Unknown".to_string(),
                to: "Admitted".to_string(),
            },
        );
        let r = replay_session(&log);
        let has_a11 = r
            .oracle_hits
            .iter()
            .any(|h| h.class == OracleClass::A11UnknownCollapse);
        assert!(has_a11, "should detect A11 unknown collapse");
    }

    #[test]
    fn unknown_collapse_with_evidence_passes_a11() {
        let mut log = SessionLog::new();
        append(&mut log, analysis_run());
        append(&mut log, finding());
        append(&mut log, receipt());
        append(&mut log, verified_ok()); // evidence present
        append(
            &mut log,
            EventActivity::AxisTransitioned {
                axis: "Gate".to_string(),
                from: "Unknown".to_string(),
                to: "Admitted".to_string(),
            },
        );
        let r = replay_session(&log);
        let has_a11 = r
            .oracle_hits
            .iter()
            .any(|h| h.class == OracleClass::A11UnknownCollapse);
        assert!(
            !has_a11,
            "A11 must not fire when ReceiptVerified evidence precedes transition"
        );
    }

    #[test]
    fn finding_after_broken_chain_is_a9() {
        let mut log = SessionLog::new();
        append(&mut log, analysis_run());
        append(&mut log, EventActivity::ChainVerified { intact: false });
        append(&mut log, finding()); // after broken chain
        let r = replay_session(&log);
        let has_a9 = r
            .oracle_hits
            .iter()
            .any(|h| h.class == OracleClass::A9TemporalAnomaly);
        assert!(has_a9, "should detect A9 temporal anomaly");
    }

    #[test]
    fn repeated_gate_blocks_are_a12() {
        let mut log = SessionLog::new();
        for _ in 0..5 {
            append(&mut log, EventActivity::GateChecked { blocked: true });
        }
        let r = replay_session(&log);
        let has_a12 = r
            .oracle_hits
            .iter()
            .any(|h| h.class == OracleClass::A12CyclicDependency);
        assert!(
            has_a12,
            "should detect A12 cyclic dependency after 5 blocked checks"
        );
    }

    #[test]
    fn gate_blocks_interrupted_by_resolution_clears_a12() {
        let mut log = SessionLog::new();
        for _ in 0..4 {
            append(&mut log, EventActivity::GateChecked { blocked: true });
        }
        append(&mut log, EventActivity::GateChecked { blocked: false }); // resolves
        for _ in 0..4 {
            append(&mut log, EventActivity::GateChecked { blocked: true });
        }
        let r = replay_session(&log);
        let has_a12 = r
            .oracle_hits
            .iter()
            .any(|h| h.class == OracleClass::A12CyclicDependency);
        assert!(
            !has_a12,
            "A12 must not fire when run resets before threshold"
        );
    }

    #[test]
    fn refused_receipt_before_intact_chain_is_a8() {
        let mut log = SessionLog::new();
        append(&mut log, analysis_run());
        append(&mut log, finding());
        append(&mut log, receipt());
        append(&mut log, EventActivity::ReceiptVerified { admitted: false }); // refused
        append(&mut log, EventActivity::ChainVerified { intact: true }); // suspicious
        let r = replay_session(&log);
        let has_a8 = r
            .oracle_hits
            .iter()
            .any(|h| h.class == OracleClass::A8AuditTampering);
        assert!(has_a8, "should detect A8 audit tampering");
    }

    #[test]
    fn session_log_digest_is_deterministic() {
        let mut a = SessionLog::new();
        let mut b = SessionLog::new();
        for log in [&mut a, &mut b] {
            append(log, EventActivity::DocumentOpened);
            append(log, analysis_run());
        }
        assert_eq!(
            a.digest(),
            b.digest(),
            "identical events must yield identical digest"
        );
    }

    #[test]
    fn session_log_digest_changes_on_edit() {
        let mut log = SessionLog::new();
        append(&mut log, EventActivity::DocumentOpened);
        let before = log.digest();
        append(&mut log, analysis_run());
        let after = log.digest();
        assert_ne!(before, after, "digest must change when events are appended");
    }

    #[test]
    fn fitness_degrades_proportionally_to_violations() {
        let mut log = SessionLog::new();
        // 2 findings with no following receipts → 2 Response violations
        append(&mut log, analysis_run());
        append(&mut log, finding());
        append(&mut log, finding());
        let r = replay_session(&log);
        assert!(
            r.fitness < 1.0,
            "fitness must degrade below 1.0 when violations exist"
        );
        assert!(r.fitness >= 0.0, "fitness must not go negative");
    }
}
