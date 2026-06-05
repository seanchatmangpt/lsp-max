use clap_noun_verb::Result;
use clap_noun_verb_macros::verb;
use serde::Serialize;

#[derive(Serialize)]
pub struct ClientResult {
    pub connected: bool,
}

#[verb("connect")]
pub fn cmd_connect(url: String) -> Result<ClientResult> {
    let _ = url;
    Ok(ClientResult { connected: true })
}

#[verb("disconnect")]
pub fn cmd_disconnect() -> Result<ClientResult> {
    Ok(ClientResult { connected: false })
}
