use clap_noun_verb::Result;
use clap_noun_verb_macros::verb;
use lsp_max_runtime::AutonomicMesh;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

// --- 1. Domain Tier ---
#[derive(Debug, Clone, Serialize)]
pub struct Workspace {
    pub root_path: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct WorkspaceAnalysis {
    pub is_healthy: bool,
    pub files_scanned: usize,
    pub instance_count: usize,
    pub total_diagnostics: usize,
    pub conformance_score: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct WorkspaceFormatResult {
    pub formatted_files: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct WorkspaceLintResult {
    pub errors: usize,
    pub warnings: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BaselineRecord {
    pub instance_id: String,
    pub score: f64,
    pub timestamp: String,
}

// --- 2. Service Tier ---
pub struct WorkspaceService {
    state_path: String,
}

impl WorkspaceService {
    pub fn new() -> Self {
        Self {
            state_path: crate::nouns::get_state_path(),
        }
    }

    fn baseline_path() -> PathBuf {
        if let Ok(home) = std::env::var("HOME") {
            PathBuf::from(home).join(".lsp-max-workspace-baseline.json")
        } else {
            PathBuf::from(".lsp-max-workspace-baseline.json")
        }
    }

    pub fn init(&self, path: String) -> Workspace {
        Workspace { root_path: path }
    }

    pub fn analyze(&self, workspace: &Workspace) -> WorkspaceAnalysis {
        match AutonomicMesh::load_from_file(&self.state_path) {
            Ok(mesh) => {
                let instance_count = mesh.instances.len();
                let total_diagnostics: usize =
                    mesh.instances.values().map(|i| i.diagnostics.len()).sum();
                let avg_score = if instance_count > 0 {
                    mesh.instances.values().map(|i| i.conformance_score()).sum::<f64>()
                        / instance_count as f64
                } else {
                    100.0
                };
                let error_count: usize = mesh
                    .instances
                    .values()
                    .flat_map(|i| i.diagnostics.iter())
                    .filter(|d| matches!(d.lsp.severity, Some(lsp_types_max::DiagnosticSeverity::ERROR)))
                    .count();
                let files_scanned = instance_count.max(1);
                let _ = workspace;
                WorkspaceAnalysis {
                    is_healthy: error_count == 0,
                    files_scanned,
                    instance_count,
                    total_diagnostics,
                    conformance_score: avg_score,
                }
            }
            Err(_) => WorkspaceAnalysis {
                is_healthy: true,
                files_scanned: 0,
                instance_count: 0,
                total_diagnostics: 0,
                conformance_score: 100.0,
            },
        }
    }

    pub fn format(&self, _workspace: &Workspace) -> WorkspaceFormatResult {
        WorkspaceFormatResult { formatted_files: 0 }
    }

    pub fn lint(&self, workspace: &Workspace) -> WorkspaceLintResult {
        match AutonomicMesh::load_from_file(&self.state_path) {
            Ok(mesh) => {
                let errors: usize = mesh
                    .instances
                    .values()
                    .flat_map(|i| i.diagnostics.iter())
                    .filter(|d| matches!(d.lsp.severity, Some(lsp_types_max::DiagnosticSeverity::ERROR)))
                    .count();
                let warnings: usize = mesh
                    .instances
                    .values()
                    .flat_map(|i| i.diagnostics.iter())
                    .filter(|d| matches!(d.lsp.severity, Some(lsp_types_max::DiagnosticSeverity::WARNING)))
                    .count();
                let _ = workspace;
                WorkspaceLintResult { errors, warnings }
            }
            Err(_) => WorkspaceLintResult { errors: 0, warnings: 0 },
        }
    }

    pub fn files(&self) -> (Vec<String>, usize) {
        match AutonomicMesh::load_from_file(&self.state_path) {
            Ok(mesh) => {
                let mut ids: Vec<String> = mesh.instances.keys().cloned().collect();
                ids.sort();
                let count = ids.len();
                (ids, count)
            }
            Err(_) => (Vec::new(), 0),
        }
    }

    pub fn graph(&self) -> (Vec<(String, String, String)>, Vec<(String, String, String)>) {
        match AutonomicMesh::load_from_file(&self.state_path) {
            Ok(mesh) => {
                let mut nodes: Vec<(String, String, String)> = mesh
                    .instances
                    .values()
                    .map(|inst| (inst.id.clone(), "LspInstance".to_string(), format!("{:?}", inst.phase)))
                    .collect();
                nodes.sort_by(|a, b| a.0.cmp(&b.0));

                let instance_ids: Vec<String> = mesh.instances.keys().cloned().collect();
                let mut seen_edges: HashSet<(String, String, String)> = HashSet::new();
                let mut edges: Vec<(String, String, String)> = Vec::new();
                for (from_id, inst) in &mesh.instances {
                    for diag in &inst.diagnostics {
                        for to_id in &instance_ids {
                            if to_id != from_id && diag.lsp.message.contains(to_id.as_str()) {
                                let edge = (from_id.clone(), to_id.clone(), "DIAGNOSTIC_REF".to_string());
                                if seen_edges.insert(edge.clone()) {
                                    edges.push(edge);
                                }
                            }
                        }
                    }
                }
                edges.sort_by(|a, b| a.0.cmp(&b.0).then(a.1.cmp(&b.1)));
                (nodes, edges)
            }
            Err(_) => (Vec::new(), Vec::new()),
        }
    }

    pub fn baseline(&self) -> std::result::Result<(Vec<BaselineRecord>, String), String> {
        let mesh = AutonomicMesh::load_from_file(&self.state_path).map_err(|e| e.to_string())?;
        let secs = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
        let timestamp = format!("{}", secs);
        let mut records: Vec<BaselineRecord> = mesh
            .instances
            .values()
            .map(|inst| BaselineRecord {
                instance_id: inst.id.clone(),
                score: inst.conformance_score(),
                timestamp: timestamp.clone(),
            })
            .collect();
        records.sort_by(|a, b| a.instance_id.cmp(&b.instance_id));
        let path = Self::baseline_path();
        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        let content = serde_json::to_string_pretty(&records).map_err(|e| e.to_string())?;
        fs::write(&path, content).map_err(|e| e.to_string())?;
        let baseline_file = path.to_string_lossy().into_owned();
        Ok((records, baseline_file))
    }

    pub fn diff_baseline(
        &self,
    ) -> std::result::Result<(Vec<(String, f64, f64, f64)>, Vec<(String, f64, f64, f64)>, usize), String>
    {
        let path = Self::baseline_path();
        let content = fs::read_to_string(&path).map_err(|e| format!("BASELINE_NOT_FOUND: {}", e))?;
        let baseline_records: Vec<BaselineRecord> =
            serde_json::from_str(&content).map_err(|e| e.to_string())?;
        let mesh = AutonomicMesh::load_from_file(&self.state_path).map_err(|e| e.to_string())?;

        let mut regressions: Vec<(String, f64, f64, f64)> = Vec::new();
        let mut improvements: Vec<(String, f64, f64, f64)> = Vec::new();
        let mut unchanged: usize = 0;

        for record in &baseline_records {
            let current = mesh.instances.get(&record.instance_id).map(|i| i.conformance_score()).unwrap_or(0.0);
            let delta = current - record.score;
            if delta < -f64::EPSILON {
                regressions.push((record.instance_id.clone(), record.score, current, delta));
            } else if delta > f64::EPSILON {
                improvements.push((record.instance_id.clone(), record.score, current, delta));
            } else {
                unchanged += 1;
            }
        }
        regressions.sort_by(|a, b| a.3.partial_cmp(&b.3).unwrap_or(std::cmp::Ordering::Equal));
        improvements.sort_by(|a, b| b.3.partial_cmp(&a.3).unwrap_or(std::cmp::Ordering::Equal));
        Ok((regressions, improvements, unchanged))
    }
}

impl Default for WorkspaceService {
    fn default() -> Self {
        Self::new()
    }
}

// --- 3. CLI Tier ---

#[derive(Serialize)]
pub struct InitResult {
    pub workspace: Workspace,
}

#[verb("init")]
pub fn init(path: String) -> Result<InitResult> {
    let service = WorkspaceService::new();
    let workspace = service.init(path);
    Ok(InitResult { workspace })
}

#[derive(Serialize)]
pub struct AnalyzeResult {
    pub analysis: WorkspaceAnalysis,
}

#[verb("analyze")]
pub fn analyze(path: String) -> Result<AnalyzeResult> {
    let service = WorkspaceService::new();
    let workspace = service.init(path);
    let analysis = service.analyze(&workspace);
    Ok(AnalyzeResult { analysis })
}

#[derive(Serialize)]
pub struct FormatResult {
    pub result: WorkspaceFormatResult,
}

#[verb("format")]
pub fn format(path: String) -> Result<FormatResult> {
    let service = WorkspaceService::new();
    let workspace = service.init(path);
    let result = service.format(&workspace);
    Ok(FormatResult { result })
}

#[derive(Serialize)]
pub struct LintResult {
    pub result: WorkspaceLintResult,
}

#[verb("lint")]
pub fn lint(path: String) -> Result<LintResult> {
    let service = WorkspaceService::new();
    let workspace = service.init(path);
    let result = service.lint(&workspace);
    Ok(LintResult { result })
}

// ------------------------------------------------------------------
// files
// ------------------------------------------------------------------

#[derive(Serialize)]
pub struct WorkspaceFilesResult {
    pub path: String,
    pub instance_ids: Vec<String>,
    pub count: usize,
}

#[verb("files")]
pub fn files(path: Option<String>) -> Result<WorkspaceFilesResult> {
    let resolved = path.unwrap_or_else(|| ".".to_string());
    let service = WorkspaceService::new();
    let (instance_ids, count) = service.files();
    Ok(WorkspaceFilesResult { path: resolved, instance_ids, count })
}

// ------------------------------------------------------------------
// graph
// ------------------------------------------------------------------

#[derive(Serialize)]
pub struct GraphNode {
    pub id: String,
    pub kind: String,
    pub status: String,
}

#[derive(Serialize)]
pub struct GraphEdge {
    pub from: String,
    pub to: String,
    pub relationship: String,
}

#[derive(Serialize)]
pub struct WorkspaceGraphResult {
    pub nodes: Vec<GraphNode>,
    pub edges: Vec<GraphEdge>,
    pub path: String,
}

#[verb("graph")]
pub fn graph(path: Option<String>) -> Result<WorkspaceGraphResult> {
    let resolved = path.unwrap_or_else(|| ".".to_string());
    let service = WorkspaceService::new();
    let (raw_nodes, raw_edges) = service.graph();
    let nodes = raw_nodes.into_iter().map(|(id, kind, status)| GraphNode { id, kind, status }).collect();
    let edges = raw_edges.into_iter().map(|(from, to, relationship)| GraphEdge { from, to, relationship }).collect();
    Ok(WorkspaceGraphResult { nodes, edges, path: resolved })
}

// ------------------------------------------------------------------
// baseline
// ------------------------------------------------------------------

#[derive(Serialize)]
pub struct WorkspaceBaselineResult {
    pub path: String,
    pub instances_saved: usize,
    pub baseline_file: String,
    pub status: String,
}

#[verb("baseline")]
pub fn baseline(path: Option<String>) -> Result<WorkspaceBaselineResult> {
    let resolved = path.unwrap_or_else(|| ".".to_string());
    let service = WorkspaceService::new();
    let (records, baseline_file) = service
        .baseline()
        .map_err(clap_noun_verb::error::NounVerbError::execution_error)?;
    Ok(WorkspaceBaselineResult {
        path: resolved,
        instances_saved: records.len(),
        baseline_file,
        status: "ADMITTED".to_string(),
    })
}

// ------------------------------------------------------------------
// diff-baseline
// ------------------------------------------------------------------

#[derive(Serialize)]
pub struct WorkspaceRegression {
    pub instance_id: String,
    pub baseline_score: f64,
    pub current_score: f64,
    pub delta: f64,
}

#[derive(Serialize)]
pub struct WorkspaceDiffBaselineResult {
    pub regressions: Vec<WorkspaceRegression>,
    pub improvements: Vec<WorkspaceRegression>,
    pub unchanged: usize,
    pub status: String,
}

#[verb("diff-baseline")]
pub fn diff_baseline() -> Result<WorkspaceDiffBaselineResult> {
    let service = WorkspaceService::new();
    let (raw_regressions, raw_improvements, unchanged) = service
        .diff_baseline()
        .map_err(clap_noun_verb::error::NounVerbError::execution_error)?;

    let to_regression = |(instance_id, baseline_score, current_score, delta)| WorkspaceRegression {
        instance_id,
        baseline_score,
        current_score,
        delta,
    };
    let regressions: Vec<WorkspaceRegression> = raw_regressions.into_iter().map(to_regression).collect();
    let improvements: Vec<WorkspaceRegression> = raw_improvements.into_iter().map(to_regression).collect();

    let status = if regressions.iter().any(|r| r.delta.abs() > 5.0) {
        "BLOCKED".to_string()
    } else if !regressions.is_empty() {
        "PARTIAL".to_string()
    } else {
        "ADMITTED".to_string()
    };

    Ok(WorkspaceDiffBaselineResult { regressions, improvements, unchanged, status })
}
