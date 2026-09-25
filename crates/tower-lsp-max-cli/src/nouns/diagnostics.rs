use clap_noun_verb::Result;
use clap_noun_verb_macros::verb;
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

pub struct DiagnosticsService;

impl DiagnosticsService {
    pub fn new() -> Self {
        Self
    }

    pub fn run(&self, target_path: &str) -> Vec<DiagnosticIssue> {
        // Mock implementation
        vec![DiagnosticIssue {
            file: target_path.to_string(),
            line: 42,
            message: "Mock diagnostic issue found".to_string(),
            severity: DiagnosticSeverity::Warning,
        }]
    }

    pub fn report(&self, format: &str) -> String {
        // Mock implementation
        format!("Generated diagnostic report in {} format", format)
    }

    pub fn clear(&self, target_path: &str) -> bool {
        // Mock implementation: always succeeds
        let _ = target_path;
        true
    }

    pub fn watch(&self, target_path: &str) -> bool {
        // Mock implementation: always returns true to indicate watching started
        let _ = target_path;
        true
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
    let issues = service.run(&target);
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
    let report_content = service.report(&format);

    Ok(ReportResult { report_content })
}

#[derive(Serialize)]
pub struct ClearResult {
    pub success: bool,
    pub target_cleared: String,
}

#[verb("clear")]
pub fn clear(target: String) -> Result<ClearResult> {
    let service = DiagnosticsService::new();
    let success = service.clear(&target);

    Ok(ClearResult {
        success,
        target_cleared: target,
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
