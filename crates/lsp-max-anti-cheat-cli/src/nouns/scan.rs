use clap_noun_verb::Result;
use clap_noun_verb_macros::verb;
use lsp_max_anti_cheat::engine::scan_directory;
use serde::Serialize;
use std::path::Path;

// ===== Domain Tier =====
#[derive(Serialize)]
pub struct ScanResult {
    pub path: String,
    pub observations_count: usize,
    pub patterns: Vec<PatternMatch>,
}

#[derive(Serialize, Clone)]
pub struct PatternMatch {
    pub file_path: String,
    pub line: usize,
    pub column: usize,
    pub kind: String,
    pub construct: String,
    pub message: String,
}

// ===== Service Tier =====
pub struct ScanService;

impl ScanService {
    pub fn scan_path(path: &str) -> std::result::Result<ScanResult, String> {
        let path_obj = Path::new(path);
        if !path_obj.exists() {
            return Err(format!("Path does not exist: {}", path));
        }

        let observations = scan_directory(path);
        let patterns: Vec<_> = observations
            .into_iter()
            .map(|obs| PatternMatch {
                file_path: obs.file_path,
                line: obs.line,
                column: obs.column,
                kind: obs.kind,
                construct: obs.construct,
                message: obs.message,
            })
            .collect();

        Ok(ScanResult {
            path: path.to_string(),
            observations_count: patterns.len(),
            patterns,
        })
    }
}

// ===== Verb Tier (CLI) =====

#[verb("directory")]
pub fn scan_directory_verb(#[arg(default_value = ".")] path: String) -> Result<ScanResult> {
    ScanService::scan_path(&path).map_err(clap_noun_verb::error::NounVerbError::execution_error)
}

#[verb("file")]
pub fn scan_file_verb(#[arg()] path: String) -> Result<ScanResult> {
    ScanService::scan_path(&path).map_err(clap_noun_verb::error::NounVerbError::execution_error)
}
