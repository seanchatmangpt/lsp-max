use clap_noun_verb::Result;
use clap_noun_verb_macros::verb;
use lsp_max_protocol::InstanceId;
use lsp_max_runtime::{AutonomicMesh, MeshAction, PolicyState};
use serde::Serialize;

// ==============================================================================
// 1. Domain Tier
// ==============================================================================

/// Represents a patch payload for modifying state.
#[derive(Debug, Clone)]
pub struct StatePatch {
    pub status: Option<String>,
}

// ==============================================================================
// 2. Service Tier
// ==============================================================================

/// Service for managing AutonomicMesh lifecycle and operations.
pub struct StateService {
    state_path: String,
}

impl StateService {
    pub fn new() -> Self {
        Self {
            state_path: crate::nouns::get_state_path(),
        }
    }

    pub fn dump(&self, state_id: &str) -> std::result::Result<String, String> {
        let mesh = AutonomicMesh::load_from_file(&self.state_path).map_err(|e| e.to_string())?;

        let state = if state_id == "all" || state_id.is_empty() {
            serde_json::to_string_pretty(&mesh.to_state()).map_err(|e| e.to_string())?
        } else {
            let inst = mesh
                .instances
                .get(state_id)
                .ok_or_else(|| format!("Instance not found: {}", state_id))?;
            serde_json::to_string_pretty(inst).map_err(|e| e.to_string())?
        };

        Ok(state)
    }

    pub fn restore(&self, state_id: &str, _revision: u64) -> bool {
        if let Ok(mut mesh) = AutonomicMesh::load_from_file(&self.state_path) {
            if let Some(inst) = mesh.instances.get_mut(state_id) {
                inst.diagnostics.clear();
                inst.receipts.clear();
                inst.policy_state = Some(lsp_max_runtime::PolicyState::Operational);
                return mesh.save_to_file(&self.state_path).is_ok();
            }
        }
        false
    }

    pub fn verify(&self, state_id: &str) -> bool {
        if let Ok(mesh) = AutonomicMesh::load_from_file(&self.state_path) {
            if let Some(inst) = mesh.instances.get(state_id) {
                return inst.conformance_score() > 50.0;
            }
        }
        false
    }

    pub fn patch(&self, state_id: &str, patch: StatePatch) -> std::result::Result<bool, String> {
        let mut mesh =
            AutonomicMesh::load_from_file(&self.state_path).map_err(|e| e.to_string())?;

        if let Some(status) = patch.status {
            let cmd = format!("patch {} {}", state_id, status);
            mesh.run_command(&cmd)?;
            mesh.save_to_file(&self.state_path)
                .map_err(|e| e.to_string())?;
            Ok(true)
        } else {
            Ok(false)
        }
    }
}

impl Default for StateService {
    fn default() -> Self {
        Self::new()
    }
}

// ==============================================================================
// 3. CLI Tier
// ==============================================================================

#[derive(Serialize)]
pub struct DumpResult {
    pub state: String,
}

#[verb("dump")]
pub fn dump(state_id: String) -> Result<DumpResult> {
    let service = StateService::new();
    let state = service
        .dump(&state_id)
        .map_err(clap_noun_verb::error::NounVerbError::execution_error)?;
    Ok(DumpResult { state })
}

#[derive(Serialize)]
pub struct RestoreResult {
    pub success: bool,
}

#[verb("restore")]
pub fn restore(state_id: String, revision: u64) -> Result<RestoreResult> {
    let service = StateService::new();
    let success = service.restore(&state_id, revision);
    Ok(RestoreResult { success })
}

#[derive(Serialize)]
pub struct VerifyResult {
    pub state_id: String,
    pub is_valid: bool,
}

#[verb("verify")]
pub fn verify(state_id: String) -> Result<VerifyResult> {
    let service = StateService::new();
    let is_valid = service.verify(&state_id);
    Ok(VerifyResult { state_id, is_valid })
}

#[derive(Serialize)]
pub struct PatchResult {
    pub success: bool,
}

#[verb("patch")]
pub fn patch(state_id: String, status_override: Option<String>) -> Result<PatchResult> {
    let service = StateService::new();

    let state_patch = StatePatch {
        status: status_override,
    };

    let success = service
        .patch(&state_id, state_patch)
        .map_err(clap_noun_verb::error::NounVerbError::execution_error)?;
    Ok(PatchResult { success })
}

#[derive(Serialize)]
pub struct StateResult {
    pub instance_id: String,
    pub phase: String,
    pub conformance_score: f64,
    pub policy_state: Option<String>,
    pub diagnostics_count: usize,
    pub receipts_count: usize,
}

#[verb("state")]
pub fn state(instance_id: String) -> Result<StateResult> {
    let mesh = AutonomicMesh::load_from_file(&crate::nouns::get_state_path())
        .map_err(|e| clap_noun_verb::error::NounVerbError::execution_error(e.to_string()))?;

    let inst = mesh.instances.get(&instance_id).ok_or_else(|| {
        clap_noun_verb::error::NounVerbError::execution_error(format!(
            "Instance not found: {}",
            instance_id
        ))
    })?;

    let policy_state = inst.policy_state.as_ref().map(|p| format!("{:?}", p));

    Ok(StateResult {
        instance_id: inst.id.clone(),
        phase: inst.phase.to_string(),
        conformance_score: inst.conformance_score(),
        policy_state,
        diagnostics_count: inst.diagnostics.len(),
        receipts_count: inst.receipts.len(),
    })
}

#[derive(Serialize)]
pub struct TransitionResult {
    pub instance_id: String,
    pub new_state: String,
    pub success: bool,
}

#[verb("transition")]
pub fn transition(instance_id: String, new_state: String) -> Result<TransitionResult> {
    let policy_state: PolicyState = new_state
        .parse()
        .map_err(|e: String| clap_noun_verb::error::NounVerbError::execution_error(e))?;

    let mut mesh = AutonomicMesh::load_from_file(&crate::nouns::get_state_path())
        .map_err(|e| clap_noun_verb::error::NounVerbError::execution_error(e.to_string()))?;

    mesh.execute_action(MeshAction::TransitionPolicyState {
        instance_id: InstanceId::from(instance_id.clone()),
        new_state: policy_state,
    });

    mesh.save_to_file(&crate::nouns::get_state_path())
        .map_err(|e| clap_noun_verb::error::NounVerbError::execution_error(e.to_string()))?;

    Ok(TransitionResult {
        instance_id,
        new_state,
        success: true,
    })
}

#[derive(Serialize)]
pub struct ActionResult {
    pub instance_id: String,
    pub action_id: String,
    pub success: bool,
}

#[verb("action")]
pub fn action(instance_id: String, action_id: String, description: String) -> Result<ActionResult> {
    let mut mesh = AutonomicMesh::load_from_file(&crate::nouns::get_state_path())
        .map_err(|e| clap_noun_verb::error::NounVerbError::execution_error(e.to_string()))?;

    mesh.execute_action(MeshAction::ExecuteBoundedAction {
        instance_id: InstanceId::from(instance_id.clone()),
        action_id: action_id.clone(),
        description,
    });

    mesh.save_to_file(&crate::nouns::get_state_path())
        .map_err(|e| clap_noun_verb::error::NounVerbError::execution_error(e.to_string()))?;

    Ok(ActionResult {
        instance_id,
        action_id,
        success: true,
    })
}

#[derive(Serialize)]
pub struct LawfulTransitionResult {
    pub instance_id: String,
    pub from_state: String,
    pub to_state: String,
    pub lawful: bool,
    pub response: serde_json::Value,
}

#[verb("lawful-transition")]
pub fn lawful_transition(
    instance_id: String,
    from_state: String,
    to_state: String,
) -> Result<LawfulTransitionResult> {
    let mut mesh = AutonomicMesh::load_from_file(&crate::nouns::get_state_path())
        .map_err(|e| clap_noun_verb::error::NounVerbError::execution_error(e.to_string()))?;

    let params = serde_json::json!({
        "from_state": from_state,
        "to_state": to_state,
    });

    let response = mesh
        .dispatch_rpc(&instance_id, "max/lawfulTransition", params)
        .map_err(clap_noun_verb::error::NounVerbError::execution_error)?;

    mesh.save_to_file(&crate::nouns::get_state_path())
        .map_err(|e| clap_noun_verb::error::NounVerbError::execution_error(e.to_string()))?;

    let lawful = response
        .get("lawful")
        .and_then(|v| v.as_bool())
        .unwrap_or(true);

    Ok(LawfulTransitionResult {
        instance_id,
        from_state,
        to_state,
        lawful,
        response,
    })
}

/// Result type for the  verb (RPC-backed).
#[derive(Serialize)]
pub struct DumpRpcResult {
    pub instance_id: String,
    pub raw: serde_json::Value,
}

#[verb("dump-rpc")]
pub fn dump_rpc(instance_id: String) -> Result<DumpRpcResult> {
    let state_path = crate::nouns::get_state_path();
    let mut mesh = AutonomicMesh::load_from_file(&state_path)
        .map_err(|e| clap_noun_verb::error::NounVerbError::execution_error(e.to_string()))?;
    let params = serde_json::json!({ "instance_id": instance_id });
    let raw = mesh
        .dispatch_rpc(&instance_id, "max/dumpState", params)
        .map_err(clap_noun_verb::error::NounVerbError::execution_error)?;
    mesh.save_to_file(&state_path)
        .map_err(|e| clap_noun_verb::error::NounVerbError::execution_error(e.to_string()))?;
    Ok(DumpRpcResult { instance_id, raw })
}

/// Result type for the  verb (RPC-backed).
#[derive(Serialize)]
pub struct RestoreRpcResult {
    pub instance_id: String,
    pub revision: u64,
    pub raw: serde_json::Value,
}

#[verb("restore-rpc")]
pub fn restore_rpc(instance_id: String, revision: u64) -> Result<RestoreRpcResult> {
    let state_path = crate::nouns::get_state_path();
    let mut mesh = AutonomicMesh::load_from_file(&state_path)
        .map_err(|e| clap_noun_verb::error::NounVerbError::execution_error(e.to_string()))?;
    let params = serde_json::json!({ "instance_id": instance_id, "revision": revision });
    let raw = mesh
        .dispatch_rpc(&instance_id, "max/restoreState", params)
        .map_err(clap_noun_verb::error::NounVerbError::execution_error)?;
    mesh.save_to_file(&state_path)
        .map_err(|e| clap_noun_verb::error::NounVerbError::execution_error(e.to_string()))?;
    Ok(RestoreRpcResult {
        instance_id,
        revision,
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

    fn make_temp_mesh() -> (tempfile::NamedTempFile, StateService) {
        let mut mesh = AutonomicMesh::new();
        mesh.add_instance(LspInstance::new("test-inst"));
        let f = tempfile::NamedTempFile::new().unwrap();
        mesh.save_to_file(f.path().to_str().unwrap()).unwrap();
        let svc = StateService {
            state_path: f.path().to_str().unwrap().to_string(),
        };
        (f, svc)
    }

    #[test]
    fn dump_all_returns_ok() {
        let (_f, svc) = make_temp_mesh();
        assert!(svc.dump("all").is_ok());
    }

    #[test]
    fn dump_instance_returns_ok() {
        let (_f, svc) = make_temp_mesh();
        assert!(svc.dump("test-inst").is_ok());
    }

    #[test]
    fn dump_unknown_instance_returns_err() {
        let (_f, svc) = make_temp_mesh();
        assert!(svc.dump("no-such").is_err());
    }

    #[test]
    fn restore_known_instance_returns_true() {
        let (_f, svc) = make_temp_mesh();
        assert!(svc.restore("test-inst", 0));
    }

    #[test]
    fn restore_unknown_instance_returns_false() {
        let (_f, svc) = make_temp_mesh();
        assert!(!svc.restore("no-such", 0));
    }

    #[test]
    fn verify_known_instance_returns_bool() {
        let (_f, svc) = make_temp_mesh();
        // just assert it doesn't panic; result depends on conformance_score
        let _ = svc.verify("test-inst");
    }

    #[test]
    fn patch_no_status_returns_ok_false() {
        let (_f, svc) = make_temp_mesh();
        let result = svc.patch("test-inst", StatePatch { status: None });
        assert!(!result.unwrap());
    }
}
