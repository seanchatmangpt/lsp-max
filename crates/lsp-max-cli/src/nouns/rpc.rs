use clap_noun_verb::error::NounVerbError;
use clap_noun_verb::Result;
use clap_noun_verb_macros::verb;
use lsp_max_runtime::AutonomicMesh;
use serde::Serialize;

// ==============================================================================
// 1. Domain Tier
// ==============================================================================

// RPC dispatch operates directly on AutonomicMesh via dispatch_rpc.

// ==============================================================================
// 2. Service Tier
// ==============================================================================

/// Service for dispatching RPC calls to mesh instances.
pub struct RpcService {
    state_path: String,
}

impl RpcService {
    pub fn new() -> Self {
        Self {
            state_path: crate::nouns::get_state_path(),
        }
    }

    pub fn dispatch(
        &self,
        instance_id: &str,
        method: &str,
        params_json: &str,
    ) -> std::result::Result<serde_json::Value, String> {
        let mut mesh = AutonomicMesh::load_from_file(&self.state_path).map_err(|e| e.to_string())?;

        let params: serde_json::Value =
            serde_json::from_str(params_json).map_err(|e| format!("Invalid params JSON: {}", e))?;

        let response = mesh.dispatch_rpc(instance_id, method, params)?;

        mesh.save_to_file(&self.state_path)
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

/// Dispatch an arbitrary `max/*` RPC method to a mesh instance with JSON params.
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
        .map_err(NounVerbError::execution_error)?;
    Ok(RpcResult {
        instance_id,
        method,
        response,
    })
}

// ==============================================================================
// 4. Tests
// ==============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use lsp_max_runtime::{AutonomicMesh, LspInstance};

    fn make_temp_mesh() -> (tempfile::NamedTempFile, RpcService) {
        let mut mesh = AutonomicMesh::new();
        mesh.add_instance(LspInstance::new("inst-1"));
        let f = tempfile::NamedTempFile::new().unwrap();
        mesh.save_to_file(f.path().to_str().unwrap()).unwrap();
        let svc = RpcService {
            state_path: f.path().to_str().unwrap().to_string(),
        };
        (f, svc)
    }

    // --- dispatch ---

    #[test]
    fn dispatch_known_method_returns_ok() {
        let (_f, svc) = make_temp_mesh();
        assert!(svc.dispatch("inst-1", "max/dumpState", "null").is_ok());
    }

    #[test]
    fn dispatch_result_is_serialisable_json_value() {
        let (_f, svc) = make_temp_mesh();
        let val = svc.dispatch("inst-1", "max/dumpState", "null").unwrap();
        let s = serde_json::to_string(&val).expect("response must serialize");
        assert!(!s.is_empty());
    }

    #[test]
    fn dispatch_invalid_json_params_returns_err() {
        let (_f, svc) = make_temp_mesh();
        let result = svc.dispatch("inst-1", "max/dumpState", "{ not valid json }");
        assert!(result.is_err(), "malformed params JSON must return Err");
        let msg = result.unwrap_err();
        assert!(
            msg.contains("Invalid params JSON"),
            "error should identify the params problem: {msg}"
        );
    }

    #[test]
    fn dispatch_fails_on_missing_state_file() {
        let svc = RpcService {
            state_path: "/tmp/nonexistent-rpc-test-state.json".to_string(),
        };
        assert!(svc.dispatch("inst-1", "max/dumpState", "null").is_err());
    }

    #[test]
    fn dispatch_preserves_instance_and_method_in_result() {
        let _lock = crate::nouns::TEST_ENV_LOCK
            .lock()
            .unwrap_or_else(|p| p.into_inner());
        let mut mesh = AutonomicMesh::new();
        mesh.add_instance(LspInstance::new("inst-2"));
        let tmpf = tempfile::NamedTempFile::new().unwrap();
        let path = tmpf.path().to_str().unwrap().to_string();
        mesh.save_to_file(&path).unwrap();
        let prev = std::env::var("LSP_MAX_STATE_PATH").ok();
        // SAFETY: under TEST_ENV_LOCK.
        unsafe { std::env::set_var("LSP_MAX_STATE_PATH", &path) };
        let result = dispatch(
            "inst-2".to_string(),
            "max/dumpState".to_string(),
            Some("null".to_string()),
        );
        // SAFETY: restoring env under TEST_ENV_LOCK.
        unsafe {
            match prev {
                Some(v) => std::env::set_var("LSP_MAX_STATE_PATH", v),
                None => std::env::remove_var("LSP_MAX_STATE_PATH"),
            }
        }
        let rpc_result = result.unwrap();
        assert_eq!(rpc_result.instance_id, "inst-2");
        assert_eq!(rpc_result.method, "max/dumpState");
    }
}
