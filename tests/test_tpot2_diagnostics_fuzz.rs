//! Randomized invariant coverage for the TPOT2 diagnostic mapper
//! ([`lsp_max_protocol::pipeline::diagnostics_for_search`]).
//!
//! The three-state law — `ADMITTED`, `REFUSED`, and `UNKNOWN` are distinct and
//! `UNKNOWN` is never coerced into either polarity — is the most load-bearing
//! invariant in this codebase. The existing unit tests pin a handful of fixed
//! points. This suite hammers the mapper across a large deterministic input
//! space (status bands, a fitness grid including out-of-range and NaN values,
//! and an admission-threshold grid) and asserts the law holds for every input.
//!
//! Per project law a passing run is a transcript of this test, not a release
//! receipt. The only out-of-band tokens here ("WINNER", the empty string, junk)
//! are deliberate fuzz inputs: they probe the unrecognized-status branch and
//! confirm the mapper stays total and emits nothing rather than guessing a
//! polarity.

use lsp_max_protocol::diagnostics::MaxDiagnostic;
use lsp_max_protocol::pipeline::{
    diagnostics_for_search, TPOT2_EMPTY_POOL, TPOT2_NONCONVERGENCE, TPOT2_OCEL_MISSING,
};
use lsp_types_max::{DiagnosticSeverity, NumberOrString};

// ---------------------------------------------------------------------------
// Deterministic PRNG — xorshift64*, no external crates.
// ---------------------------------------------------------------------------

/// A tiny deterministic generator so the whole sweep is reproducible across
/// runs and machines. Seeded once per test; the same seed yields the same
/// trajectory, so a counterexample is always replayable.
struct XorShift64(u64);

impl XorShift64 {
    fn new(seed: u64) -> Self {
        // A zero state is degenerate for xorshift; nudge it off zero.
        XorShift64(seed | 1)
    }

    fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_f491_4f6c_dd1d)
    }

    /// A value in [0.0, 1.0] derived from the top 53 bits (full f64 mantissa).
    fn next_unit_f64(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 / ((1u64 << 53) as f64)
    }
}

// ---------------------------------------------------------------------------
// Input space.
// ---------------------------------------------------------------------------

/// Bounded status bands plus deliberate out-of-band junk tokens. The junk
/// tokens exercise the unrecognized-status branch; they must never coax the
/// mapper into an ADMITTED- or REFUSED-shaped emission.
const STATUS_BANDS: &[&str] = &[
    "ADMITTED",
    "PARTIAL",
    "UNKNOWN",
    "REFUSED",
    "BLOCKED",
    // Out-of-band fuzz inputs (deliberate):
    "WINNER",
    "",
    "admitted",
    "Unknown",
    "TPOT2-EMPTY-POOL",
    "\u{1f600}junk",
];

/// Fitness values that exercise the in-range grid and the pathological edges:
/// out-of-range below and above, plus the IEEE-754 specials.
fn pathological_fitness() -> Vec<f64> {
    vec![
        -1.0,
        2.0,
        f64::NAN,
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::MIN,
        f64::MAX,
        -0.0,
    ]
}

/// Classify the `code` of an emitted diagnostic into one of the three known
/// TPOT2 family strings. A `None` means the code is not one of the three —
/// invariant 5 forbids that.
fn diagnostic_code(diag: &MaxDiagnostic) -> Option<&str> {
    match &diag.lsp.code {
        Some(NumberOrString::String(s)) => Some(s.as_str()),
        _ => None,
    }
}

/// True iff the diagnostic set contains the given TPOT2 code.
fn contains_code(diags: &[MaxDiagnostic], code: &str) -> bool {
    diags.iter().filter_map(diagnostic_code).any(|c| c == code)
}

/// True iff any diagnostic in the set carries Error severity — the REFUSED
/// polarity in this mapper.
fn has_error_severity(diags: &[MaxDiagnostic]) -> bool {
    diags
        .iter()
        .any(|d| d.lsp.severity == Some(DiagnosticSeverity::ERROR))
}

/// The three TPOT2 codes that are the only admissible `code` values.
const KNOWN_CODES: &[&str] = &[TPOT2_EMPTY_POOL, TPOT2_NONCONVERGENCE, TPOT2_OCEL_MISSING];

/// Assert the universal per-input invariants that must hold for EVERY status,
/// fitness, and threshold combination. Returns a context string for failure
/// messages. This is the heart of the suite — every driver funnels through it.
fn assert_universal_invariants(status: &str, fitness: f64, threshold: f64) {
    let diags = diagnostics_for_search(status, fitness, threshold);
    let ctx = format!("status={status:?} fitness={fitness} threshold={threshold}");

    // Invariant 5: every emitted code is one of the three known TPOT2 codes.
    for diag in &diags {
        let code = diagnostic_code(diag).unwrap_or_else(|| {
            panic!("emitted a diagnostic with a non-string / absent code ({ctx})")
        });
        assert!(
            KNOWN_CODES.contains(&code),
            "emitted a stray code {code:?} not in the known TPOT2 family ({ctx})"
        );
    }

    // Invariant 1: the UNKNOWN band is preserved — never coerced to ADMITTED
    // (no diagnostics) nor to REFUSED (empty-pool code / Error severity).
    if status == "UNKNOWN" {
        assert!(
            !contains_code(&diags, TPOT2_EMPTY_POOL),
            "UNKNOWN collapsed into the REFUSED (empty-pool) code ({ctx})"
        );
        assert!(
            !has_error_severity(&diags),
            "UNKNOWN surfaced a REFUSED-polarity (Error) diagnostic ({ctx})"
        );
        assert!(
            !diags.is_empty(),
            "UNKNOWN collapsed into ADMITTED (no diagnostics emitted) ({ctx})"
        );
        // The OCEL-missing signal is the UNKNOWN band's marker; it must persist
        // regardless of fitness or threshold.
        assert!(
            contains_code(&diags, TPOT2_OCEL_MISSING),
            "UNKNOWN dropped its OCEL-missing marker ({ctx})"
        );
    }

    // Invariant 2: REFUSED yields the empty-pool code at Error severity.
    if status == "REFUSED" {
        assert!(
            contains_code(&diags, TPOT2_EMPTY_POOL),
            "REFUSED did not yield the empty-pool code ({ctx})"
        );
        let empty_pool_is_error = diags.iter().any(|d| {
            diagnostic_code(d) == Some(TPOT2_EMPTY_POOL)
                && d.lsp.severity == Some(DiagnosticSeverity::ERROR)
        });
        assert!(
            empty_pool_is_error,
            "REFUSED empty-pool code was not Error severity ({ctx})"
        );
    }

    // Invariant 3: ADMITTED above threshold is a clean admission (no TPOT2
    // diagnostics) and crucially never emits the UNKNOWN/OCEL-missing code.
    if status == "ADMITTED" {
        assert!(
            !contains_code(&diags, TPOT2_OCEL_MISSING),
            "ADMITTED leaked the UNKNOWN (OCEL-missing) code ({ctx})"
        );
        // Above the bar, an admitted outcome is clean. (NaN comparisons are
        // always false, so a NaN fitness is excluded from this clause — the
        // total-ness check below still covers it.)
        if fitness > threshold {
            assert!(
                diags.is_empty(),
                "ADMITTED above threshold must emit no TPOT2 diagnostic ({ctx})"
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Fuzz drivers. Each test seeds the PRNG and counts the inputs it sweeps so the
// reported sweep size is exact.
// ---------------------------------------------------------------------------

/// Grid sweep across the in-range fitness/threshold lattice for every status
/// band. This is the dense deterministic core: it does not use randomness so
/// the lattice is exhaustively covered at the chosen resolution.
#[test]
fn grid_sweep_preserves_three_state_law() {
    const STEPS: usize = 41; // 0.000, 0.025, ... 1.000
    let mut swept = 0u64;

    for &status in STATUS_BANDS {
        for fi in 0..STEPS {
            let fitness = fi as f64 / (STEPS - 1) as f64;
            for ti in 0..STEPS {
                let threshold = ti as f64 / (STEPS - 1) as f64;
                assert_universal_invariants(status, fitness, threshold);
                swept += 1;
            }
        }
    }

    let expected = STATUS_BANDS.len() as u64 * STEPS as u64 * STEPS as u64;
    assert_eq!(
        swept, expected,
        "grid sweep input count drifted from the expected lattice size"
    );
    // Lower bound documents the sweep is large, not a token sample.
    assert!(
        swept >= 18_000,
        "grid sweep covered only {swept} inputs; expected a large lattice"
    );
}

/// Randomized sweep over in-range fitness/threshold pairs for every status,
/// driven by the deterministic PRNG. Complements the grid with off-lattice
/// values (irrational-looking points the regular grid never lands on).
#[test]
fn randomized_inrange_sweep_preserves_three_state_law() {
    let mut rng = XorShift64::new(0x5eed_7a02_c0ff_ee01);
    const SAMPLES_PER_STATUS: usize = 4_000;
    let mut swept = 0u64;

    for &status in STATUS_BANDS {
        for _ in 0..SAMPLES_PER_STATUS {
            let fitness = rng.next_unit_f64();
            let threshold = rng.next_unit_f64();
            assert_universal_invariants(status, fitness, threshold);
            swept += 1;
        }
    }

    let expected = STATUS_BANDS.len() as u64 * SAMPLES_PER_STATUS as u64;
    assert_eq!(
        swept, expected,
        "randomized sweep input count drifted from the expected sample budget"
    );
    assert!(
        swept >= 40_000,
        "randomized sweep covered only {swept} inputs; expected a large sample"
    );
}

/// Adversarial sweep: out-of-range fitness, IEEE-754 specials (incl. NaN), and
/// junk statuses crossed against a randomized threshold. This is the totality
/// witness — the mapper must not panic on any pathological input (invariant 4),
/// and the three-state law must still hold (invariants 1-3, 5 via the shared
/// checker).
#[test]
fn pathological_inputs_stay_total_and_bounded() {
    let mut rng = XorShift64::new(0x0bad_f00d_dead_beef);
    let bad_fitness = pathological_fitness();
    const THRESHOLDS_PER_CELL: usize = 64;
    let mut swept = 0u64;

    for &status in STATUS_BANDS {
        for &fitness in &bad_fitness {
            // A spread of thresholds: the in-range edges, plus randomized and
            // pathological thresholds (a NaN threshold is itself adversarial).
            for ti in 0..THRESHOLDS_PER_CELL {
                let threshold = match ti {
                    0 => 0.0,
                    1 => 1.0,
                    2 => f64::NAN,
                    3 => -1.0,
                    4 => 2.0,
                    _ => rng.next_unit_f64(),
                };
                // Invariant 4: no panic. assert_universal_invariants invokes the
                // mapper directly; reaching the post-call asserts proves totality
                // for this input.
                assert_universal_invariants(status, fitness, threshold);
                swept += 1;
            }
        }
    }

    let expected =
        STATUS_BANDS.len() as u64 * bad_fitness.len() as u64 * THRESHOLDS_PER_CELL as u64;
    assert_eq!(
        swept, expected,
        "pathological sweep input count drifted from the expected cell budget"
    );
    assert!(
        swept >= 5_000,
        "pathological sweep covered only {swept} inputs; expected broad edge coverage"
    );
}

/// Focused negative-control sweep on the UNKNOWN band alone, across the full
/// fitness/threshold cross-product including pathological values. The point is
/// to overweight the single most load-bearing case: UNKNOWN must NEVER produce
/// the empty-pool code, NEVER an Error-severity diagnostic, and NEVER an empty
/// (ADMITTED-shaped) result — no matter the numeric inputs.
#[test]
fn unknown_band_never_collapses_under_any_numeric_input() {
    let mut rng = XorShift64::new(0xa11_c0de_1234_5678);
    let mut fitness_values: Vec<f64> = pathological_fitness();
    // Add a dense in-range fitness grid so the UNKNOWN band is probed thoroughly.
    for i in 0..50 {
        fitness_values.push(i as f64 / 49.0);
    }
    const THRESHOLDS_PER_FITNESS: usize = 50;
    let mut swept = 0u64;

    for &fitness in &fitness_values {
        for ti in 0..THRESHOLDS_PER_FITNESS {
            let threshold = if ti == 0 {
                f64::NAN
            } else {
                rng.next_unit_f64()
            };
            let diags = diagnostics_for_search("UNKNOWN", fitness, threshold);
            let ctx = format!("UNKNOWN fitness={fitness} threshold={threshold}");

            assert!(
                !contains_code(&diags, TPOT2_EMPTY_POOL),
                "UNKNOWN emitted the REFUSED empty-pool code ({ctx})"
            );
            assert!(
                !has_error_severity(&diags),
                "UNKNOWN emitted a REFUSED-polarity Error diagnostic ({ctx})"
            );
            assert!(
                !diags.is_empty(),
                "UNKNOWN emitted nothing, collapsing toward ADMITTED ({ctx})"
            );
            assert!(
                contains_code(&diags, TPOT2_OCEL_MISSING),
                "UNKNOWN lost its OCEL-missing marker ({ctx})"
            );
            // Every code on the UNKNOWN path is a known TPOT2 code (invariant 5).
            for diag in &diags {
                let code = diagnostic_code(diag)
                    .unwrap_or_else(|| panic!("UNKNOWN emitted a non-string code ({ctx})"));
                assert!(
                    KNOWN_CODES.contains(&code),
                    "UNKNOWN emitted a stray code {code:?} ({ctx})"
                );
            }
            swept += 1;
        }
    }

    let expected = fitness_values.len() as u64 * THRESHOLDS_PER_FITNESS as u64;
    assert_eq!(
        swept, expected,
        "UNKNOWN-band sweep input count drifted from the expected budget"
    );
    assert!(
        swept >= 2_500,
        "UNKNOWN-band sweep covered only {swept} inputs; expected dense coverage"
    );
}

/// The three distinct bands stay separated at a shared input point: ADMITTED,
/// REFUSED, and UNKNOWN must each map to a different emission shape so none can
/// be confused for another. Run across a randomized set of shared inputs.
#[test]
fn distinct_bands_remain_separated() {
    let mut rng = XorShift64::new(0xfeed_face_cafe_d00d);
    const POINTS: usize = 2_000;
    let mut swept = 0u64;

    for _ in 0..POINTS {
        let fitness = rng.next_unit_f64();
        let threshold = rng.next_unit_f64();

        let admitted = diagnostics_for_search("ADMITTED", fitness, threshold);
        let refused = diagnostics_for_search("REFUSED", fitness, threshold);
        let unknown = diagnostics_for_search("UNKNOWN", fitness, threshold);
        let ctx = format!("fitness={fitness} threshold={threshold}");

        // REFUSED carries Error; UNKNOWN never does; the two are not the same shape.
        assert!(
            has_error_severity(&refused),
            "REFUSED lost its Error polarity ({ctx})"
        );
        assert!(
            !has_error_severity(&unknown),
            "UNKNOWN took on REFUSED's Error polarity ({ctx})"
        );

        // The REFUSED empty-pool code is unique to REFUSED, never on the UNKNOWN
        // or ADMITTED paths.
        assert!(
            contains_code(&refused, TPOT2_EMPTY_POOL),
            "REFUSED dropped the empty-pool code ({ctx})"
        );
        assert!(
            !contains_code(&unknown, TPOT2_EMPTY_POOL),
            "UNKNOWN borrowed REFUSED's empty-pool code ({ctx})"
        );
        assert!(
            !contains_code(&admitted, TPOT2_EMPTY_POOL),
            "ADMITTED borrowed REFUSED's empty-pool code ({ctx})"
        );

        // The UNKNOWN OCEL-missing code is unique to UNKNOWN, never on the
        // ADMITTED or REFUSED paths.
        assert!(
            contains_code(&unknown, TPOT2_OCEL_MISSING),
            "UNKNOWN dropped its OCEL-missing code ({ctx})"
        );
        assert!(
            !contains_code(&admitted, TPOT2_OCEL_MISSING),
            "ADMITTED borrowed UNKNOWN's OCEL-missing code ({ctx})"
        );
        assert!(
            !contains_code(&refused, TPOT2_OCEL_MISSING),
            "REFUSED borrowed UNKNOWN's OCEL-missing code ({ctx})"
        );

        swept += 1;
    }

    assert_eq!(swept as usize, POINTS, "band-separation point count drifted");
}
