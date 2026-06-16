use clap_noun_verb::Result;
use clap_noun_verb_macros::verb;
use serde::Serialize;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

// ==========================================
// Tier 1: Domain Tier
// ==========================================

#[derive(Debug, Clone, Serialize)]
pub struct ConfigEntity {
    pub key: String,
    pub value: String,
}

// ==========================================
// Tier 2: Service Tier
// ==========================================

pub struct ConfigService;

impl ConfigService {
    pub fn new() -> Self {
        Self
    }

    fn config_path(&self) -> PathBuf {
        if let Ok(path_str) = std::env::var("LSP_MAX_CONFIG") {
            PathBuf::from(path_str)
        } else if let Ok(home) = std::env::var("HOME") {
            PathBuf::from(home).join(".lsp-max-config.json")
        } else {
            PathBuf::from(".lsp-max-config.json")
        }
    }

    fn load_config(&self) -> HashMap<String, String> {
        let path = self.config_path();
        if path.exists() {
            if let Ok(content) = fs::read_to_string(path) {
                if let Ok(map) = serde_json::from_str::<HashMap<String, String>>(&content) {
                    return map;
                }
            }
        }
        HashMap::new()
    }

    fn save_config(&self, map: &HashMap<String, String>) -> std::result::Result<(), String> {
        let path = self.config_path();
        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        let content = serde_json::to_string_pretty(map).map_err(|e| e.to_string())?;
        fs::write(path, content).map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn view(&self, key: &str) -> Option<ConfigEntity> {
        let map = self.load_config();
        map.get(key).map(|value| ConfigEntity {
            key: key.to_string(),
            value: value.clone(),
        })
    }

    pub fn set(&self, key: &str, value: &str) -> std::result::Result<ConfigEntity, String> {
        let mut map = self.load_config();
        map.insert(key.to_string(), value.to_string());
        self.save_config(&map)?;
        Ok(ConfigEntity {
            key: key.to_string(),
            value: value.to_string(),
        })
    }

    pub fn reset(&self, key: &str) -> std::result::Result<ConfigEntity, String> {
        let mut map = self.load_config();
        map.remove(key);
        self.save_config(&map)?;
        Ok(ConfigEntity {
            key: key.to_string(),
            value: "".to_string(),
        })
    }

    pub fn list(&self) -> Vec<ConfigEntity> {
        let map = self.load_config();
        map.into_iter()
            .map(|(key, value)| ConfigEntity { key, value })
            .collect()
    }
}

impl Default for ConfigService {
    fn default() -> Self {
        Self::new()
    }
}

// ==========================================
// Tier 3: CLI Tier
// ==========================================

#[derive(Serialize)]
pub struct ViewResult {
    pub config: Option<ConfigEntity>,
}

#[verb("view")]
pub fn view(key: String) -> Result<ViewResult> {
    let service = ConfigService::new();
    let config = service.view(&key);
    Ok(ViewResult { config })
}

#[derive(Serialize)]
pub struct SetResult {
    pub config: ConfigEntity,
}

#[verb("set")]
pub fn set(key: String, value: String) -> Result<SetResult> {
    let service = ConfigService::new();
    let config = service
        .set(&key, &value)
        .map_err(clap_noun_verb::error::NounVerbError::execution_error)?;
    Ok(SetResult { config })
}

#[derive(Serialize)]
pub struct ResetResult {
    pub config: ConfigEntity,
}

#[verb("reset")]
pub fn reset(key: String) -> Result<ResetResult> {
    let service = ConfigService::new();
    let config = service
        .reset(&key)
        .map_err(clap_noun_verb::error::NounVerbError::execution_error)?;
    Ok(ResetResult { config })
}

#[derive(Serialize)]
pub struct ListResult {
    pub configs: Vec<ConfigEntity>,
}

#[verb("list")]
pub fn list() -> Result<ListResult> {
    let service = ConfigService::new();
    let configs = service.list();
    Ok(ListResult { configs })
}
