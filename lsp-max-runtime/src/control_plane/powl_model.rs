//! Declared POWL process model for lsp-max server instances.
//! The declared model is the ΔP — what the server claims it does.
//! wasm4pm checks actual execution against this declaration.

use serde::{Deserialize, Serialize};

/// The declared POWL process model for an lsp-max server instance.
/// Holds the structural description of lawful operations the server performs.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DeclaredPowlModel {
    /// Human-readable name for this model (e.g. "lsp-max-initialize-flow")
    pub name: String,
    /// POWL model serialized as JSON (bridges to wasm4pm engine)
    pub model_json: Option<String>,
    /// Minimum fitness threshold for admission (0.0–1.0)
    pub fitness_threshold: f64,
}

impl DeclaredPowlModel {
    pub fn new(name: impl Into<String>) -> Self {
        DeclaredPowlModel {
            name: name.into(),
            model_json: None,
            fitness_threshold: 0.8,
        }
    }

    pub fn with_fitness_threshold(mut self, threshold: f64) -> Self {
        self.fitness_threshold = threshold.clamp(0.0, 1.0);
        self
    }
}
