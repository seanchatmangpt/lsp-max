use crate::types::CommandResult;
use clap_noun_verb::Result;
use clap_noun_verb_macros::verb;

#[verb("inspect")]
#[allow(unused_variables)]
pub fn cmd_inspect(noun: String, verb: String) -> Result<CommandResult> {
    Ok(CommandResult { success: true })
}
