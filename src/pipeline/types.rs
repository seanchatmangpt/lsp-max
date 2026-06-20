use serde::{Deserialize, Serialize};

/// A wasm4pm cognitive breed name (must match wasm4pm-cognition BreedId string IDs).
pub type BreedName = String;

/// Configuration passed to a breed node during pipeline evaluation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BreedNodeConfig {
    /// The breed to invoke at this pipeline node.
    pub breed: BreedName,
    /// Arbitrary JSON params forwarded to the breed's BreedInput.params.
    pub params: serde_json::Value,
}

/// A linear pipeline: sequence of breed operations applied in order.
///
/// For the initial implementation, pipelines are linear chains (not trees).
/// Tree pipelines are CANDIDATE for a future iteration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BreedPipeline {
    /// Unique identifier for this pipeline instance.
    pub id: String,
    /// Ordered sequence of breed nodes to execute.
    pub nodes: Vec<BreedNodeConfig>,
}

/// Bounded status for pipeline operations.
///
/// All variants use bounded language. No victory terms.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PipelineBoundedStatus {
    /// Fitness threshold met; pipeline admitted for this evaluation.
    Admitted,
    /// Search in progress or partial convergence observed.
    Partial,
    /// Insufficient data to evaluate; gap in tracing or precondition not met.
    Unknown,
    /// Hard failure: breed error, empty pipeline, or unrecoverable state.
    Refused,
    /// Gate condition blocks evaluation; resolve active ANDON before retrying.
    Blocked,
}

impl PipelineBoundedStatus {
    /// Returns the canonical uppercase string representation of this status.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Admitted => "ADMITTED",
            Self::Partial => "PARTIAL",
            Self::Unknown => "UNKNOWN",
            Self::Refused => "REFUSED",
            Self::Blocked => "BLOCKED",
        }
    }
}

impl std::fmt::Display for PipelineBoundedStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Result of evaluating a single pipeline against an OCEL event log.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineEvalResult {
    /// ID of the evaluated pipeline (matches [`BreedPipeline::id`]).
    pub pipeline_id: String,
    /// Fitness score in [0.0, 1.0]. Higher = better conformance.
    pub fitness: f64,
    /// Bounded status of this evaluation.
    pub status: PipelineBoundedStatus,
    /// Breed-level sub-results, one entry per node in evaluation order.
    pub node_statuses: Vec<BreedNodeStatus>,
    /// Human-readable summary (one line, no victory language).
    pub summary: String,
}

/// Status of a single breed node within a pipeline evaluation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BreedNodeStatus {
    /// The breed that was evaluated.
    pub breed: BreedName,
    /// Bounded status for this node's execution.
    pub status: PipelineBoundedStatus,
    /// Short detail string describing the node outcome.
    pub detail: String,
}

/// Options for a TPOT2-style pipeline search.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineSearchConfig {
    /// Number of candidate pipelines in each generation.
    pub population_size: usize,
    /// Number of generations to evolve.
    pub generations: usize,
    /// Probability of a node mutation per generation (0.0–1.0).
    pub mutation_rate: f64,
    /// Minimum fitness to consider a pipeline ADMITTED (0.0–1.0).
    pub admission_threshold: f64,
    /// Maximum pipeline length (number of breed nodes).
    pub max_pipeline_length: usize,
    /// Minimum pipeline length.
    pub min_pipeline_length: usize,
}

impl Default for PipelineSearchConfig {
    fn default() -> Self {
        Self {
            population_size: 20,
            generations: 10,
            mutation_rate: 0.1,
            admission_threshold: 0.7,
            max_pipeline_length: 5,
            min_pipeline_length: 1,
        }
    }
}

/// Output of a TPOT2 pipeline search run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineSearchResult {
    /// Bounded status of the overall search run.
    pub status: PipelineBoundedStatus,
    /// Best pipeline found, if any generation produced a candidate.
    pub best_pipeline: Option<BreedPipeline>,
    /// Fitness score of the best pipeline (0.0 if no candidate found).
    pub best_fitness: f64,
    /// Number of generations actually executed.
    pub generations_run: usize,
    /// Total number of pipeline evaluations performed.
    pub evaluations: usize,
    /// Human-readable summary (one line, no victory language).
    pub summary: String,
}
