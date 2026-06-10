use crate::types::CommandResult;
use clap_noun_verb::Result;
use clap_noun_verb_macros::verb;

#[verb("check")]
pub fn cmd_check() -> Result<CommandResult> {
    Ok(CommandResult { success: true })
}
