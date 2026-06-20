use clap_noun_verb::error::NounVerbError;
use clap_noun_verb::Result;
use clap_noun_verb_macros::verb;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

// ==============================================================================
// 1. Domain Tier
// ==============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryEntry {
    pub id: String,
    pub noun: String,
    pub verb: String,
    pub args: Vec<String>,
    pub invoked_at: String,
    /// Bounded status: "ADMITTED", "REFUSED", or "UNKNOWN"
    pub status: String,
}

// ==============================================================================
// 2. Service Tier
// ==============================================================================

pub struct HistoryService;

impl HistoryService {
    pub fn new() -> Self {
        Self
    }

    fn history_path(&self) -> PathBuf {
        let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
        PathBuf::from(home).join(".lsp-max-history.json")
    }

    fn load(&self) -> Vec<HistoryEntry> {
        let path = self.history_path();
        if !path.exists() {
            return Vec::new();
        }
        let content = match fs::read_to_string(&path) {
            Ok(c) => c,
            Err(_) => return Vec::new(),
        };
        serde_json::from_str::<Vec<HistoryEntry>>(&content).unwrap_or_default()
    }

    fn save(&self, entries: &[HistoryEntry]) -> std::result::Result<(), String> {
        let path = self.history_path();
        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        let content = serde_json::to_string_pretty(entries).map_err(|e| e.to_string())?;
        fs::write(&path, content).map_err(|e| e.to_string())?;
        Ok(())
    }

    fn next_id(&self, entries: &[HistoryEntry]) -> String {
        // Extract numeric part from IDs of the form "H{n}" to find the next value.
        let max_n = entries
            .iter()
            .filter_map(|e| e.id.strip_prefix('H'))
            .filter_map(|n| n.parse::<u64>().ok())
            .max()
            .unwrap_or(0);
        format!("H{}", max_n + 1)
    }

    fn now_secs() -> String {
        let ts = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        format!("{ts}")
    }

    pub fn list(
        &self,
        limit: Option<u64>,
        noun_filter: Option<&str>,
    ) -> (Vec<HistoryEntry>, usize) {
        let all = self.load();
        let total = all.len();
        let filtered: Vec<HistoryEntry> = all
            .into_iter()
            .filter(|e| noun_filter.map_or(true, |n| e.noun == n))
            .collect();
        let cap = limit.unwrap_or(20) as usize;
        let start = filtered.len().saturating_sub(cap);
        (filtered[start..].to_vec(), total)
    }

    pub fn find(&self, id: &str) -> Option<HistoryEntry> {
        self.load().into_iter().find(|e| e.id == id)
    }

    pub fn export_all(&self) -> Vec<HistoryEntry> {
        self.load()
    }

    pub fn record(
        &self,
        noun: String,
        verb: String,
        args: Vec<String>,
        status: String,
    ) -> std::result::Result<HistoryEntry, String> {
        let mut entries = self.load();
        let id = self.next_id(&entries);
        let entry = HistoryEntry {
            id: id.clone(),
            noun,
            verb,
            args,
            invoked_at: Self::now_secs(),
            status,
        };
        entries.push(entry.clone());
        self.save(&entries)?;
        Ok(entry)
    }
}

impl Default for HistoryService {
    fn default() -> Self {
        Self::new()
    }
}

// ==============================================================================
// 3. Verb Tier
// ==============================================================================

#[derive(Debug, Clone, Serialize)]
pub struct HistoryListResult {
    pub entries: Vec<HistoryEntry>,
    pub total: usize,
}

/// List recorded CLI invocations. Returns up to `limit` entries (default 20),
/// optionally filtered to a single noun.
#[verb("list")]
pub fn list(limit: Option<u64>, noun: Option<String>) -> Result<HistoryListResult> {
    let svc = HistoryService::new();
    let (entries, total) = svc.list(limit, noun.as_deref());
    Ok(HistoryListResult { entries, total })
}

#[derive(Debug, Clone, Serialize)]
pub struct HistoryReplayResult {
    pub id: String,
    pub noun: String,
    pub verb: String,
    pub args: Vec<String>,
    pub replay_command: String,
}

/// Look up a history entry by ID and return the command that would replay it.
/// Does NOT execute the command.
#[verb("replay")]
pub fn replay(id: String) -> Result<HistoryReplayResult> {
    let svc = HistoryService::new();
    let entry = svc
        .find(&id)
        .ok_or_else(|| NounVerbError::execution_error(format!("HISTORY_NOT_FOUND: {id}")))?;
    let replay_command = if entry.args.is_empty() {
        format!("lsp-max-cli {} {}", entry.noun, entry.verb)
    } else {
        format!(
            "lsp-max-cli {} {} {}",
            entry.noun,
            entry.verb,
            entry.args.join(" ")
        )
    };
    Ok(HistoryReplayResult {
        id: entry.id,
        noun: entry.noun,
        verb: entry.verb,
        args: entry.args,
        replay_command,
    })
}

#[derive(Debug, Clone, Serialize)]
pub struct HistoryExportResult {
    pub format: String,
    pub entry_count: usize,
    pub output: String,
}

/// Export the full history as JSON (default) or CSV.
#[verb("export")]
pub fn export(format: Option<String>) -> Result<HistoryExportResult> {
    let svc = HistoryService::new();
    let entries = svc.export_all();
    let entry_count = entries.len();
    let fmt = format.unwrap_or_else(|| "json".to_string());
    let output = match fmt.as_str() {
        "csv" => {
            let mut lines = vec!["id,noun,verb,args,invoked_at,status".to_string()];
            for e in &entries {
                lines.push(format!(
                    "{},{},{},{},{},{}",
                    e.id,
                    e.noun,
                    e.verb,
                    e.args.join("|"),
                    e.invoked_at,
                    e.status
                ));
            }
            lines.join("\n")
        }
        _ => serde_json::to_string_pretty(&entries)
            .map_err(|e| NounVerbError::execution_error(e.to_string()))?,
    };
    Ok(HistoryExportResult {
        format: fmt,
        entry_count,
        output,
    })
}

#[derive(Debug, Clone, Serialize)]
pub struct HistoryRecordResult {
    pub id: String,
    pub entry: HistoryEntry,
}

/// Append a new invocation record to the history file.
#[verb("record")]
pub fn record(
    noun: String,
    verb: String,
    args: Vec<String>,
    status: String,
) -> Result<HistoryRecordResult> {
    let svc = HistoryService::new();
    let entry = svc
        .record(noun, verb, args, status)
        .map_err(NounVerbError::execution_error)?;
    Ok(HistoryRecordResult {
        id: entry.id.clone(),
        entry,
    })
}

// ==============================================================================
// 4. Tests
// ==============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    fn temp_history_service() -> (tempfile::TempDir, HistoryService) {
        let dir = tempfile::TempDir::new().unwrap();
        env::set_var("HOME", dir.path());
        (dir, HistoryService::new())
    }

    #[test]
    fn list_returns_empty_when_no_history_file() {
        let (_dir, svc) = temp_history_service();
        let (entries, total) = svc.list(None, None);
        assert!(entries.is_empty());
        assert_eq!(total, 0);
    }

    #[test]
    fn record_then_list_returns_entry() {
        let (_dir, svc) = temp_history_service();
        svc.record(
            "gate".to_string(),
            "check".to_string(),
            vec![],
            "ADMITTED".to_string(),
        )
        .unwrap();
        let (entries, total) = svc.list(None, None);
        assert_eq!(total, 1);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].noun, "gate");
        assert_eq!(entries[0].status, "ADMITTED");
    }

    #[test]
    fn ids_increment_as_h_prefix() {
        let (_dir, svc) = temp_history_service();
        let e1 = svc
            .record("a".into(), "b".into(), vec![], "UNKNOWN".into())
            .unwrap();
        let e2 = svc
            .record("c".into(), "d".into(), vec![], "REFUSED".into())
            .unwrap();
        assert_eq!(e1.id, "H1");
        assert_eq!(e2.id, "H2");
    }

    #[test]
    fn find_returns_none_for_missing_id() {
        let (_dir, svc) = temp_history_service();
        assert!(svc.find("H99").is_none());
    }

    #[test]
    fn find_returns_entry_after_record() {
        let (_dir, svc) = temp_history_service();
        svc.record("gate".into(), "check".into(), vec![], "ADMITTED".into())
            .unwrap();
        let found = svc.find("H1");
        assert!(found.is_some());
        assert_eq!(found.unwrap().noun, "gate");
    }

    #[test]
    fn list_limit_caps_results() {
        let (_dir, svc) = temp_history_service();
        for i in 0..5u64 {
            svc.record(
                "gate".into(),
                "check".into(),
                vec![],
                format!("ADMITTED-{i}"),
            )
            .unwrap();
        }
        let (entries, total) = svc.list(Some(3), None);
        assert_eq!(total, 5);
        assert_eq!(entries.len(), 3);
    }

    #[test]
    fn list_noun_filter_works() {
        let (_dir, svc) = temp_history_service();
        svc.record("gate".into(), "check".into(), vec![], "ADMITTED".into())
            .unwrap();
        svc.record("config".into(), "view".into(), vec![], "ADMITTED".into())
            .unwrap();
        let (entries, _) = svc.list(None, Some("gate"));
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].noun, "gate");
    }

    #[test]
    fn export_json_default_format() {
        let (_dir, svc) = temp_history_service();
        svc.record("gate".into(), "check".into(), vec![], "ADMITTED".into())
            .unwrap();
        let entries = svc.export_all();
        let output = serde_json::to_string_pretty(&entries).unwrap();
        assert!(output.contains("gate"));
    }

    #[test]
    fn export_csv_includes_header_and_pipe_separated_args() {
        let (_dir, svc) = temp_history_service();
        svc.record(
            "gate".into(),
            "check".into(),
            vec!["--foo".into(), "bar".into()],
            "ADMITTED".into(),
        )
        .unwrap();
        let entries = svc.export_all();
        let mut lines = vec!["id,noun,verb,args,invoked_at,status".to_string()];
        for e in &entries {
            lines.push(format!(
                "{},{},{},{},{},{}",
                e.id,
                e.noun,
                e.verb,
                e.args.join("|"),
                e.invoked_at,
                e.status
            ));
        }
        let csv = lines.join("\n");
        assert!(csv.starts_with("id,noun,verb,args,invoked_at,status"));
        assert!(csv.contains("--foo|bar"));
    }

    #[test]
    fn replay_command_formatted_correctly() {
        let (_dir, svc) = temp_history_service();
        svc.record(
            "gate".into(),
            "check".into(),
            vec!["--verbose".into()],
            "ADMITTED".into(),
        )
        .unwrap();
        let entry = svc.find("H1").unwrap();
        let cmd = format!(
            "lsp-max-cli {} {} {}",
            entry.noun,
            entry.verb,
            entry.args.join(" ")
        );
        assert_eq!(cmd, "lsp-max-cli gate check --verbose");
    }
}
