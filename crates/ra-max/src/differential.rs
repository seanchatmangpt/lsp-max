use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::identity::{digest_bytes, digest_parts, ProjectAdmission};
use crate::receipt::{Outcome, Receipt, ReceiptChain, ReceiptKind};
use crate::semantic::{SemanticEngine, SemanticSnapshot};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DifferentialReport {
    pub subject_hash: String,
    pub baseline_engine: String,
    pub candidate_engine: String,
    pub baseline_revision: String,
    pub candidate_revision: String,
    pub baseline_semantic_hash: String,
    pub candidate_semantic_hash: String,
    pub equivalent: bool,
    pub mismatches: Vec<String>,
    pub receipt: Receipt,
}

pub fn compare_engines(
    baseline: &dyn SemanticEngine,
    candidate: &dyn SemanticEngine,
    admission: &ProjectAdmission,
    files: &BTreeMap<String, String>,
    receipts: &mut ReceiptChain,
) -> DifferentialReport {
    let baseline_snapshot = baseline.analyze(admission, files);
    let candidate_snapshot = candidate.analyze(admission, files);
    let baseline_semantic_hash = semantic_hash(&baseline_snapshot);
    let candidate_semantic_hash = semantic_hash(&candidate_snapshot);
    let equivalent = baseline_semantic_hash == candidate_semantic_hash;
    let mut mismatches = Vec::new();

    if baseline_snapshot.diagnostics != candidate_snapshot.diagnostics {
        mismatches.push(format!(
            "diagnostics differ: baseline={} candidate={}",
            baseline_snapshot.diagnostics.len(),
            candidate_snapshot.diagnostics.len()
        ));
    }
    if baseline_snapshot.symbols != candidate_snapshot.symbols {
        mismatches.push(format!(
            "symbols differ: baseline={} candidate={}",
            baseline_snapshot.symbols.len(),
            candidate_snapshot.symbols.len()
        ));
    }

    let intent_hash = digest_parts([
        b"ra-max/differential/v1".as_slice(),
        baseline.name().as_bytes(),
        candidate.name().as_bytes(),
        admission.project_hash.as_bytes(),
    ]);
    let consequence_hash = digest_parts([
        baseline_semantic_hash.as_bytes(),
        candidate_semantic_hash.as_bytes(),
    ]);
    let receipt = receipts.append(
        ReceiptKind::Differential,
        admission.project_hash.clone(),
        intent_hash,
        consequence_hash,
        if equivalent {
            Outcome::Equivalent
        } else {
            Outcome::Divergent
        },
    );

    DifferentialReport {
        subject_hash: admission.project_hash.clone(),
        baseline_engine: baseline.name().to_owned(),
        candidate_engine: candidate.name().to_owned(),
        baseline_revision: baseline_snapshot.revision_hash,
        candidate_revision: candidate_snapshot.revision_hash,
        baseline_semantic_hash,
        candidate_semantic_hash,
        equivalent,
        mismatches,
        receipt,
    }
}

fn semantic_hash(snapshot: &SemanticSnapshot) -> String {
    let normalized = serde_json::to_vec(&(
        snapshot.admission_hash.as_str(),
        &snapshot.diagnostics,
        &snapshot.symbols,
    ))
    .unwrap_or_default();
    digest_bytes(&normalized)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::identity::SemanticSubject;
    use crate::semantic::{SemanticDiagnostic, Symbol, TreeSitterRustEngine};

    struct DivergentEngine;

    impl SemanticEngine for DivergentEngine {
        fn name(&self) -> &'static str {
            "deliberately-divergent-test-engine"
        }

        fn analyze(
            &self,
            admission: &ProjectAdmission,
            files: &BTreeMap<String, String>,
        ) -> SemanticSnapshot {
            let mut snapshot = TreeSitterRustEngine.analyze(admission, files);
            snapshot.symbols.clear();
            snapshot.revision_hash = digest_bytes(b"divergent");
            snapshot
        }
    }

    fn subject() -> (ProjectAdmission, BTreeMap<String, String>) {
        let files = BTreeMap::from([("src/lib.rs".to_owned(), "pub fn receipt() {}\n".to_owned())]);
        let admission = ProjectAdmission::from_files(
            SemanticSubject::tree_sitter_vertical_slice(),
            "/workspace",
            &files,
        );
        (admission, files)
    }

    #[test]
    fn exact_same_engine_is_equivalent() {
        let (admission, files) = subject();
        let mut receipts = ReceiptChain::default();
        let report = compare_engines(
            &TreeSitterRustEngine,
            &TreeSitterRustEngine,
            &admission,
            &files,
            &mut receipts,
        );

        assert!(report.equivalent);
        assert!(report.mismatches.is_empty());
        assert_eq!(report.receipt.outcome, Outcome::Equivalent);
        assert!(receipts.verify());
    }

    #[test]
    fn one_missing_symbol_blocks_equivalence() {
        let (admission, files) = subject();
        let mut receipts = ReceiptChain::default();
        let report = compare_engines(
            &TreeSitterRustEngine,
            &DivergentEngine,
            &admission,
            &files,
            &mut receipts,
        );

        assert!(!report.equivalent);
        assert!(report
            .mismatches
            .iter()
            .any(|item| item.starts_with("symbols differ")));
        assert_eq!(report.receipt.outcome, Outcome::Divergent);
    }

    #[allow(dead_code)]
    fn _type_witness(_: SemanticDiagnostic, _: Symbol) {}
}
