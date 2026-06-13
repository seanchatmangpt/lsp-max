use serde::{Deserialize, Serialize};
use wasm4pm_compat::conformance::ConformanceResult;

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
// LawAxisId — stable numeric index for named LawAxis variants
// ---------------------------------------------------------------------------

/// Stable numeric index for a named `LawAxis` variant.
///
/// All 11 named variants fit in bits 0–10 of a `u64`. `Custom` axes have no
/// stable numeric identity and are excluded from bitmask arithmetic.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct LawAxisId(pub u8);

impl LawAxisId {
    pub const PROTOCOL: LawAxisId = LawAxisId(0);
    pub const TYPE: LawAxisId = LawAxisId(1);
    pub const FIXTURE: LawAxisId = LawAxisId(2);
    pub const DOCUMENTATION: LawAxisId = LawAxisId(3);
    pub const RELEASE: LawAxisId = LawAxisId(4);
    pub const HOOK: LawAxisId = LawAxisId(5);
    pub const REPAIR: LawAxisId = LawAxisId(6);
    pub const RECEIPT: LawAxisId = LawAxisId(7);
    pub const SECURITY: LawAxisId = LawAxisId(8);
    pub const AUTOPOIESIS: LawAxisId = LawAxisId(9);
    pub const DOMAIN: LawAxisId = LawAxisId(10);
    /// Total number of named (non-Custom) axes.
    pub const MAX_NAMED: u8 = 11;

    /// Returns the bitmask for this axis — `1 << self.0`.
    pub fn bit(self) -> u64 {
        1u64 << self.0
    }
}

/// Bidirectional mapping between `LawAxis` and `LawAxisId`.
///
/// `Custom` axes return `None` from `axis_to_id` — they contribute no bit.
pub struct LawAxisRegistry;

impl LawAxisRegistry {
    pub fn axis_to_id(axis: &LawAxis) -> Option<LawAxisId> {
        match axis {
            LawAxis::Protocol => Some(LawAxisId::PROTOCOL),
            LawAxis::Type => Some(LawAxisId::TYPE),
            LawAxis::Fixture => Some(LawAxisId::FIXTURE),
            LawAxis::Documentation => Some(LawAxisId::DOCUMENTATION),
            LawAxis::Release => Some(LawAxisId::RELEASE),
            LawAxis::Hook => Some(LawAxisId::HOOK),
            LawAxis::Repair => Some(LawAxisId::REPAIR),
            LawAxis::Receipt => Some(LawAxisId::RECEIPT),
            LawAxis::Security => Some(LawAxisId::SECURITY),
            LawAxis::Autopoiesis => Some(LawAxisId::AUTOPOIESIS),
            LawAxis::Domain => Some(LawAxisId::DOMAIN),
            LawAxis::Custom(_) => None,
        }
    }

    pub fn id_to_axis(id: LawAxisId) -> LawAxis {
        match id.0 {
            0 => LawAxis::Protocol,
            1 => LawAxis::Type,
            2 => LawAxis::Fixture,
            3 => LawAxis::Documentation,
            4 => LawAxis::Release,
            5 => LawAxis::Hook,
            6 => LawAxis::Repair,
            7 => LawAxis::Receipt,
            8 => LawAxis::Security,
            9 => LawAxis::Autopoiesis,
            _ => LawAxis::Domain,
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
    /// Process quality from POWL conformance check. None until wasm4pm graduation.
    pub process_quality: Option<ConformanceResult>,
    /// Bitmask for admitted axes (bits 0–10 = Protocol..Domain). Not serialized.
    #[serde(skip)]
    pub admitted_bits: u64,
    /// Bitmask for refused axes. Not serialized.
    #[serde(skip)]
    pub refused_bits: u64,
    /// Bitmask for unknown axes. Not serialized.
    #[serde(skip)]
    pub unknown_bits: u64,
}

impl ConformanceVector {
    pub fn all_admitted(&self) -> bool {
        self.refused.is_empty() && self.unknown.is_empty()
    }

    pub fn admits_release(&self) -> bool {
        self.refused.is_empty() && (!self.strict_mode || self.unknown.is_empty())
    }

    /// Recompute all three bitmasks from the Vec fields.
    ///
    /// Call this after constructing from a struct literal or deserializing
    /// from JSON, so the internal index stays consistent.
    pub fn sync_bits_from_vecs(&mut self) {
        self.admitted_bits = Self::vecs_to_bits(&self.admitted);
        self.refused_bits = Self::vecs_to_bits(&self.refused);
        self.unknown_bits = Self::vecs_to_bits(&self.unknown);
        self.assert_bitmask_invariants();
    }

    fn vecs_to_bits(axes: &[LawAxis]) -> u64 {
        axes.iter()
            .filter_map(|a| LawAxisRegistry::axis_to_id(a))
            .fold(0u64, |acc, id| acc | id.bit())
    }

    /// Mark an axis as admitted, removing it from refused and unknown sets.
    pub fn set_admitted(&mut self, id: LawAxisId) {
        let bit = id.bit();
        self.refused_bits &= !bit;
        self.unknown_bits &= !bit;
        self.admitted_bits |= bit;
        debug_assert_eq!(self.admitted_bits & self.refused_bits, 0);
        debug_assert_eq!(self.admitted_bits & self.unknown_bits, 0);
    }

    /// Mark an axis as refused, removing it from admitted and unknown sets.
    pub fn set_refused(&mut self, id: LawAxisId) {
        let bit = id.bit();
        self.admitted_bits &= !bit;
        self.unknown_bits &= !bit;
        self.refused_bits |= bit;
        debug_assert_eq!(self.admitted_bits & self.refused_bits, 0);
        debug_assert_eq!(self.refused_bits & self.unknown_bits, 0);
    }

    /// Mark an axis as unknown, removing it from admitted and refused sets.
    pub fn set_unknown(&mut self, id: LawAxisId) {
        let bit = id.bit();
        self.admitted_bits &= !bit;
        self.refused_bits &= !bit;
        self.unknown_bits |= bit;
        debug_assert_eq!(self.admitted_bits & self.unknown_bits, 0);
        debug_assert_eq!(self.refused_bits & self.unknown_bits, 0);
    }

    pub fn is_admitted_bit(&self, id: LawAxisId) -> bool {
        self.admitted_bits & id.bit() != 0
    }
    pub fn is_refused_bit(&self, id: LawAxisId) -> bool {
        self.refused_bits & id.bit() != 0
    }
    pub fn is_unknown_bit(&self, id: LawAxisId) -> bool {
        self.unknown_bits & id.bit() != 0
    }

    /// Assert all three bitmasks are mutually disjoint.
    pub fn assert_bitmask_invariants(&self) {
        debug_assert_eq!(
            self.admitted_bits & self.refused_bits,
            0,
            "admitted and refused bits overlap"
        );
        debug_assert_eq!(
            self.admitted_bits & self.unknown_bits,
            0,
            "admitted and unknown bits overlap"
        );
        debug_assert_eq!(
            self.refused_bits & self.unknown_bits,
            0,
            "refused and unknown bits overlap"
        );
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
            process_quality: None,
            admitted_bits: 0,
            refused_bits: 0,
            unknown_bits: 0,
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
            process_quality: None,
            ..Default::default()
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
            process_quality: None,
            ..Default::default()
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
            process_quality: None,
            ..Default::default()
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
            process_quality: None,
            ..Default::default()
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
            process_quality: None,
            ..Default::default()
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
                process_quality: None,
                ..Default::default()
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
            process_quality: None,
            ..Default::default()
        };
        let json = serde_json::to_string(&cv).expect("serialize");
        let cv2: ConformanceVector = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(cv2.admitted.len(), 2);
        assert_eq!(cv2.refused.len(), 1);
        assert_eq!(cv2.unknown.len(), 1);
        assert!((cv2.score.unwrap() - 66.7).abs() < 1e-9);
        assert!(!cv2.strict_mode);
    }

    #[test]
    fn law_axis_registry_roundtrip_all_named() {
        let axes = [
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
        ];
        for axis in &axes {
            let id = LawAxisRegistry::axis_to_id(axis).expect("named axis has id");
            let back = LawAxisRegistry::id_to_axis(id);
            assert_eq!(std::mem::discriminant(axis), std::mem::discriminant(&back));
        }
    }

    #[test]
    fn law_axis_registry_custom_returns_none() {
        assert!(LawAxisRegistry::axis_to_id(&LawAxis::Custom("x".to_string())).is_none());
    }

    #[test]
    fn bitmask_disjointness_enforced_by_setters() {
        let mut cv = ConformanceVector::default();
        cv.set_admitted(LawAxisId::PROTOCOL);
        assert!(cv.is_admitted_bit(LawAxisId::PROTOCOL));
        assert!(!cv.is_refused_bit(LawAxisId::PROTOCOL));
        cv.set_refused(LawAxisId::PROTOCOL);
        assert!(!cv.is_admitted_bit(LawAxisId::PROTOCOL));
        assert!(cv.is_refused_bit(LawAxisId::PROTOCOL));
        cv.assert_bitmask_invariants();
    }

    #[test]
    fn sync_bits_from_vecs_matches_manual_setters() {
        let mut cv = ConformanceVector {
            admitted: vec![LawAxis::Protocol, LawAxis::Security],
            refused: vec![LawAxis::Receipt],
            unknown: vec![LawAxis::Domain],
            ..Default::default()
        };
        cv.sync_bits_from_vecs();
        assert!(cv.is_admitted_bit(LawAxisId::PROTOCOL));
        assert!(cv.is_admitted_bit(LawAxisId::SECURITY));
        assert!(cv.is_refused_bit(LawAxisId::RECEIPT));
        assert!(cv.is_unknown_bit(LawAxisId::DOMAIN));
        assert!(!cv.is_admitted_bit(LawAxisId::DOMAIN));
        cv.assert_bitmask_invariants();
    }

    #[test]
    fn bitmask_fields_skip_serde() {
        let mut cv = ConformanceVector::default();
        cv.set_admitted(LawAxisId::PROTOCOL);
        let json = serde_json::to_string(&cv).unwrap();
        assert!(!json.contains("admitted_bits"));
        assert!(!json.contains("refused_bits"));
        assert!(!json.contains("unknown_bits"));
    }
}
