//! `doctor` noun — a one-shot, READ-ONLY environment & workspace health check.
//!
//! `lsp-max-cli doctor check` is meant to be the first thing anyone runs: it
//! reports a bounded status per precondition axis this workspace actually trips
//! over (missing siblings, version-floor mismatch, toolchain drift, thin disk,
//! committed conflict markers, broken path-dep depth, leaked artifacts, ANDON
//! gate). It observes only; it never mutates tracked files, manifests, or
//! sibling repositories.

mod engine;
mod util;

use clap_noun_verb::error::NounVerbError;
use clap_noun_verb::Result;
use clap_noun_verb_macros::verb;
use serde::Serialize;

pub use engine::DoctorService;

// ==============================================================================
// 1. Domain Tier
// ==============================================================================

/// Bounded status for a single diagnostic axis. UNKNOWN is a first-class state
/// and is never coerced into ADMITTED — it signals a precondition the doctor
/// could not observe (missing tool, missing input).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum Status {
    Admitted,
    Partial,
    Blocked,
    Unknown,
}

impl Status {
    /// Severity ordering for the roll-up: BLOCKED > PARTIAL > UNKNOWN > ADMITTED.
    fn severity(self) -> u8 {
        match self {
            Status::Admitted => 0,
            Status::Unknown => 1,
            Status::Partial => 2,
            Status::Blocked => 3,
        }
    }
}

/// One axis result: a stable identifier, its bounded status, and a fix hint.
#[derive(Debug, Clone, Serialize)]
pub struct Axis {
    pub axis: String,
    pub status: Status,
    /// Human-readable detail carrying the remediation hint.
    pub detail: String,
}

impl Axis {
    pub(crate) fn new(axis: &str, status: Status, detail: impl Into<String>) -> Self {
        Self {
            axis: axis.to_string(),
            status,
            detail: detail.into(),
        }
    }
}

/// The full read-only health report.
#[derive(Debug, Clone, Serialize)]
pub struct DoctorReport {
    pub root: String,
    pub axes: Vec<Axis>,
    /// ADMITTED only when every axis is ADMITTED; else the most severe observed.
    pub overall: Status,
}

impl DoctorReport {
    pub(crate) fn rollup(root: String, axes: Vec<Axis>) -> Self {
        let overall = axes
            .iter()
            .map(|a| a.status)
            .max_by_key(|s| s.severity())
            .unwrap_or(Status::Unknown);
        Self {
            root,
            axes,
            overall,
        }
    }
}

// ==============================================================================
// 2. Verb Tier
// ==============================================================================

/// Run the read-only environment & workspace health check. Reports a bounded
/// status per axis with a fix hint. Exits 1 when the overall verdict is BLOCKED;
/// otherwise exits 0 (PARTIAL/UNKNOWN are surfaced in the report, not as
/// failures — UNKNOWN is never collapsed into a polarity).
#[verb("check")]
pub fn check() -> Result<DoctorReport> {
    let report = DoctorService::new().diagnose();
    if report.overall == Status::Blocked {
        let blocked: Vec<String> = report
            .axes
            .iter()
            .filter(|a| a.status == Status::Blocked)
            .map(|a| a.axis.clone())
            .collect();
        return Err(NounVerbError::execution_error(format!(
            "doctor verdict BLOCKED — preconditions not met: {}",
            blocked.join(", ")
        )));
    }
    Ok(report)
}

// ==============================================================================
// 3. Tests
// ==============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rollup_admitted_only_when_all_axes_admitted() {
        let axes = vec![
            Axis::new("a", Status::Admitted, ""),
            Axis::new("b", Status::Admitted, ""),
        ];
        assert_eq!(
            DoctorReport::rollup("/r".into(), axes).overall,
            Status::Admitted
        );
    }

    #[test]
    fn rollup_picks_most_severe() {
        let axes = vec![
            Axis::new("a", Status::Admitted, ""),
            Axis::new("b", Status::Partial, ""),
            Axis::new("c", Status::Blocked, ""),
            Axis::new("d", Status::Unknown, ""),
        ];
        assert_eq!(
            DoctorReport::rollup("/r".into(), axes).overall,
            Status::Blocked
        );
    }

    #[test]
    fn unknown_does_not_collapse_into_admitted() {
        // A lone UNKNOWN axis must surface as UNKNOWN, never ADMITTED.
        let axes = vec![Axis::new("a", Status::Unknown, "")];
        assert_eq!(
            DoctorReport::rollup("/r".into(), axes).overall,
            Status::Unknown
        );
    }

    #[test]
    fn partial_outranks_unknown() {
        let axes = vec![
            Axis::new("a", Status::Unknown, ""),
            Axis::new("b", Status::Partial, ""),
        ];
        assert_eq!(
            DoctorReport::rollup("/r".into(), axes).overall,
            Status::Partial
        );
    }

    #[test]
    fn report_serializes_with_uppercase_status() {
        let axes = vec![Axis::new("a", Status::Admitted, "ok")];
        let report = DoctorReport::rollup("/r".into(), axes);
        let json = serde_json::to_string(&report).unwrap();
        assert!(json.contains("\"overall\":\"ADMITTED\""));
        assert!(json.contains("\"status\":\"ADMITTED\""));
    }
}
