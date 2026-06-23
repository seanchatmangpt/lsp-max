use clap_noun_verb::error::NounVerbError;
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

/// List all plugins registered in the mesh state.
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

/// Load a plugin from the given filesystem path and register it in the mesh state.
#[verb("load")]
pub fn load(path: String) -> Result<LoadResult> {
    let service = PluginService::new();
    let plugin = service
        .load(&path)
        .map_err(NounVerbError::execution_error)?;
    Ok(LoadResult { plugin })
}

#[derive(Serialize)]
pub struct UnloadResult {
    pub plugin: Plugin,
}

/// Set a plugin's status to Unloaded by id.
#[verb("unload")]
pub fn unload(id: String) -> Result<UnloadResult> {
    let service = PluginService::new();
    let plugin = service
        .unload(&id)
        .map_err(NounVerbError::execution_error)?;
    Ok(UnloadResult { plugin })
}

#[derive(Serialize)]
pub struct UpdateResult {
    pub plugin: Plugin,
}

/// Update a plugin's version by id and set its status to Loaded.
#[verb("update")]
pub fn update(id: String, new_version: String) -> Result<UpdateResult> {
    let service = PluginService::new();
    let plugin = service
        .update(&id, &new_version)
        .map_err(NounVerbError::execution_error)?;
    Ok(UpdateResult { plugin })
}

// ==============================================================================
// 4. Tests
// ==============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    /// RAII guard — redirects LSP_MAX_STATE_PATH to a temp file.
    struct StateGuard {
        _tmp: tempfile::NamedTempFile,
        prev: Option<String>,
    }

    impl StateGuard {
        fn new() -> Self {
            let tmp = tempfile::NamedTempFile::new().unwrap();
            let path = tmp.path().to_str().unwrap().to_string();
            let prev = std::env::var("LSP_MAX_STATE_PATH").ok();
            // SAFETY: under TEST_ENV_LOCK.
            unsafe { std::env::set_var("LSP_MAX_STATE_PATH", &path) };
            Self { _tmp: tmp, prev }
        }
    }

    impl Drop for StateGuard {
        fn drop(&mut self) {
            // SAFETY: restoring env state under TEST_ENV_LOCK.
            unsafe {
                match &self.prev {
                    Some(v) => std::env::set_var("LSP_MAX_STATE_PATH", v),
                    None => std::env::remove_var("LSP_MAX_STATE_PATH"),
                }
            }
        }
    }

    // --- list ---

    #[test]
    fn list_returns_at_least_one_default_plugin() {
        let _lock = crate::nouns::TEST_ENV_LOCK
            .lock()
            .unwrap_or_else(|p| p.into_inner());
        let _g = StateGuard::new();
        let svc = PluginService::new();
        let plugins = svc.list();
        assert!(!plugins.is_empty(), "list must return at least the default plugin");
    }

    #[test]
    fn list_default_plugin_is_loaded() {
        let _lock = crate::nouns::TEST_ENV_LOCK
            .lock()
            .unwrap_or_else(|p| p.into_inner());
        let _g = StateGuard::new();
        let svc = PluginService::new();
        let plugins = svc.list();
        let first = &plugins[0];
        assert!(
            matches!(first.status, PluginStatus::Loaded),
            "default plugin must have Loaded status"
        );
    }

    // --- load ---

    #[test]
    fn load_nonexistent_path_records_error_status() {
        let _lock = crate::nouns::TEST_ENV_LOCK
            .lock()
            .unwrap_or_else(|p| p.into_inner());
        let _g = StateGuard::new();
        let svc = PluginService::new();
        let plugin = svc.load("/no/such/plugin.wasm").unwrap();
        assert!(
            matches!(plugin.status, PluginStatus::Error(_)),
            "non-existent plugin path must produce Error status"
        );
    }

    #[test]
    fn load_existing_file_returns_loaded_status() {
        let _lock = crate::nouns::TEST_ENV_LOCK
            .lock()
            .unwrap_or_else(|p| p.into_inner());
        let _g = StateGuard::new();
        let tmp = tempfile::NamedTempFile::new().unwrap();
        let path = tmp.path().to_str().unwrap().to_string();
        let svc = PluginService::new();
        let plugin = svc.load(&path).unwrap();
        assert!(matches!(plugin.status, PluginStatus::Loaded));
    }

    // --- unload ---

    #[test]
    fn unload_unknown_id_returns_err() {
        let _lock = crate::nouns::TEST_ENV_LOCK
            .lock()
            .unwrap_or_else(|p| p.into_inner());
        let _g = StateGuard::new();
        // Ensure state is initialised (list seeds defaults).
        let svc = PluginService::new();
        let _ = svc.list();
        let result = svc.unload("999");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("not found"));
    }

    #[test]
    fn unload_known_id_sets_unloaded_status() {
        let _lock = crate::nouns::TEST_ENV_LOCK
            .lock()
            .unwrap_or_else(|p| p.into_inner());
        let _g = StateGuard::new();
        let svc = PluginService::new();
        let plugins = svc.list();
        let id = plugins[0].id.clone();
        let plugin = svc.unload(&id).unwrap();
        assert!(matches!(plugin.status, PluginStatus::Unloaded));
    }

    // --- update ---

    #[test]
    fn update_unknown_id_returns_err() {
        let _lock = crate::nouns::TEST_ENV_LOCK
            .lock()
            .unwrap_or_else(|p| p.into_inner());
        let _g = StateGuard::new();
        let svc = PluginService::new();
        let _ = svc.list();
        let result = svc.update("999", "2.0.0");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("not found"));
    }

    #[test]
    fn update_known_id_sets_new_version() {
        let _lock = crate::nouns::TEST_ENV_LOCK
            .lock()
            .unwrap_or_else(|p| p.into_inner());
        let _g = StateGuard::new();
        let svc = PluginService::new();
        let plugins = svc.list();
        let id = plugins[0].id.clone();
        let plugin = svc.update(&id, "9.9.9").unwrap();
        assert_eq!(plugin.version, "9.9.9");
        assert!(matches!(plugin.status, PluginStatus::Loaded));
    }
}
