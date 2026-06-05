use clap_noun_verb::Result;
use clap_noun_verb_macros::verb;
use serde::Serialize;

#[derive(Serialize)]
pub struct DiagnosticsResult {
    pub issues: u32,
}

#[verb("run")]
pub fn cmd_run() -> Result<DiagnosticsResult> {
    Ok(DiagnosticsResult { issues: 0 })
}

#[derive(Serialize)]
pub struct DiagnosticsReportResult {
    pub content: String,
}

#[verb("report")]
pub fn cmd_report() -> Result<DiagnosticsReportResult> {
    Ok(DiagnosticsReportResult {
        content: String::from("No issues"),
    })
}
