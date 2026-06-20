use clap_noun_verb::error::NounVerbError;
use clap_noun_verb::Result;
use clap_noun_verb_macros::verb;
use lsp_max_runtime::{AutonomicMesh, LspInstance};
use serde::{Deserialize, Serialize};

// ==============================================================================
// 1. Domain Tier
// ==============================================================================

/// Summary of a merge operation: how many instances were imported vs. already present.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MergeSummary {
    pub instances_imported: usize,
    /// Instances that existed in both meshes; the import value overwrote the local one.
    pub instances_merged: usize,
}

// ==============================================================================
// 2. Service Tier
// ==============================================================================

pub struct ImportService {
    state_path: String,
}

impl ImportService {
    pub fn new() -> Self {
        Self {
            state_path: crate::nouns::get_state_path(),
        }
    }

    /// Load a mesh state JSON from `src`, merge instances into the live mesh,
    /// and persist the result.  Import overwrites on instance_id collision.
    /// Returns (instances_imported, instances_merged).
    pub fn merge_state(&self, src: &str) -> std::result::Result<MergeSummary, String> {
        // Parse the import file first so we can report a clean error.
        let raw = std::fs::read_to_string(src).map_err(|e| e.to_string())?;
        let import_state: lsp_max_runtime::AutonomicMeshState =
            serde_json::from_str(&raw).map_err(|e| format!("parse error: {}", e))?;

        // Load (or bootstrap) the live mesh.
        let mut live = AutonomicMesh::load_from_file(&self.state_path).map_err(|e| e.to_string())?;

        let mut instances_imported = 0usize;
        let mut instances_merged = 0usize;

        for (id, inst) in import_state.instances {
            if live.instances.contains_key(&id) {
                instances_merged += 1;
            } else {
                instances_imported += 1;
            }
            live.instances.insert(id, inst);
        }

        live.save_to_file(&self.state_path)
            .map_err(|e| e.to_string())?;

        Ok(MergeSummary {
            instances_imported,
            instances_merged,
        })
    }

    /// Bulk-append diagnostics from a JSON file (array of MaxDiagnostic-compatible objects)
    /// into the instance identified by `instance_id`.
    /// Returns number of diagnostics imported.
    pub fn bulk_import_diagnostics(
        &self,
        src: &str,
        instance_id: &str,
    ) -> std::result::Result<usize, String> {
        let raw = std::fs::read_to_string(src).map_err(|e| e.to_string())?;
        let values: Vec<serde_json::Value> =
            serde_json::from_str(&raw).map_err(|e| format!("parse error: {}", e))?;

        let mut mesh =
            AutonomicMesh::load_from_file(&self.state_path).map_err(|e| e.to_string())?;

        // Ensure the instance exists; create it if absent.
        if !mesh.instances.contains_key(instance_id) {
            mesh.add_instance(LspInstance::new(instance_id));
        }

        let inst = mesh
            .instances
            .get_mut(instance_id)
            .ok_or_else(|| format!("Instance not found: {}", instance_id))?;

        let mut imported = 0usize;
        for v in values {
            // Use serde_json round-trip for flexible deserialization of MaxDiagnostic.
            match serde_json::from_value::<lsp_max_protocol::MaxDiagnostic>(v) {
                Ok(diag) => {
                    inst.add_diagnostic(diag);
                    imported += 1;
                }
                Err(e) => {
                    // Skip malformed entries; a partial import is better than a hard abort.
                    tracing::warn!("import: skipping unrecognised diagnostic entry: {}", e);
                }
            }
        }

        mesh.save_to_file(&self.state_path)
            .map_err(|e| e.to_string())?;

        Ok(imported)
    }
}

impl Default for ImportService {
    fn default() -> Self {
        Self::new()
    }
}

// ==============================================================================
// 3. Verb Tier
// ==============================================================================

#[derive(Debug, Clone, Serialize)]
pub struct ImportStateResult {
    pub src: String,
    pub instances_imported: usize,
    /// Instances that collided with existing ids; import value overwrote local.
    pub instances_merged: usize,
    pub status: String,
}

#[verb("state")]
pub fn state(src: String) -> Result<ImportStateResult> {
    let svc = ImportService::new();
    let summary = svc.merge_state(&src).map_err(NounVerbError::execution_error)?;
    Ok(ImportStateResult {
        src,
        instances_imported: summary.instances_imported,
        instances_merged: summary.instances_merged,
        status: "ADMITTED".to_string(),
    })
}

#[derive(Debug, Clone, Serialize)]
pub struct ImportDiagnosticsResult {
    pub src: String,
    pub instance_id: String,
    pub imported_count: usize,
    pub status: String,
}

#[verb("diagnostics")]
pub fn diagnostics(src: String, instance_id: String) -> Result<ImportDiagnosticsResult> {
    let svc = ImportService::new();
    let imported_count = svc
        .bulk_import_diagnostics(&src, &instance_id)
        .map_err(NounVerbError::execution_error)?;
    Ok(ImportDiagnosticsResult {
        src,
        instance_id,
        imported_count,
        status: "ADMITTED".to_string(),
    })
}
