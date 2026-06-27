use clap_noun_verb::Result;
use clap_noun_verb_macros::verb;
use lsp_max_cli::gen::GgenAdapter;
use serde::Serialize;
use std::path::PathBuf;

// ==========================================
// 1. Domain Tier
// ==========================================

#[derive(Debug, Clone, Serialize)]
pub struct GgenStatusResult {
    pub available: bool,
    pub binary_path: Option<String>,
    /// Bounded status: "ADMITTED" if binary found, "CANDIDATE-FALLBACK" otherwise.
    pub status: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct GgenSyncResult {
    pub files_written: Vec<String>,
    pub status: String,
    pub dir: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct GgenInitResult {
    pub dir: String,
    pub status: String,
    pub created: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct GgenValidateResult {
    pub valid: bool,
    pub issues: Vec<String>,
    pub status: String,
    pub dir: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct GgenDiffResult {
    pub pending: Vec<String>,
    pub dir: String,
    pub status: String,
}

// ==========================================
// 2. Service Tier
// ==========================================

pub struct GgenService;

impl GgenService {
    pub fn new() -> Self {
        Self
    }

    pub fn status(&self) -> GgenStatusResult {
        let adapter = GgenAdapter::new();
        let available = adapter.is_available();
        GgenStatusResult {
            available,
            // GgenAdapter does not expose the resolved path yet; field is OPEN.
            binary_path: None,
            status: if available {
                "ADMITTED".into()
            } else {
                "CANDIDATE-FALLBACK".into()
            },
        }
    }

    pub fn sync(&self, dir: &str) -> Result<GgenSyncResult> {
        let adapter = GgenAdapter::new();
        let path = PathBuf::from(dir);
        let report = adapter
            .sync(&path)
            .map_err(|e| clap_noun_verb::error::NounVerbError::execution_error(e.to_string()))?;
        Ok(GgenSyncResult {
            files_written: report.files_written,
            status: report.status,
            dir: dir.to_string(),
        })
    }

    pub fn init(&self, name: &str, dir: &str) -> Result<GgenInitResult> {
        let adapter = GgenAdapter::new();
        let path = PathBuf::from(dir);
        adapter
            .init(name, &path)
            .map_err(|e| clap_noun_verb::error::NounVerbError::execution_error(e.to_string()))?;
        Ok(GgenInitResult {
            dir: dir.to_string(),
            status: "CANDIDATE".into(),
            created: vec!["gen.toml".into()],
        })
    }

    pub fn validate(&self, dir: &str) -> Result<GgenValidateResult> {
        let adapter = GgenAdapter::new();
        let path = PathBuf::from(dir);
        let report = adapter
            .validate(&path)
            .map_err(|e| clap_noun_verb::error::NounVerbError::execution_error(e.to_string()))?;
        Ok(GgenValidateResult {
            valid: report.valid,
            issues: report.issues,
            status: report.status,
            dir: dir.to_string(),
        })
    }

    pub fn diff(&self, dir: &str) -> Result<GgenDiffResult> {
        let adapter = GgenAdapter::new();
        let path = PathBuf::from(dir);
        let pending = adapter
            .diff(&path)
            .map_err(|e| clap_noun_verb::error::NounVerbError::execution_error(e.to_string()))?;
        Ok(GgenDiffResult {
            pending,
            dir: dir.to_string(),
            status: "OPEN".into(),
        })
    }
}

// ==========================================
// 3. CLI Tier
// ==========================================

#[verb("status")]
pub fn status() -> Result<GgenStatusResult> {
    Ok(GgenService::new().status())
}

#[verb("sync")]
pub fn sync(dir: Option<String>) -> Result<GgenSyncResult> {
    let dir = dir.unwrap_or_else(|| ".".to_string());
    GgenService::new().sync(&dir)
}

#[verb("init")]
pub fn init(name: String, dir: Option<String>) -> Result<GgenInitResult> {
    let dir = dir.unwrap_or_else(|| format!("./{name}"));
    GgenService::new().init(&name, &dir)
}

#[verb("validate")]
pub fn validate(dir: Option<String>) -> Result<GgenValidateResult> {
    let dir = dir.unwrap_or_else(|| ".".to_string());
    GgenService::new().validate(&dir)
}

#[verb("diff")]
pub fn diff(dir: Option<String>) -> Result<GgenDiffResult> {
    let dir = dir.unwrap_or_else(|| ".".to_string());
    GgenService::new().diff(&dir)
}
