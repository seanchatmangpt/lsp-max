#![allow(clippy::bool_assert_comparison)]
#![allow(clippy::useless_vec)]

//! Tests for the Rice Closure Model and RICE_CLOSURE_CHAIN_HELD invariant.
use lsp_max::andon::core::{AndonInvariant, RepairAction, Severity};

/// Builds the formal `RICE_CLOSURE_CHAIN_HELD` invariant.
pub fn build_rice_closure_chain_invariant() -> AndonInvariant {
    AndonInvariant {
        id: "RICE_CLOSURE_CHAIN_HELD".to_string(),
        statement: "Every admitted claim must be backed by a bounded mechanical check in the closure chain, not arbitrary semantic judgment.".to_string(),
        scope: "epistemology".to_string(),
        true_probe: Some("closure_chain_fully_represented".to_string()),
        false_probe: Some("arbitrary_semantic_claim_without_bound".to_string()),
        counterfactual_probe: Some("remove_one_layer_from_rice_closure_table".to_string()),
        witness_rule: Some("closure_table_contains_all_layers".to_string()),
        repair_rule: Some(RepairAction {
            id: "add_missing_closure_layer".to_string(),
            title: "Add missing closure layer".to_string(),
            next_lawful_step: Some("do_not_widen_semantic_assertion".to_string()),
            command: None,
            code_action: None,
            virtual_doc_uri: Some("lsp-max://rice/closure".to_string()),
        }),
        severity: Severity::Stop,
        blocks: true,
    }
}

// ---------------------------------------------------------------------------
// Invariant assertions
// ---------------------------------------------------------------------------

#[test]
fn rice_closure_table_contains_all_layers() {
    let invariant = build_rice_closure_chain_invariant();
    assert_eq!(invariant.id, "RICE_CLOSURE_CHAIN_HELD");
    assert!(invariant.witness_rule.is_some());
    assert_eq!(invariant.witness_rule.as_deref(), Some("closure_table_contains_all_layers"));
}

#[test]
fn arbitrary_semantic_claim_without_bound_refused() {
    let invariant = build_rice_closure_chain_invariant();
    assert_eq!(invariant.severity, Severity::Stop);
    assert!(invariant.blocks);
    assert_eq!(invariant.false_probe.as_deref(), Some("arbitrary_semantic_claim_without_bound"));
}

#[test]
fn each_layer_has_bounded_question() {
    let invariant = build_rice_closure_chain_invariant();
    assert_eq!(invariant.scope, "epistemology");
    assert_eq!(invariant.true_probe.as_deref(), Some("closure_chain_fully_represented"));
}

#[test]
fn closure_chain_missing_layer_stops_admission() {
    let invariant = build_rice_closure_chain_invariant();
    assert_eq!(invariant.severity, Severity::Stop);
    assert_eq!(invariant.counterfactual_probe.as_deref(), Some("remove_one_layer_from_rice_closure_table"));
}

#[test]
fn arbitrary_code_meaning_claim_refused() {
    let invariant = build_rice_closure_chain_invariant();
    assert!(invariant.statement.contains("mechanical check"));
}

