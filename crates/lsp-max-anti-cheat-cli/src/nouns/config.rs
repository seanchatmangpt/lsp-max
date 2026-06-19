use clap_noun_verb::Result;
use clap_noun_verb_macros::verb;
use serde::Serialize;
use std::fs;
use std::path::Path;

// ===== Domain Tier =====
#[derive(Serialize)]
pub struct ConfigInitResult {
    pub path: String,
    pub config_file: String,
    pub created: bool,
}

#[derive(Serialize)]
pub struct ConfigShowResult {
    pub config_file: String,
    pub exists: bool,
    pub content: Option<String>,
}

#[derive(Serialize)]
pub struct ConfigValidateResult {
    pub config_file: String,
    pub valid: bool,
    pub message: String,
}

// ===== Service Tier =====
pub struct ConfigService;

impl ConfigService {
    pub fn init(path: &str) -> std::result::Result<ConfigInitResult, String> {
        let config_path = Path::new(path).join("anti-llm.toml");
        let config_str = include_str!("../../default-anti-llm.toml");

        if config_path.exists() {
            return Ok(ConfigInitResult {
                path: path.to_string(),
                config_file: config_path.to_string_lossy().to_string(),
                created: false,
            });
        }

        fs::write(&config_path, config_str)
            .map_err(|e| format!("Failed to write config: {}", e))?;

        Ok(ConfigInitResult {
            path: path.to_string(),
            config_file: config_path.to_string_lossy().to_string(),
            created: true,
        })
    }

    pub fn show(path: &str) -> std::result::Result<ConfigShowResult, String> {
        let config_path = Path::new(path).join("anti-llm.toml");
        let exists = config_path.exists();
        let content = if exists {
            Some(
                fs::read_to_string(&config_path)
                    .map_err(|e| format!("Failed to read config: {}", e))?,
            )
        } else {
            None
        };

        Ok(ConfigShowResult {
            config_file: config_path.to_string_lossy().to_string(),
            exists,
            content,
        })
    }

    pub fn validate(path: &str) -> std::result::Result<ConfigValidateResult, String> {
        let config_path = Path::new(path).join("anti-llm.toml");

        if !config_path.exists() {
            return Ok(ConfigValidateResult {
                config_file: config_path.to_string_lossy().to_string(),
                valid: false,
                message: "Config file not found".to_string(),
            });
        }

        let content = fs::read_to_string(&config_path)
            .map_err(|e| format!("Failed to read config: {}", e))?;

        match toml::from_str::<toml::Value>(&content) {
            Ok(_) => Ok(ConfigValidateResult {
                config_file: config_path.to_string_lossy().to_string(),
                valid: true,
                message: "Config is valid TOML".to_string(),
            }),
            Err(e) => Ok(ConfigValidateResult {
                config_file: config_path.to_string_lossy().to_string(),
                valid: false,
                message: format!("Invalid TOML: {}", e),
            }),
        }
    }
}

// ===== Verb Tier (CLI) =====

#[verb("init")]
pub fn init_config(#[arg(default_value = ".")] path: String) -> Result<ConfigInitResult> {
    ConfigService::init(&path).map_err(clap_noun_verb::error::NounVerbError::execution_error)
}

#[verb("show")]
pub fn show_config(#[arg(default_value = ".")] path: String) -> Result<ConfigShowResult> {
    ConfigService::show(&path).map_err(clap_noun_verb::error::NounVerbError::execution_error)
}

#[verb("validate")]
pub fn validate_config(#[arg(default_value = ".")] path: String) -> Result<ConfigValidateResult> {
    ConfigService::validate(&path).map_err(clap_noun_verb::error::NounVerbError::execution_error)
}
