use clap_noun_verb::Result;
use clap_noun_verb_macros::verb;
use serde::Serialize;
use tower_lsp_max_runtime::AutonomicMesh;

// ==============================================================================
// 1. Domain Tier
// ==============================================================================

// Hook names are extracted via the Hook trait's name() method.
// hooks: Vec<Box<dyn Hook>> does not implement Serialize, so we map to String.

// ==============================================================================
// 2. Service Tier
// ==============================================================================

/// Service for listing registered mesh hooks.
pub struct HookService {
    state_path: &'static str,
}

impl HookService {
    pub fn new() -> Self {
        Self {
            state_path: ".mesh_state.json",
        }
    }

    pub fn list(&self) -> std::result::Result<Vec<String>, String> {
        let mesh = AutonomicMesh::load_from_file(self.state_path).map_err(|e| e.to_string())?;
        let names: Vec<String> = mesh.hooks.iter().map(|h| h.name().to_string()).collect();
        Ok(names)
    }
}

impl Default for HookService {
    fn default() -> Self {
        Self::new()
    }
}

// ==============================================================================
// 3. CLI Tier
// ==============================================================================

#[derive(Serialize)]
pub struct HookListResult {
    pub hooks: Vec<String>,
    pub count: usize,
}

#[verb("list")]
pub fn list() -> Result<HookListResult> {
    let service = HookService::new();
    let hooks = service
        .list()
        .map_err(|e| clap_noun_verb::error::NounVerbError::execution_error(e.to_string()))?;
    let count = hooks.len();
    Ok(HookListResult { hooks, count })
}

/// Result type for the  verb.
#[derive(Serialize)]
pub struct HookRpcResult {
    pub instance_id: String,
    pub hook_id: Option<String>,
    pub raw: serde_json::Value,
}

#[verb("hook-rpc")]
pub fn hook_rpc(instance_id: String, hook_id: Option<String>) -> clap_noun_verb::Result<HookRpcResult> {
    let state_path = crate::nouns::get_state_path();
    let mut mesh = AutonomicMesh::load_from_file(&state_path)
        .map_err(|e| clap_noun_verb::error::NounVerbError::execution_error(e.to_string()))?;
    let params = match &hook_id {
        Some(id) => serde_json::json!({ "hook_id": id }),
        None => serde_json::Value::Null,
    };
    let raw = mesh
        .dispatch_rpc(&instance_id, "max/hook", params)
        .map_err(|e| clap_noun_verb::error::NounVerbError::execution_error(e.to_string()))?;
    mesh.save_to_file(&state_path)
        .map_err(|e| clap_noun_verb::error::NounVerbError::execution_error(e.to_string()))?;
    Ok(HookRpcResult { instance_id, hook_id, raw })
}

/// Result type for the  verb.
#[derive(Serialize)]
pub struct HookGraphResult {
    pub instance_id: String,
    pub root_node_id: Option<String>,
    pub raw: serde_json::Value,
}

#[verb("hook-graph")]
pub fn hook_graph(instance_id: String, root_node_id: Option<String>) -> clap_noun_verb::Result<HookGraphResult> {
    let state_path = crate::nouns::get_state_path();
    let mut mesh = AutonomicMesh::load_from_file(&state_path)
        .map_err(|e| clap_noun_verb::error::NounVerbError::execution_error(e.to_string()))?;
    let params = match &root_node_id {
        Some(id) => serde_json::json!({ "node_id": id }),
        None => serde_json::Value::Null,
    };
    let raw = mesh
        .dispatch_rpc(&instance_id, "max/hookGraph", params)
        .map_err(|e| clap_noun_verb::error::NounVerbError::execution_error(e.to_string()))?;
    mesh.save_to_file(&state_path)
        .map_err(|e| clap_noun_verb::error::NounVerbError::execution_error(e.to_string()))?;
    Ok(HookGraphResult { instance_id, root_node_id, raw })
}

#[derive(Serialize)]
pub struct PropagateResult {
    pub instance_id: String,
    pub chain_or_hook_id: String,
    pub raw: serde_json::Value,
}

#[verb("propagate")]
pub fn propagate(instance_id: String, chain_or_hook_id: String) -> clap_noun_verb::Result<PropagateResult> {
    let state_path = crate::nouns::get_state_path();
    let mut mesh = AutonomicMesh::load_from_file(&state_path)
        .map_err(|e| clap_noun_verb::error::NounVerbError::execution_error(e.to_string()))?;
    let raw = mesh
        .dispatch_rpc(&instance_id, "max/propagate", serde_json::json!(chain_or_hook_id))
        .map_err(|e| clap_noun_verb::error::NounVerbError::execution_error(e.to_string()))?;
    mesh.save_to_file(&state_path)
        .map_err(|e| clap_noun_verb::error::NounVerbError::execution_error(e.to_string()))?;
    Ok(PropagateResult { instance_id, chain_or_hook_id, raw })
}

#[derive(Serialize)]
pub struct ChainResult {
    pub instance_id: String,
    pub chain_id: Option<String>,
    pub raw: serde_json::Value,
}

#[verb("chain")]
pub fn chain(instance_id: String, chain_id: Option<String>) -> clap_noun_verb::Result<ChainResult> {
    let state_path = crate::nouns::get_state_path();
    let mut mesh = AutonomicMesh::load_from_file(&state_path)
        .map_err(|e| clap_noun_verb::error::NounVerbError::execution_error(e.to_string()))?;
    let params = match &chain_id {
        Some(id) => serde_json::json!({ "chain_id": id }),
        None => serde_json::Value::Null,
    };
    let raw = mesh
        .dispatch_rpc(&instance_id, "max/chain", params)
        .map_err(|e| clap_noun_verb::error::NounVerbError::execution_error(e.to_string()))?;
    mesh.save_to_file(&state_path)
        .map_err(|e| clap_noun_verb::error::NounVerbError::execution_error(e.to_string()))?;
    Ok(ChainResult { instance_id, chain_id, raw })
}
