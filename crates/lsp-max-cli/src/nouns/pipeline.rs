use clap_noun_verb::Result;
use clap_noun_verb_macros::verb;
use serde::Serialize;

use lsp_max::pipeline::catalog::{breed_category, BreedCategory, KNOWN_BREEDS};
use lsp_max::pipeline::fitness::{auto_evaluator, BreedFitnessEvaluator};
use lsp_max::pipeline::search::{FitnessEvaluator, PipelineSearch};
use lsp_max::pipeline::types::{
    BreedName, BreedPipeline, PipelineBoundedStatus, PipelineSearchConfig, PipelineSearchResult,
};

// ── 1. CLI-local view types ───────────────────────────────────────────────────
//
// The breed catalog, fitness scoring, and genetic search all live in
// `lsp_max::pipeline`. This noun owns only the actuation grammar (verbs) and the
// JSON view shapes the CLI emits; all domain logic delegates to the library.

/// A pipeline view for CLI output: ordered breed names plus a fitness score.
#[derive(Debug, Clone, Serialize)]
pub struct Pipeline {
    pub id: String,
    pub breeds: Vec<String>,
    pub fitness: f64,
}

/// One breed entry for the `list-breeds` view: name plus its catalog category.
#[derive(Debug, Clone, Serialize)]
pub struct BreedInfo {
    pub name: String,
    pub category: String,
}

/// Lowercase category tag for a [`BreedCategory`], for the CLI `list-breeds` view.
fn category_label(category: &BreedCategory) -> &'static str {
    match category {
        BreedCategory::LogicBased => "logic",
        BreedCategory::RuleBased => "rule",
        BreedCategory::PlanningBased => "planning",
        BreedCategory::Probabilistic => "probabilistic",
        BreedCategory::Temporal => "temporal",
        BreedCategory::MemoryBased => "memory",
        BreedCategory::MetaBased => "meta",
    }
}

/// Map a fitness score in [0.0, 1.0] to a bounded status for a single evaluation.
///
/// An empty pipeline is REFUSED. Otherwise the score is bucketed against the
/// library admission threshold (ADMITTED), a partial-convergence band (PARTIAL),
/// or UNKNOWN when below both bands. UNKNOWN is never collapsed into either
/// polarity — it signals an inconclusive score, not a refusal.
fn eval_status(fitness: f64, breeds_empty: bool) -> PipelineBoundedStatus {
    if breeds_empty {
        PipelineBoundedStatus::Refused
    } else if fitness >= PipelineSearchConfig::default().admission_threshold {
        PipelineBoundedStatus::Admitted
    } else if fitness >= 0.3 {
        PipelineBoundedStatus::Partial
    } else {
        PipelineBoundedStatus::Unknown
    }
}

/// Bridges a [`BreedFitnessEvaluator`] (the `auto_evaluator` output: subprocess or
/// heuristic) into the [`FitnessEvaluator`] trait that drives [`PipelineSearch`].
///
/// Both traits share the `evaluate(&[String]) -> f64` signature but are distinct
/// types; this adapter forwards the call so the search engine is driven by the
/// auto-selected library evaluator without duplicating any scoring logic.
struct CliFitnessAdapter(Box<dyn BreedFitnessEvaluator>);

impl FitnessEvaluator for CliFitnessAdapter {
    fn evaluate(&self, breeds: &[BreedName]) -> f64 {
        self.0.evaluate(breeds)
    }
}

// ── 2. Service tier ──────────────────────────────────────────────────────────
//
// Poka-Yoke FM-1.1: all real logic lives here; `#[verb]` bodies stay thin.

pub struct PipelineService;

impl PipelineService {
    pub fn list_breeds(&self) -> Vec<BreedInfo> {
        KNOWN_BREEDS
            .iter()
            .map(|&name| BreedInfo {
                name: name.to_string(),
                category: category_label(&breed_category(name)).to_string(),
            })
            .collect()
    }

    /// Evaluate a breed list by delegating to the library auto-selected evaluator
    /// (wasm4pm-cli subprocess when present, heuristic otherwise).
    pub fn evaluate(&self, breeds: Vec<String>, ocel_path: Option<String>) -> (Pipeline, String) {
        let fitness = auto_evaluator(ocel_path).evaluate(&breeds);
        let status = eval_status(fitness, breeds.is_empty()).as_str().to_string();
        let pipe = Pipeline {
            id: format!("pipe-eval-{}", breeds.join("-")),
            breeds,
            fitness,
        };
        (pipe, status)
    }

    /// Run a TPOT2-style genetic search over [`KNOWN_BREEDS`], delegating to
    /// [`PipelineSearch`] from the library and driving it with the auto-selected
    /// evaluator via [`CliFitnessAdapter`].
    pub fn search(
        &self,
        generations: usize,
        population_size: usize,
        ocel_path: Option<String>,
    ) -> SearchOutput {
        let config = PipelineSearchConfig {
            population_size,
            generations,
            ..Default::default()
        };
        let evaluator = CliFitnessAdapter(auto_evaluator(ocel_path));
        let mut search = PipelineSearch::new(
            config,
            KNOWN_BREEDS,
            Box::new(evaluator),
            0xcafe_f00d_dead_beef,
        );
        SearchOutput::from(search.run())
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
    let breed_list: Vec<String> = breeds
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();
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

impl From<PipelineSearchResult> for SearchOutput {
    fn from(result: PipelineSearchResult) -> Self {
        let best_fitness = result.best_fitness;
        let best_pipeline = result.best_pipeline.map(|p: BreedPipeline| Pipeline {
            id: p.id,
            breeds: p.nodes.into_iter().map(|n| n.breed).collect(),
            fitness: best_fitness,
        });
        SearchOutput {
            status: result.status.as_str().to_string(),
            best_pipeline,
            best_fitness,
            generations_run: result.generations_run,
            evaluations: result.evaluations,
            summary: result.summary,
        }
    }
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
    let defaults = PipelineSearchConfig::default();
    Ok(PipelineSchemaResult {
        version: env!("CARGO_PKG_VERSION").to_string(),
        breed_count: KNOWN_BREEDS.len(),
        default_generations: defaults.generations,
        default_population_size: defaults.population_size,
        admission_threshold: defaults.admission_threshold,
        fitness_strategy: "heuristic (wasm4pm-cli subprocess when available)".to_string(),
    })
}
