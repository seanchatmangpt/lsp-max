use clap_noun_verb::error::NounVerbError;
use clap_noun_verb::Result;
use clap_noun_verb_macros::verb;
use lsp_max_agent::LspAgent;
use serde::{Deserialize, Serialize};

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

    pub fn reset(&self, instance_id: String) -> std::result::Result<serde_json::Value, String> {
        let path = crate::nouns::get_state_path();
        let mut mesh =
            lsp_max_runtime::AutonomicMesh::load_from_file(&path).map_err(|e| e.to_string())?;
        let result = mesh.dispatch_rpc(&instance_id, "max/reset", serde_json::Value::Null)?;
        mesh.save_to_file(&path).map_err(|e| e.to_string())?;
        Ok(result)
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

#[cfg(test)]
mod tests {
    use super::*;

    /// RAII guard that saves an env var on creation and restores it on drop.
    struct EnvGuard {
        key: &'static str,
        saved: Option<String>,
    }

    impl EnvGuard {
        fn remove(key: &'static str) -> Self {
            let saved = std::env::var(key).ok();
            // SAFETY: caller holds ENV_MUTEX for the lifetime of this guard.
            unsafe { std::env::remove_var(key) };
            Self { key, saved }
        }
    }

    impl Drop for EnvGuard {
        fn drop(&mut self) {
            match &self.saved {
                Some(val) => std::env::set_var(self.key, val),
                // SAFETY: restoring absent state under ENV_MUTEX.
                None => unsafe { std::env::remove_var(self.key) },
            }
        }
    }

    #[test]
    fn halt_returns_halted_state_with_matching_id() {
        let svc = AgentService::new();
        let result = svc.halt("agent-42".to_string());
        assert!(result.is_ok(), "halt should return Ok");
        let state = result.unwrap();
        assert_eq!(state.id, "agent-42");
        assert!(matches!(state.status, AgentStatus::Halted));
    }

    #[test]
    fn plan_returns_non_empty_steps_for_new_id() {
        let svc = AgentService::new();
        // plan falls back to default steps when agent chat fails (no API key)
        let result = svc.plan("workspace-abc".to_string());
        assert!(
            result.is_ok(),
            "plan should return Ok (uses fallback steps)"
        );
        let plan = result.unwrap();
        assert!(!plan.steps.is_empty(), "plan steps must not be empty");
    }

    #[test]
    fn invoke_without_api_key_returns_err() {
        let _lock = crate::nouns::TEST_ENV_LOCK
            .lock()
            .unwrap_or_else(|p| p.into_inner());
        let _g1 = EnvGuard::remove("TOWER_LSP_MAX_API_KEY");
        let _g2 = EnvGuard::remove("OPENAI_API_KEY");
        let svc = AgentService::new();
        let result = svc.invoke("test task".to_string());
        // Without an API key the service must return Err
        assert!(result.is_err(), "invoke without API key should return Err");
    }

    #[test]
    fn chat_without_api_key_returns_err() {
        let _lock = crate::nouns::TEST_ENV_LOCK
            .lock()
            .unwrap_or_else(|p| p.into_inner());
        let _g1 = EnvGuard::remove("TOWER_LSP_MAX_API_KEY");
        let _g2 = EnvGuard::remove("OPENAI_API_KEY");
        let svc = AgentService::new();
        let result = svc.chat("agent-1".to_string(), "hello".to_string());
        assert!(result.is_err(), "chat without API key should return Err");
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

#[derive(Debug, Clone, Serialize)]
pub struct AgentSummary {
    pub id: String,
    pub status: String,
    pub current_task: Option<String>,
}

#[derive(Serialize)]
pub struct AgentListResult {
    pub agents: Vec<AgentSummary>,
    pub count: usize,
}

#[verb("list")]
pub fn list() -> Result<AgentListResult> {
    // Use max/instanceList RPC for efficient polling — avoids loading full mesh state.
    let path = crate::nouns::get_state_path();
    let mut mesh = lsp_max_runtime::AutonomicMesh::load_from_file(&path).unwrap_or_default();
    // Pick any instance_id that exists, or fall back to full load if mesh is empty.
    let first_id = mesh.instances.keys().next().cloned();
    let agents: Vec<AgentSummary> = if let Some(ref id) = first_id {
        match mesh.dispatch_rpc(id, "max/instanceList", serde_json::Value::Null) {
            Ok(serde_json::Value::Array(entries)) => entries
                .iter()
                .filter_map(|e| {
                    let id = e.get("id")?.as_str()?.to_string();
                    let status = e
                        .get("phase")
                        .and_then(|v| v.as_str())
                        .unwrap_or("Unknown")
                        .to_string();
                    Some(AgentSummary {
                        id,
                        status,
                        current_task: None,
                    })
                })
                .collect(),
            _ => mesh
                .instances
                .values()
                .map(|inst| AgentSummary {
                    id: inst.id.clone(),
                    status: inst.phase.to_string(),
                    current_task: None,
                })
                .collect(),
        }
    } else {
        vec![]
    };
    let count = agents.len();
    Ok(AgentListResult { agents, count })
}

#[verb("halt")]
pub fn halt(id: String) -> Result<HaltResult> {
    let service = AgentService::new();
    let state = service.halt(id).map_err(NounVerbError::execution_error)?;
    Ok(HaltResult { state })
}

#[derive(serde::Serialize)]
pub struct ResetResult {
    pub reset: bool,
    pub instance_id: String,
}

#[verb("reset")]
pub fn reset(instance_id: String) -> Result<ResetResult> {
    let service = AgentService::new();
    let resp = service
        .reset(instance_id.clone())
        .map_err(NounVerbError::execution_error)?;
    Ok(ResetResult {
        reset: resp["reset"].as_bool().unwrap_or(true),
        instance_id,
    })
}

#[derive(serde::Serialize)]
pub struct ReleaseResult {
    pub released: bool,
    pub instance_id: String,
    pub blocking_axes: Vec<String>,
}

#[verb("release")]
pub fn release(instance_id: String) -> Result<ReleaseResult> {
    let path = crate::nouns::get_state_path();
    let mut mesh = lsp_max_runtime::AutonomicMesh::load_from_file(&path)
        .map_err(|e| NounVerbError::execution_error(e.to_string()))?;
    let resp = mesh
        .dispatch_rpc(
            &instance_id,
            "max/releaseActuation",
            serde_json::Value::Null,
        )
        .map_err(NounVerbError::execution_error)?;
    mesh.save_to_file(&path)
        .map_err(|e| NounVerbError::execution_error(e.to_string()))?;
    let released = resp
        .get("released")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let blocking_axes = resp
        .get("blocking_axes")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|a| a.as_str().map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_default();
    Ok(ReleaseResult {
        released,
        instance_id,
        blocking_axes,
    })
}

#[derive(serde::Serialize)]
pub struct AutonomicLoopResult {
    pub hook_count: usize,
    pub instances: Vec<String>,
}

#[verb("loop")]
pub fn autonomic_loop() -> clap_noun_verb::Result<AutonomicLoopResult> {
    let path = crate::nouns::get_state_path();
    let mut mesh = lsp_max_runtime::AutonomicMesh::load_from_file(&path).unwrap_or_default();
    let first_id = mesh.instances.keys().next().cloned();
    let result = if let Some(ref id) = first_id {
        mesh.dispatch_rpc(id, "max/autonomicLoop", serde_json::Value::Null)
            .unwrap_or_else(|_| serde_json::json!({"hook_count": 0, "instances": []}))
    } else {
        serde_json::json!({"hook_count": 0, "instances": []})
    };
    let hook_count = result["hook_count"].as_u64().unwrap_or(0) as usize;
    let instances = mesh.instances.keys().cloned().collect::<Vec<_>>();
    mesh.save_to_file(&path)
        .map_err(|e| clap_noun_verb::error::NounVerbError::execution_error(e.to_string()))?;
    Ok(AutonomicLoopResult {
        hook_count,
        instances,
    })
}
