use clap_noun_verb::error::NounVerbError;
use clap_noun_verb::Result;
use clap_noun_verb_macros::verb;
use lsp_max_runtime::AutonomicMesh;
use serde::Serialize;

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
    state_path: String,
}

impl HookService {
    pub fn new() -> Self {
        Self {
            state_path: crate::nouns::get_state_path(),
        }
    }

    pub fn list(&self) -> std::result::Result<Vec<String>, String> {
        let mesh = AutonomicMesh::load_from_file(&self.state_path).map_err(|e| e.to_string())?;
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

/// List all hook names registered in the mesh.
#[verb("list")]
pub fn list() -> Result<HookListResult> {
    let service = HookService::new();
    let hooks = service
        .list()
        .map_err(|e| NounVerbError::execution_error(e.to_string()))?;
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

/// Dispatch the `max/hook` RPC for the given instance, optionally scoped to a hook_id.
#[verb("hook-rpc")]
pub fn hook_rpc(
    instance_id: String,
    hook_id: Option<String>,
) -> clap_noun_verb::Result<HookRpcResult> {
    let state_path = crate::nouns::get_state_path();
    let mut mesh = AutonomicMesh::load_from_file(&state_path)
        .map_err(|e| NounVerbError::execution_error(e.to_string()))?;
    let params = match &hook_id {
        Some(id) => serde_json::json!({ "hook_id": id }),
        None => serde_json::Value::Null,
    };
    let raw = mesh
        .dispatch_rpc(&instance_id, "max/hook", params)
        .map_err(|e| NounVerbError::execution_error(e.to_string()))?;
    mesh.save_to_file(&state_path)
        .map_err(|e| NounVerbError::execution_error(e.to_string()))?;
    Ok(HookRpcResult {
        instance_id,
        hook_id,
        raw,
    })
}

/// Result type for the  verb.
#[derive(Serialize)]
pub struct HookGraphResult {
    pub instance_id: String,
    pub root_node_id: Option<String>,
    pub raw: serde_json::Value,
}

/// Dispatch the `max/hookGraph` RPC to retrieve the hook propagation graph.
#[verb("hook-graph")]
pub fn hook_graph(
    instance_id: String,
    root_node_id: Option<String>,
) -> clap_noun_verb::Result<HookGraphResult> {
    let state_path = crate::nouns::get_state_path();
    let mut mesh = AutonomicMesh::load_from_file(&state_path)
        .map_err(|e| NounVerbError::execution_error(e.to_string()))?;
    let params = match &root_node_id {
        Some(id) => serde_json::json!({ "node_id": id }),
        None => serde_json::Value::Null,
    };
    let raw = mesh
        .dispatch_rpc(&instance_id, "max/hookGraph", params)
        .map_err(|e| NounVerbError::execution_error(e.to_string()))?;
    mesh.save_to_file(&state_path)
        .map_err(|e| NounVerbError::execution_error(e.to_string()))?;
    Ok(HookGraphResult {
        instance_id,
        root_node_id,
        raw,
    })
}

#[derive(Serialize)]
pub struct PropagateResult {
    pub instance_id: String,
    pub chain_or_hook_id: String,
    pub raw: serde_json::Value,
}

/// Dispatch the `max/propagate` RPC to propagate events through a hook chain.
#[verb("propagate")]
pub fn propagate(
    instance_id: String,
    chain_or_hook_id: String,
) -> clap_noun_verb::Result<PropagateResult> {
    let state_path = crate::nouns::get_state_path();
    let mut mesh = AutonomicMesh::load_from_file(&state_path)
        .map_err(|e| NounVerbError::execution_error(e.to_string()))?;
    let raw = mesh
        .dispatch_rpc(
            &instance_id,
            "max/propagate",
            serde_json::json!(chain_or_hook_id),
        )
        .map_err(|e| NounVerbError::execution_error(e.to_string()))?;
    mesh.save_to_file(&state_path)
        .map_err(|e| NounVerbError::execution_error(e.to_string()))?;
    Ok(PropagateResult {
        instance_id,
        chain_or_hook_id,
        raw,
    })
}

#[derive(Serialize)]
pub struct ChainResult {
    pub instance_id: String,
    pub chain_id: Option<String>,
    pub raw: serde_json::Value,
}

/// Dispatch the `max/chain` RPC to query or trigger a named hook chain.
#[verb("chain")]
pub fn chain(instance_id: String, chain_id: Option<String>) -> clap_noun_verb::Result<ChainResult> {
    let state_path = crate::nouns::get_state_path();
    let mut mesh = AutonomicMesh::load_from_file(&state_path)
        .map_err(|e| NounVerbError::execution_error(e.to_string()))?;
    let params = match &chain_id {
        Some(id) => serde_json::json!({ "chain_id": id }),
        None => serde_json::Value::Null,
    };
    let raw = mesh
        .dispatch_rpc(&instance_id, "max/chain", params)
        .map_err(|e| NounVerbError::execution_error(e.to_string()))?;
    mesh.save_to_file(&state_path)
        .map_err(|e| NounVerbError::execution_error(e.to_string()))?;
    Ok(ChainResult {
        instance_id,
        chain_id,
        raw,
    })
}

// ==============================================================================
// 4. Tests
// ==============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use lsp_max_runtime::{AutonomicMesh, LspInstance};

    fn make_temp_mesh() -> (tempfile::NamedTempFile, HookService) {
        let mut mesh = AutonomicMesh::new();
        mesh.add_instance(LspInstance::new("inst-1"));
        let f = tempfile::NamedTempFile::new().unwrap();
        mesh.save_to_file(f.path().to_str().unwrap()).unwrap();
        let svc = HookService {
            state_path: f.path().to_str().unwrap().to_string(),
        };
        (f, svc)
    }

    // --- list ---

    #[test]
    fn list_returns_ok_for_valid_mesh() {
        let (_f, svc) = make_temp_mesh();
        assert!(svc.list().is_ok());
    }

    #[test]
    fn list_returns_vec_of_strings() {
        let (_f, svc) = make_temp_mesh();
        // All hook names must be non-empty strings.
        for name in svc.list().unwrap() {
            assert!(!name.is_empty(), "hook name must not be empty");
        }
    }

    #[test]
    fn list_fails_on_missing_state_file() {
        let svc = HookService {
            state_path: "/tmp/nonexistent-hook-test-file.json".to_string(),
        };
        assert!(svc.list().is_err());
    }

    // --- RPC verbs (integration path via isolated env) ---

    fn with_mesh_state<F: FnOnce()>(f: F) {
        let _lock = crate::nouns::TEST_ENV_LOCK
            .lock()
            .unwrap_or_else(|p| p.into_inner());
        let mut mesh = AutonomicMesh::new();
        mesh.add_instance(LspInstance::new("inst-1"));
        let tmpf = tempfile::NamedTempFile::new().unwrap();
        let path = tmpf.path().to_str().unwrap().to_string();
        mesh.save_to_file(&path).unwrap();
        let prev = std::env::var("LSP_MAX_STATE_PATH").ok();
        // SAFETY: under TEST_ENV_LOCK, single-threaded env mutation.
        unsafe { std::env::set_var("LSP_MAX_STATE_PATH", &path) };
        f();
        // SAFETY: restoring env under TEST_ENV_LOCK.
        unsafe {
            match prev {
                Some(v) => std::env::set_var("LSP_MAX_STATE_PATH", v),
                None => std::env::remove_var("LSP_MAX_STATE_PATH"),
            }
        }
    }

    #[test]
    fn hook_rpc_returns_ok_for_known_instance() {
        with_mesh_state(|| {
            let result = hook_rpc("inst-1".to_string(), None);
            assert!(result.is_ok(), "hook_rpc for known instance must return Ok");
        });
    }

    #[test]
    fn hook_rpc_result_echoes_instance_id() {
        with_mesh_state(|| {
            let res = hook_rpc("inst-1".to_string(), Some("my-hook".to_string())).unwrap();
            assert_eq!(res.instance_id, "inst-1");
            assert_eq!(res.hook_id.as_deref(), Some("my-hook"));
        });
    }

    #[test]
    fn hook_graph_returns_ok_for_known_instance() {
        with_mesh_state(|| {
            let result = hook_graph("inst-1".to_string(), None);
            assert!(result.is_ok(), "hook_graph for known instance must return Ok");
        });
    }

    #[test]
    fn propagate_returns_ok_for_known_instance() {
        with_mesh_state(|| {
            let result = propagate("inst-1".to_string(), "chain-a".to_string());
            assert!(result.is_ok(), "propagate for known instance must return Ok");
        });
    }

    #[test]
    fn chain_returns_ok_for_known_instance() {
        with_mesh_state(|| {
            let result = chain("inst-1".to_string(), None);
            assert!(result.is_ok(), "chain for known instance must return Ok");
        });
    }

    #[test]
    fn hook_rpc_fails_on_missing_state_file() {
        let _lock = crate::nouns::TEST_ENV_LOCK
            .lock()
            .unwrap_or_else(|p| p.into_inner());
        let prev = std::env::var("LSP_MAX_STATE_PATH").ok();
        // SAFETY: under TEST_ENV_LOCK.
        unsafe {
            std::env::set_var(
                "LSP_MAX_STATE_PATH",
                "/tmp/nonexistent-hook-rpc-test.json",
            )
        };
        let result = hook_rpc("inst-1".to_string(), None);
        // SAFETY: restoring env under TEST_ENV_LOCK.
        unsafe {
            match prev {
                Some(v) => std::env::set_var("LSP_MAX_STATE_PATH", v),
                None => std::env::remove_var("LSP_MAX_STATE_PATH"),
            }
        }
        assert!(result.is_err(), "missing state file must produce Err");
    }
}
