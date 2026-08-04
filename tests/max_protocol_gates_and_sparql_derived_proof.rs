//! Gall checkpoint CP10 (see ~/ggen's `80-20-gall-test-refactor-cheerful-quokka.md` plan):
//! admission gate for `ontology/max-protocol.ttl`'s `lsp:Notification` individuals
//! (mirroring `packs/lsp-max-pack/gates/{010,020,030}.rq`'s and lsif06's
//! `030_value_constraints.rq`'s translated-from-shapes pattern) plus the SPARQL-derived
//! second proof (re-query live, cross-check against the companion hand-transcribed proof in
//! `max_protocol_notification_fidelity_proof.rs` via an independently-shaped query, not the
//! same one) and a drift-injection-and-revert test proving the gate actually detects the
//! exact regression class CP10 fixed.
//!
//! `oxigraph::*` use here is in bounds per
//! `src/runtime/control_plane/semantic_graph/store.rs`'s own `OXIGRAPH_BOUNDARY_HELD` invariant,
//! which is explicitly scoped to production code ("outside tests").

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use oxigraph::sparql::{QueryResults, SparqlEvaluator};
use oxigraph::store::Store;

const MAX_PROTOCOL_TTL: &str = include_str!("../ontology/max-protocol.ttl");
const GATE_040: &str = include_str!("../ontology/gates/040_notification_wire_name_prefix.rq");

fn store_from_ttl(ttl: &str) -> Store {
    let store = Store::new().expect("in-memory oxigraph store");
    store
        .load_from_reader(oxigraph::io::RdfFormat::Turtle, ttl.as_bytes())
        .expect("ttl must parse");
    store
}

fn gate_violation_count(store: &Store, gate_sparql: &str) -> usize {
    let evaluator = SparqlEvaluator::new();
    let query = evaluator
        .parse_query(gate_sparql)
        .expect("gate query must parse");
    let QueryResults::Solutions(solutions) = query.on_store(store).execute().expect("eval")
    else {
        panic!("expected SELECT results");
    };
    solutions.count()
}

#[test]
fn gate_040_passes_zero_violations_on_the_corrected_ontology() {
    let store = store_from_ttl(MAX_PROTOCOL_TTL);
    assert_eq!(
        gate_violation_count(&store, GATE_040),
        0,
        "040_notification_wire_name_prefix.rq found an lsp:Notification individual whose \
         lsp:methodName does not start with the real lspMax/ wire prefix"
    );
}

/// SPARQL-derived second proof: an independently-shaped aggregate query (COUNT, not the
/// companion test's per-individual enumeration) re-derives the same total the hand-transcribed
/// proof in `max_protocol_notification_fidelity_proof.rs` asserts. Two differently-shaped
/// queries agreeing is stronger evidence than one query run twice.
#[test]
fn sparql_derived_lsp_max_notification_count_agrees_with_hand_transcribed_proof() {
    let store = store_from_ttl(MAX_PROTOCOL_TTL);
    let sparql = r#"
        PREFIX lsp: <https://lsp-max.rs/ontology/>
        SELECT (COUNT(DISTINCT ?individual) AS ?notificationCount)
        WHERE {
            ?individual a lsp:Notification ;
                        lsp:methodName ?methodName .
            FILTER(STRSTARTS(STR(?methodName), "lspMax/"))
        }
    "#;
    let evaluator = SparqlEvaluator::new();
    let query = evaluator.parse_query(sparql).expect("query must parse");
    let QueryResults::Solutions(mut solutions) = query.on_store(&store).execute().expect("eval")
    else {
        panic!("expected SELECT results");
    };
    let row = solutions.next().expect("one row").expect("solution");
    let count: usize = row
        .get("notificationCount")
        .expect("bound")
        .to_string()
        .trim_start_matches('"')
        .split('"')
        .next()
        .unwrap()
        .parse()
        .expect("integer literal");
    assert_eq!(
        count, 3,
        "live COUNT-aggregate query disagrees with the hand-transcribed proof's 3 lspMax/* \
         Notification individuals"
    );
}

/// Drift-injection-and-revert: deliberately corrupt one real notification's methodName back to
/// the pre-CP10 bare `max/*` form, confirm gate 040 now refuses it, then confirm the real
/// on-disk file (never mutated -- this test only mutates an in-memory string copy) still
/// passes cleanly, proving CP10's fix is real and gate 040 actually detects its regression
/// rather than trivially always passing.
#[test]
fn gate_040_refuses_a_deliberately_reintroduced_pre_cp10_bare_max_slash_name() {
    let corrupted = MAX_PROTOCOL_TTL.replacen(
        r#"lsp:methodName "lspMax/admissionChanged" ;"#,
        r#"lsp:methodName "max/admissionChanged" ;"#,
        1,
    );
    assert_ne!(
        corrupted, MAX_PROTOCOL_TTL,
        "the replacen target string must actually exist in max-protocol.ttl for this \
         injection to be real"
    );

    let corrupted_store = store_from_ttl(&corrupted);
    assert!(
        gate_violation_count(&corrupted_store, GATE_040) > 0,
        "gate 040 must refuse a Notification whose methodName lost the lspMax/ prefix -- \
         exactly the class of bug CP10 fixed"
    );

    // Revert is implicit: `corrupted` was a local String, MAX_PROTOCOL_TTL (the real file,
    // read via include_str!) was never touched. Confirm it still passes.
    let real_store = store_from_ttl(MAX_PROTOCOL_TTL);
    assert_eq!(
        gate_violation_count(&real_store, GATE_040),
        0,
        "the real on-disk max-protocol.ttl must still pass gate 040 cleanly after the \
         in-memory-only drift injection above"
    );
}
