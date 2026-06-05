use clap_noun_verb::Result;
use clap_noun_verb_macros::verb;
use serde::{Deserialize, Serialize};

// --- 1. Domain Tier ---
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Plugin {
    pub id: String,
    pub name: String,
    pub version: String,
    pub status: PluginStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PluginStatus {
    Loaded,
    Unloaded,
    Error(String),
}

// --- 2. Service Tier ---
pub struct PluginService;

impl Default for PluginService {
    fn default() -> Self {
        Self::new()
    }
}

impl PluginService {
    pub fn new() -> Self {
        Self
    }

    fn load_mesh_json() -> serde_json::Value {
        let path = crate::nouns::get_state_path();
        if std::path::Path::new(&path).exists() {
            if let Ok(content) = std::fs::read_to_string(&path) {
                if let Ok(val) = serde_json::from_str(&content) {
                    return val;
                }
            }
        }
        serde_json::json!({
            "instances": {}
        })
    }

    fn save_mesh_json(val: &serde_json::Value) -> std::result::Result<(), String> {
        let path = crate::nouns::get_state_path();
        let content = serde_json::to_string_pretty(val).map_err(|e| e.to_string())?;
        std::fs::write(&path, content).map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn list(&self) -> Vec<Plugin> {
        let mesh = Self::load_mesh_json();
        if let Some(plugins_val) = mesh.get("plugins") {
            if let Ok(list) = serde_json::from_value::<Vec<Plugin>>(plugins_val.clone()) {
                return list;
            }
        }

        // Default plugins
        let defaults = vec![Plugin {
            id: "1".to_string(),
            name: "example-plugin".to_string(),
            version: "1.0.0".to_string(),
            status: PluginStatus::Loaded,
        }];

        let mut mesh = Self::load_mesh_json();
        mesh["plugins"] = serde_json::json!(defaults);
        let _ = Self::save_mesh_json(&mesh);

        defaults
    }

    pub fn load(&self, path: &str) -> std::result::Result<Plugin, String> {
        let mut mesh = Self::load_mesh_json();
        let mut list = self.list();

        let file_exists = std::path::Path::new(path).exists();
        let status = if file_exists {
            PluginStatus::Loaded
        } else {
            PluginStatus::Error(format!("Plugin path not found: {}", path))
        };

        let name = std::path::Path::new(path)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or(path)
            .to_string();

        let new_id = (list.len() + 1).to_string();
        let plugin = Plugin {
            id: new_id,
            name,
            version: "0.1.0".to_string(),
            status,
        };

        list.push(plugin.clone());
        mesh["plugins"] = serde_json::json!(list);
        Self::save_mesh_json(&mesh)?;

        Ok(plugin)
    }

    pub fn unload(&self, id: &str) -> std::result::Result<Plugin, String> {
        let mut mesh = Self::load_mesh_json();
        let mut list = self.list();

        let mut found = None;
        for plugin in &mut list {
            if plugin.id == id {
                plugin.status = PluginStatus::Unloaded;
                found = Some(plugin.clone());
                break;
            }
        }

        let plugin = match found {
            Some(p) => p,
            None => return Err(format!("Plugin with id {} not found", id)),
        };

        mesh["plugins"] = serde_json::json!(list);
        Self::save_mesh_json(&mesh)?;

        Ok(plugin)
    }

    pub fn update(&self, id: &str, new_version: &str) -> std::result::Result<Plugin, String> {
        let mut mesh = Self::load_mesh_json();
        let mut list = self.list();

        let mut found = None;
        for plugin in &mut list {
            if plugin.id == id {
                plugin.version = new_version.to_string();
                plugin.status = PluginStatus::Loaded;
                found = Some(plugin.clone());
                break;
            }
        }

        let plugin = match found {
            Some(p) => p,
            None => return Err(format!("Plugin with id {} not found", id)),
        };

        mesh["plugins"] = serde_json::json!(list);
        Self::save_mesh_json(&mesh)?;

        Ok(plugin)
    }
}

// --- 3. CLI Tier ---

#[derive(Serialize)]
pub struct ListResult {
    pub plugins: Vec<Plugin>,
}

#[verb("list")]
pub fn list() -> Result<ListResult> {
    let service = PluginService::new();
    let plugins = service.list();
    Ok(ListResult { plugins })
}

#[derive(Serialize)]
pub struct LoadResult {
    pub plugin: Plugin,
}

#[verb("load")]
pub fn load(path: String) -> Result<LoadResult> {
    let service = PluginService::new();
    let plugin = service
        .load(&path)
        .map_err(clap_noun_verb::error::NounVerbError::execution_error)?;
    Ok(LoadResult { plugin })
}

#[derive(Serialize)]
pub struct UnloadResult {
    pub plugin: Plugin,
}

#[verb("unload")]
pub fn unload(id: String) -> Result<UnloadResult> {
    let service = PluginService::new();
    let plugin = service
        .unload(&id)
        .map_err(clap_noun_verb::error::NounVerbError::execution_error)?;
    Ok(UnloadResult { plugin })
}

#[derive(Serialize)]
pub struct UpdateResult {
    pub plugin: Plugin,
}

#[verb("update")]
pub fn update(id: String, new_version: String) -> Result<UpdateResult> {
    let service = PluginService::new();
    let plugin = service
        .update(&id, &new_version)
        .map_err(clap_noun_verb::error::NounVerbError::execution_error)?;
    Ok(UpdateResult { plugin })
}
