use clap_noun_verb::error::NounVerbError;
use clap_noun_verb::Result;
use clap_noun_verb_macros::verb;
use lsp_max_runtime::AutonomicMesh;
use serde::Serialize;

// ==============================================================================
// 1. Domain Tier
// ==============================================================================

// HookEvent is re-exported from lsp_max_runtime and derives Serialize.

// ==============================================================================
// 2. Service Tier
// ==============================================================================

/// Service for querying the mesh event log.
pub struct EventService {
    state_path: String,
}

impl EventService {
    pub fn new() -> Self {
        Self {
            state_path: crate::nouns::get_state_path(),
        }
    }

    pub fn list(
        &self,
        instance_filter: Option<&str>,
    ) -> std::result::Result<Vec<serde_json::Value>, String> {
        let mesh = AutonomicMesh::load_from_file(&self.state_path).map_err(|e| e.to_string())?;

        let events: Vec<serde_json::Value> = mesh
            .event_log
            .iter()
            .filter(|event| {
                if let Some(id) = instance_filter {
                    // Serialize and check for the instance_id field value
                    if let Ok(v) = serde_json::to_value(event) {
                        // Walk one level of object fields looking for instance_id
                        if let Some(obj) = v.as_object() {
                            return obj.values().any(|variant| {
                                variant
                                    .as_object()
                                    .and_then(|fields| fields.get("instance_id"))
                                    .and_then(|f| f.as_str())
                                    .map(|s| s == id)
                                    .unwrap_or(false)
                            });
                        }
                    }
                    false
                } else {
                    true
                }
            })
            .map(|event| serde_json::to_value(event).unwrap_or(serde_json::Value::Null))
            .collect();

        Ok(events)
    }
}

impl Default for EventService {
    fn default() -> Self {
        Self::new()
    }
}

// ==============================================================================
// 3. CLI Tier
// ==============================================================================

#[derive(Serialize)]
pub struct EventListResult {
    pub events: Vec<serde_json::Value>,
    pub count: usize,
}

/// List events from the mesh event log, optionally filtered by instance id.
#[verb("list")]
pub fn list(instance: Option<String>) -> Result<EventListResult> {
    let service = EventService::new();
    let events = service
        .list(instance.as_deref())
        .map_err(NounVerbError::execution_error)?;
    let count = events.len();
    Ok(EventListResult { events, count })
}

// ==============================================================================
// 4. Tests
// ==============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use lsp_max_runtime::{AutonomicMesh, LspInstance};

    fn make_temp_mesh() -> (tempfile::NamedTempFile, EventService) {
        let mut mesh = AutonomicMesh::new();
        mesh.add_instance(LspInstance::new("inst-1"));
        let f = tempfile::NamedTempFile::new().unwrap();
        mesh.save_to_file(f.path().to_str().unwrap()).unwrap();
        let svc = EventService {
            state_path: f.path().to_str().unwrap().to_string(),
        };
        (f, svc)
    }

    // --- list ---

    #[test]
    fn list_no_filter_returns_ok() {
        let (_f, svc) = make_temp_mesh();
        assert!(svc.list(None).is_ok());
    }

    #[test]
    fn list_new_mesh_has_empty_event_log() {
        let (_f, svc) = make_temp_mesh();
        let events = svc.list(None).unwrap();
        assert!(events.is_empty(), "fresh mesh must have an empty event log");
    }

    #[test]
    fn list_filter_for_nonexistent_instance_returns_empty_not_err() {
        let (_f, svc) = make_temp_mesh();
        let result = svc.list(Some("no-such-instance"));
        // Unknown filter target returns Ok([]) — filter narrows, it does not error.
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }

    #[test]
    fn list_fails_on_missing_state_file() {
        let svc = EventService {
            state_path: "/tmp/nonexistent-event-test.json".to_string(),
        };
        assert!(svc.list(None).is_err());
    }
}
