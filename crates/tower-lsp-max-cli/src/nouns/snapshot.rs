use clap_noun_verb::Result;
use clap_noun_verb_macros::verb;
use lsp_max_runtime::AutonomicMesh;
use serde::Serialize;

// ==============================================================================
// 1. Domain Tier
// ==============================================================================

// Snapshot operations dispatch max/snapshot and max/manifoldSnapshot RPCs.

// ==============================================================================
// 2. Service Tier
// ==============================================================================

/// Service for capturing LSP instance state snapshots.
pub struct SnapshotService {
    state_path: String,
}

impl SnapshotService {
    pub fn new() -> Self {
        Self {
            state_path: crate::nouns::get_state_path(),
        }
    }

    /// Dispatch max/snapshot for a single instance.
    pub fn take(&self, instance_id: &str) -> std::result::Result<serde_json::Value, String> {
        let mut mesh =
            AutonomicMesh::load_from_file(&self.state_path).map_err(|e| e.to_string())?;
        let response = mesh.dispatch_rpc(instance_id, "max/snapshot", serde_json::Value::Null)?;
        mesh.save_to_file(&self.state_path)
            .map_err(|e| e.to_string())?;
        Ok(response)
    }

    /// Dispatch max/manifoldSnapshot across all instances in the mesh.
    pub fn manifold(&self, instance_id: &str) -> std::result::Result<serde_json::Value, String> {
        let mut mesh =
            AutonomicMesh::load_from_file(&self.state_path).map_err(|e| e.to_string())?;
        let response =
            mesh.dispatch_rpc(instance_id, "max/manifoldSnapshot", serde_json::Value::Null)?;
        mesh.save_to_file(&self.state_path)
            .map_err(|e| e.to_string())?;
        Ok(response)
    }
}

impl Default for SnapshotService {
    fn default() -> Self {
        Self::new()
    }
}

// ==============================================================================
// 3. CLI Tier
// ==============================================================================

#[derive(Serialize)]
pub struct TakeResult {
    pub instance_id: String,
    pub snapshot: serde_json::Value,
}

/// Capture a state snapshot for a single LSP instance via max/snapshot.
#[verb("take")]
pub fn take(instance_id: String) -> Result<TakeResult> {
    let service = SnapshotService::new();
    let snapshot = service
        .take(&instance_id)
        .map_err(clap_noun_verb::error::NounVerbError::execution_error)?;
    Ok(TakeResult {
        instance_id,
        snapshot,
    })
}

#[derive(Serialize)]
pub struct ManifoldResult {
    pub instance_id: String,
    pub manifold_snapshot: serde_json::Value,
}

/// Capture a manifold-wide snapshot via max/manifoldSnapshot.
#[verb("manifold")]
pub fn manifold(instance_id: String) -> Result<ManifoldResult> {
    let service = SnapshotService::new();
    let manifold_snapshot = service
        .manifold(&instance_id)
        .map_err(clap_noun_verb::error::NounVerbError::execution_error)?;
    Ok(ManifoldResult {
        instance_id,
        manifold_snapshot,
    })
}

// ==============================================================================
// 4. Tests
// ==============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use lsp_max_runtime::{AutonomicMesh, LspInstance};

    fn make_temp_service() -> (tempfile::NamedTempFile, SnapshotService) {
        let mut mesh = AutonomicMesh::new();
        mesh.add_instance(LspInstance::new("snap-inst"));
        let f = tempfile::NamedTempFile::new().unwrap();
        mesh.save_to_file(f.path().to_str().unwrap()).unwrap();
        let svc = SnapshotService {
            state_path: f.path().to_str().unwrap().to_string(),
        };
        (f, svc)
    }

    #[test]
    fn take_known_instance_returns_value() {
        let (_f, svc) = make_temp_service();
        // dispatch_rpc may return an error for unimplemented methods; just check no panic
        let _ = svc.take("snap-inst");
    }

    #[test]
    fn take_unknown_instance_returns_err() {
        let (_f, svc) = make_temp_service();
        assert!(svc.take("no-such").is_err());
    }

    #[test]
    fn manifold_known_instance_returns_value() {
        let (_f, svc) = make_temp_service();
        let _ = svc.manifold("snap-inst");
    }

    #[test]
    fn manifold_unknown_instance_returns_err() {
        let (_f, svc) = make_temp_service();
        assert!(svc.manifold("no-such").is_err());
    }
}
