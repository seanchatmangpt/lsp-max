use clap_noun_verb::Result;
use clap_noun_verb_macros::verb;
use serde::{Deserialize, Serialize};
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

    fn profiles_path(&self) -> PathBuf {
        if let Ok(home) = std::env::var("HOME") {
            PathBuf::from(home).join(".lsp-max-config-profiles.json")
        } else {
            PathBuf::from(".lsp-max-config-profiles.json")
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

    fn load_profiles(&self) -> HashMap<String, HashMap<String, String>> {
        let path = self.profiles_path();
        if path.exists() {
            if let Ok(content) = fs::read_to_string(path) {
                if let Ok(map) =
                    serde_json::from_str::<HashMap<String, HashMap<String, String>>>(&content)
                {
                    return map;
                }
            }
        }
        HashMap::new()
    }

    fn save_profiles(
        &self,
        profiles: &HashMap<String, HashMap<String, String>>,
    ) -> std::result::Result<(), String> {
        let path = self.profiles_path();
        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        let content = serde_json::to_string_pretty(profiles).map_err(|e| e.to_string())?;
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

    pub fn profile_list(&self) -> (Vec<String>, usize) {
        let profiles = self.load_profiles();
        let mut names: Vec<String> = profiles.into_keys().collect();
        names.sort();
        let count = names.len();
        (names, count)
    }

    pub fn profile_save(&self, name: &str) -> std::result::Result<(String, usize), String> {
        let current = self.load_config();
        let key_count = current.len();
        let mut profiles = self.load_profiles();
        profiles.insert(name.to_string(), current);
        self.save_profiles(&profiles)?;
        Ok((name.to_string(), key_count))
    }

    pub fn profile_load(
        &self,
        name: &str,
    ) -> std::result::Result<(String, usize), String> {
        let profiles = self.load_profiles();
        let profile = profiles
            .get(name)
            .ok_or_else(|| format!("PROFILE_NOT_FOUND: {}", name))?
            .clone();
        let keys_applied = profile.len();
        let mut current = self.load_config();
        for (k, v) in profile {
            current.insert(k, v);
        }
        self.save_config(&current)?;
        Ok((name.to_string(), keys_applied))
    }

    pub fn diff(
        &self,
        profile_name: &str,
    ) -> std::result::Result<(Vec<String>, Vec<String>, Vec<(String, String, String)>), String>
    {
        let current = self.load_config();
        let profiles = self.load_profiles();
        let profile = profiles
            .get(profile_name)
            .ok_or_else(|| format!("PROFILE_NOT_FOUND: {}", profile_name))?;

        let mut added: Vec<String> = profile
            .keys()
            .filter(|k| !current.contains_key(*k))
            .cloned()
            .collect();
        added.sort();

        let mut removed: Vec<String> = current
            .keys()
            .filter(|k| !profile.contains_key(*k))
            .cloned()
            .collect();
        removed.sort();

        let mut changed: Vec<(String, String, String)> = current
            .iter()
            .filter_map(|(k, cv)| {
                profile.get(k).and_then(|pv| {
                    if cv != pv {
                        Some((k.clone(), cv.clone(), pv.clone()))
                    } else {
                        None
                    }
                })
            })
            .collect();
        changed.sort_by(|a, b| a.0.cmp(&b.0));

        Ok((added, removed, changed))
    }

    pub fn validate(&self) -> (Vec<String>, Vec<String>) {
        const VALID_KEYS: &[&str] = &[
            "log_level",
            "state_path",
            "gate_timeout",
            "max_instances",
            "telemetry_endpoint",
            "receipt_dir",
            "plugin_dir",
        ];
        let current = self.load_config();
        let mut valid_keys: Vec<String> = Vec::new();
        let mut unknown_keys: Vec<String> = Vec::new();
        for key in current.keys() {
            if VALID_KEYS.contains(&key.as_str()) {
                valid_keys.push(key.clone());
            } else {
                unknown_keys.push(key.clone());
            }
        }
        valid_keys.sort();
        unknown_keys.sort();
        (valid_keys, unknown_keys)
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

// ------------------------------------------------------------------
// profile-list
// ------------------------------------------------------------------

#[derive(Serialize)]
pub struct ProfileListResult {
    pub profiles: Vec<String>,
    pub count: usize,
}

#[verb("profile-list")]
pub fn profile_list() -> Result<ProfileListResult> {
    let service = ConfigService::new();
    let (profiles, count) = service.profile_list();
    Ok(ProfileListResult { profiles, count })
}

// ------------------------------------------------------------------
// profile-save
// ------------------------------------------------------------------

#[derive(Serialize)]
pub struct ProfileSaveResult {
    pub name: String,
    pub key_count: usize,
    pub status: String,
}

#[verb("profile-save")]
pub fn profile_save(name: String) -> Result<ProfileSaveResult> {
    let service = ConfigService::new();
    let (name, key_count) = service
        .profile_save(&name)
        .map_err(clap_noun_verb::error::NounVerbError::execution_error)?;
    Ok(ProfileSaveResult {
        name,
        key_count,
        status: "ADMITTED".to_string(),
    })
}

// ------------------------------------------------------------------
// profile-load
// ------------------------------------------------------------------

#[derive(Serialize)]
pub struct ProfileLoadResult {
    pub name: String,
    pub keys_applied: usize,
    pub status: String,
}

#[verb("profile-load")]
pub fn profile_load(name: String) -> Result<ProfileLoadResult> {
    let service = ConfigService::new();
    let (name, keys_applied) = service
        .profile_load(&name)
        .map_err(clap_noun_verb::error::NounVerbError::execution_error)?;
    Ok(ProfileLoadResult {
        name,
        keys_applied,
        status: "ADMITTED".to_string(),
    })
}

// ------------------------------------------------------------------
// diff
// ------------------------------------------------------------------

#[derive(Serialize, Deserialize)]
pub struct ConfigKeyChange {
    pub key: String,
    pub current_value: String,
    pub profile_value: String,
}

#[derive(Serialize)]
pub struct ConfigDiffResult {
    pub added: Vec<String>,
    pub removed: Vec<String>,
    pub changed: Vec<ConfigKeyChange>,
    pub status: String,
}

#[verb("diff")]
pub fn diff(profile_name: String) -> Result<ConfigDiffResult> {
    let service = ConfigService::new();
    let (added, removed, changed_raw) = service
        .diff(&profile_name)
        .map_err(clap_noun_verb::error::NounVerbError::execution_error)?;
    let changed: Vec<ConfigKeyChange> = changed_raw
        .into_iter()
        .map(|(key, current_value, profile_value)| ConfigKeyChange {
            key,
            current_value,
            profile_value,
        })
        .collect();
    let has_diff = !added.is_empty() || !removed.is_empty() || !changed.is_empty();
    let status = if has_diff {
        "OPEN".to_string()
    } else {
        "ADMITTED".to_string()
    };
    Ok(ConfigDiffResult {
        added,
        removed,
        changed,
        status,
    })
}

// ------------------------------------------------------------------
// validate
// ------------------------------------------------------------------

#[derive(Serialize)]
pub struct ConfigValidateResult {
    pub valid_keys: Vec<String>,
    pub unknown_keys: Vec<String>,
    pub status: String,
}

#[verb("validate")]
pub fn validate() -> Result<ConfigValidateResult> {
    let service = ConfigService::new();
    let (valid_keys, unknown_keys) = service.validate();
    let status = if unknown_keys.is_empty() {
        "ADMITTED".to_string()
    } else {
        "PARTIAL".to_string()
    };
    Ok(ConfigValidateResult {
        valid_keys,
        unknown_keys,
        status,
    })
}
