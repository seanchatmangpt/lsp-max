use crate::types::CommandResult;
use clap_noun_verb::Result;
use clap_noun_verb_macros::verb;

#[verb("export")]
#[allow(unused_variables)]
pub fn cmd_export(format: String) -> Result<CommandResult> {
    Ok(CommandResult { success: true })
}
