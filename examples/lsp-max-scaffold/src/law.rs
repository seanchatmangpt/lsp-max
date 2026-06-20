/// Tri-state admission status for a law axis.
///
/// UNKNOWN must never collapse into ADMITTED or REFUSED — it signals a
/// tracing gap or unmet precondition, not a default.
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
/// A method row is ADMITTED only when all three axes satisfy their
/// preconditions. Unknown is never coerced to either polarity.
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
