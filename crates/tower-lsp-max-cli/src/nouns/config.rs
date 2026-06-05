use clap_noun_verb::Result;
use clap_noun_verb_macros::verb;
use serde::Serialize;

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

    pub fn view(&self, key: &str) -> Option<ConfigEntity> {
        Some(ConfigEntity {
            key: key.to_string(),
            value: "mock_value".to_string(),
        })
    }

    pub fn set(&self, key: &str, value: &str) -> ConfigEntity {
        ConfigEntity {
            key: key.to_string(),
            value: value.to_string(),
        }
    }

    pub fn reset(&self, key: &str) -> ConfigEntity {
        ConfigEntity {
            key: key.to_string(),
            value: "default_value".to_string(),
        }
    }

    pub fn list(&self) -> Vec<ConfigEntity> {
        vec![ConfigEntity {
            key: "mock_key".to_string(),
            value: "mock_value".to_string(),
        }]
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
    let config = service.set(&key, &value);
    Ok(SetResult { config })
}

#[derive(Serialize)]
pub struct ResetResult {
    pub config: ConfigEntity,
}

#[verb("reset")]
pub fn reset(key: String) -> Result<ResetResult> {
    let service = ConfigService::new();
    let config = service.reset(&key);
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
