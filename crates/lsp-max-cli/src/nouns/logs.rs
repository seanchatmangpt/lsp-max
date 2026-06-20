use clap_noun_verb::error::NounVerbError;
use clap_noun_verb::Result;
use clap_noun_verb_macros::verb;
use lsp_max_runtime::AutonomicMesh;
use lsp_max_runtime::HookEvent;
use serde::Serialize;
use std::collections::HashMap;

// ==============================================================================
// 1. Domain Tier
// ==============================================================================

/// Normalized view of a `HookEvent` variant as a flat log record.
/// The `HookEvent` enum is the canonical storage type; this struct is
/// the CLI-facing projection used for filtering and export.
#[derive(Debug, Clone, Serialize)]
pub struct EventLogEntry {
    pub event_type: String,
    pub instance_id: String,
    /// Monotonic sequence index within the current event log snapshot.
    pub timestamp: String,
    pub details: Option<String>,
}

impl EventLogEntry {
    /// Project a `HookEvent` variant into a flat `EventLogEntry`.
    /// The `seq` argument provides a stable ordering token in the absence of
    /// wall-clock timestamps on individual events.
    pub fn from_hook_event(event: &HookEvent, seq: usize) -> Self {
        match event {
            HookEvent::StateTransition {
                instance_id,
                from_phase,
                to_phase,
            } => EventLogEntry {
                event_type: "StateTransition".into(),
                instance_id: instance_id.to_string(),
                timestamp: seq.to_string(),
                details: Some(format!("{} -> {}", from_phase, to_phase)),
            },
            HookEvent::DiagnosticEmitted {
                instance_id,
                diagnostic,
            } => EventLogEntry {
                event_type: "DiagnosticEmitted".into(),
                instance_id: instance_id.to_string(),
                timestamp: seq.to_string(),
                details: Some(diagnostic.lsp.message.clone()),
            },
            HookEvent::DiagnosticCleared {
                instance_id,
                diagnostic_id,
            } => EventLogEntry {
                event_type: "DiagnosticCleared".into(),
                instance_id: instance_id.to_string(),
                timestamp: seq.to_string(),
                details: Some(diagnostic_id.clone()),
            },
            HookEvent::ReceiptEmitted {
                instance_id,
                receipt,
            } => EventLogEntry {
                event_type: "ReceiptEmitted".into(),
                instance_id: instance_id.to_string(),
                timestamp: seq.to_string(),
                details: Some(receipt.receipt_id.clone()),
            },
            HookEvent::PolicyStateChanged {
                instance_id,
                from_state,
                to_state,
            } => EventLogEntry {
                event_type: "PolicyStateChanged".into(),
                instance_id: instance_id.to_string(),
                timestamp: seq.to_string(),
                details: Some(format!("{:?} -> {:?}", from_state, to_state)),
            },
            HookEvent::BoundedActionExecuted {
                instance_id,
                action_id,
                description,
            } => EventLogEntry {
                event_type: "BoundedActionExecuted".into(),
                instance_id: instance_id.to_string(),
                timestamp: seq.to_string(),
                details: Some(format!("{}: {}", action_id, description)),
            },
            HookEvent::InstanceReset { instance_id } => EventLogEntry {
                event_type: "InstanceReset".into(),
                instance_id: instance_id.to_string(),
                timestamp: seq.to_string(),
                details: None,
            },
        }
    }
}

// ==============================================================================
// 2. Service Tier
// ==============================================================================

pub struct LogsService {
    state_path: String,
}

impl LogsService {
    pub fn new() -> Self {
        Self {
            state_path: crate::nouns::get_state_path(),
        }
    }

    /// Load the full event log from mesh state and project to `EventLogEntry`.
    pub fn load_entries(&self) -> std::result::Result<Vec<EventLogEntry>, String> {
        let mesh = AutonomicMesh::load_from_file(&self.state_path)
            .unwrap_or_else(|_| AutonomicMesh::new());
        let entries = mesh
            .event_log
            .iter()
            .enumerate()
            .map(|(i, ev)| EventLogEntry::from_hook_event(ev, i))
            .collect();
        Ok(entries)
    }

    pub fn list(
        &self,
        limit: u64,
        instance_id: Option<&str>,
        event_type: Option<&str>,
    ) -> std::result::Result<(Vec<EventLogEntry>, usize), String> {
        let all = self.load_entries()?;
        let total = all.len();

        let filtered: Vec<EventLogEntry> = all
            .into_iter()
            .filter(|e| {
                let id_match = instance_id.map(|id| e.instance_id == id).unwrap_or(true);
                let type_match = event_type
                    .map(|t| e.event_type.eq_ignore_ascii_case(t))
                    .unwrap_or(true);
                id_match && type_match
            })
            .collect();

        let filtered_count = filtered.len();
        let limited: Vec<EventLogEntry> = filtered
            .into_iter()
            .rev()
            .take(limit as usize)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect();

        Ok((limited, filtered_count))
    }

    pub fn search(
        &self,
        pattern: &str,
        limit: u64,
    ) -> std::result::Result<Vec<EventLogEntry>, String> {
        let all = self.load_entries()?;
        let lower = pattern.to_lowercase();

        let matches: Vec<EventLogEntry> = all
            .into_iter()
            .filter(|e| {
                e.event_type.to_lowercase().contains(&lower)
                    || e.instance_id.to_lowercase().contains(&lower)
                    || e.details
                        .as_deref()
                        .map(|d| d.to_lowercase().contains(&lower))
                        .unwrap_or(false)
            })
            .take(limit as usize)
            .collect();

        Ok(matches)
    }

    pub fn stats(
        &self,
    ) -> std::result::Result<
        (
            usize,
            HashMap<String, usize>,
            HashMap<String, usize>,
            Option<String>,
            Option<String>,
        ),
        String,
    > {
        let all = self.load_entries()?;
        let total = all.len();
        let mut by_type: HashMap<String, usize> = HashMap::new();
        let mut by_instance: HashMap<String, usize> = HashMap::new();
        let earliest = all.first().map(|e| e.timestamp.clone());
        let latest = all.last().map(|e| e.timestamp.clone());

        for e in &all {
            *by_type.entry(e.event_type.clone()).or_insert(0) += 1;
            *by_instance.entry(e.instance_id.clone()).or_insert(0) += 1;
        }

        Ok((total, by_type, by_instance, earliest, latest))
    }
}

impl Default for LogsService {
    fn default() -> Self {
        Self::new()
    }
}

// ==============================================================================
// 3. Verb Tier
// ==============================================================================

#[derive(Serialize)]
pub struct LogListResult {
    pub entries: Vec<EventLogEntry>,
    pub total: usize,
    pub filtered: usize,
}

#[verb("list")]
pub fn list(
    limit: Option<u64>,
    instance_id: Option<String>,
    event_type: Option<String>,
) -> Result<LogListResult> {
    let svc = LogsService::new();
    let effective_limit = limit.unwrap_or(50);
    let (entries, filtered) = svc
        .list(
            effective_limit,
            instance_id.as_deref(),
            event_type.as_deref(),
        )
        .map_err(NounVerbError::execution_error)?;
    let total = entries.len();
    Ok(LogListResult {
        entries,
        total,
        filtered,
    })
}

#[derive(Serialize)]
pub struct LogSearchResult {
    pub pattern: String,
    pub matches: Vec<EventLogEntry>,
    pub match_count: usize,
}

#[verb("search")]
pub fn search(pattern: String, limit: Option<u64>) -> Result<LogSearchResult> {
    let svc = LogsService::new();
    let effective_limit = limit.unwrap_or(20);
    let matches = svc
        .search(&pattern, effective_limit)
        .map_err(NounVerbError::execution_error)?;
    let match_count = matches.len();
    Ok(LogSearchResult {
        pattern,
        matches,
        match_count,
    })
}

#[derive(Serialize)]
pub struct LogStatsResult {
    pub total_events: usize,
    pub by_type: HashMap<String, usize>,
    pub by_instance: HashMap<String, usize>,
    pub earliest: Option<String>,
    pub latest: Option<String>,
}

#[verb("stats")]
pub fn stats() -> Result<LogStatsResult> {
    let svc = LogsService::new();
    let (total_events, by_type, by_instance, earliest, latest) =
        svc.stats().map_err(NounVerbError::execution_error)?;
    Ok(LogStatsResult {
        total_events,
        by_type,
        by_instance,
        earliest,
        latest,
    })
}

#[derive(Serialize)]
pub struct LogExportResult {
    pub format: String,
    pub entry_count: usize,
    pub output: String,
}

#[verb("export")]
pub fn export(format: Option<String>) -> Result<LogExportResult> {
    let svc = LogsService::new();
    let entries = svc
        .load_entries()
        .map_err(NounVerbError::execution_error)?;
    let entry_count = entries.len();
    let fmt = format.as_deref().unwrap_or("json");

    let output = match fmt {
        "jsonl" => entries
            .iter()
            .map(|e| {
                serde_json::to_string(e)
                    .unwrap_or_else(|_| serde_json::json!({"error":"serialize"}).to_string())
            })
            .collect::<Vec<_>>()
            .join("\n"),
        _ => serde_json::to_string_pretty(&entries)
            .map_err(|e| NounVerbError::execution_error(e.to_string()))?,
    };

    Ok(LogExportResult {
        format: fmt.to_string(),
        entry_count,
        output,
    })
}

// ==============================================================================
// 4. Tests
// ==============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use lsp_max_runtime::{AutonomicMesh, LspInstance, MeshAction};
    use std::env;

    fn with_mesh_state<F: FnOnce(&str)>(f: F) {
        let _guard = crate::nouns::TEST_ENV_LOCK
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        let tmp = tempfile::NamedTempFile::new().expect("tempfile");
        let path = tmp.path().to_str().unwrap().to_string();
        // SAFETY: test-only, guarded by TEST_ENV_LOCK
        unsafe {
            env::set_var("TOWER_LSP_MAX_STATE_PATH", &path);
        }
        f(&path);
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn list_returns_empty_on_fresh_state() {
        with_mesh_state(|_| {
            let svc = LogsService::new();
            let (entries, filtered) = svc.list(50, None, None).unwrap();
            assert_eq!(filtered, 0);
            assert!(entries.is_empty());
        });
    }

    #[test]
    fn stats_returns_zero_on_empty_log() {
        with_mesh_state(|_| {
            let svc = LogsService::new();
            let (total, by_type, by_instance, earliest, latest) = svc.stats().unwrap();
            assert_eq!(total, 0);
            assert!(by_type.is_empty());
            assert!(by_instance.is_empty());
            assert!(earliest.is_none());
            assert!(latest.is_none());
        });
    }

    #[test]
    fn search_returns_empty_on_no_match() {
        with_mesh_state(|_| {
            let svc = LogsService::new();
            let matches = svc.search("xyzzy-no-match", 20).unwrap();
            assert!(matches.is_empty());
        });
    }

    #[test]
    fn export_json_produces_valid_json() {
        with_mesh_state(|path| {
            let mut mesh = AutonomicMesh::new();
            let inst = LspInstance::new("export-test");
            mesh.add_instance(inst);
            mesh.execute_action(MeshAction::ExecuteBoundedAction {
                instance_id: lsp_max_runtime::InstanceId::from("export-test".to_string()),
                action_id: "test-action-1".into(),
                description: "test export action".into(),
            });
            mesh.save_to_file(path).unwrap();

            let svc = LogsService::new();
            let entries = svc.load_entries().unwrap();
            let json_str = serde_json::to_string_pretty(&entries).unwrap();
            serde_json::from_str::<serde_json::Value>(&json_str)
                .expect("export output must be valid JSON");
        });
    }

    #[test]
    fn from_hook_event_bounded_action_includes_details() {
        let ev = HookEvent::BoundedActionExecuted {
            instance_id: lsp_max_runtime::InstanceId::from("inst-1".to_string()),
            action_id: "act-001".into(),
            description: "ran an action".into(),
        };
        let entry = EventLogEntry::from_hook_event(&ev, 0);
        assert_eq!(entry.event_type, "BoundedActionExecuted");
        assert_eq!(entry.instance_id, "inst-1");
        assert!(entry.details.is_some());
        let det = entry.details.unwrap();
        assert!(det.contains("act-001"));
        assert!(det.contains("ran an action"));
    }
}
