use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// LawAxis — replaces ad-hoc string law_ids
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum LawAxis {
    Protocol,
    Type,
    Fixture,
    Documentation,
    Release,
    Hook,
    Repair,
    Receipt,
    Security,
    Autopoiesis,
    Domain,
    Custom(String),
}

impl Default for LawAxis {
    fn default() -> Self {
        LawAxis::Custom(String::new())
    }
}

impl LawAxis {
    pub fn all_named() -> &'static [LawAxis] {
        &[
            LawAxis::Protocol,
            LawAxis::Type,
            LawAxis::Fixture,
            LawAxis::Documentation,
            LawAxis::Release,
            LawAxis::Hook,
            LawAxis::Repair,
            LawAxis::Receipt,
            LawAxis::Security,
            LawAxis::Autopoiesis,
            LawAxis::Domain,
        ]
    }
}

impl std::fmt::Display for LawAxis {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LawAxis::Protocol => write!(f, "Protocol"),
            LawAxis::Type => write!(f, "Type"),
            LawAxis::Fixture => write!(f, "Fixture"),
            LawAxis::Documentation => write!(f, "Documentation"),
            LawAxis::Release => write!(f, "Release"),
            LawAxis::Hook => write!(f, "Hook"),
            LawAxis::Repair => write!(f, "Repair"),
            LawAxis::Receipt => write!(f, "Receipt"),
            LawAxis::Security => write!(f, "Security"),
            LawAxis::Autopoiesis => write!(f, "Autopoiesis"),
            LawAxis::Domain => write!(f, "Domain"),
            LawAxis::Custom(s) => write!(f, "Custom({})", s),
        }
    }
}

// ---------------------------------------------------------------------------
// ConformanceGrade — DfLSS CTQ compiler-enforced grade levels
// ---------------------------------------------------------------------------

/// Typed grade derived from a raw conformance score.
///
/// DfLSS CTQ requires grade-level branching to be compiler-enforced rather
/// than stringly typed.  Use [`ConformanceGrade::from_score`] to convert the
/// raw `f64` produced by `LspInstance::conformance_score()`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ConformanceGrade {
    /// Score ≥ 100.0 — zero defects, fully conformant.
    Perfect,
    /// Score ≥ 75.0 — within acceptable operating bounds.
    Good,
    /// Score ≥ 50.0 — degraded; corrective action recommended.
    Degraded,
    /// Score < 50.0 — critical; immediate intervention required.
    Critical,
}

impl ConformanceGrade {
    /// Map a raw conformance score to its grade level.
    pub fn from_score(s: f64) -> Self {
        if s >= 100.0 {
            ConformanceGrade::Perfect
        } else if s >= 75.0 {
            ConformanceGrade::Good
        } else if s >= 50.0 {
            ConformanceGrade::Degraded
        } else {
            ConformanceGrade::Critical
        }
    }

    /// Return the canonical string label used in JSON responses.
    pub fn as_str(&self) -> &'static str {
        match self {
            ConformanceGrade::Perfect => "perfect",
            ConformanceGrade::Good => "good",
            ConformanceGrade::Degraded => "degraded",
            ConformanceGrade::Critical => "critical",
        }
    }
}

impl std::fmt::Display for ConformanceGrade {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

// ---------------------------------------------------------------------------
// ConformanceVector — doctrine-correct: Admitted/Refused/Unknown are distinct
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConformanceVector {
    /// Law axes that have been admitted (evidence present and valid)
    pub admitted: Vec<LawAxis>,
    /// Law axes that have been explicitly refused (evidence present, violation confirmed)
    pub refused: Vec<LawAxis>,
    /// Law axes where admissibility cannot be determined (NEVER collapsed into admitted or refused)
    pub unknown: Vec<LawAxis>,
    /// Derived score: 100 * admitted / (admitted + refused + unknown), None if all unknown
    pub score: Option<f64>,
    /// Whether unknown axes block release actuation
    pub strict_mode: bool,
}

impl ConformanceVector {
    pub fn all_admitted(&self) -> bool {
        self.refused.is_empty() && self.unknown.is_empty()
    }

    pub fn admits_release(&self) -> bool {
        self.refused.is_empty() && (!self.strict_mode || self.unknown.is_empty())
    }
}

impl Default for ConformanceVector {
    fn default() -> Self {
        Self {
            admitted: Vec::new(),
            refused: Vec::new(),
            unknown: Vec::new(),
            score: None,
            strict_mode: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn conformance_vector_all_admitted_empty_is_true() {
        let cv = ConformanceVector {
            admitted: vec![LawAxis::Protocol],
            refused: vec![],
            unknown: vec![],
            score: Some(100.0),
            strict_mode: true,
        };
        assert!(cv.all_admitted());
    }

    #[test]
    fn conformance_vector_all_admitted_with_refused_is_false() {
        let cv = ConformanceVector {
            admitted: vec![LawAxis::Protocol],
            refused: vec![LawAxis::Security],
            unknown: vec![],
            score: Some(50.0),
            strict_mode: true,
        };
        assert!(!cv.all_admitted());
    }

    #[test]
    fn conformance_vector_all_admitted_with_unknown_is_false() {
        let cv = ConformanceVector {
            admitted: vec![LawAxis::Protocol],
            refused: vec![],
            unknown: vec![LawAxis::Domain],
            score: None,
            strict_mode: true,
        };
        assert!(!cv.all_admitted());
    }

    #[test]
    fn conformance_vector_score_recomputes_from_grade_boundaries() {
        assert_eq!(
            ConformanceGrade::from_score(100.0),
            ConformanceGrade::Perfect
        );
        assert_eq!(ConformanceGrade::from_score(99.9), ConformanceGrade::Good);
        assert_eq!(ConformanceGrade::from_score(75.0), ConformanceGrade::Good);
        assert_eq!(
            ConformanceGrade::from_score(74.9),
            ConformanceGrade::Degraded
        );
        assert_eq!(
            ConformanceGrade::from_score(50.0),
            ConformanceGrade::Degraded
        );
        assert_eq!(
            ConformanceGrade::from_score(49.9),
            ConformanceGrade::Critical
        );
        assert_eq!(
            ConformanceGrade::from_score(0.0),
            ConformanceGrade::Critical
        );
    }

    #[test]
    fn admits_release_strict_mode_blocks_unknown() {
        let cv = ConformanceVector {
            admitted: vec![LawAxis::Protocol],
            refused: vec![],
            unknown: vec![LawAxis::Domain],
            score: None,
            strict_mode: true,
        };
        assert!(
            !cv.admits_release(),
            "strict_mode=true must block when unknown is non-empty"
        );
    }

    #[test]
    fn admits_release_non_strict_mode_allows_unknown() {
        let cv = ConformanceVector {
            admitted: vec![LawAxis::Protocol],
            refused: vec![],
            unknown: vec![LawAxis::Domain],
            score: None,
            strict_mode: false,
        };
        assert!(
            cv.admits_release(),
            "strict_mode=false must allow unknown axes"
        );
    }

    #[test]
    fn admits_release_refused_always_blocks_regardless_of_strict_mode() {
        for strict in [true, false] {
            let cv = ConformanceVector {
                admitted: vec![],
                refused: vec![LawAxis::Security],
                unknown: vec![],
                score: Some(0.0),
                strict_mode: strict,
            };
            assert!(
                !cv.admits_release(),
                "refused must block release regardless of strict_mode"
            );
        }
    }

    #[test]
    fn conformance_vector_default_is_strict_and_empty() {
        let cv = ConformanceVector::default();
        assert!(cv.admitted.is_empty());
        assert!(cv.refused.is_empty());
        assert!(cv.unknown.is_empty());
        assert!(cv.strict_mode);
        assert!(cv.score.is_none());
    }

    #[test]
    fn law_axis_all_named_has_no_custom_variants() {
        for axis in LawAxis::all_named() {
            assert!(
                !matches!(axis, LawAxis::Custom(_)),
                "all_named must not include Custom variants"
            );
        }
    }

    #[test]
    fn law_axis_custom_display() {
        let axis = LawAxis::Custom("my-law".to_string());
        assert_eq!(axis.to_string(), "Custom(my-law)");
    }

    #[test]
    fn conformance_grade_as_str_matches_display() {
        let grades = [
            ConformanceGrade::Perfect,
            ConformanceGrade::Good,
            ConformanceGrade::Degraded,
            ConformanceGrade::Critical,
        ];
        for g in &grades {
            assert_eq!(g.as_str(), g.to_string().as_str());
        }
    }

    #[test]
    fn conformance_vector_serde_roundtrip() {
        let cv = ConformanceVector {
            admitted: vec![LawAxis::Protocol, LawAxis::Security],
            refused: vec![LawAxis::Hook],
            unknown: vec![LawAxis::Domain],
            score: Some(66.7),
            strict_mode: false,
        };
        let json = serde_json::to_string(&cv).expect("serialize");
        let cv2: ConformanceVector = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(cv2.admitted.len(), 2);
        assert_eq!(cv2.refused.len(), 1);
        assert_eq!(cv2.unknown.len(), 1);
        assert!((cv2.score.unwrap() - 66.7).abs() < 1e-9);
        assert!(!cv2.strict_mode);
    }
}
