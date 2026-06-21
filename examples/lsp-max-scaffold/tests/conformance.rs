/// Conformance vector law tests.
///
/// These tests assert the invariants that the law-state runtime must maintain.
/// They are NOT receipts — test stdout is not evidence of admission.
use lsp_max_scaffold::law::{ScaffoldAxis, ScaffoldConformanceVector};

#[test]
fn all_axes_start_unknown() {
    let v = ScaffoldConformanceVector::new();
    assert!(
        !v.unknown.is_empty(),
        "axes must start UNKNOWN, not pre-ADMITTED"
    );
    assert!(v.admitted.is_empty());
    assert!(v.refused.is_empty());
}

#[test]
fn unknown_never_collapses_to_admitted_without_evidence() {
    let v = ScaffoldConformanceVector::new();
    // Every axis that starts UNKNOWN must not appear in the admitted set —
    // the constructor must not pre-admit anything.
    for axis in &v.unknown {
        assert!(
            !v.admitted.contains(axis),
            "axis {axis:?} is in unknown but also in admitted — UNKNOWN collapsed without evidence"
        );
    }
    // And the admitted set must be empty from construction.
    assert!(
        v.admitted.is_empty(),
        "ScaffoldConformanceVector::new() must not pre-admit any axis"
    );
}

#[test]
fn admit_transitions_axis_from_unknown() {
    let mut v = ScaffoldConformanceVector::new();
    let was_unknown = v.unknown.contains(&ScaffoldAxis::Gate);
    let promoted = v.admit_axis(ScaffoldAxis::Gate);
    assert_eq!(
        was_unknown, promoted,
        "admit_axis returns true only when axis was UNKNOWN"
    );
    assert!(!v.unknown.contains(&ScaffoldAxis::Gate));
    assert!(v.admitted.contains(&ScaffoldAxis::Gate));
}

#[test]
fn refuse_axis_does_not_admit() {
    let mut v = ScaffoldConformanceVector::new();
    v.refuse_axis(ScaffoldAxis::Receipt);
    assert!(!v.admitted.contains(&ScaffoldAxis::Receipt));
    assert!(v.refused.contains(&ScaffoldAxis::Receipt));
}

#[test]
fn status_label_is_bounded_vocabulary() {
    let v = ScaffoldConformanceVector::new();
    let label = v.status_label();
    let allowed = [
        "ADMITTED",
        "REFUSED",
        "PARTIAL",
        "UNKNOWN",
        "BLOCKED",
        "CANDIDATE",
        "OPEN",
    ];
    assert!(
        allowed.contains(&label),
        "status label must be from bounded vocabulary, got: {label}"
    );
}

#[test]
fn score_none_when_no_axes_resolved() {
    let v = ScaffoldConformanceVector::new();
    assert!(
        v.score().is_none(),
        "score must be None when all axes are UNKNOWN (no denominator)"
    );
}

#[test]
fn partial_score_reflects_admitted_proportion() {
    let mut v = ScaffoldConformanceVector::new();
    v.admit_axis(ScaffoldAxis::Gate);
    v.admit_axis(ScaffoldAxis::Protocol);
    v.refuse_axis(ScaffoldAxis::Receipt);
    let score = v.score().expect("score available after resolutions");
    let expected = 2.0_f64 / 3.0_f64;
    assert!(
        (score - expected).abs() < 1e-10,
        "score should be 2/3, got {score}"
    );
}
