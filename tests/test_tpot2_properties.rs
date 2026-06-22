//! Property-based invariant suite for the TPOT2 breed-pipeline search engine.
//!
//! This is a hand-rolled property tester — no external proptest/quickcheck
//! crate. A deterministic driver generates many randomized [`PipelineSearchConfig`]
//! values and seeds, runs the library search over each, and asserts that a set of
//! universal invariants holds across the whole sweep.
//!
//! The motivating regression: a single PRNG seed once cancelled the xorshift
//! state to zero, freezing the stream and collapsing every search to a single
//! breed. A positive-case unit test on one lucky seed never observes that;
//! universally-quantified invariants over a seed/config sweep do. Invariant
//! [`prop_non_degeneracy_multi_node_pipeline_reachable`] is the explicit guard
//! for that class of defect.
//!
//! Law note: test stdout is not a receipt under project law. These assertions
//! are the evidence — every check is a real `assert!` with an informative
//! message, never a bare `println!`.

use lsp_max::pipeline::catalog::KNOWN_BREEDS;
use lsp_max::pipeline::search::{DiversityFitnessEvaluator, PipelineSearch};
use lsp_max::pipeline::types::{
    PipelineBoundedStatus, PipelineSearchConfig, PipelineSearchResult,
};

/// A single generated test point: one config paired with one seed.
#[derive(Clone)]
struct Case {
    config: PipelineSearchConfig,
    seed: u64,
}

/// Deterministic counter-driven seed spreader.
///
/// xorshift-style mixing of a monotonic counter so the seed set is varied but
/// fully reproducible run-to-run. Not the engine PRNG — only the driver's way
/// of choosing which seeds to probe.
fn spread_seed(counter: u64) -> u64 {
    let mut x = counter.wrapping_mul(0x9e3779b97f4a7c15).wrapping_add(0x1234_5678);
    x ^= x >> 30;
    x = x.wrapping_mul(0xbf58_476d_1ce4_e5b9);
    x ^= x >> 27;
    x
}

/// Build the canonical sweep of generated cases.
///
/// Varies population_size 1..=30, generations 0..=20, admission_threshold
/// across 0.0..=1.0, and min/max pipeline length within sane bounds
/// (min in 1..=3, max in min..=6). The breed pool is supplied by the caller so
/// the same generator drives both the small-pool and full-catalog sweeps.
///
/// The cardinality is fixed and deterministic so the exercised run count is
/// reportable. Each tuple in `GRID` contributes one case.
fn generate_cases() -> Vec<Case> {
    // (population_size, generations, threshold_milli, min_len, max_len)
    // threshold_milli is the admission threshold * 1000, kept integral so the
    // grid stays exact and reproducible across platforms.
    const GRID: &[(usize, usize, u32, usize, usize)] = &[
        (1, 0, 0, 1, 1),
        (1, 1, 0, 1, 2),
        (1, 5, 250, 1, 3),
        (2, 1, 500, 1, 2),
        (2, 3, 700, 1, 4),
        (3, 0, 1000, 2, 4),
        (3, 2, 300, 1, 3),
        (4, 5, 800, 2, 5),
        (5, 1, 0, 1, 1),
        (5, 4, 1000, 1, 6),
        (6, 6, 500, 2, 4),
        (7, 3, 333, 1, 5),
        (8, 8, 750, 2, 6),
        (9, 2, 900, 1, 3),
        (10, 5, 600, 2, 5),
        (10, 10, 1000, 1, 6),
        (12, 7, 250, 1, 4),
        (14, 4, 700, 2, 6),
        (16, 9, 500, 1, 5),
        (18, 6, 850, 2, 4),
        (20, 3, 0, 1, 2),
        (20, 12, 1000, 1, 6),
        (24, 8, 400, 2, 5),
        (28, 5, 650, 1, 4),
        (30, 0, 500, 1, 6),
        (30, 15, 1000, 2, 6),
        (30, 20, 700, 1, 6),
        (15, 11, 333, 3, 6),
        (11, 13, 950, 1, 3),
        (13, 14, 100, 2, 5),
    ];

    let mut cases = Vec::with_capacity(GRID.len() * 2);
    let mut counter: u64 = 0;
    for &(pop, gens, thr_milli, min_len, max_len) in GRID {
        // Two distinct seeds per grid point, broadening seed coverage without
        // exploding the run count.
        for _ in 0..2 {
            let config = PipelineSearchConfig {
                population_size: pop,
                generations: gens,
                mutation_rate: 0.2,
                admission_threshold: thr_milli as f64 / 1000.0,
                max_pipeline_length: max_len,
                min_pipeline_length: min_len,
            };
            cases.push(Case {
                config,
                seed: spread_seed(counter),
            });
            counter += 1;
        }
    }
    cases
}

/// Run one search over the small fixed pool for a single case.
fn run_small_pool(case: &Case) -> PipelineSearchResult {
    const POOL: &[&str] = &[
        "cbr",
        "asp",
        "bayesian_network",
        "ltl_monitor",
        "frame",
        "production_rules",
        "strips",
        "prolog",
    ];
    let mut search = PipelineSearch::new(
        case.config.clone(),
        POOL,
        Box::new(DiversityFitnessEvaluator),
        case.seed,
    );
    search.run()
}

/// Run one search over the full 57-breed catalog for a single case.
fn run_full_catalog(case: &Case) -> PipelineSearchResult {
    let mut search = PipelineSearch::new(
        case.config.clone(),
        KNOWN_BREEDS,
        Box::new(DiversityFitnessEvaluator),
        case.seed,
    );
    search.run()
}

/// Extract the ordered breed sequence of the best pipeline, if any.
fn best_breed_sequence(result: &PipelineSearchResult) -> Option<Vec<String>> {
    result
        .best_pipeline
        .as_ref()
        .map(|p| p.nodes.iter().map(|n| n.breed.clone()).collect())
}

/// True iff `status` is one of the five bounded variants. Exhaustive match
/// means a future variant addition forces this guard to be revisited rather
/// than silently passing.
fn is_bounded_status(status: &PipelineBoundedStatus) -> bool {
    matches!(
        status,
        PipelineBoundedStatus::Admitted
            | PipelineBoundedStatus::Partial
            | PipelineBoundedStatus::Unknown
            | PipelineBoundedStatus::Refused
            | PipelineBoundedStatus::Blocked
    )
}

// ---------------------------------------------------------------------------
// Invariant 1: best_fitness is always within the closed unit interval.
// ---------------------------------------------------------------------------

#[test]
fn prop_fitness_within_unit_interval() {
    let cases = generate_cases();
    assert!(!cases.is_empty(), "case generator yielded no cases");
    for case in &cases {
        for result in [run_small_pool(case), run_full_catalog(case)] {
            assert!(
                result.best_fitness.is_finite(),
                "best_fitness must be finite (pop={}, gens={}, seed={}); got {}",
                case.config.population_size,
                case.config.generations,
                case.seed,
                result.best_fitness
            );
            assert!(
                (0.0..=1.0).contains(&result.best_fitness),
                "best_fitness out of [0.0, 1.0] (pop={}, gens={}, thr={}, seed={}); got {}",
                case.config.population_size,
                case.config.generations,
                case.config.admission_threshold,
                case.seed,
                result.best_fitness
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Invariant 2: Admitted status implies the admission threshold was met.
// No false admission may slip through.
// ---------------------------------------------------------------------------

#[test]
fn prop_admitted_implies_threshold_met() {
    let cases = generate_cases();
    for case in &cases {
        for result in [run_small_pool(case), run_full_catalog(case)] {
            if result.status == PipelineBoundedStatus::Admitted {
                assert!(
                    result.best_fitness >= case.config.admission_threshold,
                    "ADMITTED with best_fitness {} below threshold {} (pop={}, gens={}, seed={})",
                    result.best_fitness,
                    case.config.admission_threshold,
                    case.config.population_size,
                    case.config.generations,
                    case.seed
                );
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Invariant 3: with a non-empty pool and at least one generation requested,
// the search actually ran: evaluations >= population_size and
// generations_run >= 1.
// ---------------------------------------------------------------------------

#[test]
fn prop_search_actually_ran_with_nonempty_pool() {
    let cases = generate_cases();
    for case in &cases {
        for result in [run_small_pool(case), run_full_catalog(case)] {
            assert!(
                result.evaluations >= case.config.population_size,
                "evaluations {} < population_size {} (gens={}, seed={})",
                result.evaluations,
                case.config.population_size,
                case.config.generations,
                case.seed
            );
            if case.config.generations >= 1 {
                assert!(
                    result.generations_run >= 1,
                    "generations_run was 0 despite generations={} (pop={}, seed={})",
                    case.config.generations,
                    case.config.population_size,
                    case.seed
                );
            }
            assert!(
                result.generations_run <= case.config.generations,
                "generations_run {} exceeded requested generations {} (seed={})",
                result.generations_run,
                case.config.generations,
                case.seed
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Invariant 4: determinism. The same (config, seed) twice yields identical
// best_fitness AND identical best-pipeline breed sequence.
//
// Note: the pipeline `id` field is intentionally NOT compared — the engine
// salts ids with wall-clock subsec_nanos, so ids legitimately differ between
// runs while the breed sequence and fitness remain seed-deterministic.
// ---------------------------------------------------------------------------

#[test]
fn prop_determinism_same_seed_same_outcome() {
    let cases = generate_cases();
    for case in &cases {
        let first = run_small_pool(case);
        let second = run_small_pool(case);
        assert_eq!(
            first.best_fitness.to_bits(),
            second.best_fitness.to_bits(),
            "best_fitness diverged across identical runs (pop={}, gens={}, seed={}): {} vs {}",
            case.config.population_size,
            case.config.generations,
            case.seed,
            first.best_fitness,
            second.best_fitness
        );
        assert_eq!(
            best_breed_sequence(&first),
            best_breed_sequence(&second),
            "best-pipeline breed sequence diverged across identical runs (pop={}, gens={}, seed={})",
            case.config.population_size,
            case.config.generations,
            case.seed
        );
        assert_eq!(
            first.status, second.status,
            "status diverged across identical runs (pop={}, gens={}, seed={})",
            case.config.population_size, case.config.generations, case.seed
        );

        // Cross-check determinism on the full catalog as well.
        let full_a = run_full_catalog(case);
        let full_b = run_full_catalog(case);
        assert_eq!(
            best_breed_sequence(&full_a),
            best_breed_sequence(&full_b),
            "full-catalog best sequence diverged across identical runs (seed={})",
            case.seed
        );
    }
}

// ---------------------------------------------------------------------------
// Invariant 5: non-degeneracy guard (the frozen-PRNG regression class).
//
// Across the seed set, on the FULL 57-breed catalog with population >= 10 and
// generations >= 5, at least one run must produce a best pipeline with MORE
// THAN ONE node. A healthy search must not universally collapse to a single
// breed. If the PRNG stream were frozen, every pipeline would be length-driven
// to a single repeated index and this assertion would fail.
// ---------------------------------------------------------------------------

#[test]
fn prop_non_degeneracy_multi_node_pipeline_reachable() {
    // Probe a dedicated band of seeds with a config that admits multi-node
    // pipelines (max_pipeline_length > 1), under the population/generation
    // floor the invariant specifies.
    let config = PipelineSearchConfig {
        population_size: 12,
        generations: 6,
        mutation_rate: 0.3,
        admission_threshold: 1.5, // unreachable: forces full generation budget
        max_pipeline_length: 5,
        min_pipeline_length: 1,
    };

    let mut observed_multi_node = false;
    let mut probed = 0usize;
    for counter in 0..24u64 {
        let seed = spread_seed(counter ^ 0xa5a5_a5a5);
        let mut search = PipelineSearch::new(
            config.clone(),
            KNOWN_BREEDS,
            Box::new(DiversityFitnessEvaluator),
            seed,
        );
        let result = search.run();
        probed += 1;

        // Floors the invariant asserts the search honored.
        assert!(
            result.generations_run >= 1,
            "non-degeneracy probe did not run a generation (seed={seed})"
        );
        assert!(
            result.evaluations >= config.population_size,
            "non-degeneracy probe under-evaluated (seed={seed})"
        );

        if let Some(seq) = best_breed_sequence(&result) {
            if seq.len() > 1 {
                observed_multi_node = true;
                break;
            }
        }
    }

    assert!(
        observed_multi_node,
        "frozen-PRNG regression guard: across {probed} full-catalog seeds (pop=12, gens=6) \
         no best pipeline ever exceeded a single node; the search collapsed degenerately"
    );
}

// ---------------------------------------------------------------------------
// Invariant 6: status is always one of the five bounded variants — never an
// out-of-band string. Unknown is never coerced into another polarity here; it
// is simply admitted as a legal variant of the bounded set.
// ---------------------------------------------------------------------------

#[test]
fn prop_status_is_bounded_variant() {
    let cases = generate_cases();
    for case in &cases {
        for result in [run_small_pool(case), run_full_catalog(case)] {
            assert!(
                is_bounded_status(&result.status),
                "status outside the five-variant bounded set (pop={}, gens={}, seed={}): {:?}",
                case.config.population_size,
                case.config.generations,
                case.seed,
                result.status
            );
            // The canonical string projection must also stay inside the set.
            assert!(
                matches!(
                    result.status.as_str(),
                    "ADMITTED" | "PARTIAL" | "UNKNOWN" | "REFUSED" | "BLOCKED"
                ),
                "status as_str() out of band (seed={}): {}",
                case.seed,
                result.status.as_str()
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Negative control: an empty breed pool must be REFUSED, with no best pipeline,
// across several seeds. This is the lawful boundary — the search refuses rather
// than fabricating a candidate from nothing.
// ---------------------------------------------------------------------------

#[test]
fn prop_empty_pool_negative_control_refused() {
    const EMPTY_POOL: &[&str] = &[];
    let seeds: [u64; 8] = [
        0,
        1,
        7,
        spread_seed(3),
        spread_seed(11),
        0xcafef00d_deadbeef, // the historically dangerous mix-constant seed
        u64::MAX,
        0x9e3779b97f4a7c15,
    ];
    for &seed in &seeds {
        let mut search = PipelineSearch::new(
            PipelineSearchConfig::default(),
            EMPTY_POOL,
            Box::new(DiversityFitnessEvaluator),
            seed,
        );
        let result = search.run();
        assert_eq!(
            result.status,
            PipelineBoundedStatus::Refused,
            "empty pool must yield REFUSED (seed={seed}); got {:?}",
            result.status
        );
        assert!(
            result.best_pipeline.is_none(),
            "empty pool must not produce a best pipeline (seed={seed})"
        );
        assert_eq!(
            result.best_fitness, 0.0,
            "empty pool best_fitness must be 0.0 (seed={seed}); got {}",
            result.best_fitness
        );
        assert_eq!(
            result.evaluations, 0,
            "empty pool must perform no evaluations (seed={seed}); got {}",
            result.evaluations
        );
    }
}
