use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::broker::{ActuationBroker, BrokerError, WorkspaceState};
use crate::differential::{compare_engines, DifferentialReport};
use crate::identity::{digest_bytes, ProjectAdmission, SemanticSubject};
use crate::intelligence::{IntelligenceError, WorkspaceIndex};
use crate::receipt::{Outcome, Receipt, ReceiptChain, ReceiptKind};
use crate::semantic::{SemanticEngine, TreeSitterRustEngine};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DemoReport {
    pub initial_project_hash: String,
    pub initial_semantic_revision: String,
    pub final_project_hash: String,
    pub final_semantic_revision: String,
    pub workspace_index_hash: String,
    pub workspace_index_valid: bool,
    pub symbols: Vec<String>,
    pub diagnostics: Vec<String>,
    pub hover_signature: String,
    pub definition_path: String,
    pub reference_count: usize,
    pub completion_labels: Vec<String>,
    pub rename_plan_hash: String,
    pub rename_edit_count: usize,
    pub lexical_only: bool,
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
    #[error(transparent)]
    Intelligence(#[from] IntelligenceError),
    #[error("demo fixture is missing: {0}")]
    Fixture(String),
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

    let index = WorkspaceIndex::build(&final_admission, &final_snapshot, workspace.files());
    let main_source = workspace
        .get("src/main.rs")
        .ok_or_else(|| DemoError::Fixture("src/main.rs".to_owned()))?;
    let (call_line, call_column) = nth_position(main_source, "meaning", 1)
        .ok_or_else(|| DemoError::Fixture("meaning call".to_owned()))?;
    let hover = index.hover_at("src/main.rs", call_line, call_column)?;
    let definition = index.definition_at("src/main.rs", call_line, call_column)?;
    let references = index.references_at("src/main.rs", call_line, call_column, true)?;
    let completions = index.completions("src/main.rs", call_line, call_column + 4, 20)?;
    let rename = index.rename_plan("src/main.rs", call_line, call_column, "ultimate_meaning")?;

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
        workspace_index_hash: index.index_hash.clone(),
        workspace_index_valid: index.verify(),
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
        hover_signature: hover.symbol.signature,
        definition_path: definition.path,
        reference_count: references.len(),
        completion_labels: completions
            .into_iter()
            .map(|candidate| candidate.label)
            .collect(),
        rename_plan_hash: rename.plan_hash,
        rename_edit_count: rename.edits.values().map(Vec::len).sum(),
        lexical_only: rename.lexical_only,
        rustc_version: tool.stdout.trim().to_owned(),
        differential,
        semantic_receipts: semantic_receipts.receipts().to_vec(),
        broker_receipts: broker.receipts().to_vec(),
        receipt_chains_valid: semantic_receipts.verify() && broker.verify_receipts(),
    })
}

fn nth_position(source: &str, needle: &str, occurrence: usize) -> Option<(u32, u32)> {
    let byte = source.match_indices(needle).nth(occurrence)?.0;
    let before = &source[..byte];
    let line = before.bytes().filter(|byte| *byte == b'\n').count() as u32;
    let column = before
        .rsplit_once('\n')
        .map_or(before.len(), |(_, tail)| tail.len()) as u32;
    Some((line, column))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn demo_executes_real_rust_and_editor_intelligence() {
        let report = run_demo().expect("demo should execute under the active Rust toolchain");

        assert_ne!(report.initial_project_hash, report.final_project_hash);
        assert_ne!(
            report.initial_semantic_revision,
            report.final_semantic_revision
        );
        assert!(report.rustc_version.starts_with("rustc "));
        assert!(report.differential.equivalent);
        assert!(report.receipt_chains_valid);
        assert!(report.workspace_index_valid);
        assert_eq!(report.definition_path, "src/main.rs");
        assert_eq!(report.reference_count, 2);
        assert_eq!(report.rename_edit_count, 2);
        assert!(report
            .completion_labels
            .iter()
            .any(|label| label == "meaning"));
        assert!(report.hover_signature.starts_with("fn meaning"));
        assert!(report.lexical_only);
        assert!(report
            .symbols
            .iter()
            .any(|symbol| symbol.ends_with(":function:meaning")));
        assert!(report.diagnostics.is_empty());
    }
}
