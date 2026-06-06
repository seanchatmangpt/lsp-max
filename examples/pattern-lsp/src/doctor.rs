use clap_noun_verb::Result;
use clap_noun_verb_macros::verb;
use serde::Serialize;

#[derive(Serialize)]
pub struct DoctorCheckResult {
    pub healthy: bool,
}

/// Run doctor checks
#[verb("check")]
pub fn cmd_check() -> Result<DoctorCheckResult> {
    Ok(DoctorCheckResult { healthy: true })
}
