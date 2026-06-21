use crate::pipeline::types::{
    BreedName, BreedNodeConfig, BreedPipeline, PipelineBoundedStatus, PipelineSearchConfig,
    PipelineSearchResult,
};

/// A fitness function: takes a pipeline and returns a score in [0.0, 1.0].
///
/// Implementors receive the breed names in pipeline order; they must not
/// mutate external state and must be `Send + Sync` for multi-threaded use.
pub trait FitnessEvaluator: Send + Sync {
    /// Score a candidate pipeline. Return value must be in [0.0, 1.0].
    fn evaluate(&self, breeds: &[BreedName]) -> f64;
}

/// Simple pseudo-random number generator (no external deps).
///
/// Uses xorshift64 — deterministic for a given seed, suitable for
/// reproducible genetic search runs.
pub struct Prng {
    state: u64,
}

impl std::fmt::Debug for Prng {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Prng").finish()
    }
}

impl Prng {
    /// Seed the generator. Mixed with a constant, then guarded so no input seed
    /// can leave the xorshift state at 0 (which would freeze the stream): a seed
    /// equal to the mixing constant would otherwise cancel to zero.
    pub fn new(seed: u64) -> Self {
        let mixed = seed ^ 0xcafef00d_deadbeef;
        Self {
            state: if mixed == 0 {
                0x9e3779b97f4a7c15
            } else {
                mixed
            },
        }
    }

    /// Advance the xorshift64 state and return the next value.
    pub fn next_u64(&mut self) -> u64 {
        self.state ^= self.state << 13;
        self.state ^= self.state >> 7;
        self.state ^= self.state << 17;
        self.state
    }

    /// Return a uniform float in [0.0, 1.0).
    pub fn next_f64(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 / (1u64 << 53) as f64
    }

    /// Return a uniform usize in [0, n).
    pub fn next_usize(&mut self, n: usize) -> usize {
        (self.next_u64() % n as u64) as usize
    }
}

/// TPOT2-style breed pipeline genetic search engine.
///
/// Evolves a population of `BreedPipeline` candidates over generations using
/// tournament selection, single-point crossover, and point mutation.
/// Stops early when `admission_threshold` is met (ADMITTED status).
pub struct PipelineSearch<'a> {
    /// Search hyper-parameters (population size, mutation rate, etc.).
    pub config: PipelineSearchConfig,
    /// Available breed names the search may assemble into pipelines.
    pub breed_pool: &'a [&'a str],
    /// Fitness function evaluated against each candidate.
    pub evaluator: Box<dyn FitnessEvaluator>,
    /// Deterministic PRNG; seed it for reproducible runs.
    pub rng: Prng,
    /// Tournament size: number of candidates drawn per selection event.
    pub tournament_size: usize,
    /// Crossover probability in [0.0, 1.0].
    pub crossover_rate: f64,
}

impl<'a> std::fmt::Debug for PipelineSearch<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PipelineSearch")
            .field("config", &self.config)
            .field("tournament_size", &self.tournament_size)
            .field("crossover_rate", &self.crossover_rate)
            .finish()
    }
}

impl<'a> PipelineSearch<'a> {
    /// Construct a new search engine.
    ///
    /// `seed` controls the PRNG; identical seeds produce identical runs.
    pub fn new(
        config: PipelineSearchConfig,
        breed_pool: &'a [&'a str],
        evaluator: Box<dyn FitnessEvaluator>,
        seed: u64,
    ) -> Self {
        Self {
            config,
            breed_pool,
            evaluator,
            rng: Prng::new(seed),
            tournament_size: 3,
            crossover_rate: 0.7,
        }
    }

    /// Build a random pipeline of length `len` drawn from `breed_pool`.
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
        // Id is derived from the PRNG only (no wall-clock), so a fixed seed makes
        // the entire serialized result — id included — reproducible, matching the
        // determinism contract `pareto.rs` already holds. The PRNG is advanced
        // exactly once here, leaving the breed-selection stream unchanged.
        BreedPipeline {
            id: format!("pipe-{:016x}", self.rng.next_u64()),
            nodes,
        }
    }

    /// Initialize a random population.
    fn init_population(&mut self) -> Vec<(BreedPipeline, f64)> {
        let range = self.config.max_pipeline_length - self.config.min_pipeline_length + 1;
        (0..self.config.population_size)
            .map(|_| {
                let len = self.rng.next_usize(range) + self.config.min_pipeline_length;
                let pipe = self.random_pipeline(len);
                (pipe, 0.0)
            })
            .collect()
    }

    /// Evaluate fitness for every pipeline in the population; returns evaluation count.
    fn evaluate_all(&self, population: &mut [(BreedPipeline, f64)]) -> usize {
        let mut count = 0;
        for (pipe, fitness) in population.iter_mut() {
            let breeds: Vec<BreedName> = pipe.nodes.iter().map(|n| n.breed.clone()).collect();
            *fitness = self.evaluator.evaluate(&breeds);
            count += 1;
        }
        count
    }

    /// Tournament selection: return the index of the best among `k` random draws.
    fn tournament_select(&mut self, population: &[(BreedPipeline, f64)]) -> usize {
        let k = self.tournament_size.min(population.len());
        let mut best = self.rng.next_usize(population.len());
        for _ in 1..k {
            let idx = self.rng.next_usize(population.len());
            if population[idx].1 > population[best].1 {
                best = idx;
            }
        }
        best
    }

    /// Single-point crossover of two parent pipelines.
    /// Falls back to cloning the first parent when crossover is skipped.
    fn crossover(&mut self, a: &BreedPipeline, b: &BreedPipeline) -> BreedPipeline {
        if self.rng.next_f64() > self.crossover_rate || a.nodes.is_empty() || b.nodes.is_empty() {
            return a.clone();
        }
        let split_a = self.rng.next_usize(a.nodes.len());
        let split_b = self.rng.next_usize(b.nodes.len());
        let mut nodes: Vec<BreedNodeConfig> = a.nodes[..split_a].to_vec();
        nodes.extend_from_slice(&b.nodes[split_b..]);
        nodes.truncate(self.config.max_pipeline_length);
        if nodes.len() < self.config.min_pipeline_length {
            let idx = self.rng.next_usize(self.breed_pool.len());
            nodes.push(BreedNodeConfig {
                breed: self.breed_pool[idx].to_string(),
                params: serde_json::Value::Null,
            });
        }
        BreedPipeline {
            id: format!("pipe-xo-{:016x}", self.rng.next_u64()),
            nodes,
        }
    }

    /// Point mutation: replace a random node with a breed from the pool.
    /// Resets the pipeline id so stale fitness is not reused.
    fn mutate(&mut self, pipeline: &mut BreedPipeline) {
        if pipeline.nodes.is_empty() || self.rng.next_f64() > self.config.mutation_rate {
            return;
        }
        let node_idx = self.rng.next_usize(pipeline.nodes.len());
        let new_idx = self.rng.next_usize(self.breed_pool.len());
        pipeline.nodes[node_idx].breed = self.breed_pool[new_idx].to_string();
        pipeline.id = format!("pipe-mut-{:08x}", self.rng.next_u64() as u32);
    }

    /// Run the full genetic search and return a `PipelineSearchResult`.
    ///
    /// Returns `Refused` status when the breed pool is empty; otherwise
    /// returns `Admitted` when `admission_threshold` is met and `Partial`
    /// when the best fitness remains below threshold after all generations.
    pub fn run(&mut self) -> PipelineSearchResult {
        if self.breed_pool.is_empty() {
            return PipelineSearchResult {
                status: PipelineBoundedStatus::Refused,
                best_pipeline: None,
                best_fitness: 0.0,
                generations_run: 0,
                evaluations: 0,
                summary: "breed pool empty; search refused".to_string(),
            };
        }

        let mut population = self.init_population();
        let mut total_evals = self.evaluate_all(&mut population);

        population.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        let mut best_fitness = population.first().map_or(0.0, |(_, f)| *f);
        let mut best_pipeline = population.first().map(|(p, _)| p.clone());
        let mut gens_run = 0;

        for _ in 0..self.config.generations {
            gens_run += 1;

            if best_fitness >= self.config.admission_threshold {
                break;
            }

            let mut next_gen: Vec<(BreedPipeline, f64)> =
                Vec::with_capacity(self.config.population_size);

            // Elitism: carry top 2 unchanged.
            for (elite, fit) in population.iter().take(2) {
                next_gen.push((elite.clone(), *fit));
            }

            while next_gen.len() < self.config.population_size {
                let a_idx = self.tournament_select(&population);
                let b_idx = self.tournament_select(&population);
                let mut child = self.crossover(&population[a_idx].0, &population[b_idx].0);
                self.mutate(&mut child);
                next_gen.push((child, 0.0));
            }

            // Only re-evaluate pipelines whose fitness was reset (mutated/new).
            let mut new_evals = 0;
            for (pipe, fitness) in next_gen.iter_mut() {
                if *fitness == 0.0 {
                    let breeds: Vec<BreedName> =
                        pipe.nodes.iter().map(|n| n.breed.clone()).collect();
                    *fitness = self.evaluator.evaluate(&breeds);
                    new_evals += 1;
                }
            }
            total_evals += new_evals;

            next_gen.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

            if let Some((candidate, f)) = next_gen.first() {
                if *f > best_fitness {
                    best_fitness = *f;
                    best_pipeline = Some(candidate.clone());
                }
            }

            population = next_gen;
        }

        let status = if best_fitness >= self.config.admission_threshold {
            PipelineBoundedStatus::Admitted
        } else {
            PipelineBoundedStatus::Partial
        };

        PipelineSearchResult {
            status,
            best_pipeline,
            best_fitness,
            generations_run: gens_run,
            evaluations: total_evals,
            summary: format!(
                "best_fitness={:.3} gens={} evals={}",
                best_fitness, gens_run, total_evals
            ),
        }
    }
}

/// A simple fitness evaluator for testing: scores by breed diversity and length preference.
///
/// Returns a value in [0.0, 1.0]. Pipelines with 2–4 unique breeds score highest.
#[derive(Debug)]
pub struct DiversityFitnessEvaluator;

impl FitnessEvaluator for DiversityFitnessEvaluator {
    fn evaluate(&self, breeds: &[BreedName]) -> f64 {
        if breeds.is_empty() {
            return 0.0;
        }
        let unique: std::collections::HashSet<&BreedName> = breeds.iter().collect();
        let diversity = unique.len() as f64 / breeds.len() as f64;
        let length_score = match breeds.len() {
            2..=4 => 1.0,
            1 => 0.5,
            n if n > 4 => 4.0 / n as f64,
            _ => 0.0,
        };
        (diversity * 0.7 + length_score * 0.3).min(1.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pipeline::types::PipelineSearchConfig;

    const TEST_BREEDS: &[&str] = &[
        "cbr",
        "asp",
        "bayesian_network",
        "ltl_monitor",
        "frame",
        "production_rules",
    ];

    #[test]
    fn search_produces_nonempty_result() {
        let mut search = PipelineSearch::new(
            PipelineSearchConfig {
                population_size: 5,
                generations: 3,
                ..Default::default()
            },
            TEST_BREEDS,
            Box::new(DiversityFitnessEvaluator),
            42,
        );
        let result = search.run();
        assert!(
            result.best_pipeline.is_some(),
            "search must produce a best pipeline"
        );
        assert!(
            result.evaluations > 0,
            "must have evaluated at least one pipeline"
        );
        assert!(
            result.generations_run > 0,
            "must have run at least one generation"
        );
    }

    #[test]
    fn search_early_stops_on_admission() {
        // DiversityFitnessEvaluator gives ~1.0 for diverse 3-4 length pipelines.
        // With admission_threshold=0.5, search stops before exhausting generations.
        let mut search = PipelineSearch::new(
            PipelineSearchConfig {
                population_size: 10,
                generations: 100,
                admission_threshold: 0.5,
                ..Default::default()
            },
            TEST_BREEDS,
            Box::new(DiversityFitnessEvaluator),
            99,
        );
        let result = search.run();
        assert!(result.best_pipeline.is_some());
        assert!(
            result.generations_run < 100,
            "expected early stop, but ran {} generations",
            result.generations_run
        );
    }

    #[test]
    fn mutation_resets_pipeline_id() {
        let mut search = PipelineSearch::new(
            PipelineSearchConfig::default(),
            TEST_BREEDS,
            Box::new(DiversityFitnessEvaluator),
            0,
        );
        let mut pipe = search.random_pipeline(2);
        // Force mutation by setting rate to 1.0.
        search.config.mutation_rate = 1.0;
        search.mutate(&mut pipe);
        // id must change (fitness sentinel reset path exercises id mutation)
        assert!(pipe.id.starts_with("pipe-mut-"));
    }

    #[test]
    fn prng_is_deterministic() {
        let mut r1 = Prng::new(42);
        let mut r2 = Prng::new(42);
        for _ in 0..20 {
            assert_eq!(r1.next_u64(), r2.next_u64());
        }
    }

    #[test]
    fn prng_seed_colliding_with_mix_constant_does_not_freeze() {
        // A seed equal to the internal mixing constant cancels to a zero state;
        // xorshift64 from zero would emit only zeros and collapse any search to
        // the first breed. The guard must keep the stream live and varied.
        let mut r = Prng::new(0xcafef00d_deadbeef);
        let first = r.next_u64();
        assert_ne!(first, 0, "stream must not be frozen at zero");
        let mut distinct = std::collections::HashSet::new();
        for _ in 0..16 {
            distinct.insert(r.next_u64());
        }
        assert!(distinct.len() > 1, "stream must vary, not repeat one value");
    }

    #[test]
    fn empty_breed_pool_refused() {
        let mut search = PipelineSearch::new(
            PipelineSearchConfig::default(),
            &[],
            Box::new(DiversityFitnessEvaluator),
            0,
        );
        let result = search.run();
        assert_eq!(result.status, PipelineBoundedStatus::Refused);
        assert!(result.best_pipeline.is_none());
    }

    #[test]
    fn full_result_including_id_is_reproducible() {
        // Determinism now covers the serialized result in full, the pipeline id
        // included: a fixed seed yields byte-identical JSON across runs. The id
        // previously embedded wall-clock nanoseconds, which broke this contract
        // and forced the reproducibility witness to project the id out.
        let run = || {
            let mut s = PipelineSearch::new(
                PipelineSearchConfig {
                    population_size: 8,
                    generations: 5,
                    ..Default::default()
                },
                TEST_BREEDS,
                Box::new(DiversityFitnessEvaluator),
                7,
            );
            serde_json::to_string(&s.run()).unwrap()
        };
        assert_eq!(
            run(),
            run(),
            "fixed-seed search must reproduce its full result, id included"
        );
    }
}
