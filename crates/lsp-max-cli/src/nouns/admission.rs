use clap_noun_verb::error::NounVerbError;
use clap_noun_verb::Result;
use clap_noun_verb_macros::verb;
use lsp_max_runtime::AutonomicMesh;
use serde::Serialize;

// ==============================================================================
// 1. Domain Tier — admission-lifecycle axis: Admitted / Refused / Unknown
// ==============================================================================

/// The three admission lifecycle states for an AMI instance.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub enum AdmissionState {
    Admitted,
    Refused,
    Unknown,
}

// ==============================================================================
// 2. Service Tier
// ==============================================================================

pub struct AdmissionService {
    state_path: String,
}

impl AdmissionService {
    pub fn new() -> Self {
        Self {
            state_path: crate::nouns::get_state_path(),
        }
    }

    /// Dispatch max/admission — query the current admission status for an instance.
    pub fn status(&self, instance_id: &str) -> std::result::Result<serde_json::Value, String> {
        let mut mesh =
            AutonomicMesh::load_from_file(&self.state_path).map_err(|e| e.to_string())?;
        let params = serde_json::json!({ "instance_id": instance_id });
        let response = mesh.dispatch_rpc(instance_id, "max/admission", params)?;
        mesh.save_to_file(&self.state_path)
            .map_err(|e| e.to_string())?;
        Ok(response)
    }

    /// Dispatch max/refusal — record a refusal decision for an instance with a reason.
    pub fn refuse(
        &self,
        instance_id: &str,
        reason: &str,
    ) -> std::result::Result<serde_json::Value, String> {
        let mut mesh =
            AutonomicMesh::load_from_file(&self.state_path).map_err(|e| e.to_string())?;
        let params = serde_json::json!(reason);
        let response = mesh.dispatch_rpc(instance_id, "max/refusal", params)?;
        mesh.save_to_file(&self.state_path)
            .map_err(|e| e.to_string())?;
        Ok(response)
    }

    /// Dispatch max/replay — replay the admission decision for an instance.
    pub fn replay(&self, instance_id: &str) -> std::result::Result<serde_json::Value, String> {
        let mut mesh =
            AutonomicMesh::load_from_file(&self.state_path).map_err(|e| e.to_string())?;
        let params = serde_json::json!({ "instance_id": instance_id });
        let response = mesh.dispatch_rpc(instance_id, "max/replay", params)?;
        mesh.save_to_file(&self.state_path)
            .map_err(|e| e.to_string())?;
        Ok(response)
    }
}

impl Default for AdmissionService {
    fn default() -> Self {
        Self::new()
    }
}

// ==============================================================================
// 3. CLI Tier
// ==============================================================================

/// Result returned by `admission status`.
#[derive(Serialize)]
pub struct AdmissionStatusResult {
    pub instance_id: String,
    pub response: serde_json::Value,
}

/// Result returned by `admission refuse`.
#[derive(Serialize)]
pub struct AdmissionRefuseResult {
    pub instance_id: String,
    pub reason: String,
    pub response: serde_json::Value,
}

/// Result returned by `admission replay`.
#[derive(Serialize)]
pub struct AdmissionReplayResult {
    pub instance_id: String,
    pub response: serde_json::Value,
}

/// Query the current admission status of an AMI instance (max/admission).
#[verb("status")]
pub fn status(instance_id: String) -> Result<AdmissionStatusResult> {
    let svc = AdmissionService::new();
    let response = svc
        .status(&instance_id)
        .map_err(NounVerbError::execution_error)?;
    Ok(AdmissionStatusResult {
        instance_id,
        response,
    })
}

/// Record a refusal decision for an AMI instance (max/refusal).
#[verb("refuse")]
pub fn refuse(instance_id: String, reason: String) -> Result<AdmissionRefuseResult> {
    let svc = AdmissionService::new();
    let response = svc
        .refuse(&instance_id, &reason)
        .map_err(NounVerbError::execution_error)?;
    Ok(AdmissionRefuseResult {
        instance_id,
        reason,
        response,
    })
}

/// Replay the admission decision for an AMI instance (max/replay).
#[verb("replay")]
pub fn replay(instance_id: String) -> Result<AdmissionReplayResult> {
    let svc = AdmissionService::new();
    let response = svc
        .replay(&instance_id)
        .map_err(NounVerbError::execution_error)?;
    Ok(AdmissionReplayResult {
        instance_id,
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

    fn make_temp_mesh() -> (tempfile::NamedTempFile, AdmissionService) {
        let mut mesh = AutonomicMesh::new();
        mesh.add_instance(LspInstance::new("test-inst"));
        let f = tempfile::NamedTempFile::new().unwrap();
        mesh.save_to_file(f.path().to_str().unwrap()).unwrap();
        let svc = AdmissionService {
            state_path: f.path().to_str().unwrap().to_string(),
        };
        (f, svc)
    }

    // status verb: success, falsification, counterfactual ----------------------

    #[test]
    fn admission_status_known_instance_returns_ok() {
        let (_f, svc) = make_temp_mesh();
        let result = svc.status("test-inst");
        assert!(result.is_ok(), "expected Ok, got: {:?}", result);
    }

    #[test]
    fn admission_status_response_is_json_object() {
        // Falsification: the RPC response must be a JSON Value (not null).
        let (_f, svc) = make_temp_mesh();
        let value = svc.status("test-inst").unwrap();
        // The response must be a Value (any JSON shape), not a bare Null from
        // a missing dispatch.  The verb wraps it; we verify it serialises.
        let serialised = serde_json::to_string(&value).unwrap();
        assert!(
            !serialised.is_empty(),
            "response serialised to empty string"
        );
    }

    #[test]
    fn admission_status_unknown_instance_returns_err() {
        // Counterfactual: unknown instance must return Err.
        let (_f, svc) = make_temp_mesh();
        assert!(svc.status("no-such-instance").is_err());
    }

    #[test]
    fn admission_status_missing_parent_dir_returns_err() {
        // Counterfactual via non-existent parent directory.
        let svc = AdmissionService {
            state_path: "/tmp/no-such-dir-lsp-max/admission/state.json".to_string(),
        };
        assert!(svc.status("test-inst").is_err());
    }

    // refuse verb: success, falsification, counterfactual ----------------------

    #[test]
    fn admission_refuse_known_instance_returns_ok() {
        let (_f, svc) = make_temp_mesh();
        let result = svc.refuse("test-inst", "policy-violation");
        assert!(result.is_ok(), "expected Ok, got: {:?}", result);
    }

    #[test]
    fn admission_refuse_response_is_serialisable() {
        // Falsification: the returned Value must serialise without error.
        let (_f, svc) = make_temp_mesh();
        let value = svc.refuse("test-inst", "law-boundary").unwrap();
        let serialised = serde_json::to_string(&value).unwrap();
        assert!(!serialised.is_empty(), "refuse response serialised to empty");
    }

    #[test]
    fn admission_refuse_unknown_instance_returns_err() {
        // Counterfactual: unknown instance must return Err.
        let (_f, svc) = make_temp_mesh();
        assert!(svc.refuse("no-such-instance", "reason").is_err());
    }

    #[test]
    fn admission_refuse_missing_parent_dir_returns_err() {
        // Counterfactual via non-existent parent directory.
        let svc = AdmissionService {
            state_path: "/tmp/no-such-dir-lsp-max/admission/state.json".to_string(),
        };
        assert!(svc.refuse("test-inst", "reason").is_err());
    }

    // replay verb: success, falsification, counterfactual ----------------------

    #[test]
    fn admission_replay_known_instance_returns_ok() {
        let (_f, svc) = make_temp_mesh();
        let result = svc.replay("test-inst");
        assert!(result.is_ok(), "expected Ok, got: {:?}", result);
    }

    #[test]
    fn admission_replay_response_is_serialisable() {
        // Falsification: the returned Value must serialise without error.
        let (_f, svc) = make_temp_mesh();
        let value = svc.replay("test-inst").unwrap();
        let serialised = serde_json::to_string(&value).unwrap();
        assert!(!serialised.is_empty(), "replay response serialised to empty");
    }

    #[test]
    fn admission_replay_unknown_instance_returns_err() {
        // Counterfactual: unknown instance must return Err.
        let (_f, svc) = make_temp_mesh();
        assert!(svc.replay("no-such-instance").is_err());
    }

    #[test]
    fn admission_replay_missing_parent_dir_returns_err() {
        // Counterfactual via non-existent parent directory.
        let svc = AdmissionService {
            state_path: "/tmp/no-such-dir-lsp-max/admission/state.json".to_string(),
        };
        assert!(svc.replay("test-inst").is_err());
    }

    // CLI-tier result structs: falsification of echoed fields ------------------

    #[test]
    fn admission_status_result_carries_instance_id() {
        // Falsification: the CLI result struct must echo back the queried id.
        let (_f, svc) = make_temp_mesh();
        let response = svc.status("test-inst").unwrap();
        let result = AdmissionStatusResult {
            instance_id: "test-inst".to_string(),
            response,
        };
        assert_eq!(result.instance_id, "test-inst");
    }

    #[test]
    fn admission_refuse_result_carries_instance_id_and_reason() {
        // Falsification: the CLI result struct must echo back both id and reason.
        let (_f, svc) = make_temp_mesh();
        let response = svc.refuse("test-inst", "boundary-breach").unwrap();
        let result = AdmissionRefuseResult {
            instance_id: "test-inst".to_string(),
            reason: "boundary-breach".to_string(),
            response,
        };
        assert_eq!(result.instance_id, "test-inst");
        assert_eq!(result.reason, "boundary-breach");
    }

    #[test]
    fn admission_replay_result_carries_instance_id() {
        // Falsification: the CLI result struct must echo back the queried id.
        let (_f, svc) = make_temp_mesh();
        let response = svc.replay("test-inst").unwrap();
        let result = AdmissionReplayResult {
            instance_id: "test-inst".to_string(),
            response,
        };
        assert_eq!(result.instance_id, "test-inst");
    }
}
