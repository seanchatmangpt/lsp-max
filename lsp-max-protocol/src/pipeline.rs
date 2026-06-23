//! Read-only `max/*` protocol surface for the TPOT2 breed-pipeline optimizer.
//!
//! TPOT2 searches a population of candidate analysis pipelines ("breeds") and
//! reports the fittest. This module projects that optimizer onto the lsp-max
//! protocol surface: it declares the method names, the serde request/result
//! shapes, and the `TPOT2-*` diagnostic family.
//!
//! This surface is **read-only**. It emits diagnostics, search results, and
//! intents; it never mutates files. A search outcome is reported as a
//! `PipelineSearchResultMsg` and, where a law axis is unsatisfied, as zero or
//! more [`MaxDiagnostic`] values. Any future mutation would have to route
//! through the `CodeAction -> clap-noun-verb -> Receipt` chain, not through this
//! observation surface.
//!
//! All status fields carry **bounded statuses only** (`ADMITTED`, `PARTIAL`,
//! `UNKNOWN`, `REFUSED`, `BLOCKED`, `CANDIDATE`, `OPEN`). The three-state law
//! holds: `UNKNOWN` is never coerced into `ADMITTED` or `REFUSED`. In
//! particular, a requested-but-absent OCEL log yields `UNKNOWN`
//! (`TPOT2-OCEL-MISSING`) and stays `UNKNOWN`.

use crate::diagnostics::MaxDiagnostic;
use lsp_types_max::{Diagnostic, DiagnosticSeverity, NumberOrString};
use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Method name constants (read-only surface — the LSP never mutates files)
// ---------------------------------------------------------------------------

/// max/pipelineSearch — Run a breed-pipeline search and report the fittest
/// observed breeds. Read-only: returns a [`PipelineSearchResultMsg`]; never
/// writes the discovered pipeline to disk.
pub const METHOD_PIPELINE_SEARCH: &str = "max/pipelineSearch";

/// max/pipelineEvaluate — Evaluate a single named breed and report its fitness.
/// Read-only: returns a [`PipelineEvaluateResultMsg`]; never persists the breed.
pub const METHOD_PIPELINE_EVALUATE: &str = "max/pipelineEvaluate";

/// max/pipelineBreeds — Enumerate the current breed pool. Read-only: returns a
/// [`PipelineBreedsResultMsg`]; never alters the pool.
pub const METHOD_PIPELINE_BREEDS: &str = "max/pipelineBreeds";

// ---------------------------------------------------------------------------
// Request / result params (serde)
// ---------------------------------------------------------------------------

/// Parameters for `max/pipelineSearch`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineSearchParams {
    /// Number of generations to evolve. `None` lets the optimizer choose.
    pub generations: Option<usize>,
    /// Population size per generation. `None` lets the optimizer choose.
    pub population_size: Option<usize>,
    /// Path to the OCEL event log the search reads from. `None` means the
    /// optimizer uses its already-loaded observations; `Some(path)` that is
    /// absent on disk yields `UNKNOWN` (see `TPOT2-OCEL-MISSING`).
    pub ocel_path: Option<String>,
}

/// Result of `max/pipelineSearch`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineSearchResultMsg {
    /// Bounded status string (`ADMITTED`, `PARTIAL`, `UNKNOWN`, `REFUSED`, ...).
    pub status: String,
    /// Identifiers of the fittest breeds observed, best first.
    pub best_breeds: Vec<String>,
    /// Fitness of the best observed breed.
    pub best_fitness: f64,
    /// Generations actually evolved.
    pub generations_run: usize,
    /// Total breed evaluations performed.
    pub evaluations: usize,
    /// Human-readable summary of the search outcome.
    pub summary: String,
}

/// Parameters for `max/pipelineEvaluate`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineEvaluateParams {
    /// Identifier of the breed to evaluate.
    pub breed_id: String,
    /// Path to the OCEL event log to evaluate against. `Some(path)` that is
    /// absent on disk yields `UNKNOWN`.
    pub ocel_path: Option<String>,
}

/// Result of `max/pipelineEvaluate`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineEvaluateResultMsg {
    /// Bounded status string.
    pub status: String,
    /// Identifier of the evaluated breed.
    pub breed_id: String,
    /// Fitness observed for this breed.
    pub fitness: f64,
    /// Human-readable summary of the evaluation outcome.
    pub summary: String,
}

/// Parameters for `max/pipelineBreeds`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineBreedsParams {
    /// Optional substring filter applied to breed identifiers. `None` lists all.
    pub filter: Option<String>,
}

/// Result of `max/pipelineBreeds`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineBreedsResultMsg {
    /// Bounded status string.
    pub status: String,
    /// Identifiers of the breeds currently in the pool.
    pub breeds: Vec<String>,
    /// Size of the reported pool.
    pub pool_size: usize,
    /// Human-readable summary of the pool state.
    pub summary: String,
}

// ---------------------------------------------------------------------------
// TPOT2-* diagnostic family
// ---------------------------------------------------------------------------

/// Emitted when a pipeline search ends below the admission threshold
/// (status `PARTIAL` or `UNKNOWN`). The search produced a result but it does
/// not clear the bar; the law axis is not admitted.
pub const TPOT2_NONCONVERGENCE: &str = "TPOT2-NONCONVERGENCE";

/// Emitted when the breed pool is empty (status `REFUSED`). With no breeds to
/// evaluate there is nothing to admit.
pub const TPOT2_EMPTY_POOL: &str = "TPOT2-EMPTY-POOL";

/// Emitted when an OCEL path is requested but absent (status `UNKNOWN`).
/// Absence of the observation source is not a refusal — admissibility cannot be
/// determined, so this stays `UNKNOWN` and is never coerced to `REFUSED` or
/// `ADMITTED`.
pub const TPOT2_OCEL_MISSING: &str = "TPOT2-OCEL-MISSING";

/// Bounded status: the search cleared the admission threshold.
pub const STATUS_ADMITTED: &str = "ADMITTED";
/// Bounded status: a result exists but is below the admission threshold.
pub const STATUS_PARTIAL: &str = "PARTIAL";
/// Bounded status: admissibility cannot be determined.
pub const STATUS_UNKNOWN: &str = "UNKNOWN";
/// Bounded status: an explicit refusal (e.g. empty breed pool).
pub const STATUS_REFUSED: &str = "REFUSED";

/// Build a single `TPOT2-*` diagnostic with the given code, severity, and
/// message. The LSP `code` carries the family string so downstream consumers
/// can route on it without parsing the message.
fn tpot2_diagnostic(code: &str, severity: DiagnosticSeverity, message: String) -> MaxDiagnostic {
    let lsp = Diagnostic {
        severity: Some(severity),
        code: Some(NumberOrString::String(code.to_string())),
        source: Some("lsp-max:tpot2".to_string()),
        message,
        ..Default::default()
    };
    MaxDiagnostic {
        lsp,
        diagnostic_id: code.to_string(),
        law_id: code.to_string(),
        // TPOT2 governs a process-mining optimizer; its law axis is Domain.
        law_axis: crate::conformance::LawAxis::Domain,
        violated_axes: vec![crate::conformance::LawAxis::Domain.to_string()],
        ..Default::default()
    }
}

/// Map a pipeline-search outcome to zero or more `TPOT2-*` diagnostics using
/// only bounded statuses.
///
/// Read-only: this inspects an outcome and reports; it does not change pool or
/// pipeline state.
///
/// Status handling, three-state law preserved:
/// - `REFUSED` -> `TPOT2-EMPTY-POOL` (Error). An explicit refusal.
/// - `UNKNOWN` -> `TPOT2-OCEL-MISSING` (Information) **and** `TPOT2-NONCONVERGENCE`
///   (Information). `UNKNOWN` is reported as `UNKNOWN`; it is never coerced into
///   `REFUSED` or `ADMITTED`.
/// - `PARTIAL` -> `TPOT2-NONCONVERGENCE` (Warning) when `best_fitness` is below
///   `admission_threshold`.
/// - `ADMITTED` -> no diagnostics (the outcome cleared the bar).
/// - Any other (unrecognized) status -> no diagnostics; the caller's own
///   `Λ(a)` judgment governs outside this bounded set.
pub fn diagnostics_for_search(
    status: &str,
    best_fitness: f64,
    admission_threshold: f64,
) -> Vec<MaxDiagnostic> {
    match status {
        STATUS_REFUSED => vec![tpot2_diagnostic(
            TPOT2_EMPTY_POOL,
            DiagnosticSeverity::ERROR,
            "breed pool is empty; status REFUSED — no breeds to evaluate".to_string(),
        )],
        STATUS_UNKNOWN => vec![
            tpot2_diagnostic(
                TPOT2_OCEL_MISSING,
                DiagnosticSeverity::INFORMATION,
                "OCEL observation source absent; status UNKNOWN — admissibility \
                 cannot be determined and is not coerced to REFUSED or ADMITTED"
                    .to_string(),
            ),
            tpot2_diagnostic(
                TPOT2_NONCONVERGENCE,
                DiagnosticSeverity::INFORMATION,
                format!(
                    "pipeline search did not reach admission threshold {admission_threshold}; \
                     status UNKNOWN — best observed fitness {best_fitness}"
                ),
            ),
        ],
        STATUS_PARTIAL if best_fitness < admission_threshold => vec![tpot2_diagnostic(
            TPOT2_NONCONVERGENCE,
            DiagnosticSeverity::WARNING,
            format!(
                "pipeline search ended below admission threshold {admission_threshold}; \
                 status PARTIAL — best observed fitness {best_fitness}"
            ),
        )],
        // ADMITTED, or any status outside the bounded set, emits nothing.
        _ => Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn codes(diags: &[MaxDiagnostic]) -> Vec<&str> {
        diags
            .iter()
            .filter_map(|d| match &d.lsp.code {
                Some(NumberOrString::String(s)) => Some(s.as_str()),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn partial_below_threshold_emits_nonconvergence() {
        let diags = diagnostics_for_search(STATUS_PARTIAL, 0.40, 0.80);
        assert_eq!(codes(&diags), vec![TPOT2_NONCONVERGENCE]);
        assert_eq!(diags[0].lsp.severity, Some(DiagnosticSeverity::WARNING));
        assert_eq!(diags[0].law_axis, crate::conformance::LawAxis::Domain);
    }

    #[test]
    fn refused_empty_pool_emits_empty_pool_error() {
        let diags = diagnostics_for_search(STATUS_REFUSED, 0.0, 0.80);
        assert_eq!(codes(&diags), vec![TPOT2_EMPTY_POOL]);
        assert_eq!(diags[0].lsp.severity, Some(DiagnosticSeverity::ERROR));
    }

    #[test]
    fn admitted_outcome_emits_no_diagnostics() {
        let diags = diagnostics_for_search(STATUS_ADMITTED, 0.95, 0.80);
        assert!(
            diags.is_empty(),
            "an admitted outcome must not emit any TPOT2 diagnostic"
        );
    }

    #[test]
    fn unknown_path_stays_unknown_and_is_not_coerced() {
        let diags = diagnostics_for_search(STATUS_UNKNOWN, 0.0, 0.80);
        let emitted = codes(&diags);
        // The OCEL-missing signal is present and the path is reported as UNKNOWN.
        assert!(emitted.contains(&TPOT2_OCEL_MISSING));
        assert!(emitted.contains(&TPOT2_NONCONVERGENCE));
        // Three-state law: UNKNOWN must not collapse into the REFUSED signal.
        assert!(
            !emitted.contains(&TPOT2_EMPTY_POOL),
            "UNKNOWN must never be coerced into the REFUSED (empty-pool) outcome"
        );
        // No diagnostic on the UNKNOWN path carries Error severity (which is the
        // REFUSED polarity here); UNKNOWN stays informational.
        assert!(
            diags
                .iter()
                .all(|d| d.lsp.severity != Some(DiagnosticSeverity::ERROR)),
            "UNKNOWN path must not surface a REFUSED-polarity (Error) diagnostic"
        );
    }
}
