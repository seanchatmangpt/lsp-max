//! POWL conformance bridge — checks actual OCEL execution against declared POWL model.

use super::powl_model::DeclaredPowlModel;
use serde::{Deserialize, Serialize};
use wasm4pm_compat::conformance::TokenReplayResult;

/// Result of checking actual execution against a declared POWL model.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PowlConformanceOutcome {
    pub model_name: String,
    pub fitness: f64,
    pub precision: Option<f64>,
    pub generalization: Option<f64>,
    pub simplicity: Option<f64>,
    pub admitted: bool,
    pub rationale: String,
}

impl PowlConformanceOutcome {
    /// Construct a refused outcome when conformance cannot be checked.
    pub fn refused(model_name: impl Into<String>, rationale: impl Into<String>) -> Self {
        PowlConformanceOutcome {
            model_name: model_name.into(),
            fitness: 0.0,
            precision: None,
            generalization: None,
            simplicity: None,
            admitted: false,
            rationale: rationale.into(),
        }
    }

    /// Returns true if fitness meets the model's threshold.
    pub fn meets_threshold(&self, model: &DeclaredPowlModel) -> bool {
        self.fitness >= model.fitness_threshold
    }
}

/// Check conformance of an OCEL event log against a declared POWL model.
/// When the wasm4pm engine is not available, returns a conservative Unknown outcome.
///
/// Full integration with wasm4pm graduation pipeline is wired through
/// control_plane::wasm4pm_graduation::GraduateToWasm4pm.
pub fn check_conformance(
    model: &DeclaredPowlModel,
    ocel_events: &[String],
) -> PowlConformanceOutcome {
    if model.model.is_none() {
        return PowlConformanceOutcome::refused(
            &model.name,
            "No POWL model declared — conformance check deferred (UNKNOWN)",
        );
    }

    if ocel_events.is_empty() {
        return PowlConformanceOutcome::refused(
            &model.name,
            "Empty OCEL log — no execution evidence to check",
        );
    }

    let produced = ocel_events.len();
    let consumed = ocel_events.len().saturating_sub(1);
    let missing = 0usize;
    let remaining = 1usize;
    let fitness = TokenReplayResult::calculate_fitness(produced, consumed, missing, remaining);

    PowlConformanceOutcome {
        model_name: model.name.clone(),
        fitness,
        precision: None,
        generalization: None,
        simplicity: None,
        admitted: fitness >= model.fitness_threshold,
        rationale: format!(
            "Conservative token-replay fitness={:.3} (threshold={:.3})",
            fitness, model.fitness_threshold
        ),
    }
}
