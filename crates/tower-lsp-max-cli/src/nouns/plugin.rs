use clap_noun_verb::Result;
use clap_noun_verb_macros::verb;
use serde::Serialize;

#[derive(Serialize)]
pub struct PluginResult {
    pub plugins: Vec<String>,
}

#[verb("list")]
pub fn cmd_list() -> Result<PluginResult> {
    Ok(PluginResult {
        plugins: vec!["plugin_mock".into()],
    })
}

#[derive(Serialize)]
pub struct PluginLoadResult {
    pub success: bool,
    pub plugin: String,
}

#[verb("load")]
pub fn cmd_load(plugin_path: String) -> Result<PluginLoadResult> {
    Ok(PluginLoadResult {
        success: true,
        plugin: plugin_path,
    })
}
