use clap_noun_verb::Result;
use clap_noun_verb_macros::verb;
use serde::Serialize;

// --- 1. Domain Tier ---
#[derive(Debug, Clone, Serialize)]
pub struct Plugin {
    pub id: String,
    pub name: String,
    pub version: String,
    pub status: PluginStatus,
}

#[derive(Debug, Clone, Serialize)]
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

    pub fn list(&self) -> Vec<Plugin> {
        vec![Plugin {
            id: "1".to_string(),
            name: "example-plugin".to_string(),
            version: "1.0.0".to_string(),
            status: PluginStatus::Loaded,
        }]
    }

    pub fn load(&self, path: &str) -> Plugin {
        Plugin {
            id: "2".to_string(),
            name: format!("loaded-from-{}", path),
            version: "0.1.0".to_string(),
            status: PluginStatus::Loaded,
        }
    }

    pub fn unload(&self, id: &str) -> Plugin {
        Plugin {
            id: id.to_string(),
            name: "unloaded-plugin".to_string(),
            version: "unknown".to_string(),
            status: PluginStatus::Unloaded,
        }
    }

    pub fn update(&self, id: &str, new_version: &str) -> Plugin {
        Plugin {
            id: id.to_string(),
            name: "updated-plugin".to_string(),
            version: new_version.to_string(),
            status: PluginStatus::Loaded,
        }
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
    let plugin = service.load(&path);
    Ok(LoadResult { plugin })
}

#[derive(Serialize)]
pub struct UnloadResult {
    pub plugin: Plugin,
}

#[verb("unload")]
pub fn unload(id: String) -> Result<UnloadResult> {
    let service = PluginService::new();
    let plugin = service.unload(&id);
    Ok(UnloadResult { plugin })
}

#[derive(Serialize)]
pub struct UpdateResult {
    pub plugin: Plugin,
}

#[verb("update")]
pub fn update(id: String, new_version: String) -> Result<UpdateResult> {
    let service = PluginService::new();
    let plugin = service.update(&id, &new_version);
    Ok(UpdateResult { plugin })
}
