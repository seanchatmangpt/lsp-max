use crate::types::CommandResult;
use clap_noun_verb::Result;
use clap_noun_verb_macros::verb;

#[verb("workspace")]
#[allow(unused_variables)]
pub fn cmd_workspace(format: String) -> Result<CommandResult> {
    Ok(CommandResult { success: true })
}
