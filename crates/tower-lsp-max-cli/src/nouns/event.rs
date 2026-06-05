use clap_noun_verb::Result;
use clap_noun_verb_macros::verb;
use serde::Serialize;
use tower_lsp_max_runtime::AutonomicMesh;

// ==============================================================================
// 1. Domain Tier
// ==============================================================================

// HookEvent is re-exported from tower_lsp_max_runtime and derives Serialize.

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

#[verb("list")]
pub fn list(instance: Option<String>) -> Result<EventListResult> {
    let service = EventService::new();
    let events = service
        .list(instance.as_deref())
        .map_err(clap_noun_verb::error::NounVerbError::execution_error)?;
    let count = events.len();
    Ok(EventListResult { events, count })
}
