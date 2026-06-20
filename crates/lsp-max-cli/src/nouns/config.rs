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

// ==========================================
// Static catalog of known config keys
// ==========================================

#[derive(Debug, Serialize, Clone)]
pub struct ConfigKeySchema {
    pub key: String,
    pub env_var: String,
    pub default_value: String,
    pub description: String,
    pub required: bool,
}

fn known_keys() -> Vec<ConfigKeySchema> {
    vec![
        ConfigKeySchema {
            key: "api_base".to_string(),
            env_var: "LSP_MAX_API_BASE".to_string(),
            default_value: "https://api.openai.com/v1".to_string(),
            description: "API base URL (also read from OPENAI_API_BASE)".to_string(),
            required: false,
        },
        ConfigKeySchema {
            key: "model".to_string(),
            env_var: "LSP_MAX_MODEL".to_string(),
            default_value: "gpt-4o".to_string(),
            description: "Model name (also read from OPENAI_MODEL)".to_string(),
            required: false,
        },
        ConfigKeySchema {
            key: "api_key".to_string(),
            env_var: "LSP_MAX_API_KEY".to_string(),
            default_value: "".to_string(),
            description: "LLM API key — required; no sensible default (also read from OPENAI_API_KEY)".to_string(),
            required: true,
        },
        ConfigKeySchema {
            key: "state_path".to_string(),
            env_var: "LSP_MAX_STATE_PATH".to_string(),
            default_value: ".mesh_state.json".to_string(),
            description: "Mesh state file path (env-only; not yet read from JSON config)".to_string(),
            required: false,
        },
        ConfigKeySchema {
            key: "database_path".to_string(),
            env_var: "LSP_MAX_DB_PATH".to_string(),
            default_value: "".to_string(),
            description: "Graph DB directory path".to_string(),
            required: false,
        },
        ConfigKeySchema {
            key: "timeout".to_string(),
            env_var: "LSP_MAX_TIMEOUT".to_string(),
            default_value: "150".to_string(),
            description: "Upstream timeout in milliseconds (env-only; not yet read from JSON config)".to_string(),
            required: false,
        },
    ]
}

// ==========================================
// Schema verb
// ==========================================

#[derive(Serialize)]
pub struct SchemaResult {
    pub keys: Vec<ConfigKeySchema>,
    pub config_path: String,
}

#[verb("schema")]
pub fn schema() -> Result<SchemaResult> {
    let service = ConfigService::new();
    let config_path = service.config_path().to_string_lossy().into_owned();
    Ok(SchemaResult {
        keys: known_keys(),
        config_path,
    })
}

// ==========================================
// Doctor verb
// ==========================================

#[derive(Debug, Serialize, Clone)]
pub struct ConfigKeyStatus {
    pub key: String,
    pub status: String,
    pub effective_value: String,
    pub source: String,
}

#[derive(Serialize)]
pub struct ConfigDoctorResult {
    pub overall: String,
    pub keys: Vec<ConfigKeyStatus>,
}

fn mask_if_sensitive(key: &str, value: &str) -> String {
    if !value.is_empty()
        && (key.contains("key") || key.contains("secret") || key.contains("token"))
    {
        "***".to_string()
    } else {
        value.to_string()
    }
}

impl ConfigService {
    pub fn doctor(&self) -> ConfigDoctorResult {
        let file_config = self.load_config();
        let catalog = known_keys();
        let mut any_unknown = false;
        let mut any_partial = false;

        let keys: Vec<ConfigKeyStatus> = catalog
            .iter()
            .map(|s| self.resolve_key_status(s, &file_config, &mut any_unknown, &mut any_partial))
            .collect();

        let overall = if any_unknown { "UNKNOWN" } else if any_partial { "PARTIAL" } else { "ADMITTED" };
        ConfigDoctorResult { overall: overall.to_string(), keys }
    }

    fn resolve_key_status(
        &self,
        schema: &ConfigKeySchema,
        file_config: &std::collections::HashMap<String, String>,
        any_unknown: &mut bool,
        any_partial: &mut bool,
    ) -> ConfigKeyStatus {
        if let Ok(env_val) = std::env::var(&schema.env_var) {
            return ConfigKeyStatus {
                key: schema.key.clone(),
                status: "ADMITTED".to_string(),
                effective_value: mask_if_sensitive(&schema.key, &env_val),
                source: "env".to_string(),
            };
        }
        if let Some(file_val) = file_config.get(&schema.key) {
            let (status, effective) = if file_val.is_empty() && schema.required {
                *any_unknown = true;
                ("UNKNOWN", String::new())
            } else {
                ("ADMITTED", mask_if_sensitive(&schema.key, file_val))
            };
            return ConfigKeyStatus { key: schema.key.clone(), status: status.to_string(), effective_value: effective, source: "file".to_string() };
        }
        if !schema.default_value.is_empty() {
            *any_partial = true;
            return ConfigKeyStatus { key: schema.key.clone(), status: "PARTIAL".to_string(), effective_value: mask_if_sensitive(&schema.key, &schema.default_value), source: "default".to_string() };
        }
        if schema.required { *any_unknown = true; } else { *any_partial = true; }
        ConfigKeyStatus { key: schema.key.clone(), status: if schema.required { "UNKNOWN" } else { "PARTIAL" }.to_string(), effective_value: String::new(), source: "default".to_string() }
    }
}

#[verb("doctor")]
pub fn doctor() -> Result<ConfigDoctorResult> {
    Ok(ConfigService::new().doctor())
}
