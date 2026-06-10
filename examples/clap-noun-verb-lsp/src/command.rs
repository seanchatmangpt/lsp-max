use crate::types::CommandResult;
use clap_noun_verb::Result;
use clap_noun_verb_macros::verb;

#[verb("inspect")]
pub fn cmd_inspect(_noun: String, _verb: String) -> Result<CommandResult> {
    Ok(CommandResult { success: true })
}
