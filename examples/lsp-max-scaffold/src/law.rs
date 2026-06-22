/// Tri-state admission status for a single law axis.
///
/// The three states are disjoint and form the core of the conformance model:
///
/// - `Admitted` — all preconditions verified (receipt + transcript + neg-control)
/// - `Refused`  — the axis is intentionally rejected by law; refusal must be cited
/// - `Unknown`  — status not yet traced; **never** collapses to either other state
///
/// A diagnostic (`SCAFFOLD-AXIS-001`) is emitted whenever code attempts to
/// coerce `Unknown` into `Admitted` without producing the required evidence.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum AxisState {
    Admitted,
    Refused,
    Unknown,
}

impl std::fmt::Display for AxisState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AxisState::Admitted => write!(f, "ADMITTED"),
            AxisState::Refused => write!(f, "REFUSED"),
            AxisState::Unknown => write!(f, "UNKNOWN"),
        }
    }
}

/// Law axis carried by this scaffold's conformance vector.
#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ScaffoldAxis {
    /// LSP protocol surface: hover, diagnostics, definitions.
    Protocol,
    /// Receipt chain: sha256 digests, boundary markers, negative controls.
    Receipt,
    /// Gate compliance: ANDON clear before every Bash action.
    Gate,
    /// Ontology: TTL method declarations with law-status annotations.
    Ontology,
    /// Custom axis — use for domain-specific law enforcement.
    Custom(String),
}

/// Conformance vector for a single scaffold session.
///
/// Tracks the admission state of every law axis through three disjoint sets.
/// Each axis begins in `unknown` and may only transition to `admitted` once
/// its receipt chain is verified (see `AGENTS.md` Law #1).
///
/// The vector never merges its sets — the type system enforces the invariant
/// that an axis cannot be both `unknown` and `admitted` simultaneously.
///
/// Use `status_label()` for a single bounded-vocabulary summary token, and
/// `score()` for a numeric ratio. Neither output is a receipt.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct ScaffoldConformanceVector {
    pub admitted: Vec<ScaffoldAxis>,
    pub refused: Vec<ScaffoldAxis>,
    pub unknown: Vec<ScaffoldAxis>,
}

impl ScaffoldConformanceVector {
    pub fn new() -> Self {
        Self {
            admitted: vec![],
            refused: vec![],
            unknown: vec![
                ScaffoldAxis::Protocol,
                ScaffoldAxis::Receipt,
                ScaffoldAxis::Gate,
                ScaffoldAxis::Ontology,
            ],
        }
    }

    /// Promote an axis from UNKNOWN to ADMITTED when all preconditions pass.
    ///
    /// Silently ignores axes not currently in the unknown set — caller must not
    /// assume promotion happened; check the returned bool.
    pub fn admit_axis(&mut self, axis: ScaffoldAxis) -> bool {
        if let Some(pos) = self.unknown.iter().position(|a| a == &axis) {
            self.unknown.remove(pos);
            self.admitted.push(axis);
            true
        } else {
            false
        }
    }

    /// Record a law refusal for an axis that cannot be admitted.
    pub fn refuse_axis(&mut self, axis: ScaffoldAxis) -> bool {
        if let Some(pos) = self.unknown.iter().position(|a| a == &axis) {
            self.unknown.remove(pos);
            self.refused.push(axis);
            true
        } else {
            false
        }
    }

    /// Overall conformance score: admitted / (admitted + refused).
    ///
    /// Returns None when the denominator is zero (all axes still UNKNOWN).
    pub fn score(&self) -> Option<f64> {
        let denom = self.admitted.len() + self.refused.len();
        if denom == 0 {
            None
        } else {
            Some(self.admitted.len() as f64 / denom as f64)
        }
    }

    /// Bounded status label — never a victory assertion.
    pub fn status_label(&self) -> &'static str {
        if !self.refused.is_empty() {
            "REFUSED"
        } else if !self.unknown.is_empty() {
            "PARTIAL"
        } else if !self.admitted.is_empty() {
            "ADMITTED"
        } else {
            "UNKNOWN"
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unknown_does_not_collapse_to_admitted() {
        let v = ScaffoldConformanceVector::new();
        assert!(!v.unknown.is_empty(), "axes start UNKNOWN");
        assert!(v.admitted.is_empty());
        assert_eq!(v.status_label(), "PARTIAL");
    }

    #[test]
    fn admit_axis_transitions_from_unknown() {
        let mut v = ScaffoldConformanceVector::new();
        let promoted = v.admit_axis(ScaffoldAxis::Gate);
        assert!(promoted);
        assert!(!v.unknown.contains(&ScaffoldAxis::Gate));
        assert!(v.admitted.contains(&ScaffoldAxis::Gate));
    }

    #[test]
    fn refuse_axis_does_not_admit() {
        let mut v = ScaffoldConformanceVector::new();
        v.refuse_axis(ScaffoldAxis::Receipt);
        assert!(v.refused.contains(&ScaffoldAxis::Receipt));
        assert!(!v.admitted.contains(&ScaffoldAxis::Receipt));
        assert_eq!(v.status_label(), "REFUSED");
    }

    #[test]
    fn score_is_none_when_all_unknown() {
        let v = ScaffoldConformanceVector::new();
        assert!(v.score().is_none());
    }

    #[test]
    fn score_reflects_admitted_proportion() {
        let mut v = ScaffoldConformanceVector::new();
        v.admit_axis(ScaffoldAxis::Gate);
        v.refuse_axis(ScaffoldAxis::Receipt);
        let s = v.score().expect("score exists");
        assert!((s - 0.5).abs() < f64::EPSILON);
    }
}
