use clap_noun_verb::error::NounVerbError;
use clap_noun_verb::Result;
use clap_noun_verb_macros::verb;
use lsp_max_runtime::AutonomicMesh;
use serde::Serialize;
use std::path::Path;

// ==============================================================================
// 1. Domain Tier
// ==============================================================================

#[derive(Debug, Clone, Serialize)]
pub struct DoctorCheck {
    pub name: String,
    pub status: String,
    pub detail: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct DoctorHealthResult {
    pub status: String,
    pub checks: Vec<DoctorCheck>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ComplianceViolation {
    pub file: String,
    pub line: usize,
    pub kind: String,
    pub text: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ComplianceResult {
    pub violations: Vec<ComplianceViolation>,
    pub status: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct DoctorLintResult {
    pub error_count: usize,
    pub warning_count: usize,
    pub info_count: usize,
    pub status: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct UndocumentedFn {
    pub file: String,
    pub line: usize,
    pub fn_name: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct DoctorDocsResult {
    pub undocumented_fns: Vec<UndocumentedFn>,
    pub coverage_status: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct DoctorReport {
    pub health: DoctorHealthResult,
    pub compliance: ComplianceResult,
    pub lint: DoctorLintResult,
    pub overall_status: String,
}

// ==============================================================================
// 2. Service Tier
// ==============================================================================

pub struct DoctorService {
    state_path: String,
}

impl DoctorService {
    pub fn new() -> Self {
        Self { state_path: crate::nouns::get_state_path() }
    }

    pub fn health(&self) -> DoctorHealthResult {
        let mut checks = Vec::new();

        // Gate file — canonical path formula must match lsp-max-compositor
        let gate_path = crate::nouns::gate::GateService::gate_file_path();
        checks.push(DoctorCheck {
            name: "gate_file".to_string(),
            status: if gate_path.exists() { "ADMITTED" } else { "UNKNOWN" }.to_string(),
            detail: gate_path.display().to_string(),
        });

        // Mesh state file
        checks.push(DoctorCheck {
            name: "mesh_state_file".to_string(),
            status: if Path::new(&self.state_path).exists() { "ADMITTED" } else { "UNKNOWN" }.to_string(),
            detail: self.state_path.clone(),
        });

        // Config file — same derivation as ConfigService
        let config_path = if let Ok(p) = std::env::var("LSP_MAX_CONFIG") {
            std::path::PathBuf::from(p)
        } else if let Ok(home) = std::env::var("HOME") {
            std::path::PathBuf::from(home).join(".lsp-max-config.json")
        } else {
            std::path::PathBuf::from(".lsp-max-config.json")
        };
        checks.push(DoctorCheck {
            name: "config_file".to_string(),
            status: if config_path.exists() { "ADMITTED" } else { "UNKNOWN" }.to_string(),
            detail: config_path.display().to_string(),
        });

        // Scripts — spot-check the compliance script as a proxy for the scripts/ dir
        let script_path = std::path::PathBuf::from("scripts/check-law-compliance.sh");
        checks.push(DoctorCheck {
            name: "scripts_present".to_string(),
            status: if script_path.exists() { "ADMITTED" } else { "BLOCKED" }.to_string(),
            detail: script_path.display().to_string(),
        });

        let overall = if checks.iter().any(|c| c.status == "BLOCKED") {
            "BLOCKED"
        } else if checks.iter().all(|c| c.status == "ADMITTED") {
            "ADMITTED"
        } else {
            "PARTIAL"
        };
        DoctorHealthResult { status: overall.to_string(), checks }
    }

    pub fn check_compliance(&self) -> ComplianceResult {
        let mut violations = Vec::new();
        for root in &["src", "crates"] {
            if let Ok(files) = walkdir_rs(Path::new(root)) {
                for path in files {
                    if let Ok(content) = std::fs::read_to_string(&path) {
                        scan_file_for_violations(&path.display().to_string(), &content, &mut violations);
                    }
                }
            }
        }
        let status = if violations.is_empty() { "ADMITTED" } else { "BLOCKED" };
        ComplianceResult { violations, status: status.to_string() }
    }

    pub fn lint(&self) -> std::result::Result<DoctorLintResult, String> {
        let mesh = AutonomicMesh::load_from_file(&self.state_path)
            .unwrap_or_else(|_| AutonomicMesh::new());
        let mut error_count = 0usize;
        let mut warning_count = 0usize;
        let mut info_count = 0usize;
        for inst in mesh.instances.values() {
            for diag in &inst.diagnostics {
                match diag.lsp.severity {
                    Some(s) if s == lsp_types_max::DiagnosticSeverity::ERROR => error_count += 1,
                    Some(s) if s == lsp_types_max::DiagnosticSeverity::WARNING => warning_count += 1,
                    Some(s) if s == lsp_types_max::DiagnosticSeverity::INFORMATION => info_count += 1,
                    _ => {}
                }
            }
        }
        let status = if error_count == 0 { "ADMITTED" } else { "BLOCKED" };
        Ok(DoctorLintResult { error_count, warning_count, info_count, status: status.to_string() })
    }

    pub fn docs(&self) -> DoctorDocsResult {
        let mut undocumented = Vec::new();
        for root in &["src", "crates"] {
            if let Ok(paths) = walkdir_rs(Path::new(root)) {
                for path in paths {
                    if let Ok(content) = std::fs::read_to_string(&path) {
                        scan_file_for_undocumented(&path.display().to_string(), &content, &mut undocumented);
                    }
                }
            }
        }
        let coverage_status = if undocumented.is_empty() { "ADMITTED" } else { "PARTIAL" };
        DoctorDocsResult { undocumented_fns: undocumented, coverage_status: coverage_status.to_string() }
    }

    pub fn report(&self) -> DoctorReport {
        let health = self.health();
        let compliance = self.check_compliance();
        let lint = self.lint().unwrap_or_else(|_| DoctorLintResult {
            error_count: 0,
            warning_count: 0,
            info_count: 0,
            status: "UNKNOWN".to_string(),
        });
        let overall_status = match (
            health.status == "ADMITTED",
            compliance.status == "ADMITTED",
            lint.status == "ADMITTED",
        ) {
            (true, true, true) => "ADMITTED",
            (false, false, false) => "BLOCKED",
            _ => "PARTIAL",
        };
        DoctorReport { health, compliance, lint, overall_status: overall_status.to_string() }
    }
}

fn walkdir_rs(root: &Path) -> std::result::Result<Vec<std::path::PathBuf>, std::io::Error> {
    let mut result = Vec::new();
    if root.exists() {
        walk_collect(root, &mut result)?;
    }
    Ok(result)
}

fn walk_collect(dir: &Path, out: &mut Vec<std::path::PathBuf>) -> std::result::Result<(), std::io::Error> {
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            walk_collect(&path, out)?;
        } else if path.extension().and_then(|e| e.to_str()) == Some("rs") {
            out.push(path);
        }
    }
    Ok(())
}

// Detect plain tower-lsp references and victory language in comment lines.
// Lines containing "NOT tower-lsp" / "NOT tower_lsp" are negative-control fixtures and exempt.
fn scan_file_for_violations(path: &str, content: &str, out: &mut Vec<ComplianceViolation>) {
    for (idx, line) in content.lines().enumerate() {
        let lineno = idx + 1;
        let has_ref = line.contains("tower-lsp") || line.contains("tower_lsp");
        let negated = line.contains("NOT tower-lsp") || line.contains("NOT tower_lsp");
        if has_ref && !negated {
            out.push(ComplianceViolation {
                file: path.to_string(), line: lineno,
                kind: "TOWER_LSP_REF".to_string(), text: line.trim().to_string(),
            });
        }
        let trimmed = line.trim();
        if trimmed.starts_with("//") {
            for word in &[" done", " solved", " guaranteed"] {
                if line.contains(word) {
                    out.push(ComplianceViolation {
                        file: path.to_string(), line: lineno,
                        kind: "VICTORY_LANGUAGE".to_string(), text: trimmed.to_string(),
                    });
                    break;
                }
            }
        }
    }
}

// Detect `pub fn` declarations without a preceding `///` doc comment.
fn scan_file_for_undocumented(path: &str, content: &str, out: &mut Vec<UndocumentedFn>) {
    let lines: Vec<&str> = content.lines().collect();
    for (idx, line) in lines.iter().enumerate() {
        let trimmed = line.trim();
        if !trimmed.starts_with("pub fn ") && !trimmed.contains(" pub fn ") {
            continue;
        }
        let fn_name = extract_fn_name(trimmed).unwrap_or_else(|| "unknown".to_string());
        if !preceding_doc_comment(&lines, idx) {
            out.push(UndocumentedFn { file: path.to_string(), line: idx + 1, fn_name });
        }
    }
}

fn extract_fn_name(line: &str) -> Option<String> {
    let fn_pos = line.find("fn ")?;
    let after_fn = &line[fn_pos + 3..];
    let end = after_fn.find(|c: char| !c.is_alphanumeric() && c != '_').unwrap_or(after_fn.len());
    Some(after_fn[..end].to_string())
}

fn preceding_doc_comment(lines: &[&str], fn_line_idx: usize) -> bool {
    // Walk backwards through attributes and blanks looking for a `///` doc comment.
    let mut i = fn_line_idx;
    while i > 0 {
        i -= 1;
        let t = lines[i].trim();
        if t.starts_with("///") { return true; }
        if t.starts_with("#[") || t.starts_with("#![") || t.is_empty() { continue; }
        break;
    }
    false
}

// ==============================================================================
// 3. Verb Tier
// ==============================================================================

/// Run a quick system-level health scan of the lsp-max workspace.
#[verb("health")]
pub fn health() -> Result<DoctorHealthResult> {
    Ok(DoctorService::new().health())
}

/// Scan the codebase for law violations: plain tower-lsp references and victory language.
#[verb("check-compliance")]
pub fn check_compliance() -> Result<ComplianceResult> {
    Ok(DoctorService::new().check_compliance())
}

/// Read the mesh state and report a diagnostic summary by severity.
#[verb("lint")]
pub fn lint() -> Result<DoctorLintResult> {
    DoctorService::new().lint().map_err(NounVerbError::execution_error)
}

/// Scan source files for `pub fn` declarations that lack a preceding `///` doc comment.
#[verb("docs")]
pub fn docs() -> Result<DoctorDocsResult> {
    Ok(DoctorService::new().docs())
}

/// Aggregate health, compliance, and lint into a single report.
#[verb("report")]
pub fn report() -> Result<DoctorReport> {
    Ok(DoctorService::new().report())
}

// ==============================================================================
// 4. Tests
// ==============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn health_returns_valid_statuses() {
        let svc = DoctorService::new();
        let result = svc.health();
        let valid = ["ADMITTED", "BLOCKED", "PARTIAL", "UNKNOWN"];
        assert!(valid.contains(&result.status.as_str()), "unexpected status: {}", result.status);
        for check in &result.checks {
            assert!(valid.contains(&check.status.as_str()), "unexpected check status: {}", check.status);
        }
    }

    #[test]
    fn compliance_status_is_admitted_or_blocked() {
        let result = DoctorService::new().check_compliance();
        assert!(result.status == "ADMITTED" || result.status == "BLOCKED");
    }

    #[test]
    fn lint_returns_bounded_status() {
        let result = DoctorService::new().lint().unwrap();
        assert!(result.status == "ADMITTED" || result.status == "BLOCKED");
    }

    #[test]
    fn docs_returns_bounded_coverage_status() {
        let result = DoctorService::new().docs();
        assert!(result.coverage_status == "ADMITTED" || result.coverage_status == "PARTIAL");
    }

    #[test]
    fn scan_detects_tower_lsp_ref() {
        let mut v = Vec::new();
        // NOT tower-lsp — negative-control marker; this fixture line must NOT itself trigger.
        scan_file_for_violations("test.rs", "use tower-lsp::something;\n", &mut v);
        assert_eq!(v.len(), 1);
        assert_eq!(v[0].kind, "TOWER_LSP_REF");
    }

    #[test]
    fn scan_skips_negative_control_line() {
        let mut v = Vec::new();
        // Negative-control fixture: NOT tower-lsp — must be exempt.
        scan_file_for_violations("test.rs", "// NOT tower-lsp reference\n", &mut v);
        assert_eq!(v.len(), 0);
    }

    #[test]
    fn scan_detects_victory_language_in_comment() {
        let mut v = Vec::new();
        scan_file_for_violations("test.rs", "// all solved\n", &mut v);
        assert_eq!(v.len(), 1);
        assert_eq!(v[0].kind, "VICTORY_LANGUAGE");
    }

    #[test]
    fn scan_ignores_victory_language_outside_comment() {
        let mut v = Vec::new();
        scan_file_for_violations("test.rs", "let x = already_solved();\n", &mut v);
        assert_eq!(v.len(), 0);
    }

    #[test]
    fn report_overall_status_is_bounded() {
        let result = DoctorService::new().report();
        let valid = ["ADMITTED", "BLOCKED", "PARTIAL"];
        assert!(valid.contains(&result.overall_status.as_str()), "unexpected: {}", result.overall_status);
    }
}
