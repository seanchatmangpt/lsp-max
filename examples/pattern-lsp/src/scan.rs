use crate::scanner;
use clap_noun_verb::Result;
use clap_noun_verb_macros::verb;
use serde::Serialize;

#[derive(Serialize)]
pub struct ScanResult {
    pub success: bool,
    pub findings: usize,
}

/// Scan the workspace
/// # Arguments
/// * `format` - Output format
#[verb("workspace")]
pub fn cmd_workspace(format: Option<String>) -> Result<ScanResult> {
    let _ = format;
    let count = scanner::scan_workspace()?;
    Ok(ScanResult {
        success: true,
        findings: count,
    })
}
