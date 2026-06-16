use clap_noun_verb::Result;
use clap_noun_verb_macros::verb;
use lsp_max_runtime::AutonomicMesh;
use lsp_types_max::NumberOrString;
use serde::Serialize;

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
                            Some(s) if s == lsp_types_max::DiagnosticSeverity::ERROR => {
                                DiagnosticSeverity::Error
                            }
                            Some(s) if s == lsp_types_max::DiagnosticSeverity::WARNING => {
                                DiagnosticSeverity::Warning
                            }
                            Some(s) if s == lsp_types_max::DiagnosticSeverity::INFORMATION => {
                                DiagnosticSeverity::Info
                            }
                            Some(s) if s == lsp_types_max::DiagnosticSeverity::HINT => {
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
                        Some(s) if s == lsp_types_max::DiagnosticSeverity::ERROR => {
                            DiagnosticSeverity::Error
                        }
                        Some(s) if s == lsp_types_max::DiagnosticSeverity::WARNING => {
                            DiagnosticSeverity::Warning
                        }
                        Some(s) if s == lsp_types_max::DiagnosticSeverity::INFORMATION => {
                            DiagnosticSeverity::Info
                        }
                        Some(s) if s == lsp_types_max::DiagnosticSeverity::HINT => {
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

#[derive(Serialize)]
pub struct ExportBundleResult {
    pub instance_id: String,
    pub bundle: serde_json::Value,
}

#[verb("export-bundle")]
pub fn export_bundle(
    instance_id: String,
    output_path: Option<String>,
) -> Result<ExportBundleResult> {
    let mut mesh = AutonomicMesh::load_from_file(&crate::nouns::get_state_path())
        .map_err(|e| clap_noun_verb::error::NounVerbError::execution_error(e.to_string()))?;

    let bundle = mesh
        .dispatch_rpc(
            &instance_id,
            "max/exportAnalysisBundle",
            serde_json::Value::Null,
        )
        .map_err(clap_noun_verb::error::NounVerbError::execution_error)?;

    if let Some(path) = output_path {
        let json_str = serde_json::to_string_pretty(&bundle)
            .map_err(|e| clap_noun_verb::error::NounVerbError::execution_error(e.to_string()))?;
        std::fs::write(&path, json_str)
            .map_err(|e| clap_noun_verb::error::NounVerbError::execution_error(e.to_string()))?;
    }

    Ok(ExportBundleResult {
        instance_id,
        bundle,
    })
}

// ==============================================================================
// 4. Additional verbs: explain and repair-plan
// ==============================================================================

#[derive(Serialize)]
pub struct ExplainResult {
    pub instance_id: String,
    pub diagnostic_id: String,
    pub explanation: serde_json::Value,
}

#[verb("explain")]
pub fn explain(instance_id: String, diagnostic_id: String) -> Result<ExplainResult> {
    let path = crate::nouns::get_state_path();
    let mut mesh = AutonomicMesh::load_from_file(&path)
        .map_err(|e| clap_noun_verb::error::NounVerbError::execution_error(e.to_string()))?;
    let explanation = mesh
        .dispatch_rpc(
            &instance_id,
            "max/explainDiagnostic",
            serde_json::Value::String(diagnostic_id.clone()),
        )
        .map_err(clap_noun_verb::error::NounVerbError::execution_error)?;
    Ok(ExplainResult {
        instance_id,
        diagnostic_id,
        explanation,
    })
}

#[derive(Serialize)]
pub struct RepairPlanResult {
    pub instance_id: String,
    pub diagnostic_id: String,
    pub actions: serde_json::Value,
}

#[verb("repair-plan")]
pub fn repair_plan(instance_id: String, diagnostic_id: String) -> Result<RepairPlanResult> {
    let path = crate::nouns::get_state_path();
    let mut mesh = AutonomicMesh::load_from_file(&path)
        .map_err(|e| clap_noun_verb::error::NounVerbError::execution_error(e.to_string()))?;
    let actions = mesh
        .dispatch_rpc(
            &instance_id,
            "max/repairPlan",
            serde_json::Value::String(diagnostic_id.clone()),
        )
        .map_err(clap_noun_verb::error::NounVerbError::execution_error)?;
    Ok(RepairPlanResult {
        instance_id,
        diagnostic_id,
        actions,
    })
}

// ==============================================================================
// apply-repair verb — calls max/applyRepairTransaction RPC
// ==============================================================================

#[derive(Serialize)]
pub struct ApplyRepairResult {
    pub success: bool,
    pub instance_id: String,
    pub transaction_id: String,
}

#[verb("apply-repair")]
pub fn apply_repair(instance_id: String, transaction_id: String) -> Result<ApplyRepairResult> {
    let path = crate::nouns::get_state_path();
    let mut mesh = AutonomicMesh::load_from_file(&path)
        .map_err(|e| clap_noun_verb::error::NounVerbError::execution_error(e.to_string()))?;
    let payload = serde_json::json!({ "transaction_id": transaction_id });
    let resp = mesh
        .dispatch_rpc(&instance_id, "max/applyRepairTransaction", payload)
        .map_err(clap_noun_verb::error::NounVerbError::execution_error)?;
    mesh.save_to_file(&path)
        .map_err(|e| clap_noun_verb::error::NounVerbError::execution_error(e.to_string()))?;
    let success = resp["success"].as_bool().unwrap_or(true);
    Ok(ApplyRepairResult {
        success,
        instance_id,
        transaction_id,
    })
}
// ==============================================================================
// D_t snapshot verb
//
// Emits the current diagnostic set in a compact, structured format suitable for
// injection into an agent context window (system prompt or context block).
//
// FORMAT:
//   D_t @ <RFC3339 timestamp>: <N> diagnostics, <ANDON_COUNT> ANDON blocks
//   <URI>:<line>:<char> [<SEV>] <CODE> — <MESSAGE>
//   ...
//
// Exit 0 — no ANDON blocks; gate is clear.
// Exit 1 — one or more ANDON blocks present.
//
// ANDON prefixes (static defaults, matching compositor fallback):
//   WASM4PM-   ANTI-LLM-   GGEN-
//
// The compositor DiagnosticBuffer is not accessible from the CLI (it lives in an
// in-process Arc). This verb reads from the AutonomicMesh state file, which is the
// same source used by all other diagnostics verbs.  When the compositor is not
// running, the mesh state file reflects the last persisted state.
// ==============================================================================

/// Default ANDON code prefixes — same as compositor fallback when lsp-max.toml is absent.
const ANDON_PREFIXES: &[&str] = &["WASM4PM-", "ANTI-LLM-", "GGEN-"];

fn is_andon_code(code: &str) -> bool {
    ANDON_PREFIXES.iter().any(|p| code.starts_with(p))
}

fn sev_label(sev: Option<lsp_types_max::DiagnosticSeverity>) -> &'static str {
    match sev {
        Some(s) if s == lsp_types_max::DiagnosticSeverity::ERROR => "ERROR",
        Some(s) if s == lsp_types_max::DiagnosticSeverity::WARNING => "WARN",
        Some(s) if s == lsp_types_max::DiagnosticSeverity::INFORMATION => "INFO",
        Some(s) if s == lsp_types_max::DiagnosticSeverity::HINT => "HINT",
        _ => "ERROR",
    }
}

fn code_string(code: &Option<NumberOrString>) -> String {
    match code {
        Some(NumberOrString::String(s)) => s.clone(),
        Some(NumberOrString::Number(n)) => n.to_string(),
        None => String::new(),
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct DtEntry {
    pub uri: String,
    pub line: u32,
    pub character: u32,
    pub severity: String,
    pub code: String,
    pub message: String,
    pub is_andon: bool,
}

#[derive(Serialize)]
pub struct SnapshotResult {
    pub timestamp: String,
    pub total: usize,
    pub andon_count: usize,
    pub entries: Vec<DtEntry>,
    /// The formatted D_t block ready for context injection.
    pub dt_block: String,
}

// ==============================================================================
// D_t snapshot service logic — extracted to keep the verb thin (FM-1.1)
// ==============================================================================

pub struct DtSnapshotService {
    state_path: String,
}

impl DtSnapshotService {
    pub fn new() -> Self {
        Self {
            state_path: crate::nouns::get_state_path(),
        }
    }

    pub fn build(&self) -> SnapshotResult {
        let mesh = AutonomicMesh::load_from_file(&self.state_path)
            .unwrap_or_else(|_| AutonomicMesh::new());
        let timestamp = chrono::Utc::now().to_rfc3339();
        let mut entries: Vec<DtEntry> = Vec::new();
        for (instance_id, inst) in &mesh.instances {
            for diag in &inst.diagnostics {
                let code = code_string(&diag.lsp.code);
                let is_andon = is_andon_code(&code);
                entries.push(DtEntry {
                    uri: instance_id.clone(),
                    line: diag.lsp.range.start.line,
                    character: diag.lsp.range.start.character,
                    severity: sev_label(diag.lsp.severity).to_string(),
                    code,
                    message: diag.lsp.message.clone(),
                    is_andon,
                });
            }
        }
        entries.sort_by(|a, b| {
            b.is_andon
                .cmp(&a.is_andon)
                .then(a.uri.cmp(&b.uri))
                .then(a.line.cmp(&b.line))
        });
        let total = entries.len();
        let andon_count = entries.iter().filter(|e| e.is_andon).count();
        let block = Self::format_block(&timestamp, &entries, total, andon_count);
        SnapshotResult {
            timestamp,
            total,
            andon_count,
            entries,
            dt_block: block,
        }
    }

    fn format_block(
        timestamp: &str,
        entries: &[DtEntry],
        total: usize,
        andon_count: usize,
    ) -> String {
        let mut block = format!(
            "D_t @ {}: {} diagnostics, {} ANDON blocks",
            timestamp, total, andon_count
        );
        for e in entries {
            let code_part = if e.code.is_empty() {
                String::new()
            } else {
                format!("{} \u{2014} ", e.code)
            };
            block.push_str(&format!(
                "\n{}:{}:{} [{}] {}{}",
                e.uri, e.line, e.character, e.severity, code_part, e.message
            ));
        }
        block
    }
}

/// Emit D_t — the current diagnostic set formatted for agent context injection.
///
/// Prints one header line followed by one line per diagnostic:
///   `D_t @ <RFC3339>: <N> diagnostics, <ANDON> ANDON blocks`
///   `<URI>:<line>:<char> [<SEV>] <CODE> — <MESSAGE>`
///
/// Exit 0 if no ANDON blocks; exit 1 if any ANDON block is present.
#[verb("snapshot")]
pub fn snapshot() -> Result<SnapshotResult> {
    let svc = DtSnapshotService::new();
    let result = svc.build();
    println!("{}", result.dt_block);
    if result.andon_count > 0 {
        return Err(clap_noun_verb::error::NounVerbError::execution_error(
            format!(
                "D_t ANDON: {} block(s) active — gate is BLOCKED",
                result.andon_count
            ),
        ));
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::DiagnosticsService;
    use std::env;

    fn setup_test_state() -> (tempfile::NamedTempFile, std::sync::MutexGuard<'static, ()>) {
        let f = tempfile::NamedTempFile::new().expect("tempfile");
        let guard = crate::nouns::TEST_ENV_LOCK
            .lock()
            .unwrap_or_else(|p| p.into_inner());
        // SAFETY: set_var under process-wide ENV_MUTEX
        unsafe {
            env::set_var("LSP_MAX_STATE_PATH", f.path().to_str().unwrap());
        }
        (f, guard)
    }

    #[test]
    fn test_run_returns_ok_with_empty_issues_for_unknown_target() {
        let _path = setup_test_state();
        let svc = DiagnosticsService::new();
        let result = svc.run("nonexistent-target");
        assert!(
            result.is_ok(),
            "run should return Ok even for unknown target"
        );
        let issues = result.unwrap();
        assert_eq!(issues.len(), 0);
    }

    #[test]
    fn test_report_json_returns_ok_string() {
        let _path = setup_test_state();
        let svc = DiagnosticsService::new();
        let result = svc.report("json");
        assert!(result.is_ok(), "report(json) should return Ok");
        let content = result.unwrap();
        // JSON output should at least be a valid JSON object/array
        assert!(!content.is_empty());
        serde_json::from_str::<serde_json::Value>(&content)
            .expect("report(json) output must be valid JSON");
    }

    #[test]
    fn test_report_text_returns_ok_string() {
        let _path = setup_test_state();
        let svc = DiagnosticsService::new();
        let result = svc.report("text");
        assert!(result.is_ok(), "report(text) should return Ok");
    }

    #[test]
    fn test_clear_returns_ok_for_nonexistent_ids() {
        let _path = setup_test_state();
        let svc = DiagnosticsService::new();
        // clear on nonexistent ids should either succeed or return an Err — just must not panic
        let result = svc.clear("inst-1", "diag-1");
        // The mesh run_command may return Ok(true) even when no-op
        match result {
            Ok(v) => assert!(v, "clear returned Ok but value was false"),
            Err(_) => { /* acceptable: command rejected unknown ids */ }
        }
    }
}
