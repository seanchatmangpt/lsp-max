use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use std::process::Command;

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::identity::{digest_bytes, digest_parts};
use crate::receipt::{Outcome, Receipt, ReceiptChain, ReceiptKind};

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkspaceState {
    files: BTreeMap<String, String>,
}

impl WorkspaceState {
    pub fn new(files: BTreeMap<String, String>) -> Self {
        Self { files }
    }

    pub fn files(&self) -> &BTreeMap<String, String> {
        &self.files
    }

    pub fn get(&self, path: &str) -> Option<&str> {
        self.files.get(path).map(String::as_str)
    }

    pub fn insert_observation(&mut self, path: impl Into<String>, content: impl Into<String>) {
        self.files.insert(path.into(), content.into());
    }

    pub fn remove_observation(&mut self, path: &str) {
        self.files.remove(path);
    }

    pub fn digest(&self) -> String {
        let mut hasher = blake3::Hasher::new();
        hasher.update(b"ra-max/workspace-state/v1\0");
        for (path, content) in &self.files {
            hasher.update(path.as_bytes());
            hasher.update(&[0]);
            hasher.update(digest_bytes(content.as_bytes()).as_bytes());
            hasher.update(&[0]);
        }
        hasher.finalize().to_hex().to_string()
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConstructedEdit {
    pub path: String,
    pub subject_revision: String,
    pub before_hash: String,
    pub after_hash: String,
    pub replacement: String,
    pub construction_receipt: Receipt,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AppliedEdit {
    pub before_workspace_hash: String,
    pub after_workspace_hash: String,
    pub authorization_receipt: Receipt,
    pub consequence_receipt: Receipt,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolExecution {
    pub program: String,
    pub args: Vec<String>,
    pub status_code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
    pub authorization_receipt: Receipt,
    pub consequence_receipt: Receipt,
}

#[derive(Debug, Error)]
pub enum BrokerError {
    #[error("workspace path is not admitted: {0}")]
    MissingPath(String),
    #[error("constructed edit is stale for {path}: expected {expected}, observed {observed}")]
    StaleEdit {
        path: String,
        expected: String,
        observed: String,
    },
    #[error("tool is outside the broker allowlist: {0}")]
    ToolRefused(String),
    #[error("tool execution could not start: {0}")]
    ToolSpawn(String),
}

/// Exclusive DO path for the vertical slice.
#[derive(Clone, Debug)]
pub struct ActuationBroker {
    chain: ReceiptChain,
    allowed_tools: BTreeSet<String>,
}

impl Default for ActuationBroker {
    fn default() -> Self {
        Self::new(["rustc", "cargo"])
    }
}

impl ActuationBroker {
    pub fn new(tools: impl IntoIterator<Item = impl Into<String>>) -> Self {
        Self {
            chain: ReceiptChain::default(),
            allowed_tools: tools.into_iter().map(Into::into).collect(),
        }
    }

    pub fn receipts(&self) -> &[Receipt] {
        self.chain.receipts()
    }

    pub fn verify_receipts(&self) -> bool {
        self.chain.verify()
    }

    /// CONSTRUCT only: this manufactures a candidate and a construction receipt.
    /// It cannot mutate the workspace.
    pub fn construct_edit(
        &mut self,
        workspace: &WorkspaceState,
        subject_revision: impl Into<String>,
        path: impl Into<String>,
        replacement: impl Into<String>,
    ) -> Result<ConstructedEdit, BrokerError> {
        let subject_revision = subject_revision.into();
        let path = path.into();
        let replacement = replacement.into();
        let before = workspace
            .get(&path)
            .ok_or_else(|| BrokerError::MissingPath(path.clone()))?;
        let before_hash = digest_bytes(before.as_bytes());
        let after_hash = digest_bytes(replacement.as_bytes());
        let intent_hash = digest_parts([
            b"ra-max/construct-edit/v1".as_slice(),
            path.as_bytes(),
            before_hash.as_bytes(),
            after_hash.as_bytes(),
        ]);
        let construction_receipt = self.chain.append(
            ReceiptKind::Construction,
            subject_revision.clone(),
            intent_hash,
            after_hash.clone(),
            Outcome::Constructed,
        );

        Ok(ConstructedEdit {
            path,
            subject_revision,
            before_hash,
            after_hash,
            replacement,
            construction_receipt,
        })
    }

    /// DO: authorization is receipted before the in-memory consequence occurs.
    pub fn apply_edit(
        &mut self,
        workspace: &mut WorkspaceState,
        edit: &ConstructedEdit,
    ) -> Result<AppliedEdit, BrokerError> {
        let observed = workspace
            .get(&edit.path)
            .ok_or_else(|| BrokerError::MissingPath(edit.path.clone()))?;
        let observed_hash = digest_bytes(observed.as_bytes());
        let intent_bytes = serde_json::to_vec(edit).unwrap_or_default();
        let intent_hash = digest_bytes(&intent_bytes);

        if observed_hash != edit.before_hash {
            self.chain.append(
                ReceiptKind::Refusal,
                edit.subject_revision.clone(),
                intent_hash,
                observed_hash.clone(),
                Outcome::Refused,
            );
            return Err(BrokerError::StaleEdit {
                path: edit.path.clone(),
                expected: edit.before_hash.clone(),
                observed: observed_hash,
            });
        }

        let before_workspace_hash = workspace.digest();
        let authorization_receipt = self.chain.append(
            ReceiptKind::Authorization,
            edit.subject_revision.clone(),
            intent_hash.clone(),
            edit.after_hash.clone(),
            Outcome::Authorized,
        );

        workspace
            .files
            .insert(edit.path.clone(), edit.replacement.clone());
        let after_workspace_hash = workspace.digest();
        let consequence_receipt = self.chain.append(
            ReceiptKind::Consequence,
            edit.subject_revision.clone(),
            intent_hash,
            after_workspace_hash.clone(),
            Outcome::Executed,
        );

        Ok(AppliedEdit {
            before_workspace_hash,
            after_workspace_hash,
            authorization_receipt,
            consequence_receipt,
        })
    }

    /// DO: execute an allowlisted tool.  An authorization receipt is committed
    /// before `Command::output`; a consequence receipt binds status/stdout/stderr.
    pub fn execute_tool(
        &mut self,
        subject_hash: impl Into<String>,
        program: impl Into<String>,
        args: &[String],
        current_dir: Option<&Path>,
    ) -> Result<ToolExecution, BrokerError> {
        let subject_hash = subject_hash.into();
        let program = program.into();
        let basename = Path::new(&program)
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or(&program)
            .trim_end_matches(".exe");
        let args_bytes = serde_json::to_vec(args).unwrap_or_default();
        let intent_hash = digest_parts([
            b"ra-max/tool-intent/v1".as_slice(),
            basename.as_bytes(),
            args_bytes.as_slice(),
        ]);

        if !self.allowed_tools.contains(basename) {
            self.chain.append(
                ReceiptKind::Refusal,
                subject_hash,
                intent_hash,
                digest_bytes(program.as_bytes()),
                Outcome::Refused,
            );
            return Err(BrokerError::ToolRefused(program));
        }

        let authorization_receipt = self.chain.append(
            ReceiptKind::Authorization,
            subject_hash.clone(),
            intent_hash.clone(),
            digest_bytes(basename.as_bytes()),
            Outcome::Authorized,
        );

        let mut command = Command::new(&program);
        command.args(args);
        if let Some(directory) = current_dir {
            command.current_dir(directory);
        }
        let output = match command.output() {
            Ok(output) => output,
            Err(error) => {
                self.chain.append(
                    ReceiptKind::Consequence,
                    subject_hash,
                    intent_hash,
                    digest_bytes(error.to_string().as_bytes()),
                    Outcome::Refused,
                );
                return Err(BrokerError::ToolSpawn(error.to_string()));
            }
        };

        let status_code = output.status.code();
        let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
        let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
        let status_bytes = status_code.unwrap_or(i32::MIN).to_le_bytes();
        let consequence_hash = digest_parts([
            b"ra-max/tool-consequence/v1".as_slice(),
            status_bytes.as_slice(),
            stdout.as_bytes(),
            stderr.as_bytes(),
        ]);
        let consequence_receipt = self.chain.append(
            ReceiptKind::Consequence,
            subject_hash,
            intent_hash,
            consequence_hash,
            Outcome::Executed,
        );

        Ok(ToolExecution {
            program,
            args: args.to_vec(),
            status_code,
            stdout,
            stderr,
            authorization_receipt,
            consequence_receipt,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn workspace() -> WorkspaceState {
        WorkspaceState::new(BTreeMap::from([(
            "src/main.rs".to_owned(),
            "fn main() { let value = 1; }\n".to_owned(),
        )]))
    }

    #[test]
    fn construction_has_no_mutation_authority() {
        let mut broker = ActuationBroker::default();
        let workspace = workspace();
        let before = workspace.digest();
        let edit = broker
            .construct_edit(
                &workspace,
                "revision-1",
                "src/main.rs",
                "fn main() { let value = 2; }\n",
            )
            .expect("construction should be admitted");

        assert_eq!(workspace.digest(), before);
        assert_ne!(edit.before_hash, edit.after_hash);
        assert_eq!(broker.receipts().len(), 1);
        assert_eq!(broker.receipts()[0].kind, ReceiptKind::Construction);
    }

    #[test]
    fn broker_authorizes_before_applying_and_binds_consequence() {
        let mut broker = ActuationBroker::default();
        let mut workspace = workspace();
        let edit = broker
            .construct_edit(
                &workspace,
                "revision-1",
                "src/main.rs",
                "fn main() { let value = 2; }\n",
            )
            .expect("construction should be admitted");
        let applied = broker
            .apply_edit(&mut workspace, &edit)
            .expect("fresh edit should execute");

        assert_eq!(applied.authorization_receipt.outcome, Outcome::Authorized);
        assert_eq!(applied.consequence_receipt.outcome, Outcome::Executed);
        assert_eq!(
            workspace.get("src/main.rs"),
            Some("fn main() { let value = 2; }\n")
        );
        assert!(broker.verify_receipts());
    }

    #[test]
    fn stale_edit_is_refused_without_mutation() {
        let mut broker = ActuationBroker::default();
        let mut workspace = workspace();
        let edit = broker
            .construct_edit(&workspace, "revision-1", "src/main.rs", "fn main() {}\n")
            .expect("construction should be admitted");
        workspace.insert_observation("src/main.rs", "fn main() { /* drift */ }\n");
        let drift_hash = workspace.digest();

        assert!(matches!(
            broker.apply_edit(&mut workspace, &edit),
            Err(BrokerError::StaleEdit { .. })
        ));
        assert_eq!(workspace.digest(), drift_hash);
        assert_eq!(
            broker.receipts().last().map(|r| r.outcome),
            Some(Outcome::Refused)
        );
    }

    #[test]
    fn actual_rust_compiler_execution_is_brokered_and_receipted() {
        let mut broker = ActuationBroker::default();
        let rustc = std::env::var("RUSTC").unwrap_or_else(|_| "rustc".to_owned());
        let execution = broker
            .execute_tool("toolchain-subject", rustc, &["--version".to_owned()], None)
            .expect("the test is running under an accessible Rust toolchain");

        assert_eq!(execution.status_code, Some(0));
        assert!(execution.stdout.starts_with("rustc "));
        assert_eq!(execution.authorization_receipt.outcome, Outcome::Authorized);
        assert_eq!(execution.consequence_receipt.outcome, Outcome::Executed);
        assert!(broker.verify_receipts());
    }
}
