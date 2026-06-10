use crate::types::CommandResult;
use clap_noun_verb::Result;
use clap_noun_verb_macros::verb;

#[verb("export")]
pub fn cmd_export(_format: String) -> Result<CommandResult> {
    Ok(CommandResult { success: true })
}
