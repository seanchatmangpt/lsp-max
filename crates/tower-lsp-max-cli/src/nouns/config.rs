use clap_noun_verb::Result;
use clap_noun_verb_macros::verb;
use serde::Serialize;

#[derive(Serialize)]
pub struct ConfigResult {
    pub value: String,
}

#[verb("view")]
pub fn cmd_view(key: String) -> Result<ConfigResult> {
    let _ = key;
    Ok(ConfigResult {
        value: "mock_value".into(),
    })
}

#[verb("set")]
pub fn cmd_set(key: String, value: String) -> Result<ConfigResult> {
    let _ = (key, value);
    Ok(ConfigResult {
        value: "mock_value".into(),
    })
}
