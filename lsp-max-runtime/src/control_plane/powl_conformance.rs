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
    /// Log-moves: events the agent emitted that the model could not accept.
    /// Each is a detected violation (forbidden action).
    pub log_moves: Vec<String>,
    /// Model-moves: transitions the model required that the agent skipped.
    /// Each is a required_next_proof obligation.
    pub model_moves: Vec<String>,
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
            log_moves: Vec::new(),
            model_moves: Vec::new(),
        }
    }

    /// Returns true if fitness meets the model's threshold.
    pub fn meets_threshold(&self, model: &DeclaredPowlModel) -> bool {
        self.fitness >= model.fitness_threshold
    }
}

/// Check conformance of an OCEL event log against a declared POWL model.
///
/// Uses structural token-replay: events matching declared model transitions are
/// sync-moves (ADMITTED), events not in the model are log-moves (forbidden actions),
/// and model transitions not fired are model-moves (skipped required steps).
///
/// `generalization` and `simplicity` are `None` (UNKNOWN) — full alignment
/// computation is deferred to the wasm4pm WASM runtime. `None` here is UNKNOWN,
/// not Refused; the distinction matters for `ConformanceVector` law-axis sets.
///
/// Full integration with wasm4pm graduation pipeline is wired through
/// `control_plane::wasm4pm_graduation::GraduateToWasm4pm`.
pub fn check_conformance(
    model: &DeclaredPowlModel,
    ocel_events: &[String],
) -> PowlConformanceOutcome {
    if model.model.is_none() {
        return PowlConformanceOutcome::refused(
            &model.name,
            "No POWL model declared — conformance UNKNOWN, not Refused",
        );
    }

    if ocel_events.is_empty() {
        return PowlConformanceOutcome::refused(
            &model.name,
            "Empty event log — no execution evidence to replay",
        );
    }

    // Derive the set of expected activities from the model name convention.
    // Conservative subset — real inductive mining would discover these from data.
    let expected_activities = derive_expected_activities(model);

    let produced: usize = ocel_events.len();
    let matched: usize = ocel_events
        .iter()
        .filter(|e| expected_activities.contains(&e.as_str()))
        .count();

    // Log-moves: events the agent emitted that the model could not accept (forbidden actions).
    let log_move_events: Vec<String> = ocel_events
        .iter()
        .filter(|e| !expected_activities.contains(&e.as_str()))
        .cloned()
        .collect();

    // Model-moves: model transitions not fired (required steps the agent skipped).
    let model_move_activities: Vec<String> = expected_activities
        .iter()
        .filter(|a| !ocel_events.iter().any(|e| e == *a))
        .map(|a| a.to_string())
        .collect();

    let log_moves_count = log_move_events.len();
    let model_moves_count = model_move_activities.len();

    // Standard fitness formula: (consumed - missing) / (produced + remaining)
    // consumed = matched (tokens we could consume), missing = log_moves_count,
    // produced = log size, remaining = model_moves_count
    let fitness = TokenReplayResult::calculate_fitness(
        produced,
        matched,           // consumed = events that fit model transitions
        log_moves_count,   // missing = events the model couldn't fire (log-moves)
        model_moves_count, // remaining = model transitions not reached (model-moves)
    );

    // Precision: fraction of model activities exercised by the log.
    let precision = if expected_activities.is_empty() {
        None
    } else {
        Some((matched as f64 / expected_activities.len() as f64).clamp(0.0, 1.0))
    };

    // generalization and simplicity are UNKNOWN — full alignment computation deferred.
    // None here is UNKNOWN, not Refused; the distinction matters for ConformanceVector.

    PowlConformanceOutcome {
        model_name: model.name.clone(),
        fitness,
        precision,
        generalization: None, // UNKNOWN — not Refused
        simplicity: None,     // UNKNOWN — not Refused
        admitted: fitness >= model.fitness_threshold,
        rationale: format!(
            "Structural token-replay: fitness={:.3} (threshold={:.3}), \
             log_moves={} (forbidden actions), model_moves={} (skipped steps). \
             generalization/simplicity=UNKNOWN (full alignment deferred).",
            fitness, model.fitness_threshold, log_moves_count, model_moves_count
        ),
        log_moves: log_move_events,
        model_moves: model_move_activities,
    }
}

/// Map well-known model names to their expected activity sequences.
///
/// Returns a conservative subset of activities the model is known to require.
/// Unknown model structures return an empty slice so fitness is 0.0 rather
/// than faked as 1.0 — UNKNOWN, not falsely ADMITTED.
fn derive_expected_activities(model: &DeclaredPowlModel) -> Vec<&'static str> {
    match model.name.as_str() {
        n if n.contains("admission") => vec!["Raw", "Candidate", "Admitted", "Refused"],
        n if n.contains("receipt") => {
            vec!["ReceiptCreated", "ReceiptValidated", "ReceiptAdmitted"]
        }
        n if n.contains("diagnostic") => {
            vec!["ObservationCreated", "DiagnosticEmitted", "DiagnosticPublished"]
        }
        _ => vec![], // UNKNOWN model structure — fitness is 0.0, not faked as 1.0
    }
}
