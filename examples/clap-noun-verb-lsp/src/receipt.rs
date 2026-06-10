use crate::types::CommandResult;
use clap_noun_verb::Result;
use clap_noun_verb_macros::verb;

#[verb("show")]
pub fn cmd_show(_latest: bool) -> Result<CommandResult> {
    Ok(CommandResult { success: true })
}
