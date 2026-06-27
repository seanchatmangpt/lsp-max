use clap_noun_verb::error::NounVerbError;
use clap_noun_verb::Result;
use clap_noun_verb_macros::verb;
use lsp_max::max_runtime::{AutonomicMesh, MeshAction, PolicyState};
use lsp_max_protocol::InstanceId;
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
                inst.policy_state = Some(lsp_max::max_runtime::PolicyState::Operational);
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

/// Dump the serialized state of an instance (or all instances when state_id is "all").
#[verb("dump")]
pub fn dump(state_id: String) -> Result<DumpResult> {
    let service = StateService::new();
    let state = service
        .dump(&state_id)
        .map_err(NounVerbError::execution_error)?;
    Ok(DumpResult { state })
}

#[derive(Serialize)]
pub struct RestoreResult {
    pub success: bool,
}

/// Reset an instance to a clean Operational policy state, clearing diagnostics and receipts.
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

/// Return whether an instance's conformance score exceeds 50.
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

/// Apply a status override command to an instance via the run_command interface.
#[verb("patch")]
pub fn patch(state_id: String, status_override: Option<String>) -> Result<PatchResult> {
    let service = StateService::new();

    let state_patch = StatePatch {
        status: status_override,
    };

    let success = service
        .patch(&state_id, state_patch)
        .map_err(NounVerbError::execution_error)?;
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

/// Return phase, conformance score, policy state, and diagnostic/receipt counts for an instance.
#[verb("state")]
pub fn state(instance_id: String) -> Result<StateResult> {
    let mesh = AutonomicMesh::load_from_file(&crate::nouns::get_state_path())
        .map_err(|e| NounVerbError::execution_error(e.to_string()))?;

    let inst = mesh.instances.get(&instance_id).ok_or_else(|| {
        NounVerbError::execution_error(format!("Instance not found: {}", instance_id))
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

/// Transition an instance to the given PolicyState via the TransitionPolicyState mesh action.
#[verb("transition")]
pub fn transition(instance_id: String, new_state: String) -> Result<TransitionResult> {
    let policy_state: PolicyState = new_state
        .parse()
        .map_err(|e: String| NounVerbError::execution_error(e))?;

    let mut mesh = AutonomicMesh::load_from_file(&crate::nouns::get_state_path())
        .map_err(|e| NounVerbError::execution_error(e.to_string()))?;

    mesh.execute_action(MeshAction::TransitionPolicyState {
        instance_id: InstanceId::from(instance_id.clone()),
        new_state: policy_state,
    });

    mesh.save_to_file(&crate::nouns::get_state_path())
        .map_err(|e| NounVerbError::execution_error(e.to_string()))?;

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

/// Execute a bounded action on an instance and persist it to the mesh.
#[verb("action")]
pub fn action(instance_id: String, action_id: String, description: String) -> Result<ActionResult> {
    let mut mesh = AutonomicMesh::load_from_file(&crate::nouns::get_state_path())
        .map_err(|e| NounVerbError::execution_error(e.to_string()))?;

    mesh.execute_action(MeshAction::ExecuteBoundedAction {
        instance_id: InstanceId::from(instance_id.clone()),
        action_id: action_id.clone(),
        description,
    });

    mesh.save_to_file(&crate::nouns::get_state_path())
        .map_err(|e| NounVerbError::execution_error(e.to_string()))?;

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

/// Check whether a state transition is lawful via the `max/lawfulTransition` RPC.
#[verb("lawful-transition")]
pub fn lawful_transition(
    instance_id: String,
    from_state: String,
    to_state: String,
) -> Result<LawfulTransitionResult> {
    let mut mesh = AutonomicMesh::load_from_file(&crate::nouns::get_state_path())
        .map_err(|e| NounVerbError::execution_error(e.to_string()))?;

    // max/lawfulTransition deserialises params as the bare target-phase string
    // and reports admissibility via the `admitted` field.
    let params = serde_json::json!(to_state);

    let response = mesh
        .dispatch_rpc(&instance_id, "max/lawfulTransition", params)
        .map_err(NounVerbError::execution_error)?;

    mesh.save_to_file(&crate::nouns::get_state_path())
        .map_err(|e| NounVerbError::execution_error(e.to_string()))?;

    let lawful = response
        .get("admitted")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

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

/// Dump instance state via the `max/dumpState` RPC.
#[verb("dump-rpc")]
pub fn dump_rpc(instance_id: String) -> Result<DumpRpcResult> {
    let state_path = crate::nouns::get_state_path();
    let mut mesh = AutonomicMesh::load_from_file(&state_path)
        .map_err(|e| NounVerbError::execution_error(e.to_string()))?;
    let params = serde_json::json!({ "instance_id": instance_id });
    let raw = mesh
        .dispatch_rpc(&instance_id, "max/dumpState", params)
        .map_err(NounVerbError::execution_error)?;
    mesh.save_to_file(&state_path)
        .map_err(|e| NounVerbError::execution_error(e.to_string()))?;
    Ok(DumpRpcResult { instance_id, raw })
}

/// Result of restoring mesh state from a snapshot file (RPC-backed).
#[derive(Serialize)]
pub struct RestoreRpcResult {
    /// The instance used as the dispatch target.
    pub instance_id: String,
    pub snapshot_path: String,
    /// Number of instances present after the restore.
    pub restored_instances: usize,
    pub raw: serde_json::Value,
}

/// Restore the mesh from a previously dumped state snapshot via `max/restoreState`.
///
/// `snapshot_path` is a JSON file holding a full `AutonomicMeshState`, as written
/// by `state dump-rpc` (its `raw` field) or by the mesh state file itself.
/// `instance_id` is the dispatch target and must exist in the current mesh.
#[verb("restore-rpc")]
pub fn restore_rpc(instance_id: String, snapshot_path: String) -> Result<RestoreRpcResult> {
    let content = std::fs::read_to_string(&snapshot_path).map_err(|e| {
        NounVerbError::execution_error(format!("SNAPSHOT_NOT_FOUND: {snapshot_path}: {e}"))
    })?;
    let snapshot: serde_json::Value = serde_json::from_str(&content)
        .map_err(|e| NounVerbError::execution_error(format!("Invalid snapshot JSON: {e}")))?;

    let state_path = crate::nouns::get_state_path();
    let mut mesh = AutonomicMesh::load_from_file(&state_path)
        .map_err(|e| NounVerbError::execution_error(e.to_string()))?;
    let raw = mesh
        .dispatch_rpc(&instance_id, "max/restoreState", snapshot)
        .map_err(NounVerbError::execution_error)?;
    mesh.save_to_file(&state_path)
        .map_err(|e| NounVerbError::execution_error(e.to_string()))?;
    let restored_instances = mesh.instances.len();
    Ok(RestoreRpcResult {
        instance_id,
        snapshot_path,
        restored_instances,
        raw,
    })
}

// ==============================================================================
// 4. Tests
// ==============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use lsp_max::max_runtime::{AutonomicMesh, LspInstance};

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

    // --- dump falsification ---

    #[test]
    fn dump_all_output_contains_instances_key() {
        let (_f, svc) = make_temp_mesh();
        let json = svc.dump("all").unwrap();
        assert!(
            json.contains("instances"),
            "dump all must include 'instances' key"
        );
    }

    // --- verify falsification ---

    #[test]
    fn verify_unknown_instance_returns_false() {
        let (_f, svc) = make_temp_mesh();
        // Unknown instance → mesh.get returns None → false
        assert!(!svc.verify("no-such"));
    }

    // --- RPC verb tests via isolated env ---

    fn with_mesh_state<F: FnOnce()>(f: F) {
        let _lock = crate::nouns::TEST_ENV_LOCK
            .lock()
            .unwrap_or_else(|p| p.into_inner());
        let mut mesh = AutonomicMesh::new();
        mesh.add_instance(LspInstance::new("test-inst"));
        let tmpf = tempfile::NamedTempFile::new().unwrap();
        let path = tmpf.path().to_str().unwrap().to_string();
        mesh.save_to_file(&path).unwrap();
        let prev = std::env::var("LSP_MAX_STATE_PATH").ok();
        // SAFETY: under TEST_ENV_LOCK.
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
    fn transition_to_clarification_requested_returns_ok() {
        with_mesh_state(|| {
            // ClarificationRequested is a real PolicyState variant; "Degraded" is not.
            let result = transition(
                "test-inst".to_string(),
                "ClarificationRequested".to_string(),
            );
            assert!(
                result.is_ok(),
                "transition to a valid PolicyState must return Ok"
            );
            let r = result.unwrap();
            assert_eq!(r.instance_id, "test-inst");
            assert_eq!(r.new_state, "ClarificationRequested");
            assert!(r.success);
        });
    }

    #[test]
    fn transition_invalid_state_name_returns_err() {
        with_mesh_state(|| {
            let result = transition("test-inst".to_string(), "NotARealState".to_string());
            assert!(result.is_err(), "invalid state name must return Err");
        });
    }

    #[test]
    fn action_records_action_and_returns_ok() {
        with_mesh_state(|| {
            let result = action(
                "test-inst".to_string(),
                "act-001".to_string(),
                "apply remediation".to_string(),
            );
            assert!(result.is_ok());
            let r = result.unwrap();
            assert_eq!(r.instance_id, "test-inst");
            assert_eq!(r.action_id, "act-001");
            assert!(r.success);
        });
    }

    #[test]
    fn lawful_transition_returns_ok() {
        with_mesh_state(|| {
            let result = lawful_transition(
                "test-inst".to_string(),
                "Operational".to_string(),
                "Degraded".to_string(),
            );
            assert!(result.is_ok());
            let r = result.unwrap();
            assert_eq!(r.from_state, "Operational");
            assert_eq!(r.to_state, "Degraded");
        });
    }

    #[test]
    fn dump_rpc_returns_ok() {
        with_mesh_state(|| {
            assert!(dump_rpc("test-inst".to_string()).is_ok());
        });
    }

    #[test]
    fn dump_rpc_result_carries_instance_id() {
        with_mesh_state(|| {
            let res = dump_rpc("test-inst".to_string()).unwrap();
            assert_eq!(res.instance_id, "test-inst");
        });
    }

    #[test]
    fn restore_rpc_roundtrips_dumped_state() {
        with_mesh_state(|| {
            // Dump the live mesh state, persist it, then restore from that file.
            let dumped = dump_rpc("test-inst".to_string()).unwrap();
            let snap = tempfile::NamedTempFile::new().unwrap();
            std::fs::write(snap.path(), serde_json::to_string(&dumped.raw).unwrap()).unwrap();
            let res = restore_rpc(
                "test-inst".to_string(),
                snap.path().to_str().unwrap().to_string(),
            )
            .unwrap();
            assert_eq!(res.instance_id, "test-inst");
            assert!(res.restored_instances >= 1);
        });
    }

    #[test]
    fn restore_rpc_missing_snapshot_returns_err() {
        with_mesh_state(|| {
            let res = restore_rpc(
                "test-inst".to_string(),
                "/tmp/no-such-dir-lsp-max/snap.json".to_string(),
            );
            assert!(res.is_err());
        });
    }
}
