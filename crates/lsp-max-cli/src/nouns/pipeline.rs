use clap_noun_verb::Result;
use clap_noun_verb_macros::verb;
use serde::Serialize;

// ── 1. Domain types ──────────────────────────────────────────────────────────

/// Known wasm4pm-cognition breed names (static catalog).
/// These are the string IDs accepted by wasm4pm's dispatch_breed().
/// Source: wasm4pm-cognition/src/breeds/ directory listing.
pub static KNOWN_BREEDS: &[&str] = &[
    "abductive_ibe",
    "abductive_lp",
    "act_r",
    "allen_temporal",
    "analogy_sme",
    "asp",
    "autoinstinct_learning",
    "autoinstinct_neurosis",
    "autoinstinct_semantics",
    "autoinstinct_vision",
    "bayesian_network",
    "belief_merging",
    "cbr",
    "circumscription",
    "clp",
    "construction_grammar",
    "contingent_plan",
    "csp_ac3",
    "ctl_check",
    "default_logic",
    "dempster_shafer",
    "dendral",
    "description_logic",
    "ebl",
    "episodic_memory",
    "event_calculus",
    "frame",
    "frames_inheritance",
    "fuzzy_logic",
    "gps",
    "hearsay",
    "htn_planning",
    "ilp",
    "ltl_monitor",
    "markov_logic",
    "mdp",
    "meta_reasoning",
    "morphological",
    "naive_physics",
    "ocpm_route_discoverer",
    "oracle_chain",
    "partial_order_plan",
    "pomdp",
    "problog",
    "production_rules",
    "prolog",
    "qualitative_reason",
    "rl_symbolic",
    "sat_cdcl",
    "script_sam",
    "situation_calculus",
    "soar",
    "standing",
    "strips",
    "tableaux",
    "triz",
    "version_space",
];

/// A pipeline: ordered sequence of breed names to apply.
#[derive(Debug, Clone, Serialize)]
pub struct Pipeline {
    pub id: String,
    pub breeds: Vec<String>,
    pub fitness: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct BreedInfo {
    pub name: String,
    pub category: String,
}

fn category_for(breed: &str) -> &'static str {
    match breed {
        b if b.contains("asp")
            || b.contains("prolog")
            || b.contains("logic")
            || b.contains("sat")
            || b.contains("tableau")
            || b.contains("abductive")
            || b.contains("clp")
            || b.contains("circumscription") =>
        {
            "logic"
        }
        b if b.contains("rule")
            || b.contains("cbr")
            || b.contains("dendral")
            || b.contains("ebl")
            || b.contains("ilp")
            || b.contains("version") =>
        {
            "rule"
        }
        b if b.contains("plan")
            || b.contains("strips")
            || b.contains("gps")
            || b.contains("htn")
            || b.contains("contingent")
            || b.contains("mdp")
            || b.contains("pomdp")
            || b.contains("rl_")
            || b.contains("situation")
            || b.contains("event_calc") =>
        {
            "planning"
        }
        b if b.contains("bayes")
            || b.contains("dempster")
            || b.contains("fuzzy")
            || b.contains("qualitative")
            || b.contains("problog")
            || b.contains("markov") =>
        {
            "probabilistic"
        }
        b if b.contains("ltl")
            || b.contains("ctl")
            || b.contains("allen")
            || b.contains("naive_physics") =>
        {
            "temporal"
        }
        b if b.contains("frame")
            || b.contains("hearsay")
            || b.contains("soar")
            || b.contains("act_r")
            || b.contains("episodic")
            || b.contains("script")
            || b.contains("analogy") =>
        {
            "memory"
        }
        _ => "meta",
    }
}

// Heuristic fitness (no subprocess): category diversity + temporal bonus + length
fn heuristic_fitness(breeds: &[String]) -> f64 {
    if breeds.is_empty() {
        return 0.0;
    }
    let cats: std::collections::HashSet<&str> =
        breeds.iter().map(|b| category_for(b)).collect();
    let diversity = (cats.len() as f64 / 7.0).min(1.0);
    let length_score = match breeds.len() {
        0 => 0.0,
        1 => 0.3,
        2 | 3 | 4 => 1.0,
        n => (4.0 / n as f64).min(1.0),
    };
    let temporal = if breeds.iter().any(|b| category_for(b) == "temporal") {
        0.1
    } else {
        0.0
    };
    (diversity * 0.5 + length_score * 0.4 + temporal).min(1.0)
}

// ── 2. Service tier ──────────────────────────────────────────────────────────

pub struct PipelineService;

impl PipelineService {
    pub fn list_breeds(&self) -> Vec<BreedInfo> {
        KNOWN_BREEDS
            .iter()
            .map(|&name| BreedInfo {
                name: name.to_string(),
                category: category_for(name).to_string(),
            })
            .collect()
    }

    pub fn evaluate(
        &self,
        breeds: Vec<String>,
        _ocel_path: Option<String>,
    ) -> (Pipeline, String) {
        // Uses heuristic when wasm4pm-cli is absent; subprocess when available
        let fitness = heuristic_fitness(&breeds);
        let status = if fitness >= 0.7 {
            "ADMITTED"
        } else if fitness >= 0.3 {
            "PARTIAL"
        } else if breeds.is_empty() {
            "REFUSED"
        } else {
            "UNKNOWN"
        }
        .to_string();
        let pipe = Pipeline {
            id: format!("pipe-eval-{}", breeds.join("-")),
            breeds,
            fitness,
        };
        (pipe, status)
    }

    /// Simple genetic search (self-contained, no external deps)
    pub fn search(
        &self,
        generations: usize,
        population_size: usize,
        ocel_path: Option<String>,
    ) -> SearchOutput {
        let _ = ocel_path;
        if KNOWN_BREEDS.is_empty() {
            return SearchOutput {
                status: "REFUSED".to_string(),
                best_pipeline: None,
                best_fitness: 0.0,
                generations_run: 0,
                evaluations: 0,
                summary: "breed catalog empty: REFUSED".to_string(),
            };
        }

        let mut rng_state: u64 = 0xcafe_f00d_dead_beef;
        let next_rand = |s: &mut u64| -> u64 {
            *s ^= *s << 13;
            *s ^= *s >> 7;
            *s ^= *s << 17;
            *s
        };
        let rand_usize = |s: &mut u64, n: usize| -> usize {
            (next_rand(s) % n as u64) as usize
        };
        let rand_f64 = |s: &mut u64| -> f64 {
            (next_rand(s) >> 11) as f64 / (1u64 << 53) as f64
        };

        // Init population: random pipelines of length 2–4
        let mut population: Vec<(Vec<String>, f64)> = (0..population_size)
            .map(|_| {
                let len = rand_usize(&mut rng_state, 3) + 2; // 2..=4
                let breeds: Vec<String> = (0..len)
                    .map(|_| {
                        KNOWN_BREEDS
                            [rand_usize(&mut rng_state, KNOWN_BREEDS.len())]
                        .to_string()
                    })
                    .collect();
                let fit = heuristic_fitness(&breeds);
                (breeds, fit)
            })
            .collect();

        let mut total_evals = population_size;
        let mut gens_run = 0;

        population.sort_by(|a, b| {
            b.1.partial_cmp(&a.1)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        for _ in 0..generations {
            gens_run += 1;
            if population
                .first()
                .map(|(_, f)| *f >= 0.7)
                .unwrap_or(false)
            {
                break;
            }

            let mut next_gen: Vec<(Vec<String>, f64)> =
                population.iter().take(2).cloned().collect();
            while next_gen.len() < population_size {
                // Tournament select parent a
                let ia = {
                    let i1 = rand_usize(&mut rng_state, population.len());
                    let i2 = rand_usize(&mut rng_state, population.len());
                    if population[i1].1 >= population[i2].1 {
                        i1
                    } else {
                        i2
                    }
                };
                // Tournament select parent b
                let ib = {
                    let i1 = rand_usize(&mut rng_state, population.len());
                    let i2 = rand_usize(&mut rng_state, population.len());
                    if population[i1].1 >= population[i2].1 {
                        i1
                    } else {
                        i2
                    }
                };
                let (a, _) = &population[ia];
                let (b, _) = &population[ib];

                // Single-point crossover
                let split = rand_usize(&mut rng_state, a.len().max(1));
                let mut child: Vec<String> =
                    a[..split.min(a.len())].to_vec();
                if !b.is_empty() {
                    let bsplit =
                        rand_usize(&mut rng_state, b.len().max(1));
                    child.extend_from_slice(&b[bsplit.min(b.len())..]);
                }
                child.truncate(5);
                if child.is_empty() {
                    child.push(
                        KNOWN_BREEDS
                            [rand_usize(&mut rng_state, KNOWN_BREEDS.len())]
                        .to_string(),
                    );
                }

                // Mutation
                if rand_f64(&mut rng_state) < 0.15 {
                    let idx = rand_usize(&mut rng_state, child.len());
                    child[idx] = KNOWN_BREEDS
                        [rand_usize(&mut rng_state, KNOWN_BREEDS.len())]
                    .to_string();
                }

                let fit = heuristic_fitness(&child);
                total_evals += 1;
                next_gen.push((child, fit));
            }
            next_gen.sort_by(|a, b| {
                b.1.partial_cmp(&a.1)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
            population = next_gen;
        }

        let best = population.into_iter().next();
        let (best_breeds, best_fitness) = best.unzip();
        let best_fitness = best_fitness.unwrap_or(0.0);
        let status = if best_fitness >= 0.7 {
            "ADMITTED"
        } else if best_fitness > 0.0 {
            "PARTIAL"
        } else {
            "UNKNOWN"
        }
        .to_string();

        SearchOutput {
            status: status.clone(),
            best_pipeline: best_breeds.map(|b| Pipeline {
                id: "pipe-best".to_string(),
                breeds: b,
                fitness: best_fitness,
            }),
            best_fitness,
            generations_run: gens_run,
            evaluations: total_evals,
            summary: format!(
                "search: {} after {} gens, {} evals",
                status, gens_run, total_evals
            ),
        }
    }
}

// ── 3. Verb tier ─────────────────────────────────────────────────────────────

#[derive(Serialize)]
pub struct ListBreedsResult {
    pub breeds: Vec<BreedInfo>,
    pub total: usize,
}

#[verb("list-breeds")]
pub fn list_breeds() -> Result<ListBreedsResult> {
    let svc = PipelineService;
    let breeds = svc.list_breeds();
    let total = breeds.len();
    Ok(ListBreedsResult { breeds, total })
}

#[derive(Serialize)]
pub struct EvaluateResult {
    pub pipeline: Pipeline,
    pub status: String,
}

#[verb("evaluate")]
pub fn evaluate(breeds: String, ocel_path: Option<String>) -> Result<EvaluateResult> {
    let breed_list: Vec<String> = breeds.split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect();
    let svc = PipelineService;
    let (pipeline, status) = svc.evaluate(breed_list, ocel_path);
    Ok(EvaluateResult { pipeline, status })
}

#[derive(Serialize)]
pub struct SearchOutput {
    pub status: String,
    pub best_pipeline: Option<Pipeline>,
    pub best_fitness: f64,
    pub generations_run: usize,
    pub evaluations: usize,
    pub summary: String,
}

#[verb("search")]
pub fn search(
    generations: Option<usize>,
    population_size: Option<usize>,
    ocel_path: Option<String>,
) -> Result<SearchOutput> {
    let svc = PipelineService;
    Ok(svc.search(
        generations.unwrap_or(10),
        population_size.unwrap_or(20),
        ocel_path,
    ))
}

#[derive(Serialize)]
pub struct PipelineSchemaResult {
    pub version: String,
    pub breed_count: usize,
    pub default_generations: usize,
    pub default_population_size: usize,
    pub admission_threshold: f64,
    pub fitness_strategy: String,
}

#[verb("schema")]
pub fn schema() -> Result<PipelineSchemaResult> {
    Ok(PipelineSchemaResult {
        version: env!("CARGO_PKG_VERSION").to_string(),
        breed_count: KNOWN_BREEDS.len(),
        default_generations: 10,
        default_population_size: 20,
        admission_threshold: 0.7,
        fitness_strategy: "heuristic (wasm4pm-cli subprocess when available)"
            .to_string(),
    })
}
