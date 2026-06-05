use clap_noun_verb::error::NounVerbError;
use clap_noun_verb::Result;
use clap_noun_verb_macros::verb;
use serde::{Deserialize, Serialize};
use tower_lsp_max_agent::LspAgent;

// --- Domain Tier ---

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AgentStatus {
    Idle,
    Working,
    Planning,
    Halted,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentState {
    pub id: String,
    pub status: AgentStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentMessage {
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentPlan {
    pub steps: Vec<String>,
}

// --- Service Tier ---

pub struct AgentService;

impl AgentService {
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

    pub fn invoke(&self, task: String) -> std::result::Result<AgentMessage, String> {
        let content = LspAgent::invoke_task(&task)?;
        Ok(AgentMessage { content })
    }

    pub fn chat(&self, _id: String, message: String) -> std::result::Result<AgentMessage, String> {
        let content = LspAgent::chat(&message)?;
        Ok(AgentMessage { content })
    }

    pub fn plan(&self, id: String) -> std::result::Result<AgentPlan, String> {
        let mut mesh = Self::load_mesh_json();
        if !mesh.is_object() {
            mesh = serde_json::json!({});
        }
        let plans = mesh
            .as_object_mut()
            .unwrap()
            .entry("agent_plans")
            .or_insert_with(|| serde_json::json!({}));
        if let Some(plan_json) = plans.get(&id) {
            if let Ok(steps) = serde_json::from_value::<Vec<String>>(plan_json.clone()) {
                return Ok(AgentPlan { steps });
            }
        }

        // Generate plan (try using agent chat, fallback if fail)
        let steps = match LspAgent::chat(&format!(
            "Give a short 3-step numbered list of tasks to execute for: {}",
            id
        )) {
            Ok(resp) => {
                let mut parsed = Vec::new();
                for line in resp.lines() {
                    let cleaned = line.trim();
                    if !cleaned.is_empty()
                        && (cleaned.starts_with(|c: char| c.is_ascii_digit())
                            || cleaned.starts_with('-')
                            || cleaned.starts_with('*'))
                    {
                        parsed.push(cleaned.to_string());
                    }
                }
                if parsed.is_empty() {
                    parsed.push(format!("Step 1: Check instances and diagnostics in {}", id));
                    parsed.push("Step 2: Verify policy rules conformance".to_string());
                    parsed.push("Step 3: Clear resolved diagnostics and emit receipts".to_string());
                }
                parsed
            }
            Err(_) => {
                vec![
                    format!("Step 1: Check instances and diagnostics in {}", id),
                    "Step 2: Verify policy rules conformance".to_string(),
                    "Step 3: Clear resolved diagnostics and emit receipts".to_string(),
                ]
            }
        };

        plans[id.clone()] = serde_json::json!(steps);
        Self::save_mesh_json(&mesh)?;

        Ok(AgentPlan { steps })
    }

    pub fn halt(&self, id: String) -> std::result::Result<AgentState, String> {
        let mut mesh = Self::load_mesh_json();
        if !mesh.is_object() {
            mesh = serde_json::json!({});
        }
        let agents = mesh
            .as_object_mut()
            .unwrap()
            .entry("agents")
            .or_insert_with(|| serde_json::json!({}));
        agents[id.clone()] = serde_json::json!({
            "id": id.clone(),
            "status": "Halted"
        });

        Self::save_mesh_json(&mesh)?;

        Ok(AgentState {
            id,
            status: AgentStatus::Halted,
        })
    }
}

impl Default for AgentService {
    fn default() -> Self {
        Self::new()
    }
}

// --- CLI Tier ---

#[derive(Serialize)]
pub struct InvokeResult {
    pub message: AgentMessage,
}

#[verb("invoke")]
pub fn invoke(task: String) -> Result<InvokeResult> {
    let service = AgentService::new();
    let message = service
        .invoke(task)
        .map_err(NounVerbError::execution_error)?;
    Ok(InvokeResult { message })
}

#[derive(Serialize)]
pub struct ChatResult {
    pub message: AgentMessage,
}

#[verb("chat")]
pub fn chat(id: String, message: String) -> Result<ChatResult> {
    let service = AgentService::new();
    let msg = service
        .chat(id, message)
        .map_err(NounVerbError::execution_error)?;
    Ok(ChatResult { message: msg })
}

#[derive(Serialize)]
pub struct PlanResult {
    pub plan: AgentPlan,
}

#[verb("plan")]
pub fn plan(id: String) -> Result<PlanResult> {
    let service = AgentService::new();
    let plan = service.plan(id).map_err(NounVerbError::execution_error)?;
    Ok(PlanResult { plan })
}

#[derive(Serialize)]
pub struct HaltResult {
    pub state: AgentState,
}

#[verb("halt")]
pub fn halt(id: String) -> Result<HaltResult> {
    let service = AgentService::new();
    let state = service.halt(id).map_err(NounVerbError::execution_error)?;
    Ok(HaltResult { state })
}
