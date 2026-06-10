use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use tower_lsp_max_protocol::AnalysisBundle;

pub struct AgentExporter;

impl AgentExporter {
    pub fn export_bundle(bundle: &AnalysisBundle) -> String {
        serde_json::to_string_pretty(bundle).unwrap()
    }
}

pub struct AgentConfig {
    pub api_key: Option<String>,
    pub api_base: String,
    pub model: String,
}

impl AgentConfig {
    pub fn load() -> Self {
        let api_key = std::env::var("TOWER_LSP_MAX_API_KEY")
            .ok()
            .or_else(|| std::env::var("OPENAI_API_KEY").ok());
        let api_base = std::env::var("TOWER_LSP_MAX_API_BASE")
            .ok()
            .or_else(|| std::env::var("OPENAI_API_BASE").ok())
            .unwrap_or_else(|| "https://api.openai.com/v1".to_string());
        let model = std::env::var("TOWER_LSP_MAX_MODEL")
            .ok()
            .or_else(|| std::env::var("OPENAI_MODEL").ok())
            .unwrap_or_else(|| "gpt-4o".to_string());

        let mut config_file_map = HashMap::new();
        let config_path = if let Ok(path_str) = std::env::var("TOWER_LSP_MAX_CONFIG") {
            Some(PathBuf::from(path_str))
        } else if let Ok(home) = std::env::var("HOME") {
            Some(PathBuf::from(home).join(".tower-lsp-max-config.json"))
        } else {
            Some(PathBuf::from(".tower-lsp-max-config.json"))
        };

        if let Some(path) = config_path {
            if path.exists() {
                if let Ok(content) = fs::read_to_string(path) {
                    if let Ok(map) = serde_json::from_str::<HashMap<String, String>>(&content) {
                        config_file_map = map;
                    }
                }
            }
        }

        let final_api_key = api_key.or_else(|| {
            config_file_map
                .get("api_key")
                .or_else(|| config_file_map.get("openai_api_key"))
                .cloned()
        });

        let final_api_base = if std::env::var("TOWER_LSP_MAX_API_BASE").is_ok()
            || std::env::var("OPENAI_API_BASE").is_ok()
        {
            api_base
        } else {
            config_file_map
                .get("api_base")
                .or_else(|| config_file_map.get("openai_api_base"))
                .cloned()
                .unwrap_or(api_base)
        };

        let final_model = if std::env::var("TOWER_LSP_MAX_MODEL").is_ok()
            || std::env::var("OPENAI_MODEL").is_ok()
        {
            model
        } else {
            config_file_map
                .get("model")
                .or_else(|| config_file_map.get("openai_model"))
                .cloned()
                .unwrap_or(model)
        };

        Self {
            api_key: final_api_key,
            api_base: final_api_base,
            model: final_model,
        }
    }
}

#[derive(Serialize)]
struct ChatMessage {
    role: String,
    content: String,
}

#[derive(Serialize)]
struct ChatRequest {
    model: String,
    messages: Vec<ChatMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
}

#[derive(Deserialize)]
struct ChatChoiceMessage {
    content: String,
}

#[derive(Deserialize)]
struct ChatChoice {
    message: ChatChoiceMessage,
}

#[derive(Deserialize)]
struct ChatResponse {
    choices: Vec<ChatChoice>,
}

pub struct LspAgent;

impl LspAgent {
    pub fn invoke_task(task: &str) -> Result<String, String> {
        let config = AgentConfig::load();
        let system_prompt = "You are the tower-lsp-max AI Agent. Your role is to perform analysis, code generation, and diagnostic checking for the Language Server Protocol implementation. Please process the user's task and return a precise, structured, and complete answer.";
        Self::run_query(&config, system_prompt, task)
    }

    pub fn chat(message: &str) -> Result<String, String> {
        let config = AgentConfig::load();
        let system_prompt = "You are the tower-lsp-max AI Agent. You are in a chat session with the user. Help them with their queries regarding Language Server Protocol, cargo workspaces, capabilities, and system configuration.";
        Self::run_query(&config, system_prompt, message)
    }

    fn run_query(
        config: &AgentConfig,
        system_prompt: &str,
        user_prompt: &str,
    ) -> Result<String, String> {
        let api_key = match &config.api_key {
            Some(k) => k,
            None => return Err("API key is not configured. Please set the TOWER_LSP_MAX_API_KEY environment variable or run `tower-lsp-max-cli config set api_key <your-key>`.".to_string()),
        };

        let url = format!("{}/chat/completions", config.api_base.trim_end_matches('/'));
        let body = ChatRequest {
            model: config.model.clone(),
            messages: vec![
                ChatMessage {
                    role: "system".to_string(),
                    content: system_prompt.to_string(),
                },
                ChatMessage {
                    role: "user".to_string(),
                    content: user_prompt.to_string(),
                },
            ],
            temperature: Some(0.7),
        };

        let response = ureq::post(&url)
            .set("Authorization", &format!("Bearer {}", api_key))
            .set("Content-Type", "application/json")
            .send_json(&body)
            .map_err(|e| format!("HTTP request failed: {}", e))?;

        let res_body: ChatResponse = response
            .into_json()
            .map_err(|e| format!("Failed to parse response JSON: {}", e))?;

        if let Some(choice) = res_body.choices.first() {
            Ok(choice.message.content.clone())
        } else {
            Err("Received empty response from the AI provider".to_string())
        }
    }
}
