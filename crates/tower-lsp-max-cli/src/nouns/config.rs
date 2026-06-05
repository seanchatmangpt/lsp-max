use clap_noun_verb::Result;
use clap_noun_verb_macros::verb;
use serde::Serialize;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

#[derive(Serialize)]
pub struct ConfigResult {
    pub value: String,
}

fn get_config_path() -> PathBuf {
    if let Ok(path_str) = std::env::var("TOWER_LSP_MAX_CONFIG") {
        return PathBuf::from(path_str);
    }
    if let Ok(home) = std::env::var("HOME") {
        return PathBuf::from(home).join(".tower-lsp-max-config.json");
    }
    PathBuf::from(".tower-lsp-max-config.json")
}

fn load_config() -> std::io::Result<HashMap<String, String>> {
    let path = get_config_path();
    if !path.exists() {
        return Ok(HashMap::new());
    }
    let content = fs::read_to_string(path)?;
    let config: HashMap<String, String> = serde_json::from_str(&content)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    Ok(config)
}

fn save_config(config: &HashMap<String, String>) -> std::io::Result<()> {
    let path = get_config_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let content = serde_json::to_string_pretty(config)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    fs::write(path, content)?;
    Ok(())
}

#[verb("view")]
pub fn cmd_view(key: String) -> Result<ConfigResult> {
    if let Ok(val) = std::env::var(format!("TOWER_LSP_MAX_{}", key.to_uppercase())) {
        return Ok(ConfigResult { value: val });
    }
    if let Ok(val) = std::env::var(&key) {
        return Ok(ConfigResult { value: val });
    }

    let config = load_config().map_err(|e| clap_noun_verb::NounVerbError::ExecutionError {
        message: format!("Failed to read configuration file: {}", e),
    })?;

    if let Some(val) = config.get(&key) {
        Ok(ConfigResult { value: val.clone() })
    } else {
        Err(clap_noun_verb::NounVerbError::ExecutionError {
            message: format!(
                "Configuration key '{}' not found in file or environment",
                key
            ),
        })
    }
}

#[verb("set")]
pub fn cmd_set(key: String, value: String) -> Result<ConfigResult> {
    let mut config = load_config().map_err(|e| clap_noun_verb::NounVerbError::ExecutionError {
        message: format!("Failed to read configuration file: {}", e),
    })?;

    config.insert(key.clone(), value.clone());

    save_config(&config).map_err(|e| clap_noun_verb::NounVerbError::ExecutionError {
        message: format!("Failed to write configuration file: {}", e),
    })?;

    Ok(ConfigResult { value })
}
