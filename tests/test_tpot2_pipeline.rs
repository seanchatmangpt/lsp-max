//! Integration coverage for the public `lsp_max::pipeline` API (TPOT2-style
//! breed-pipeline search over the wasm4pm-cognition breed catalog).
//!
//! These cases exercise the catalog, the bounded-status types, the genetic
//! search engine, and the heuristic fitness evaluator end-to-end through the
//! crate's public surface. Assertions are the evidence here; per project law a
//! passing run is a transcript of the test, not a release receipt.

use std::collections::HashSet;

use lsp_max::pipeline::catalog::{breed_category, BreedCategory, KNOWN_BREEDS};
use lsp_max::pipeline::fitness::{BreedFitnessEvaluator, HeuristicFitnessEvaluator};
use lsp_max::pipeline::search::{DiversityFitnessEvaluator, PipelineSearch};
use lsp_max::pipeline::types::{PipelineBoundedStatus, PipelineSearchConfig};

/// The catalog must expose the full breed set with no repeated names.
#[test]
fn catalog_has_full_breed_set() {
    assert_eq!(
        KNOWN_BREEDS.len(),
        57,
        "KNOWN_BREEDS expected to carry 57 entries, found {}",
        KNOWN_BREEDS.len()
    );

    let unique: HashSet<&&str> = KNOWN_BREEDS.iter().collect();
    assert_eq!(
        unique.len(),
        KNOWN_BREEDS.len(),
        "KNOWN_BREEDS contains duplicate entries: {} unique vs {} total",
        unique.len(),
        KNOWN_BREEDS.len()
    );
}

/// Representative breeds must land in their declared categories, and an
/// unrecognized name must fall back to `MetaBased` rather than silently
/// vanishing.
#[test]
fn breed_category_partitions_known_breeds() {
    assert_eq!(breed_category("ltl_monitor"), BreedCategory::Temporal);
    assert_eq!(breed_category("asp"), BreedCategory::LogicBased);
    assert_eq!(breed_category("cbr"), BreedCategory::RuleBased);
    assert_eq!(
        breed_category("bayesian_network"),
        BreedCategory::Probabilistic
    );
    assert_eq!(breed_category("frame"), BreedCategory::MemoryBased);
    assert_eq!(breed_category("strips"), BreedCategory::PlanningBased);
    assert_eq!(breed_category("meta_reasoning"), BreedCategory::MetaBased);

    assert_eq!(
        breed_category("not_a_real_breed_name"),
        BreedCategory::MetaBased,
        "unknown breed names must fall back to MetaBased"
    );
}

/// A search over the full catalog with a fixed seed must reproduce both the
/// best fitness and the exact best-pipeline breed sequence across runs.
#[test]
fn search_over_full_catalog_is_reproducible() {
    let seed = 0x5eed_1234_u64;

    let mut first = PipelineSearch::new(
        PipelineSearchConfig::default(),
        KNOWN_BREEDS,
        Box::new(DiversityFitnessEvaluator),
        seed,
    );
    let first_result = first.run();

    let mut second = PipelineSearch::new(
        PipelineSearchConfig::default(),
        KNOWN_BREEDS,
        Box::new(DiversityFitnessEvaluator),
        seed,
    );
    let second_result = second.run();

    assert_eq!(
        first_result.best_fitness, second_result.best_fitness,
        "identical seed must yield identical best_fitness"
    );

    let first_seq: Vec<String> = first_result
        .best_pipeline
        .expect("seeded run must yield a best pipeline")
        .nodes
        .iter()
        .map(|n| n.breed.clone())
        .collect();
    let second_seq: Vec<String> = second_result
        .best_pipeline
        .expect("seeded run must yield a best pipeline")
        .nodes
        .iter()
        .map(|n| n.breed.clone())
        .collect();

    assert_eq!(
        first_seq, second_seq,
        "identical seed must yield identical best-pipeline breed sequence"
    );
}

/// NEGATIVE CONTROL: an empty breed pool must be Refused with no candidate.
/// The Refused outcome must not collapse into Admitted or Partial.
#[test]
fn search_empty_pool_is_refused() {
    let empty_pool: &[&str] = &[];
    let mut search = PipelineSearch::new(
        PipelineSearchConfig::default(),
        empty_pool,
        Box::new(DiversityFitnessEvaluator),
        0,
    );
    let result = search.run();

    assert_eq!(
        result.status,
        PipelineBoundedStatus::Refused,
        "empty breed pool must be REFUSED, got {}",
        result.status.as_str()
    );
    assert!(
        result.best_pipeline.is_none(),
        "REFUSED run must carry no best pipeline"
    );
    assert_ne!(
        result.status,
        PipelineBoundedStatus::Admitted,
        "REFUSED must not collapse into ADMITTED"
    );
    assert_ne!(
        result.status,
        PipelineBoundedStatus::Partial,
        "REFUSED must not collapse into PARTIAL"
    );
}

/// Over the full catalog the run must report a bounded fitness in range, run at
/// least one generation, evaluate at least the initial population, and report a
/// bounded status that is never a victory term.
#[test]
fn search_bounded_status_and_fitness_range() {
    let config = PipelineSearchConfig::default();
    let population_size = config.population_size;

    let mut search =
        PipelineSearch::new(config, KNOWN_BREEDS, Box::new(DiversityFitnessEvaluator), 7);
    let result = search.run();

    assert!(
        (0.0..=1.0).contains(&result.best_fitness),
        "best_fitness {} must be within [0.0, 1.0]",
        result.best_fitness
    );
    assert!(
        result.generations_run >= 1,
        "search must run at least one generation, ran {}",
        result.generations_run
    );
    assert!(
        result.evaluations >= population_size,
        "evaluations {} must be at least the initial population {}",
        result.evaluations,
        population_size
    );
    assert!(
        matches!(
            result.status,
            PipelineBoundedStatus::Admitted | PipelineBoundedStatus::Partial
        ),
        "non-empty catalog run must report ADMITTED or PARTIAL, got {}",
        result.status.as_str()
    );
}

/// The heuristic evaluator must be bounded to [0.0, 1.0], score the empty
/// pipeline at 0.0, and rank a diverse multi-category pipeline strictly above a
/// homogeneous one of the same length.
#[test]
fn heuristic_fitness_is_bounded_and_monotone_on_diversity() {
    let empty: Vec<String> = Vec::new();
    let empty_score = HeuristicFitnessEvaluator.evaluate(&empty);
    assert_eq!(empty_score, 0.0, "empty pipeline must score 0.0");

    let diverse: Vec<String> = vec![
        "cbr".to_string(),
        "ltl_monitor".to_string(),
        "asp".to_string(),
    ];
    let homogeneous: Vec<String> = vec!["cbr".to_string(), "cbr".to_string(), "cbr".to_string()];

    let diverse_score = HeuristicFitnessEvaluator.evaluate(&diverse);
    let homogeneous_score = HeuristicFitnessEvaluator.evaluate(&homogeneous);

    for (label, score) in [
        ("empty", empty_score),
        ("diverse", diverse_score),
        ("homogeneous", homogeneous_score),
    ] {
        assert!(
            (0.0..=1.0).contains(&score),
            "{} score {} must be within [0.0, 1.0]",
            label,
            score
        );
    }

    assert!(
        diverse_score > homogeneous_score,
        "diverse pipeline ({}) must score strictly above homogeneous ({})",
        diverse_score,
        homogeneous_score
    );
}

/// A high admission threshold must gate the ADMITTED status: a reported
/// ADMITTED outcome must imply that best_fitness actually reached the
/// threshold. Below threshold the run reports PARTIAL, never a victory term.
#[test]
fn admission_threshold_gate_is_honored() {
    let threshold = 0.99_f64;
    let config = PipelineSearchConfig {
        admission_threshold: threshold,
        ..Default::default()
    };

    let mut search = PipelineSearch::new(
        config,
        KNOWN_BREEDS,
        Box::new(DiversityFitnessEvaluator),
        1234,
    );
    let result = search.run();

    if result.status == PipelineBoundedStatus::Admitted {
        assert!(
            result.best_fitness >= threshold,
            "ADMITTED implies best_fitness {} >= admission_threshold {}",
            result.best_fitness,
            threshold
        );
    } else {
        assert_eq!(
            result.status,
            PipelineBoundedStatus::Partial,
            "below-threshold run must report PARTIAL, got {}",
            result.status.as_str()
        );
        assert!(
            result.best_fitness < threshold,
            "PARTIAL implies best_fitness {} < admission_threshold {}",
            result.best_fitness,
            threshold
        );
    }
}
