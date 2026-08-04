use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::broker::{ActuationBroker, BrokerError, WorkspaceState};
use crate::differential::{compare_engines, DifferentialReport};
use crate::identity::{digest_bytes, ProjectAdmission, SemanticSubject};
use crate::receipt::{Outcome, Receipt, ReceiptChain, ReceiptKind};
use crate::semantic::{SemanticEngine, TreeSitterRustEngine};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DemoReport {
    pub initial_project_hash: String,
    pub initial_semantic_revision: String,
    pub final_project_hash: String,
    pub final_semantic_revision: String,
    pub symbols: Vec<String>,
    pub diagnostics: Vec<String>,
    pub rustc_version: String,
    pub differential: DifferentialReport,
    pub semantic_receipts: Vec<Receipt>,
    pub broker_receipts: Vec<Receipt>,
    pub receipt_chains_valid: bool,
}

#[derive(Debug, Error)]
pub enum DemoError {
    #[error(transparent)]
    Broker(#[from] BrokerError),
}

pub fn run_demo() -> Result<DemoReport, DemoError> {
    let subject = SemanticSubject::tree_sitter_vertical_slice();
    let initial_files = BTreeMap::from([
        (
            "Cargo.toml".to_owned(),
            "[package]\nname = \"ra-max-demo\"\nversion = \"0.1.0\"\n".to_owned(),
        ),
        (
            "src/main.rs".to_owned(),
            "fn meaning() -> u32 { 41 }\nfn main() { let _ = meaning(); }\n".to_owned(),
        ),
    ]);
    let mut workspace = WorkspaceState::new(initial_files);
    let engine = TreeSitterRustEngine;
    let mut semantic_receipts = ReceiptChain::default();

    let initial_admission =
        ProjectAdmission::from_files(subject.clone(), "/virtual/ra-max-demo", workspace.files());
    semantic_receipts.append(
        ReceiptKind::Admission,
        subject.digest(),
        digest_bytes(b"admit demo project"),
        initial_admission.project_hash.clone(),
        Outcome::Admitted,
    );
    let initial_snapshot = engine.analyze(&initial_admission, workspace.files());
    semantic_receipts.append(
        ReceiptKind::SemanticRevision,
        initial_admission.project_hash.clone(),
        digest_bytes(engine.name().as_bytes()),
        initial_snapshot.revision_hash.clone(),
        Outcome::Executed,
    );

    let mut broker = ActuationBroker::default();
    let edit = broker.construct_edit(
        &workspace,
        initial_snapshot.revision_hash.clone(),
        "src/main.rs",
        "fn meaning() -> u32 { 42 }\nfn main() { let answer = meaning(); assert_eq!(answer, 42); }\n",
    )?;
    broker.apply_edit(&mut workspace, &edit)?;

    let final_admission =
        ProjectAdmission::from_files(subject, "/virtual/ra-max-demo", workspace.files());
    semantic_receipts.append(
        ReceiptKind::Admission,
        initial_admission.project_hash.clone(),
        digest_bytes(b"re-admit after brokered edit"),
        final_admission.project_hash.clone(),
        Outcome::Admitted,
    );
    let final_snapshot = engine.analyze(&final_admission, workspace.files());
    semantic_receipts.append(
        ReceiptKind::SemanticRevision,
        final_admission.project_hash.clone(),
        digest_bytes(engine.name().as_bytes()),
        final_snapshot.revision_hash.clone(),
        Outcome::Executed,
    );

    let differential = compare_engines(
        &engine,
        &TreeSitterRustEngine,
        &final_admission,
        workspace.files(),
        &mut semantic_receipts,
    );

    let rustc = std::env::var("RUSTC").unwrap_or_else(|_| "rustc".to_owned());
    let tool = broker.execute_tool(
        final_admission.project_hash.clone(),
        rustc,
        &["--version".to_owned()],
        None,
    )?;

    Ok(DemoReport {
        initial_project_hash: initial_admission.project_hash,
        initial_semantic_revision: initial_snapshot.revision_hash,
        final_project_hash: final_admission.project_hash,
        final_semantic_revision: final_snapshot.revision_hash,
        symbols: final_snapshot
            .symbols
            .iter()
            .map(|symbol| format!("{}:{}:{}", symbol.path, symbol.kind, symbol.name))
            .collect(),
        diagnostics: final_snapshot
            .diagnostics
            .iter()
            .map(|diagnostic| format!("{}:{}", diagnostic.code, diagnostic.message))
            .collect(),
        rustc_version: tool.stdout.trim().to_owned(),
        differential,
        semantic_receipts: semantic_receipts.receipts().to_vec(),
        broker_receipts: broker.receipts().to_vec(),
        receipt_chains_valid: semantic_receipts.verify() && broker.verify_receipts(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn demo_executes_real_rust_and_replays_receipts() {
        let report = run_demo().expect("demo should execute under the active Rust toolchain");

        assert_ne!(report.initial_project_hash, report.final_project_hash);
        assert_ne!(
            report.initial_semantic_revision,
            report.final_semantic_revision
        );
        assert!(report.rustc_version.starts_with("rustc "));
        assert!(report.differential.equivalent);
        assert!(report.receipt_chains_valid);
        assert!(report
            .symbols
            .iter()
            .any(|symbol| symbol.ends_with(":function:meaning")));
        assert!(report.diagnostics.is_empty());
    }
}
