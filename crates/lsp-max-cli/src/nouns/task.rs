use clap_noun_verb::error::NounVerbError;
use clap_noun_verb::Result;
use clap_noun_verb_macros::verb;
use lsp_max::max_runtime::AutonomicMesh;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ==============================================================================
// 1. Domain Tier
// ==============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskSummary {
    pub task_id: String,
    pub instance_id: String,
    pub task_type: String,
    pub status: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskDetail {
    pub task_id: String,
    pub task_type: String,
    pub status: String,
    pub created_at: String,
    pub result: Option<String>,
}

/// Flat projection of a HookEvent for log output.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventEntry {
    pub event_type: String,
    pub instance_id: String,
    pub details: Option<String>,
}

pub struct TaskStatsData {
    pub total: usize,
    pub by_status: HashMap<String, usize>,
    pub by_type: HashMap<String, usize>,
}

// ==============================================================================
// 2. Service Tier
// ==============================================================================

pub struct TaskService {
    state_path: String,
}

impl TaskService {
    pub fn new() -> Self {
        Self {
            state_path: crate::nouns::get_state_path(),
        }
    }

    // Tasks are stored under extra["agent_tasks"][instance_id] as JSON arrays.
    fn load_all_tasks(mesh: &AutonomicMesh) -> Vec<(String, serde_json::Value)> {
        let Some(by_instance) = mesh.extra.get("agent_tasks") else {
            return vec![];
        };
        let Some(obj) = by_instance.as_object() else {
            return vec![];
        };
        let mut out = Vec::new();
        for (iid, arr_val) in obj {
            if let Some(arr) = arr_val.as_array() {
                for task in arr {
                    out.push((iid.clone(), task.clone()));
                }
            }
        }
        out
    }

    fn task_field<'a>(val: &'a serde_json::Value, key: &str) -> &'a str {
        val.get(key).and_then(|v| v.as_str()).unwrap_or("")
    }

    pub fn list(
        &self,
        instance_id: Option<&str>,
        status_filter: Option<&str>,
    ) -> std::result::Result<Vec<TaskSummary>, String> {
        let mesh = match AutonomicMesh::load_from_file(&self.state_path) {
            Ok(m) => m,
            Err(_) => return Ok(vec![]),
        };
        Ok(Self::load_all_tasks(&mesh)
            .into_iter()
            .filter(|(iid, _)| instance_id.is_none_or(|f| iid == f))
            .filter(|(_, t)| status_filter.is_none_or(|sf| Self::task_field(t, "status") == sf))
            .map(|(iid, t)| TaskSummary {
                task_id: Self::task_field(&t, "task_id").to_string(),
                instance_id: iid,
                task_type: Self::task_field(&t, "task_type").to_string(),
                status: Self::task_field(&t, "status").to_string(),
                created_at: Self::task_field(&t, "created_at").to_string(),
            })
            .collect())
    }

    pub fn show(&self, task_id: &str) -> std::result::Result<(TaskDetail, String), String> {
        let mesh = AutonomicMesh::load_from_file(&self.state_path).map_err(|e| e.to_string())?;
        for (iid, t) in Self::load_all_tasks(&mesh) {
            if Self::task_field(&t, "task_id") != task_id {
                continue;
            }
            return Ok((
                TaskDetail {
                    task_id: task_id.to_string(),
                    task_type: Self::task_field(&t, "task_type").to_string(),
                    status: Self::task_field(&t, "status").to_string(),
                    created_at: Self::task_field(&t, "created_at").to_string(),
                    result: t.get("result").and_then(|v| v.as_str()).map(str::to_string),
                },
                iid,
            ));
        }
        Err(format!("TASK_NOT_FOUND: {}", task_id))
    }

    pub fn log(&self, task_id: &str) -> std::result::Result<Vec<EventEntry>, String> {
        let mesh = AutonomicMesh::load_from_file(&self.state_path).map_err(|e| e.to_string())?;
        let mut entries = Vec::new();
        for event in &mesh.event_log {
            let Ok(json_val) = serde_json::to_value(event) else {
                continue;
            };
            let serialized = json_val.to_string();
            if !serialized.contains(task_id) {
                continue;
            }
            // HookEvent serializes as {"VariantName": {fields...}}
            let Some(obj) = json_val.as_object() else {
                continue;
            };
            let Some((variant, inner)) = obj.iter().next() else {
                continue;
            };
            // InstanceId is a newtype struct — try as object {"0":"..."} then as plain string.
            let iid = inner
                .get("instance_id")
                .and_then(|v| {
                    v.as_object()
                        .and_then(|o| o.get("0"))
                        .and_then(|s| s.as_str())
                        .or_else(|| v.as_str())
                })
                .unwrap_or("")
                .to_string();
            entries.push(EventEntry {
                event_type: variant.clone(),
                instance_id: iid,
                details: Some(serialized),
            });
        }
        Ok(entries)
    }

    pub fn cancel(&self, task_id: &str) -> std::result::Result<(String, String, String), String> {
        let mut mesh =
            AutonomicMesh::load_from_file(&self.state_path).map_err(|e| e.to_string())?;

        let snap = mesh
            .extra
            .get("agent_tasks")
            .cloned()
            .unwrap_or_else(|| serde_json::json!({}));

        let (found_iid, prev_status) = snap
            .as_object()
            .and_then(|obj| {
                for (iid, arr_val) in obj {
                    if let Some(arr) = arr_val.as_array() {
                        for t in arr {
                            if Self::task_field(t, "task_id") == task_id {
                                return Some((
                                    iid.clone(),
                                    Self::task_field(t, "status").to_string(),
                                ));
                            }
                        }
                    }
                }
                None
            })
            .ok_or_else(|| format!("TASK_NOT_FOUND: {}", task_id))?;

        if prev_status == "ADMITTED" || prev_status == "REFUSED" {
            return Err(format!(
                "TASK_NOT_CANCELLABLE: current status is {}",
                prev_status
            ));
        }

        let tasks_map = mesh
            .extra
            .entry("agent_tasks".to_string())
            .or_insert_with(|| serde_json::json!({}));

        if let Some(arr) = tasks_map.get_mut(&found_iid).and_then(|v| v.as_array_mut()) {
            for t in arr.iter_mut() {
                if Self::task_field(t, "task_id") == task_id {
                    if let Some(obj) = t.as_object_mut() {
                        obj.insert(
                            "status".to_string(),
                            serde_json::Value::String("REFUSED".to_string()),
                        );
                    }
                    break;
                }
            }
        }

        mesh.save_to_file(&self.state_path)
            .map_err(|e| e.to_string())?;
        Ok((prev_status, "REFUSED".to_string(), found_iid))
    }

    pub fn stats(&self) -> std::result::Result<TaskStatsData, String> {
        let mesh = match AutonomicMesh::load_from_file(&self.state_path) {
            Ok(m) => m,
            Err(_) => {
                return Ok(TaskStatsData {
                    total: 0,
                    by_status: HashMap::new(),
                    by_type: HashMap::new(),
                })
            }
        };
        let raw = Self::load_all_tasks(&mesh);
        let mut by_status: HashMap<String, usize> = HashMap::new();
        let mut by_type: HashMap<String, usize> = HashMap::new();
        for (_, t) in &raw {
            *by_status
                .entry(Self::task_field(t, "status").to_string())
                .or_insert(0) += 1;
            *by_type
                .entry(Self::task_field(t, "task_type").to_string())
                .or_insert(0) += 1;
        }
        Ok(TaskStatsData {
            total: raw.len(),
            by_status,
            by_type,
        })
    }
}

impl Default for TaskService {
    fn default() -> Self {
        Self::new()
    }
}

// ==============================================================================
// 3. Verb Tier
// ==============================================================================

#[derive(Serialize)]
pub struct TaskListResult {
    pub tasks: Vec<TaskSummary>,
    pub total: usize,
}

#[verb("list")]
pub fn list(instance_id: Option<String>, status_filter: Option<String>) -> Result<TaskListResult> {
    let service = TaskService::new();
    let tasks = service
        .list(instance_id.as_deref(), status_filter.as_deref())
        .map_err(NounVerbError::execution_error)?;
    let total = tasks.len();
    Ok(TaskListResult { tasks, total })
}

#[derive(Serialize)]
pub struct TaskShowResult {
    pub task: TaskDetail,
    pub instance_id: String,
}

#[verb("show")]
pub fn show(task_id: String) -> Result<TaskShowResult> {
    let service = TaskService::new();
    let (task, instance_id) = service
        .show(&task_id)
        .map_err(NounVerbError::execution_error)?;
    Ok(TaskShowResult { task, instance_id })
}

#[derive(Serialize)]
pub struct TaskLogResult {
    pub task_id: String,
    pub events: Vec<EventEntry>,
    pub count: usize,
}

#[verb("log")]
pub fn log(task_id: String) -> Result<TaskLogResult> {
    let service = TaskService::new();
    let events = service
        .log(&task_id)
        .map_err(NounVerbError::execution_error)?;
    let count = events.len();
    Ok(TaskLogResult {
        task_id,
        events,
        count,
    })
}

#[derive(Serialize)]
pub struct TaskCancelResult {
    pub task_id: String,
    pub previous_status: String,
    pub new_status: String,
    pub instance_id: String,
}

#[verb("cancel")]
pub fn cancel(task_id: String) -> Result<TaskCancelResult> {
    let service = TaskService::new();
    let (previous_status, new_status, instance_id) = service
        .cancel(&task_id)
        .map_err(NounVerbError::execution_error)?;
    Ok(TaskCancelResult {
        task_id,
        previous_status,
        new_status,
        instance_id,
    })
}

#[derive(Serialize)]
pub struct TaskStatsResult {
    pub total: usize,
    pub by_status: HashMap<String, usize>,
    pub by_type: HashMap<String, usize>,
}

#[verb("stats")]
pub fn stats() -> Result<TaskStatsResult> {
    let service = TaskService::new();
    let data = service.stats().map_err(NounVerbError::execution_error)?;
    Ok(TaskStatsResult {
        total: data.total,
        by_status: data.by_status,
        by_type: data.by_type,
    })
}

// ==============================================================================
// 4. Tests
// ==============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use lsp_max::max_runtime::{AutonomicMesh, LspInstance};
    use std::env;

    fn isolated_state<F: FnOnce(String)>(f: F) {
        let _lock = crate::nouns::TEST_ENV_LOCK
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        let tmp = tempfile::NamedTempFile::new().expect("tempfile");
        let path = tmp.path().to_str().unwrap().to_string();
        let prev = env::var("LSP_MAX_STATE_PATH").ok();
        // SAFETY: test-only, guarded by TEST_ENV_LOCK
        unsafe {
            env::set_var("LSP_MAX_STATE_PATH", &path);
        }
        let _ = std::fs::remove_file(&path);
        f(path.clone());
        let _ = std::fs::remove_file(&path);
        // SAFETY: test-only, guarded by TEST_ENV_LOCK
        unsafe {
            match prev {
                Some(v) => env::set_var("LSP_MAX_STATE_PATH", v),
                None => env::remove_var("LSP_MAX_STATE_PATH"),
            }
        }
    }

    fn write_mesh_with_tasks(path: &str, tasks: Vec<serde_json::Value>) {
        let mut mesh = AutonomicMesh::new();
        mesh.add_instance(LspInstance::new("inst-1"));
        let mut map = serde_json::Map::new();
        map.insert("inst-1".to_string(), serde_json::json!(tasks));
        mesh.extra
            .insert("agent_tasks".to_string(), serde_json::Value::Object(map));
        mesh.save_to_file(path).unwrap();
    }

    fn task_json(id: &str, status: &str) -> serde_json::Value {
        serde_json::json!({
            "task_id": id, "task_type": "ANALYSIS",
            "status": status, "created_at": "2026-06-20T00:00:00Z", "result": null
        })
    }

    #[test]
    fn list_empty_when_no_state_file() {
        isolated_state(|_| {
            assert!(TaskService::new().list(None, None).unwrap().is_empty());
        });
    }

    #[test]
    fn list_all_tasks_and_filter_by_status() {
        isolated_state(|path| {
            write_mesh_with_tasks(
                &path,
                vec![task_json("t1", "OPEN"), task_json("t2", "ADMITTED")],
            );
            let svc = TaskService::new();
            assert_eq!(svc.list(None, None).unwrap().len(), 2);
            let open = svc.list(None, Some("OPEN")).unwrap();
            assert_eq!(open.len(), 1);
            assert_eq!(open[0].task_id, "t1");
        });
    }

    #[test]
    fn show_finds_task_and_errors_on_missing() {
        isolated_state(|path| {
            write_mesh_with_tasks(&path, vec![task_json("t-show", "OPEN")]);
            let svc = TaskService::new();
            let (detail, iid) = svc.show("t-show").unwrap();
            assert_eq!(detail.task_id, "t-show");
            assert_eq!(iid, "inst-1");
            let err = svc.show("no-such").unwrap_err();
            assert!(err.contains("TASK_NOT_FOUND"), "got: {err}");
        });
    }

    #[test]
    fn log_returns_empty_for_unknown_task_id() {
        isolated_state(|path| {
            write_mesh_with_tasks(&path, vec![task_json("t1", "OPEN")]);
            assert!(TaskService::new().log("nonexistent").unwrap().is_empty());
        });
    }

    #[test]
    fn cancel_open_task_sets_refused() {
        isolated_state(|path| {
            write_mesh_with_tasks(&path, vec![task_json("t-cancel", "OPEN")]);
            let (prev, next, iid) = TaskService::new().cancel("t-cancel").unwrap();
            assert_eq!(prev, "OPEN");
            assert_eq!(next, "REFUSED");
            assert_eq!(iid, "inst-1");
        });
    }

    #[test]
    fn cancel_blocks_admitted_and_missing() {
        isolated_state(|path| {
            write_mesh_with_tasks(&path, vec![task_json("t-adm", "ADMITTED")]);
            let svc = TaskService::new();
            assert!(svc
                .cancel("t-adm")
                .unwrap_err()
                .contains("TASK_NOT_CANCELLABLE"));
            assert!(svc.cancel("ghost").unwrap_err().contains("TASK_NOT_FOUND"));
        });
    }

    #[test]
    fn stats_aggregates_correctly() {
        isolated_state(|path| {
            write_mesh_with_tasks(
                &path,
                vec![
                    task_json("t1", "OPEN"),
                    task_json("t2", "OPEN"),
                    task_json("t3", "ADMITTED"),
                ],
            );
            let data = TaskService::new().stats().unwrap();
            assert_eq!(data.total, 3);
            assert_eq!(data.by_status.get("OPEN").copied().unwrap_or(0), 2);
            assert_eq!(data.by_status.get("ADMITTED").copied().unwrap_or(0), 1);
            assert_eq!(data.by_type.get("ANALYSIS").copied().unwrap_or(0), 3);
        });
    }
}
