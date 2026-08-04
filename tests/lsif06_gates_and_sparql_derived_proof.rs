//! Gall checkpoint CP2 (see ~/ggen's `80-20-gall-test-refactor-cheerful-quokka.md`
//! plan): admission gates for `ontology/lsif06.ttl` (mirroring
//! `packs/lsp-max-pack/gates/{010,020,030}.rq`'s translated-from-shapes pattern)
//! plus the SPARQL-derived second proof (mirrors that pack's own documented
//! "cap05" pattern -- re-query live, cross-check against the CP1 hand-transcribed
//! proof via an independently-shaped query, not the same one).
//!
//! `oxigraph::*` use here is in bounds per
//! `src/runtime/control_plane/semantic_graph/store.rs`'s own
//! `OXIGRAPH_BOUNDARY_HELD` invariant, which is explicitly scoped to production
//! code ("outside tests").

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use oxigraph::sparql::{QueryResults, SparqlEvaluator};
use oxigraph::store::Store;

const LSIF06_TTL: &str = include_str!("../ontology/lsif06.ttl");
const GATE_010: &str = include_str!("../ontology/gates/010_required.rq");
const GATE_020: &str = include_str!("../ontology/gates/020_single_valued.rq");
const GATE_030: &str = include_str!("../ontology/gates/030_value_constraints.rq");

fn store_from_ttl(ttl: &str) -> Store {
    let store = Store::new().expect("in-memory oxigraph store");
    store
        .load_from_reader(oxigraph::io::RdfFormat::Turtle, ttl.as_bytes())
        .expect("ttl must parse");
    store
}

fn gate_violation_count(store: &Store, gate_sparql: &str) -> usize {
    let evaluator = SparqlEvaluator::new();
    let query = evaluator.parse_query(gate_sparql).expect("gate query must parse");
    let QueryResults::Solutions(solutions) = query.on_store(store).execute().expect("eval")
    else {
        panic!("expected SELECT results");
    };
    solutions.count()
}

#[test]
fn all_three_gates_pass_zero_violations_on_the_corrected_ontology() {
    let store = store_from_ttl(LSIF06_TTL);
    assert_eq!(
        gate_violation_count(&store, GATE_010),
        0,
        "010_required.rq found a Vertex/Edge subclass or ItemEdgeProperty individual \
         missing rdfs:label or lsif:wireLabel"
    );
    assert_eq!(
        gate_violation_count(&store, GATE_020),
        0,
        "020_single_valued.rq found a class with two distinct lsif:wireLabel values"
    );
    assert_eq!(
        gate_violation_count(&store, GATE_030),
        0,
        "030_value_constraints.rq found an empty wireLabel or a TextDocument*-named \
         edge class missing its real textDocument/ wire prefix"
    );
}

/// SPARQL-derived second proof: an independently-shaped aggregate query
/// (COUNT + GROUP, not CP1's per-class enumeration) re-derives the same
/// totals CP1's hand-transcribed proof asserts. Two differently-shaped
/// queries agreeing is stronger evidence than one query run twice.
#[test]
fn sparql_derived_vertex_and_edge_totals_agree_with_cp1s_hand_transcribed_proof() {
    let store = store_from_ttl(LSIF06_TTL);
    let sparql = r#"
        PREFIX rdfs: <http://www.w3.org/2000/01/rdf-schema#>
        PREFIX lsif: <https://lsp-max.rs/ontology/lsif/>
        SELECT (COUNT(DISTINCT ?vertex) AS ?vertexCount) (COUNT(DISTINCT ?edge) AS ?edgeCount)
        WHERE {
            OPTIONAL {
                ?vertex rdfs:subClassOf lsif:Vertex ; lsif:wireLabel ?vwl .
            }
            OPTIONAL {
                ?edge rdfs:subClassOf lsif:Edge ; lsif:wireLabel ?ewl .
            }
        }
    "#;
    let evaluator = SparqlEvaluator::new();
    let query = evaluator.parse_query(sparql).expect("query must parse");
    let QueryResults::Solutions(mut solutions) = query.on_store(&store).execute().expect("eval")
    else {
        panic!("expected SELECT results");
    };
    let row = solutions.next().expect("one row").expect("solution");
    let vertex_count: usize = row
        .get("vertexCount")
        .expect("bound")
        .to_string()
        .trim_start_matches('"')
        .split('"')
        .next()
        .unwrap()
        .parse()
        .expect("integer literal");
    let edge_count: usize = row
        .get("edgeCount")
        .expect("bound")
        .to_string()
        .trim_start_matches('"')
        .split('"')
        .next()
        .unwrap()
        .parse()
        .expect("integer literal");
    assert_eq!(vertex_count, 24, "live COUNT-aggregate query disagrees with CP1's 24 vertex kinds");
    assert_eq!(edge_count, 21, "live COUNT-aggregate query disagrees with CP1's 21 edge kinds");
}

/// Drift-injection-and-revert: deliberately corrupt one real edge's wireLabel
/// back to the pre-CP1 bare form, confirm gate 030 now refuses it, then confirm
/// the real on-disk file (never mutated -- this test only mutates an in-memory
/// string copy) still passes cleanly, proving CP1's fix is real and gate 030
/// actually detects its regression rather than trivially always passing.
#[test]
fn gate_030_refuses_a_deliberately_reintroduced_pre_cp1_bare_wire_label() {
    let corrupted = LSIF06_TTL.replacen(
        r#"rdfs:label "TextDocumentDefinition" ; lsif:wireLabel "textDocument/definition" ;"#,
        r#"rdfs:label "TextDocumentDefinition" ; lsif:wireLabel "definition" ;"#,
        1,
    );
    assert_ne!(
        corrupted, LSIF06_TTL,
        "the replacen target string must actually exist in lsif06.ttl for this injection to be real"
    );

    let corrupted_store = store_from_ttl(&corrupted);
    assert!(
        gate_violation_count(&corrupted_store, GATE_030) > 0,
        "gate 030 must refuse a TextDocument*-named edge class whose wireLabel lost \
         the textDocument/ prefix -- exactly the class of bug CP1 fixed"
    );

    // Revert is implicit: `corrupted` was a local String, LSIF06_TTL (the real
    // file, read via include_str!) was never touched. Confirm it still passes.
    let real_store = store_from_ttl(LSIF06_TTL);
    assert_eq!(
        gate_violation_count(&real_store, GATE_030),
        0,
        "the real on-disk lsif06.ttl must still pass gate 030 cleanly after the \
         in-memory-only drift injection above"
    );
}
