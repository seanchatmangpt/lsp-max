use clap_noun_verb::Result;
use clap_noun_verb_macros::verb;
use serde::Serialize;
use tower_lsp_max_runtime::AutonomicMesh;

// ==============================================================================
// 1. Domain Tier
// Pure Rust structs/enums representing domain entities.
// ==============================================================================

#[derive(Debug, Clone, Serialize)]
pub struct DiagnosticIssue {
    pub file: String,
    pub line: u32,
    pub message: String,
    pub severity: DiagnosticSeverity,
}

#[derive(Debug, Clone, Serialize)]
pub enum DiagnosticSeverity {
    Error,
    Warning,
    Info,
    Hint,
}

// ==============================================================================
// 2. Service Tier
// A pure Rust `DiagnosticsService` struct with methods for business logic.
// ==============================================================================

pub struct DiagnosticsService {
    state_path: String,
}

impl DiagnosticsService {
    pub fn new() -> Self {
        Self {
            state_path: crate::nouns::get_state_path(),
        }
    }

    pub fn run(&self, target_path: &str) -> std::result::Result<Vec<DiagnosticIssue>, String> {
        let mesh = AutonomicMesh::load_from_file(&self.state_path).map_err(|e| e.to_string())?;

        let mut issues = Vec::new();

        if target_path == "all" || target_path.is_empty() {
            for (id, inst) in &mesh.instances {
                for diag in &inst.diagnostics {
                    issues.push(DiagnosticIssue {
                        file: id.clone(),
                        line: 1,
                        message: diag.lsp.message.clone(),
                        severity: match diag.lsp.severity {
                            Some(s) if s == lsp_types::DiagnosticSeverity::ERROR => {
                                DiagnosticSeverity::Error
                            }
                            Some(s) if s == lsp_types::DiagnosticSeverity::WARNING => {
                                DiagnosticSeverity::Warning
                            }
                            Some(s) if s == lsp_types::DiagnosticSeverity::INFORMATION => {
                                DiagnosticSeverity::Info
                            }
                            Some(s) if s == lsp_types::DiagnosticSeverity::HINT => {
                                DiagnosticSeverity::Hint
                            }
                            _ => DiagnosticSeverity::Error,
                        },
                    });
                }
            }
        } else if let Some(inst) = mesh.instances.get(target_path) {
            for diag in &inst.diagnostics {
                issues.push(DiagnosticIssue {
                    file: target_path.to_string(),
                    line: 1,
                    message: diag.lsp.message.clone(),
                    severity: match diag.lsp.severity {
                        Some(s) if s == lsp_types::DiagnosticSeverity::ERROR => {
                            DiagnosticSeverity::Error
                        }
                        Some(s) if s == lsp_types::DiagnosticSeverity::WARNING => {
                            DiagnosticSeverity::Warning
                        }
                        Some(s) if s == lsp_types::DiagnosticSeverity::INFORMATION => {
                            DiagnosticSeverity::Info
                        }
                        Some(s) if s == lsp_types::DiagnosticSeverity::HINT => {
                            DiagnosticSeverity::Hint
                        }
                        _ => DiagnosticSeverity::Error,
                    },
                });
            }
        }

        Ok(issues)
    }

    pub fn report(&self, format: &str) -> std::result::Result<String, String> {
        let mesh = AutonomicMesh::load_from_file(&self.state_path).map_err(|e| e.to_string())?;

        let report_content = if format == "json" {
            serde_json::to_string_pretty(&mesh.to_state()).map_err(|e| e.to_string())?
        } else {
            let mut report = String::new();
            for (id, inst) in &mesh.instances {
                report.push_str(&format!(
                    "Instance: {} | Phase: {} | Conformance Score: {}\n",
                    id,
                    inst.phase,
                    inst.conformance_score()
                ));
                for diag in &inst.diagnostics {
                    report.push_str(&format!(
                        "  - [{:?}] {}: {}\n",
                        diag.lsp.severity, diag.diagnostic_id, diag.lsp.message
                    ));
                }
            }
            report
        };

        Ok(report_content)
    }

    pub fn clear(
        &self,
        instance_id: &str,
        diagnostic_id: &str,
    ) -> std::result::Result<bool, String> {
        let mut mesh =
            AutonomicMesh::load_from_file(&self.state_path).map_err(|e| e.to_string())?;

        let cmd = format!("clear {} {}", instance_id, diagnostic_id);
        mesh.run_command(&cmd)?;

        mesh.save_to_file(&self.state_path)
            .map_err(|e| e.to_string())?;

        Ok(true)
    }

    pub fn watch(&self, target_path: &str) -> bool {
        if let Ok(mesh) = AutonomicMesh::load_from_file(&self.state_path) {
            return mesh.instances.contains_key(target_path);
        }
        false
    }

    pub fn diagnose(
        &self,
        instance_id: &str,
        diagnostic_id: &str,
        law_id: &str,
        severity: &str,
        message: &str,
    ) -> std::result::Result<bool, String> {
        let mut mesh =
            AutonomicMesh::load_from_file(&self.state_path).map_err(|e| e.to_string())?;

        let cmd = format!(
            "diagnose {} {} {} {} {}",
            instance_id, diagnostic_id, law_id, severity, message
        );
        mesh.run_command(&cmd)?;

        mesh.save_to_file(&self.state_path)
            .map_err(|e| e.to_string())?;

        Ok(true)
    }
}

// ==============================================================================
// 3. CLI Tier
// `#[verb]` functions mapping primitive inputs, calling the Service tier,
// and returning `<Verb>Result` structs implementing `Serialize`.
// ==============================================================================

#[derive(Serialize)]
pub struct RunResult {
    pub issues: Vec<DiagnosticIssue>,
    pub count: usize,
}

#[verb("run")]
pub fn run(target: String) -> Result<RunResult> {
    let service = DiagnosticsService::new();
    let issues = service
        .run(&target)
        .map_err(clap_noun_verb::error::NounVerbError::execution_error)?;
    let count = issues.len();

    Ok(RunResult { issues, count })
}

#[derive(Serialize)]
pub struct ReportResult {
    pub report_content: String,
}

#[verb("report")]
pub fn report(format: String) -> Result<ReportResult> {
    let service = DiagnosticsService::new();
    let report_content = service
        .report(&format)
        .map_err(clap_noun_verb::error::NounVerbError::execution_error)?;

    Ok(ReportResult { report_content })
}

#[derive(Serialize)]
pub struct ClearResult {
    pub success: bool,
    pub instance_id: String,
    pub diagnostic_id: String,
}

#[verb("clear")]
pub fn clear(instance_id: String, diagnostic_id: String) -> Result<ClearResult> {
    let service = DiagnosticsService::new();
    let success = service
        .clear(&instance_id, &diagnostic_id)
        .map_err(clap_noun_verb::error::NounVerbError::execution_error)?;

    Ok(ClearResult {
        success,
        instance_id,
        diagnostic_id,
    })
}

#[derive(Serialize)]
pub struct WatchResult {
    pub watching: bool,
    pub target: String,
}

#[verb("watch")]
pub fn watch(target: String) -> Result<WatchResult> {
    let service = DiagnosticsService::new();
    let watching = service.watch(&target);

    Ok(WatchResult { watching, target })
}

#[derive(Serialize)]
pub struct DiagnoseResult {
    pub success: bool,
    pub instance_id: String,
    pub diagnostic_id: String,
    pub message: String,
}

#[verb("diagnose")]
pub fn diagnose(
    instance_id: String,
    diagnostic_id: String,
    law_id: String,
    severity: String,
    message: String,
) -> Result<DiagnoseResult> {
    let service = DiagnosticsService::new();
    let success = service
        .diagnose(&instance_id, &diagnostic_id, &law_id, &severity, &message)
        .map_err(clap_noun_verb::error::NounVerbError::execution_error)?;

    Ok(DiagnoseResult {
        success,
        instance_id,
        diagnostic_id,
        message,
    })
}
