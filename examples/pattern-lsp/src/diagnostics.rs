use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Finding {
    pub source: String,
    pub rule_id: String,
    pub path: String,
    pub line: usize,
    pub column: usize,
    pub severity: String,
    pub matched_text: String,
    pub workspace_root: String,
    pub scan_scope: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Receipt {
    pub source: String,
    pub rule_id: String,
    pub path: String,
    pub line: usize,
    pub column: usize,
    pub severity: String,
    pub matched_text: String,
    pub workspace_root: String,
    pub scan_scope: Vec<String>,
    pub timestamp: String,
}

impl From<Finding> for Receipt {
    fn from(f: Finding) -> Self {
        Self {
            source: f.source,
            rule_id: f.rule_id,
            path: f.path,
            line: f.line,
            column: f.column,
            severity: f.severity,
            matched_text: f.matched_text,
            workspace_root: f.workspace_root,
            scan_scope: f.scan_scope,
            timestamp: chrono::Utc::now().to_rfc3339(),
        }
    }
}
