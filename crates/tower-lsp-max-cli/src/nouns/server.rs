use clap_noun_verb::Result;
use clap_noun_verb_macros::verb;
use serde::Serialize;

#[derive(Serialize)]
pub struct ServerResult {
    pub success: bool,
    pub message: String,
}

#[verb("start")]
pub fn cmd_start(port: u16) -> Result<ServerResult> {
    Ok(ServerResult {
        success: true,
        message: format!("Started on {}", port),
    })
}

#[verb("stop")]
pub fn cmd_stop() -> Result<ServerResult> {
    Ok(ServerResult {
        success: true,
        message: "Server stopped".to_string(),
    })
}

#[verb("status")]
pub fn cmd_status() -> Result<ServerResult> {
    Ok(ServerResult {
        success: true,
        message: "Server status: OK".to_string(),
    })
}
