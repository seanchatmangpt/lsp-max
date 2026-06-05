use clap_noun_verb::Result;
use clap_noun_verb_macros::verb;
use serde::Serialize;

#[derive(Serialize)]
pub struct AgentResult {
    pub response: String,
}

#[verb("invoke")]
pub fn cmd_invoke(task: String) -> Result<AgentResult> {
    let _ = task;
    Ok(AgentResult {
        response: "Task received".into(),
    })
}

#[verb("chat")]
pub fn cmd_chat(message: String) -> Result<AgentResult> {
    let _ = message;
    Ok(AgentResult {
        response: "Chat message received".into(),
    })
}
