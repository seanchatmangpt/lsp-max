//! Pareto / multi-objective variant of the TPOT2 breed-pipeline search.
//!
//! The scalar search in [`crate::pipeline::search`] collapses every concern into a
//! single fitness number. This module instead scores each candidate pipeline on
//! SEVERAL bounded objectives at once and returns the *Pareto front*: the set of
//! pipelines that are non-dominated, i.e. no other pipeline is at least as good on
//! every objective and strictly better on one. The caller then trades objectives
//! off explicitly rather than accepting a pre-baked weighting.
//!
//! Determinism: every run is seeded through [`Prng`]; an identical seed yields an
//! identical front (same pipelines and same objective values).
//!
//! Bounded status only — no victory language. An empty breed pool is [`Refused`];
//! a non-empty front holding a member at or above the admission threshold (on the
//! convenience scalarization) is [`Admitted`]; anything else is [`Partial`].
//! [`Unknown`] is never produced here — its absence is intentional, not a gap.
//!
//! [`Refused`]: PipelineBoundedStatus::Refused
//! [`Admitted`]: PipelineBoundedStatus::Admitted
//! [`Partial`]: PipelineBoundedStatus::Partial
//! [`Unknown`]: PipelineBoundedStatus::Unknown

use crate::pipeline::catalog::{breed_category, BreedCategory, KNOWN_BREEDS};
use crate::pipeline::search::Prng;
use crate::pipeline::types::{
    BreedNodeConfig, BreedPipeline, PipelineBoundedStatus, PipelineSearchConfig,
};

/// Count of distinct [`BreedCategory`] variants, used to normalize
/// [`Objectives::category_coverage`] into [0.0, 1.0].
const CATEGORY_COUNT: f64 = 7.0;

/// The number of objectives held by [`Objectives`]; used to average the
/// convenience scalarization.
const OBJECTIVE_COUNT: f64 = 4.0;

/// A bounded multi-objective score for one candidate pipeline.
///
/// Every field is constrained to [0.0, 1.0] where higher is better. The objectives
/// are deliberately orthogonal so the Pareto front exposes genuine trade-offs
/// rather than four restatements of one quantity.
#[derive(Debug, Clone, PartialEq)]
pub struct Objectives {
    /// Distinct breed categories present divided by the total category count (7).
    ///
    /// Rewards pipelines that draw on a broad spread of cognitive styles rather
    /// than stacking many breeds of a single category.
    pub category_coverage: f64,
    /// Closeness to the preferred 2..=4 node length.
    ///
    /// Pipelines of length 2, 3, or 4 score 1.0; length 1 scores 0.5; longer
    /// pipelines decay as `4 / len`; an empty pipeline scores 0.0. Favors
    /// compact, legible pipelines over sprawling ones.
    pub brevity: f64,
    /// 1.0 when at least one [`BreedCategory::Temporal`] breed is present, else 0.0.
    ///
    /// Temporal reasoning (LTL, CTL, Allen, naive physics) is frequently required
    /// for event-log conformance; this objective surfaces its presence as a
    /// first-class trade-off axis.
    pub temporal_presence: f64,
    /// Distinct breeds divided by pipeline length.
    ///
    /// 1.0 when every node is a different breed; lower when breeds repeat. An
    /// empty pipeline scores 0.0. Discourages degenerate pipelines that repeat
    /// the same breed.
    pub breed_uniqueness: f64,
}

impl Objectives {
    /// Score a pipeline's breed sequence across all objectives.
    ///
    /// `breeds` is the pipeline in node order. The returned [`Objectives`] has
    /// every field within [0.0, 1.0]; an empty slice yields all-zero objectives.
    pub fn evaluate(breeds: &[String]) -> Self {
        if breeds.is_empty() {
            return Self {
                category_coverage: 0.0,
                brevity: 0.0,
                temporal_presence: 0.0,
                breed_uniqueness: 0.0,
            };
        }

        // BreedCategory does not implement Hash, so distinct categories are
        // collected into a Vec and deduplicated by PartialEq rather than a set.
        let mut categories: Vec<BreedCategory> = Vec::new();
        for breed in breeds {
            let cat = breed_category(breed);
            if !categories.contains(&cat) {
                categories.push(cat);
            }
        }
        let category_coverage = (categories.len() as f64 / CATEGORY_COUNT).clamp(0.0, 1.0);

        let brevity = match breeds.len() {
            2..=4 => 1.0,
            1 => 0.5,
            n => (4.0 / n as f64).clamp(0.0, 1.0),
        };

        let temporal_presence = if categories.contains(&BreedCategory::Temporal) {
            1.0
        } else {
            0.0
        };

        let mut distinct: Vec<&String> = Vec::new();
        for breed in breeds {
            if !distinct.contains(&breed) {
                distinct.push(breed);
            }
        }
        let breed_uniqueness = (distinct.len() as f64 / breeds.len() as f64).clamp(0.0, 1.0);

        Self {
            category_coverage,
            brevity,
            temporal_presence,
            breed_uniqueness,
        }
    }

    /// The four objectives as a fixed-order array, for uniform iteration.
    fn axes(&self) -> [f64; 4] {
        [
            self.category_coverage,
            self.brevity,
            self.temporal_presence,
            self.breed_uniqueness,
        ]
    }

    /// Pareto dominance: `self` dominates `other` iff `self` is greater than or
    /// equal on every objective and strictly greater on at least one.
    ///
    /// Reflexively, a value does not dominate an equal value (no strict axis).
    pub fn dominates(&self, other: &Objectives) -> bool {
        let mine = self.axes();
        let theirs = other.axes();
        let mut strictly_better_somewhere = false;
        for (a, b) in mine.iter().zip(theirs.iter()) {
            if a < b {
                return false;
            }
            if a > b {
                strictly_better_somewhere = true;
            }
        }
        strictly_better_somewhere
    }

    /// Equal-weight mean of the objectives, in [0.0, 1.0].
    ///
    /// This is a convenience collapse for ranking and the admission check only;
    /// the Pareto front itself never relies on it.
    pub fn scalarized(&self) -> f64 {
        (self.axes().iter().sum::<f64>() / OBJECTIVE_COUNT).clamp(0.0, 1.0)
    }
}

/// One member of a Pareto front: a pipeline paired with its objective scores.
#[derive(Debug, Clone)]
pub struct ParetoMember {
    /// The candidate pipeline.
    pub pipeline: BreedPipeline,
    /// The pipeline's bounded multi-objective score.
    pub objectives: Objectives,
    /// Equal-weight mean of [`ParetoMember::objectives`], for convenience only.
    pub scalarized: f64,
}

/// Outcome of a [`ParetoSearch`] run.
#[derive(Debug, Clone)]
pub struct ParetoSearchResult {
    /// Bounded status of the run. Never [`PipelineBoundedStatus::Unknown`].
    pub status: PipelineBoundedStatus,
    /// The non-dominated set discovered. Internally non-dominated by construction.
    pub front: Vec<ParetoMember>,
    /// Number of generations actually evolved.
    pub generations_run: usize,
    /// Total number of [`Objectives::evaluate`] calls performed.
    pub evaluations: usize,
    /// One-line bounded summary (no victory language).
    pub summary: String,
}

/// A small genetic algorithm that returns a Pareto front over [`KNOWN_BREEDS`].
///
/// Each generation evaluates a population on all [`Objectives`], folds the
/// non-dominated members into a running archive (the front), and breeds the next
/// generation from the archive via single-point crossover and point mutation. The
/// PRNG seed makes every run reproducible.
pub struct ParetoSearch<'a> {
    /// Search hyper-parameters (population size, generations, lengths, threshold).
    pub config: PipelineSearchConfig,
    /// Breed names the search may assemble; defaults to [`KNOWN_BREEDS`].
    pub breed_pool: &'a [&'a str],
    /// Deterministic PRNG; identical seeds yield identical fronts.
    pub rng: Prng,
    /// Crossover probability in [0.0, 1.0].
    pub crossover_rate: f64,
}

impl std::fmt::Debug for ParetoSearch<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ParetoSearch")
            .field("config", &self.config)
            .field("breed_pool_len", &self.breed_pool.len())
            .field("crossover_rate", &self.crossover_rate)
            .finish()
    }
}

impl<'a> ParetoSearch<'a> {
    /// Construct a search over [`KNOWN_BREEDS`] with the given config and seed.
    pub fn new(config: PipelineSearchConfig, seed: u64) -> Self {
        Self {
            config,
            breed_pool: KNOWN_BREEDS,
            rng: Prng::new(seed),
            crossover_rate: 0.7,
        }
    }

    /// Construct a search over a caller-supplied breed pool with the given seed.
    ///
    /// An empty pool drives the run to [`PipelineBoundedStatus::Refused`].
    pub fn with_pool(config: PipelineSearchConfig, breed_pool: &'a [&'a str], seed: u64) -> Self {
        Self {
            config,
            breed_pool,
            rng: Prng::new(seed),
            crossover_rate: 0.7,
        }
    }

    /// Build a random pipeline of `len` nodes drawn from the breed pool.
    ///
    /// The id is derived from the PRNG only (no wall-clock), keeping runs
    /// reproducible across machines and time.
    fn random_pipeline(&mut self, len: usize) -> BreedPipeline {
        let nodes: Vec<BreedNodeConfig> = (0..len)
            .map(|_| {
                let idx = self.rng.next_usize(self.breed_pool.len());
                BreedNodeConfig {
                    breed: self.breed_pool[idx].to_string(),
                    params: serde_json::Value::Null,
                }
            })
            .collect();
        BreedPipeline {
            id: format!("pareto-{:016x}", self.rng.next_u64()),
            nodes,
        }
    }

    /// Initialize a random population sized by `config.population_size`.
    fn init_population(&mut self) -> Vec<BreedPipeline> {
        let range = self.config.max_pipeline_length - self.config.min_pipeline_length + 1;
        (0..self.config.population_size)
            .map(|_| {
                let len = self.rng.next_usize(range) + self.config.min_pipeline_length;
                self.random_pipeline(len)
            })
            .collect()
    }

    /// Single-point crossover; falls back to cloning the first parent when skipped.
    fn crossover(&mut self, a: &BreedPipeline, b: &BreedPipeline) -> BreedPipeline {
        if self.rng.next_f64() > self.crossover_rate || a.nodes.is_empty() || b.nodes.is_empty() {
            let mut clone = a.clone();
            clone.id = format!("pareto-{:016x}", self.rng.next_u64());
            return clone;
        }
        let split_a = self.rng.next_usize(a.nodes.len());
        let split_b = self.rng.next_usize(b.nodes.len());
        let mut nodes: Vec<BreedNodeConfig> = a.nodes[..split_a].to_vec();
        nodes.extend_from_slice(&b.nodes[split_b..]);
        nodes.truncate(self.config.max_pipeline_length);
        while nodes.len() < self.config.min_pipeline_length {
            let idx = self.rng.next_usize(self.breed_pool.len());
            nodes.push(BreedNodeConfig {
                breed: self.breed_pool[idx].to_string(),
                params: serde_json::Value::Null,
            });
        }
        BreedPipeline {
            id: format!("pareto-xo-{:016x}", self.rng.next_u64()),
            nodes,
        }
    }

    /// Point mutation: replace a random node with a random breed from the pool.
    fn mutate(&mut self, pipeline: &mut BreedPipeline) {
        if pipeline.nodes.is_empty() || self.rng.next_f64() > self.config.mutation_rate {
            return;
        }
        let node_idx = self.rng.next_usize(pipeline.nodes.len());
        let new_idx = self.rng.next_usize(self.breed_pool.len());
        pipeline.nodes[node_idx].breed = self.breed_pool[new_idx].to_string();
        pipeline.id = format!("pareto-mut-{:016x}", self.rng.next_u64());
    }

    /// Fold `candidates` into `archive`, retaining only the non-dominated set.
    ///
    /// A candidate is admitted to the archive unless an archive member dominates
    /// it; on admission, every archive member it dominates is dropped. Exact
    /// objective duplicates are not re-added, which keeps the front finite and the
    /// run terminating. The result is internally non-dominated by construction.
    fn merge_front(archive: &mut Vec<ParetoMember>, candidates: Vec<ParetoMember>) {
        for cand in candidates {
            let dominated_by_archive = archive
                .iter()
                .any(|m| m.objectives.dominates(&cand.objectives));
            if dominated_by_archive {
                continue;
            }
            // BreedNodeConfig is not PartialEq, so identity is compared on the
            // ordered breed-name sequence (the load-bearing content) plus objectives.
            let cand_breeds: Vec<&str> = cand
                .pipeline
                .nodes
                .iter()
                .map(|n| n.breed.as_str())
                .collect();
            let already_present = archive.iter().any(|m| {
                m.objectives == cand.objectives
                    && m.pipeline
                        .nodes
                        .iter()
                        .map(|n| n.breed.as_str())
                        .eq(cand_breeds.iter().copied())
            });
            if already_present {
                continue;
            }
            archive.retain(|m| !cand.objectives.dominates(&m.objectives));
            archive.push(cand);
        }
    }

    /// Score a pipeline into a [`ParetoMember`].
    fn member_of(pipeline: BreedPipeline) -> ParetoMember {
        let breeds: Vec<String> = pipeline.nodes.iter().map(|n| n.breed.clone()).collect();
        let objectives = Objectives::evaluate(&breeds);
        let scalarized = objectives.scalarized();
        ParetoMember {
            pipeline,
            objectives,
            scalarized,
        }
    }

    /// Run the search and return the Pareto front with a bounded status.
    ///
    /// Status rules: an empty breed pool returns [`PipelineBoundedStatus::Refused`]
    /// with an empty front; a non-empty front with any member whose scalarized
    /// score meets `config.admission_threshold` returns
    /// [`PipelineBoundedStatus::Admitted`]; otherwise
    /// [`PipelineBoundedStatus::Partial`]. [`PipelineBoundedStatus::Unknown`] is
    /// never returned.
    pub fn run(&mut self) -> ParetoSearchResult {
        if self.breed_pool.is_empty() {
            return ParetoSearchResult {
                status: PipelineBoundedStatus::Refused,
                front: Vec::new(),
                generations_run: 0,
                evaluations: 0,
                summary: "breed pool empty; pareto search refused".to_string(),
            };
        }

        let population = self.init_population();
        let mut evaluations = 0usize;
        let mut archive: Vec<ParetoMember> = Vec::new();

        let initial: Vec<ParetoMember> = population
            .into_iter()
            .map(|p| {
                evaluations += 1;
                Self::member_of(p)
            })
            .collect();
        let mut current = initial.clone();
        Self::merge_front(&mut archive, initial);

        let mut generations_run = 0usize;
        for _ in 0..self.config.generations {
            generations_run += 1;

            let breeding_stock: Vec<&ParetoMember> = if archive.is_empty() {
                current.iter().collect()
            } else {
                archive.iter().collect()
            };
            if breeding_stock.is_empty() {
                break;
            }

            let mut next: Vec<BreedPipeline> = Vec::with_capacity(self.config.population_size);
            while next.len() < self.config.population_size {
                let a = breeding_stock[self.rng.next_usize(breeding_stock.len())]
                    .pipeline
                    .clone();
                let b = breeding_stock[self.rng.next_usize(breeding_stock.len())]
                    .pipeline
                    .clone();
                let mut child = self.crossover(&a, &b);
                self.mutate(&mut child);
                next.push(child);
            }

            let scored: Vec<ParetoMember> = next
                .into_iter()
                .map(|p| {
                    evaluations += 1;
                    Self::member_of(p)
                })
                .collect();
            current = scored.clone();
            Self::merge_front(&mut archive, scored);
        }

        // Stable ordering so a fixed seed yields a byte-stable front: rank by
        // descending scalarized score, breaking ties by pipeline id.
        archive.sort_by(|x, y| {
            y.scalarized
                .partial_cmp(&x.scalarized)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| x.pipeline.id.cmp(&y.pipeline.id))
        });

        let admitted = archive
            .iter()
            .any(|m| m.scalarized >= self.config.admission_threshold);
        let status = if admitted {
            PipelineBoundedStatus::Admitted
        } else {
            PipelineBoundedStatus::Partial
        };

        let summary = format!(
            "front_size={} status={} gens={} evals={}",
            archive.len(),
            status,
            generations_run,
            evaluations
        );

        ParetoSearchResult {
            status,
            front: archive,
            generations_run,
            evaluations,
            summary,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cfg() -> PipelineSearchConfig {
        PipelineSearchConfig {
            population_size: 24,
            generations: 8,
            admission_threshold: 0.6,
            ..Default::default()
        }
    }

    // NEGATIVE CONTROL: an empty breed pool must refuse with an empty front and
    // must never fabricate a Pareto member out of nothing.
    #[test]
    fn empty_pool_refused_with_empty_front() {
        let empty: &[&str] = &[];
        let mut search = ParetoSearch::with_pool(cfg(), empty, 7);
        let result = search.run();
        assert_eq!(result.status, PipelineBoundedStatus::Refused);
        assert!(result.front.is_empty(), "refused run must have empty front");
        assert_eq!(result.evaluations, 0);
        assert_eq!(result.generations_run, 0);
        assert_ne!(
            result.status,
            PipelineBoundedStatus::Unknown,
            "Refused must not collapse into Unknown"
        );
    }

    #[test]
    fn returned_front_is_internally_non_dominated() {
        let mut search = ParetoSearch::new(cfg(), 1234);
        let result = search.run();
        assert!(!result.front.is_empty(), "front should be populated");
        for (i, a) in result.front.iter().enumerate() {
            for (j, b) in result.front.iter().enumerate() {
                if i == j {
                    continue;
                }
                assert!(
                    !a.objectives.dominates(&b.objectives),
                    "front member {} dominates member {}; front is not a Pareto set",
                    i,
                    j
                );
            }
        }
    }

    #[test]
    fn determinism_same_seed_yields_identical_front() {
        let mut a = ParetoSearch::new(cfg(), 2026);
        let mut b = ParetoSearch::new(cfg(), 2026);
        let ra = a.run();
        let rb = b.run();
        assert_eq!(ra.front.len(), rb.front.len(), "front sizes must match");
        assert_eq!(ra.status, rb.status);
        assert_eq!(ra.evaluations, rb.evaluations);
        for (ma, mb) in ra.front.iter().zip(rb.front.iter()) {
            assert_eq!(ma.objectives, mb.objectives, "objectives must match");
            assert_eq!(ma.scalarized, mb.scalarized, "scalarized must match");
            let breeds_a: Vec<&String> = ma.pipeline.nodes.iter().map(|n| &n.breed).collect();
            let breeds_b: Vec<&String> = mb.pipeline.nodes.iter().map(|n| &n.breed).collect();
            assert_eq!(breeds_a, breeds_b, "pipeline breed sequences must match");
        }
    }

    #[test]
    fn all_objective_and_scalarized_values_are_bounded() {
        let mut search = ParetoSearch::new(cfg(), 55);
        let result = search.run();
        assert!(!result.front.is_empty());
        for m in &result.front {
            for v in [
                m.objectives.category_coverage,
                m.objectives.brevity,
                m.objectives.temporal_presence,
                m.objectives.breed_uniqueness,
                m.scalarized,
            ] {
                assert!(
                    (0.0..=1.0).contains(&v),
                    "objective/scalarized {} out of [0,1]",
                    v
                );
            }
        }
    }

    #[test]
    fn strictly_better_pipeline_dominates_worse() {
        // A length-3, all-distinct, multi-category pipeline that includes a
        // Temporal breed scores 1.0 on every objective.
        let strong = Objectives::evaluate(&[
            "ltl_monitor".to_string(),
            "asp".to_string(),
            "bayesian_network".to_string(),
        ]);
        // A length-1 logic-only pipeline is worse on every axis: lower coverage,
        // lower brevity (0.5), no temporal presence, and uniqueness 1.0 only
        // because a single node is trivially distinct.
        let weak = Objectives::evaluate(&["asp".to_string()]);
        assert!(
            strong.dominates(&weak),
            "strictly-better objectives must dominate: {:?} vs {:?}",
            strong,
            weak
        );
        assert!(
            !weak.dominates(&strong),
            "worse objectives must not dominate the better"
        );
    }

    #[test]
    fn dominance_is_irreflexive_for_equal_objectives() {
        let o = Objectives::evaluate(&["frame".to_string(), "asp".to_string()]);
        assert!(
            !o.dominates(&o.clone()),
            "an objective set must not dominate an equal one"
        );
    }
}
