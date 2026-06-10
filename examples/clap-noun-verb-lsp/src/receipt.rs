use crate::types::CommandResult;
use clap_noun_verb::Result;
use clap_noun_verb_macros::verb;

#[verb("show")]
#[allow(unused_variables)]
pub fn cmd_show(latest: bool) -> Result<CommandResult> {
    Ok(CommandResult { success: true })
}
