use serde::{Deserialize, Serialize};

/// A wasm4pm cognitive breed name (must match wasm4pm-cognition BreedId string IDs)
pub type BreedName = String;

/// Configuration passed to a breed node during pipeline evaluation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BreedNodeConfig {
    pub breed: BreedName,
    /// Arbitrary JSON params forwarded to the breed's BreedInput.params
    pub params: serde_json::Value,
}

/// A linear pipeline: sequence of breed operations applied in order.
/// For the initial implementation, pipelines are linear chains (not trees).
/// Tree pipelines are CANDIDATE for a future iteration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BreedPipeline {
    pub id: String,
    pub nodes: Vec<BreedNodeConfig>,
}

/// Bounded status for pipeline operations.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PipelineBoundedStatus {
    Admitted,   // Fitness threshold met
    Partial,    // Search in progress or partial convergence
    Unknown,    // Insufficient data to evaluate
    Refused,    // Hard failure (breed error, empty pipeline)
    Blocked,    // Gate condition blocks evaluation
}

impl PipelineBoundedStatus {
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
    pub pipeline_id: String,
    /// Fitness score in [0.0, 1.0]. Higher = better.
    pub fitness: f64,
    pub status: PipelineBoundedStatus,
    /// Breed-level sub-results (one per node)
    pub node_statuses: Vec<BreedNodeStatus>,
    /// Human-readable summary (one line, no victory language)
    pub summary: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BreedNodeStatus {
    pub breed: BreedName,
    pub status: PipelineBoundedStatus,
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
    pub status: PipelineBoundedStatus,
    pub best_pipeline: Option<BreedPipeline>,
    pub best_fitness: f64,
    pub generations_run: usize,
    pub evaluations: usize,
    pub summary: String,
}
