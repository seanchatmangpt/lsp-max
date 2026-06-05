use clap_noun_verb::Result;
use clap_noun_verb_macros::verb;
use serde::Serialize;

#[derive(Serialize)]
pub struct StateResult {
    pub dumped: bool,
}

#[derive(Serialize)]
pub struct RestoreResult {
    pub restored: bool,
}

#[verb("dump")]
pub fn cmd_dump() -> Result<StateResult> {
    Ok(StateResult { dumped: true })
}

#[verb("restore")]
pub fn cmd_restore() -> Result<RestoreResult> {
    Ok(RestoreResult { restored: true })
}
