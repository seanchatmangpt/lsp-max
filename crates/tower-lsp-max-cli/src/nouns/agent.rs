use clap_noun_verb::Result;
use clap_noun_verb_macros::verb;
use serde::Serialize;
use tower_lsp_max_agent::LspAgent;

#[derive(Serialize)]
pub struct AgentResult {
    pub response: String,
}

#[verb("invoke")]
pub fn cmd_invoke(task: String) -> Result<AgentResult> {
    match LspAgent::invoke_task(&task) {
        Ok(response) => Ok(AgentResult { response }),
        Err(message) => Err(clap_noun_verb::NounVerbError::ExecutionError { message }),
    }
}

#[verb("chat")]
pub fn cmd_chat(message: String) -> Result<AgentResult> {
    match LspAgent::chat(&message) {
        Ok(response) => Ok(AgentResult { response }),
        Err(message) => Err(clap_noun_verb::NounVerbError::ExecutionError { message }),
    }
}
