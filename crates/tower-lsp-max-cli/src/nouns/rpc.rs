use clap_noun_verb::Result;
use clap_noun_verb_macros::verb;
use serde::Serialize;
use tower_lsp_max_runtime::AutonomicMesh;

// ==============================================================================
// 1. Domain Tier
// ==============================================================================

// RPC dispatch operates directly on AutonomicMesh via dispatch_rpc.

// ==============================================================================
// 2. Service Tier
// ==============================================================================

/// Service for dispatching RPC calls to mesh instances.
pub struct RpcService {
    state_path: &'static str,
}

impl RpcService {
    pub fn new() -> Self {
        Self {
            state_path: ".mesh_state.json",
        }
    }

    pub fn dispatch(
        &self,
        instance_id: &str,
        method: &str,
        params_json: &str,
    ) -> std::result::Result<serde_json::Value, String> {
        let mut mesh = AutonomicMesh::load_from_file(self.state_path).map_err(|e| e.to_string())?;

        let params: serde_json::Value =
            serde_json::from_str(params_json).map_err(|e| format!("Invalid params JSON: {}", e))?;

        let response = mesh.dispatch_rpc(instance_id, method, params)?;

        mesh.save_to_file(self.state_path)
            .map_err(|e| e.to_string())?;

        Ok(response)
    }
}

impl Default for RpcService {
    fn default() -> Self {
        Self::new()
    }
}

// ==============================================================================
// 3. CLI Tier
// ==============================================================================

#[derive(Serialize)]
pub struct RpcResult {
    pub instance_id: String,
    pub method: String,
    pub response: serde_json::Value,
}

#[verb("dispatch")]
pub fn dispatch(
    instance_id: String,
    method: String,
    params_json: Option<String>,
) -> Result<RpcResult> {
    let service = RpcService::new();
    let params = params_json.unwrap_or_else(|| "null".to_string());
    let response = service
        .dispatch(&instance_id, &method, &params)
        .map_err(clap_noun_verb::error::NounVerbError::execution_error)?;
    Ok(RpcResult {
        instance_id,
        method,
        response,
    })
}
