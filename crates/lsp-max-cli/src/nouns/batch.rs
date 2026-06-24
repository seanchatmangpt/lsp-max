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

/// Outcome of one command in an executed batch.
#[derive(Debug, Clone, Serialize)]
pub struct CommandOutcome {
    pub command: String,
    /// Process exit code; `None` if the child was terminated by a signal.
    pub exit_code: Option<i32>,
    pub success: bool,
    pub stdout: String,
    pub stderr: String,
}

/// Result of executing a batch's command sequence.
#[derive(Debug, Clone, Serialize)]
pub struct BatchExecution {
    pub name: String,
    pub total_commands: usize,
    /// Commands actually run (≤ total under fail-fast).
    pub executed: usize,
    pub outcomes: Vec<CommandOutcome>,
    /// True when every command ran and exited 0.
    pub all_succeeded: bool,
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
        // An empty or whitespace-only store file is an absent store, not corruption.
        if content.trim().is_empty() {
            return Ok(BatchStore::default());
        }
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
        let lines = batch.commands.iter().map(Self::command_line).collect();
        Ok(lines)
    }

    /// Reconstruct the `lsp-max-cli` command line for one batch command.
    fn command_line(cmd: &BatchCommand) -> String {
        let mut parts = vec![
            "lsp-max-cli".to_string(),
            cmd.noun.clone(),
            cmd.verb.clone(),
        ];
        parts.extend(cmd.args.iter().cloned());
        parts.join(" ")
    }

    /// Execute a batch by spawning each command as a child of `program`,
    /// stopping at the first failure (fail-fast). At runtime `program` is the
    /// `lsp-max-cli` binary; the registry mutex is held for the duration of the
    /// parent verb, so re-entering the CLI in-process would deadlock — a child
    /// process is the only safe dispatch path. Tests inject a harmless stand-in
    /// program to exercise the spawn plumbing without re-entering the harness.
    pub fn execute_with<P: AsRef<std::ffi::OsStr>>(
        &self,
        name: &str,
        program: P,
    ) -> std::result::Result<BatchExecution, String> {
        let batch = self.show(name)?;
        let total_commands = batch.commands.len();
        let mut outcomes = Vec::new();
        let mut all_succeeded = true;
        for cmd in &batch.commands {
            let output = std::process::Command::new(program.as_ref())
                .arg(&cmd.noun)
                .arg(&cmd.verb)
                .args(&cmd.args)
                .output()
                .map_err(|e| format!("spawn failed: {e}"))?;
            let success = output.status.success();
            outcomes.push(CommandOutcome {
                command: Self::command_line(cmd),
                exit_code: output.status.code(),
                success,
                stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
                stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
            });
            if !success {
                all_succeeded = false;
                break;
            }
        }
        Ok(BatchExecution {
            name: name.to_string(),
            total_commands,
            executed: outcomes.len(),
            outcomes,
            all_succeeded,
        })
    }

    /// Execute a batch against the current `lsp-max-cli` binary.
    pub fn execute(&self, name: &str) -> std::result::Result<BatchExecution, String> {
        let exe = std::env::current_exe().map_err(|e| e.to_string())?;
        self.execute_with(name, exe)
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
        note: "DRY_RUN: commands listed but not executed — use `batch exec <name>` to run them"
            .to_string(),
    })
}

#[derive(Serialize)]
pub struct BatchExecResult {
    pub execution: BatchExecution,
    /// ADMITTED when all commands ran and exited 0; PARTIAL otherwise.
    pub status: String,
}

/// Execute a batch: run each command as a child `lsp-max-cli` process,
/// stopping at the first failure (fail-fast).
#[verb("exec")]
pub fn exec(name: String) -> Result<BatchExecResult> {
    let svc = BatchService::new();
    let execution = svc.execute(&name).map_err(NounVerbError::execution_error)?;
    let status = if execution.all_succeeded {
        "ADMITTED"
    } else {
        "PARTIAL"
    }
    .to_string();
    Ok(BatchExecResult { execution, status })
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

    #[test]
    fn execute_with_runs_all_commands_on_success() {
        let (_f, svc) = make_temp_service();
        svc.create("seq", None).unwrap();
        svc.add_command("seq", "gate", "check", None).unwrap();
        svc.add_command("seq", "gate", "list", None).unwrap();
        // `true` ignores its arguments and exits 0 — exercises spawn plumbing.
        let exec = svc.execute_with("seq", "true").unwrap();
        assert_eq!(exec.total_commands, 2);
        assert_eq!(exec.executed, 2);
        assert!(exec.all_succeeded);
        assert!(exec.outcomes.iter().all(|o| o.success));
    }

    #[test]
    fn execute_with_fail_fast_stops_at_first_failure() {
        let (_f, svc) = make_temp_service();
        svc.create("seq", None).unwrap();
        svc.add_command("seq", "a", "b", None).unwrap();
        svc.add_command("seq", "c", "d", None).unwrap();
        // `false` exits 1 — fail-fast must stop after the first command.
        let exec = svc.execute_with("seq", "false").unwrap();
        assert_eq!(exec.total_commands, 2);
        assert_eq!(
            exec.executed, 1,
            "fail-fast stops after the first failing command"
        );
        assert!(!exec.all_succeeded);
    }

    #[test]
    fn execute_with_unknown_batch_returns_err() {
        let (_f, svc) = make_temp_service();
        let result = svc.execute_with("ghost", "true");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("BATCH_NOT_FOUND"));
    }

    #[test]
    fn execute_empty_batch_succeeds_vacuously() {
        let (_f, svc) = make_temp_service();
        svc.create("empty", None).unwrap();
        let exec = svc.execute_with("empty", "true").unwrap();
        assert_eq!(exec.executed, 0);
        assert!(exec.all_succeeded);
    }
}
