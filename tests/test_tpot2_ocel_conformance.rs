//! Integration coverage for the object-centric conformance grounding and the
//! phase-shift model, exercised against the real OCEL 2.0 fixture
//! `tests/fixtures/tpot2/sample.ocel.json`.
//!
//! Build boundary: this test compiles against the `lsp-max` crate, which depends
//! on the sibling `lsp-types-max` checkout. Where that sibling is absent (Claude
//! Code web sessions) the workspace build is BLOCKED and this test does not run;
//! the same logic is verified in isolation by `scripts/tpot2-harness-verify.sh`.

use lsp_max::pipeline::ocel::{read_ocel_log, LogProfile};
use lsp_max::pipeline::phase::{phase_for, ConformancePhase, PhaseInput};

const FIXTURE: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/fixtures/tpot2/sample.ocel.json"
);

#[test]
fn reads_the_real_ocel_fixture() {
    let log = read_ocel_log(FIXTURE).expect("sample OCEL fixture must read");
    assert_eq!(log.events.len(), 6, "fixture has six events e1..e6");
    assert_eq!(log.objects.len(), 6, "fixture has six objects");
}

// NEGATIVE CONTROL: an absent source is never fabricated into a log.
#[test]
fn absent_source_is_none() {
    assert!(read_ocel_log("/no/such/fixture.ocel.json").is_none());
}

#[test]
fn fixture_profile_is_bounded_and_structurally_nontrivial() {
    let log = read_ocel_log(FIXTURE).unwrap();
    let p = LogProfile::from_log(&log);
    for v in [
        p.activity_variety,
        p.object_type_spread,
        p.temporal_density,
        p.divergence,
        p.convergence,
        p.df_density,
    ] {
        assert!((0.0..=1.0).contains(&v), "signal {v} out of [0,1]");
    }
    // Order o1 runs place -> pick -> pick -> deliver: a multi-event, divergent
    // trace (the repeated "pick item"), with object types Order/Item/Customer.
    assert!(
        p.temporal_density > 0.0,
        "multi-event object traces present"
    );
    assert!(
        p.divergence > 0.0,
        "a repeated per-object activity is present"
    );
    assert!(p.object_type_spread > 0.0, "multiple object types present");
}

#[test]
fn temporal_breed_helps_on_the_real_log() {
    let log = read_ocel_log(FIXTURE).unwrap();
    let p = LogProfile::from_log(&log);
    let without = vec!["asp".to_string(), "cbr".to_string()];
    let with = vec!["asp".to_string(), "ltl_monitor".to_string()];
    assert!(
        p.demand_match(&with) > p.demand_match(&without),
        "a Temporal breed must raise log-grounded fitness on a temporal log"
    );
}

#[test]
fn demand_match_drives_the_phase_across_the_boiling_point() {
    let log = read_ocel_log(FIXTURE).unwrap();
    let p = LogProfile::from_log(&log);
    let score = p.demand_match(&[
        "ltl_monitor".to_string(),
        "asp".to_string(),
        "bayesian_network".to_string(),
    ]);

    // A boiling point just under the score admits (Vapor); just over stays Liquid.
    let below_bp = (score - 0.01).max(0.0);
    let above_bp = (score + 0.01).min(1.0);
    let admitted = phase_for(&PhaseInput {
        andon_active: false,
        refused: false,
        unknown: false,
        conformance: score,
        boiling_point: below_bp,
    });
    let partial = phase_for(&PhaseInput {
        andon_active: false,
        refused: false,
        unknown: false,
        conformance: score,
        boiling_point: above_bp,
    });
    assert_eq!(admitted, ConformancePhase::Vapor);
    assert_eq!(admitted.expansion_factor(), 1700);
    assert_eq!(partial, ConformancePhase::Liquid);
}

// THREE-STATE: an undetermined measurement stays Unsettled regardless of how hot
// the (ignored) conformance reading is.
#[test]
fn unknown_measurement_never_boils() {
    let phase = phase_for(&PhaseInput {
        andon_active: false,
        refused: false,
        unknown: true,
        conformance: 1.0,
        boiling_point: 0.0,
    });
    assert_eq!(phase, ConformancePhase::Unsettled);
    assert_eq!(phase.as_status(), "UNKNOWN");
    assert_ne!(phase, ConformancePhase::Vapor);
}
