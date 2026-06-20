use clap_noun_verb::error::NounVerbError;
use clap_noun_verb::Result;
use clap_noun_verb_macros::verb;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ==============================================================================
// 1. Domain Tier — batch command sequences for CLI replay
// ==============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchCommand {
    pub noun: String,
    pub verb: String,
    pub args: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Batch {
    pub name: String,
    pub description: String,
    pub commands: Vec<BatchCommand>,
    pub created_at: String,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BatchStore {
    pub batches: HashMap<String, Batch>,
}

// ==============================================================================
// 2. Service Tier
// ==============================================================================

pub struct BatchService {
    store_path: String,
}

impl BatchService {
    pub fn new() -> Self {
        let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
        Self {
            store_path: format!("{}/.lsp-max-batches.json", home),
        }
    }

    fn load(&self) -> std::result::Result<BatchStore, String> {
        if !std::path::Path::new(&self.store_path).exists() {
            return Ok(BatchStore::default());
        }
        let content = std::fs::read_to_string(&self.store_path).map_err(|e| e.to_string())?;
        serde_json::from_str(&content).map_err(|e| e.to_string())
    }

    fn save(&self, store: &BatchStore) -> std::result::Result<(), String> {
        let content = serde_json::to_string_pretty(store).map_err(|e| e.to_string())?;
        std::fs::write(&self.store_path, content).map_err(|e| e.to_string())
    }

    pub fn create(
        &self,
        name: &str,
        description: Option<&str>,
    ) -> std::result::Result<Batch, String> {
        let mut store = self.load()?;
        if store.batches.contains_key(name) {
            return Err(format!("BATCH_EXISTS: {}", name));
        }
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs().to_string())
            .unwrap_or_else(|_| "0".to_string());
        let batch = Batch {
            name: name.to_string(),
            description: description.unwrap_or("").to_string(),
            commands: Vec::new(),
            created_at: now,
            status: "OPEN".to_string(),
        };
        store.batches.insert(name.to_string(), batch.clone());
        self.save(&store)?;
        Ok(batch)
    }

    pub fn add_command(
        &self,
        batch_name: &str,
        noun: &str,
        verb: &str,
        args: Option<&str>,
    ) -> std::result::Result<(usize, usize), String> {
        let mut store = self.load()?;
        let batch = store
            .batches
            .get_mut(batch_name)
            .ok_or_else(|| format!("BATCH_NOT_FOUND: {}", batch_name))?;
        let parsed_args: Vec<String> = args
            .unwrap_or("")
            .split_whitespace()
            .map(|s| s.to_string())
            .filter(|s| !s.is_empty())
            .collect();
        batch.commands.push(BatchCommand {
            noun: noun.to_string(),
            verb: verb.to_string(),
            args: parsed_args,
        });
        let total = batch.commands.len();
        let index = total - 1;
        self.save(&store)?;
        Ok((index, total))
    }

    pub fn list(&self) -> std::result::Result<Vec<BatchSummary>, String> {
        let store = self.load()?;
        let mut summaries: Vec<BatchSummary> = store
            .batches
            .values()
            .map(|b| BatchSummary {
                name: b.name.clone(),
                description: b.description.clone(),
                command_count: b.commands.len(),
                status: b.status.clone(),
            })
            .collect();
        summaries.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(summaries)
    }

    pub fn show(&self, name: &str) -> std::result::Result<Batch, String> {
        let store = self.load()?;
        store
            .batches
            .get(name)
            .cloned()
            .ok_or_else(|| format!("BATCH_NOT_FOUND: {}", name))
    }

    pub fn clear(&self, name: &str) -> std::result::Result<bool, String> {
        let mut store = self.load()?;
        let was_present = store.batches.remove(name).is_some();
        self.save(&store)?;
        Ok(was_present)
    }

    pub fn run(&self, name: &str) -> std::result::Result<Vec<String>, String> {
        let batch = self.show(name)?;
        let lines = batch
            .commands
            .iter()
            .map(|cmd| {
                let mut parts =
                    vec!["lsp-max-cli".to_string(), cmd.noun.clone(), cmd.verb.clone()];
                parts.extend(cmd.args.iter().cloned());
                parts.join(" ")
            })
            .collect();
        Ok(lines)
    }
}

impl Default for BatchService {
    fn default() -> Self {
        Self::new()
    }
}

// ==============================================================================
// 3. Verb Tier
// ==============================================================================

#[derive(Debug, Clone, Serialize)]
pub struct BatchSummary {
    pub name: String,
    pub description: String,
    pub command_count: usize,
    pub status: String,
}

#[derive(Serialize)]
pub struct BatchNewResult {
    pub name: String,
    pub batch: Batch,
}

/// Create a new empty batch with the given name.
#[verb("new")]
pub fn new(name: String, description: Option<String>) -> Result<BatchNewResult> {
    let svc = BatchService::new();
    let batch = svc
        .create(&name, description.as_deref())
        .map_err(NounVerbError::execution_error)?;
    Ok(BatchNewResult { name, batch })
}

#[derive(Serialize)]
pub struct BatchAddResult {
    pub batch_name: String,
    pub command_index: usize,
    pub total_commands: usize,
}

/// Append a command to an existing batch.
#[verb("add-command")]
pub fn add_command(
    batch_name: String,
    noun: String,
    verb: String,
    args: Option<String>,
) -> Result<BatchAddResult> {
    let svc = BatchService::new();
    let (command_index, total_commands) = svc
        .add_command(&batch_name, &noun, &verb, args.as_deref())
        .map_err(NounVerbError::execution_error)?;
    Ok(BatchAddResult {
        batch_name,
        command_index,
        total_commands,
    })
}

#[derive(Serialize)]
pub struct BatchListResult {
    pub batches: Vec<BatchSummary>,
    pub count: usize,
}

/// List all batches with summary information.
#[verb("list")]
pub fn list() -> Result<BatchListResult> {
    let svc = BatchService::new();
    let batches = svc.list().map_err(NounVerbError::execution_error)?;
    let count = batches.len();
    Ok(BatchListResult { batches, count })
}

#[derive(Serialize)]
pub struct BatchShowResult {
    pub batch: Batch,
}

/// Show full details of a named batch including all commands.
#[verb("show")]
pub fn show(name: String) -> Result<BatchShowResult> {
    let svc = BatchService::new();
    let batch = svc.show(&name).map_err(NounVerbError::execution_error)?;
    Ok(BatchShowResult { batch })
}

#[derive(Serialize)]
pub struct BatchClearResult {
    pub name: String,
    pub was_present: bool,
}

/// Delete a batch from the store.
#[verb("clear")]
pub fn clear(name: String) -> Result<BatchClearResult> {
    let svc = BatchService::new();
    let was_present = svc.clear(&name).map_err(NounVerbError::execution_error)?;
    Ok(BatchClearResult { name, was_present })
}

#[derive(Serialize)]
pub struct BatchRunResult {
    pub name: String,
    pub commands: Vec<String>,
    pub note: String,
}

/// Dry-run a batch: print the command sequence without executing it.
#[verb("run")]
pub fn run(name: String) -> Result<BatchRunResult> {
    let svc = BatchService::new();
    let commands = svc.run(&name).map_err(NounVerbError::execution_error)?;
    Ok(BatchRunResult {
        name,
        commands,
        note: "DRY_RUN: commands listed but not executed — invoke each manually".to_string(),
    })
}

// ==============================================================================
// 4. Tests
// ==============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn make_temp_service() -> (tempfile::NamedTempFile, BatchService) {
        let f = tempfile::NamedTempFile::new().unwrap();
        let svc = BatchService {
            store_path: f.path().to_str().unwrap().to_string(),
        };
        (f, svc)
    }

    #[test]
    fn batch_create_new_returns_open_status() {
        let (_f, svc) = make_temp_service();
        let batch = svc.create("my-batch", Some("test batch")).unwrap();
        assert_eq!(batch.status, "OPEN");
        assert_eq!(batch.name, "my-batch");
        assert!(batch.commands.is_empty());
    }

    #[test]
    fn batch_create_duplicate_returns_err() {
        let (_f, svc) = make_temp_service();
        svc.create("dup", None).unwrap();
        let result = svc.create("dup", None);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("BATCH_EXISTS"));
    }

    #[test]
    fn batch_add_command_increments_count() {
        let (_f, svc) = make_temp_service();
        svc.create("seq", None).unwrap();
        let (idx, total) = svc.add_command("seq", "gate", "check", None).unwrap();
        assert_eq!(idx, 0);
        assert_eq!(total, 1);
        let (idx2, total2) = svc
            .add_command("seq", "gate", "list", Some("--all"))
            .unwrap();
        assert_eq!(idx2, 1);
        assert_eq!(total2, 2);
    }

    #[test]
    fn batch_add_command_unknown_batch_returns_err() {
        let (_f, svc) = make_temp_service();
        let result = svc.add_command("no-such", "gate", "check", None);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("BATCH_NOT_FOUND"));
    }

    #[test]
    fn batch_list_returns_all_batches() {
        let (_f, svc) = make_temp_service();
        svc.create("alpha", Some("first")).unwrap();
        svc.create("beta", Some("second")).unwrap();
        let list = svc.list().unwrap();
        assert_eq!(list.len(), 2);
    }

    #[test]
    fn batch_show_unknown_returns_err() {
        let (_f, svc) = make_temp_service();
        let result = svc.show("ghost");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("BATCH_NOT_FOUND"));
    }

    #[test]
    fn batch_clear_removes_batch() {
        let (_f, svc) = make_temp_service();
        svc.create("to-clear", None).unwrap();
        let was_present = svc.clear("to-clear").unwrap();
        assert!(was_present);
        let was_present_again = svc.clear("to-clear").unwrap();
        assert!(!was_present_again);
    }

    #[test]
    fn batch_run_formats_commands_correctly() {
        let (_f, svc) = make_temp_service();
        svc.create("run-test", None).unwrap();
        svc.add_command("run-test", "gate", "check", None).unwrap();
        svc.add_command("run-test", "receipt", "show", Some("--id abc"))
            .unwrap();
        let cmds = svc.run("run-test").unwrap();
        assert_eq!(cmds[0], "lsp-max-cli gate check");
        assert_eq!(cmds[1], "lsp-max-cli receipt show --id abc");
    }
}
