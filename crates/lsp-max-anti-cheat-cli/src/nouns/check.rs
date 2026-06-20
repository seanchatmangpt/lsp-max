use clap_noun_verb::Result;
use clap_noun_verb_macros::verb;
use lsp_max_anti_cheat::{
    engine::{evaluate_diagnostics, evaluate_diagnostics_with_config, scan_directory},
    AntiLlmDiagnostic,
};
use serde::Serialize;
use std::path::Path;

// ===== Domain Tier =====
#[derive(Serialize)]
pub struct CheckResult {
    pub path: String,
    pub observations_count: usize,
    pub diagnostics: Vec<AntiLlmDiagnostic>,
    pub summary: CheckSummary,
    pub exit_code: u32,
}

#[derive(Serialize)]
pub struct CheckSummary {
    pub total: usize,
    pub blocking: usize,
    pub warnings: usize,
}

impl CheckSummary {
    fn from_diagnostics(diags: &[AntiLlmDiagnostic]) -> Self {
        let blocking = diags.iter().filter(|d| d.blocking).count();
        let warnings = diags.len() - blocking;
        Self {
            total: diags.len(),
            blocking,
            warnings,
        }
    }
}

// ===== Service Tier =====
pub struct CheckService;

impl CheckService {
    pub fn run_all_checks(
        path: &str,
        config_path: Option<&str>,
    ) -> std::result::Result<CheckResult, String> {
        let path_obj = Path::new(path);
        if !path_obj.exists() {
            return Err(format!("Path does not exist: {}", path));
        }

        let observations = scan_directory(path)?;
        let diagnostics = if let Some(cfg_path) = config_path {
            evaluate_diagnostics_with_config(&observations, cfg_path)?
        } else {
            evaluate_diagnostics(&observations)
        };

        let summary = CheckSummary::from_diagnostics(&diagnostics);
        let exit_code = if summary.blocking > 0 { 1 } else { 0 };

        Ok(CheckResult {
            path: path.to_string(),
            observations_count: observations.len(),
            diagnostics,
            summary,
            exit_code,
        })
    }

    pub fn check_category(
        path: &str,
        category: &str,
        config_path: Option<&str>,
    ) -> std::result::Result<CheckResult, String> {
        let result = Self::run_all_checks(path, config_path)?;
        let filtered: Vec<_> = result
            .diagnostics
            .iter()
            .filter(|d| d.category == category)
            .cloned()
            .collect();

        let summary = CheckSummary::from_diagnostics(&filtered);
        let exit_code = if summary.blocking > 0 { 1 } else { 0 };

        Ok(CheckResult {
            path: result.path,
            observations_count: result.observations_count,
            diagnostics: filtered,
            summary,
            exit_code,
        })
    }
}

// ===== Verb Tier (CLI) =====

#[verb("all")]
pub fn check_all(
    #[arg(long, default_value = ".")] path: String,
    #[arg(long)] config: Option<String>,
) -> Result<CheckResult> {
    let config_ref = config.as_deref();
    CheckService::run_all_checks(&path, config_ref)
        .map_err(clap_noun_verb::error::NounVerbError::execution_error)
}

#[verb("tower-lsp")]
pub fn check_tower_lsp(
    #[arg(long, default_value = ".")] path: String,
    #[arg(long)] config: Option<String>,
) -> Result<CheckResult> {
    let config_ref = config.as_deref();
    CheckService::check_category(&path, "surface", config_ref)
        .map_err(clap_noun_verb::error::NounVerbError::execution_error)
}

#[verb("victory-language")]
pub fn check_victory_language(
    #[arg(long, default_value = ".")] path: String,
    #[arg(long)] config: Option<String>,
) -> Result<CheckResult> {
    let config_ref = config.as_deref();
    CheckService::check_category(&path, "claims", config_ref)
        .map_err(clap_noun_verb::error::NounVerbError::execution_error)
}

#[verb("receipts")]
pub fn check_receipts(
    #[arg(long, default_value = ".")] path: String,
    #[arg(long)] config: Option<String>,
) -> Result<CheckResult> {
    let config_ref = config.as_deref();
    CheckService::check_category(&path, "receipts", config_ref)
        .map_err(clap_noun_verb::error::NounVerbError::execution_error)
}

#[verb("routes")]
pub fn check_routes(
    #[arg(long, default_value = ".")] path: String,
    #[arg(long)] config: Option<String>,
) -> Result<CheckResult> {
    let config_ref = config.as_deref();
    CheckService::check_category(&path, "routes", config_ref)
        .map_err(clap_noun_verb::error::NounVerbError::execution_error)
}

#[verb("authority")]
pub fn check_authority(
    #[arg(long, default_value = ".")] path: String,
    #[arg(long)] config: Option<String>,
) -> Result<CheckResult> {
    let config_ref = config.as_deref();
    CheckService::check_category(&path, "authority", config_ref)
        .map_err(clap_noun_verb::error::NounVerbError::execution_error)
}
