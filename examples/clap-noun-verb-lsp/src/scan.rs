use crate::types::CommandResult;
use clap_noun_verb::Result;
use clap_noun_verb_macros::verb;

#[verb("workspace")]
pub fn cmd_workspace(_format: String) -> Result<CommandResult> {
    Ok(CommandResult { success: true })
}
