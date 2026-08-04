//! Gall checkpoint CP10 (see ~/ggen's `80-20-gall-test-refactor-cheerful-quokka.md` plan):
//! proves `ontology/max-protocol.ttl`'s three `lsp:Notification` individuals now name the
//! real, working `lspMax/*` wire methods implemented in `src/andon/lsp.rs` (the `Notification`
//! impls) and sent via `src/service/client/lsp_methods.rs:406-430`'s wrapper methods, instead
//! of the pre-CP10 `max/gateChanged`/`max/admissionChanged`/`max/receiptArrived` names that had
//! zero Rust implementation anywhere.
//!
//! Deliberately hand-transcribed, NOT SPARQL-derived from the ontology itself (that would make
//! every assertion a tautology -- the companion `max_protocol_gates_and_sparql_derived_proof.rs`
//! test adds the SPARQL-derived second proof). The `EXPECTED_*` consts below were transcribed
//! directly from reading `src/andon/lsp.rs`'s `const METHOD: &'static str = "..."` associated
//! consts, not from this ontology file.
//!
//! `oxigraph::*` use here is in bounds per
//! `src/runtime/control_plane/semantic_graph/store.rs`'s own `OXIGRAPH_BOUNDARY_HELD` invariant,
//! which is explicitly scoped to production code ("outside tests").

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::collections::BTreeSet;

use oxigraph::sparql::{QueryResults, SparqlEvaluator};
use oxigraph::store::Store;

const MAX_PROTOCOL_TTL: &str = include_str!("../ontology/max-protocol.ttl");

/// The real `Notification::METHOD` wire names declared in `src/andon/lsp.rs`, restricted to
/// the three this ontology file declares individuals for (`LspMaxAndonRaised`,
/// `LspMaxTruthTableChanged`, and `LspMaxCounterfactualFailed` are real and wired too, but this
/// ontology file never declared individuals for them -- adding those is future work, not part
/// of this fidelity correction). Hand-transcribed from:
///   `impl Notification for LspMaxAdmissionChanged { const METHOD: &'static str = "lspMax/admissionChanged"; }`
const EXPECTED_METHOD_NAMES: &[&str] = &[
    "lspMax/admissionChanged",
    "lspMax/receiptArrived",
    "lspMax/gateChanged",
];

fn load_store() -> Store {
    let store = Store::new().expect("in-memory oxigraph store");
    store
        .load_from_reader(oxigraph::io::RdfFormat::Turtle, MAX_PROTOCOL_TTL.as_bytes())
        .expect("max-protocol.ttl must parse as valid Turtle");
    store
}

fn select_notification_method_names(store: &Store) -> BTreeSet<String> {
    let sparql = r#"
        PREFIX lsp: <https://lsp-max.rs/ontology/>
        SELECT ?methodName WHERE {
            ?individual a lsp:Notification ;
                        lsp:methodName ?methodName .
            FILTER(STRSTARTS(STR(?methodName), "lspMax/"))
        }
    "#;
    let evaluator = SparqlEvaluator::new();
    let query = evaluator.parse_query(sparql).expect("query must parse");
    let QueryResults::Solutions(solutions) = query.on_store(store).execute().expect("eval")
    else {
        panic!("expected SELECT results");
    };
    solutions
        .map(|s| {
            let s = s.expect("solution");
            s.get("methodName")
                .expect("?methodName bound")
                .to_string()
                .trim_matches('"')
                .to_string()
        })
        .collect()
}

#[test]
fn max_protocol_ttl_parses_as_valid_turtle() {
    let _store = load_store();
}

#[test]
fn lsp_max_notification_method_names_match_the_real_andon_lsp_rs_consts_exactly() {
    let store = load_store();
    let declared = select_notification_method_names(&store);
    let expected: BTreeSet<String> = EXPECTED_METHOD_NAMES
        .iter()
        .map(|s| s.to_string())
        .collect();
    assert_eq!(
        declared, expected,
        "max-protocol.ttl's lspMax/* Notification individuals have drifted from src/andon/lsp.rs's \
         real `Notification::METHOD` consts (hand-transcribed EXPECTED_METHOD_NAMES)"
    );
}

#[test]
fn no_max_slash_notification_names_remain_in_max_protocol_ttl() {
    // The pre-CP10 bug: max/gateChanged, max/admissionChanged, max/receiptArrived had zero
    // Rust implementation anywhere. Confirm none of those three exact bare `max/*` strings
    // remain as an `lsp:Notification`'s `lsp:methodName` value.
    let store = load_store();
    let sparql = r#"
        PREFIX lsp: <https://lsp-max.rs/ontology/>
        SELECT ?methodName WHERE {
            ?individual a lsp:Notification ;
                        lsp:methodName ?methodName .
            FILTER(?methodName = "max/gateChanged" || ?methodName = "max/admissionChanged" || ?methodName = "max/receiptArrived")
        }
    "#;
    let evaluator = SparqlEvaluator::new();
    let query = evaluator.parse_query(sparql).expect("query must parse");
    let QueryResults::Solutions(solutions) = query.on_store(&store).execute().expect("eval")
    else {
        panic!("expected SELECT results");
    };
    assert_eq!(
        solutions.count(),
        0,
        "max-protocol.ttl must not declare any Notification under the old, unimplemented \
         max/gateChanged | max/admissionChanged | max/receiptArrived names"
    );
}

#[test]
fn admission_changed_param_type_matches_the_real_rust_struct_name() {
    let store = load_store();
    let sparql = r#"
        PREFIX lsp: <https://lsp-max.rs/ontology/>
        SELECT ?paramType WHERE {
            ?individual a lsp:Notification ;
                        lsp:methodName "lspMax/admissionChanged" ;
                        lsp:paramType ?paramType .
        }
    "#;
    let evaluator = SparqlEvaluator::new();
    let query = evaluator.parse_query(sparql).expect("query must parse");
    let QueryResults::Solutions(mut solutions) = query.on_store(&store).execute().expect("eval")
    else {
        panic!("expected SELECT results");
    };
    let row = solutions
        .next()
        .expect("one row for lspMax/admissionChanged")
        .expect("solution");
    let param_type = row
        .get("paramType")
        .expect("?paramType bound")
        .to_string();
    assert!(
        param_type.contains("AdmissionChangedParams"),
        "lspMax/admissionChanged's lsp:paramType must reference the real \
         crate::andon::lsp::AdmissionChangedParams struct, got {param_type}"
    );
}
