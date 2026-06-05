use clap_noun_verb::Result;
use clap_noun_verb_macros::verb;
use serde::Serialize;

#[derive(Serialize)]
pub struct WorkspaceResult {
    pub initialized: bool,
}

#[verb("init")]
pub fn cmd_init(path: String) -> Result<WorkspaceResult> {
    let _ = path;
    Ok(WorkspaceResult { initialized: true })
}

#[derive(Serialize)]
pub struct AnalyzeResult {
    pub analyzed: bool,
    pub issues: usize,
}

#[verb("analyze")]
pub fn cmd_analyze(path: String) -> Result<AnalyzeResult> {
    let _ = path;
    Ok(AnalyzeResult {
        analyzed: true,
        issues: 0,
    })
}
