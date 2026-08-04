//! Proves `ontology/lsp-max-maturity.ttl` (the CP0-CP12 maturity ledger,
//! mirroring `~/ggen/.specify/maturity.ttl`'s shape) is valid, loadable
//! Turtle and that its 7 capability rows are all present as `mat:ScoreRow`
//! individuals. Mirrors the same oxigraph-load pattern used by
//! `tests/lsif06_ontology_fidelity_proof.rs` and
//! `tests/lsp318_candidate_contracts_are_queryable.rs`.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use oxigraph::sparql::{QueryResults, SparqlEvaluator};
use oxigraph::store::Store;

const MATURITY_TTL: &str = include_str!("../ontology/lsp-max-maturity.ttl");

#[test]
fn maturity_ttl_parses_as_valid_turtle() {
    let store = Store::new().expect("in-memory store");
    store
        .load_from_reader(oxigraph::io::RdfFormat::Turtle, MATURITY_TTL.as_bytes())
        .expect("lsp-max-maturity.ttl must parse as valid Turtle");
    assert!(store.len().unwrap() > 0, "store must not be empty after load");
}

#[test]
fn seven_score_rows_present_matching_the_seven_scored_checkpoints() {
    let store = Store::new().expect("in-memory store");
    store
        .load_from_reader(oxigraph::io::RdfFormat::Turtle, MATURITY_TTL.as_bytes())
        .expect("parse");

    let query = r#"
        PREFIX mat: <http://ggen.org/maturity#>
        SELECT ?row ?pack WHERE {
            ?row a mat:ScoreRow ;
                 mat:pack ?pack .
        } ORDER BY ?row
    "#;

    let results = SparqlEvaluator::new()
        .parse_query(query)
        .expect("query parses")
        .on_store(&store)
        .execute()
        .expect("query executes");

    let mut packs: Vec<String> = Vec::new();
    if let QueryResults::Solutions(solutions) = results {
        for sol in solutions {
            let sol = sol.expect("solution");
            if let Some(term) = sol.get("pack") {
                packs.push(term.to_string());
            }
        }
    } else {
        panic!("expected solutions");
    }

    assert_eq!(
        packs.len(),
        7,
        "expected exactly 7 scored capability rows (CP1/CP2, CP3, CP4, CP6, CP10, CP11, CP12), got: {packs:?}"
    );
}

#[test]
fn every_score_row_has_all_seven_dimension_cells() {
    let store = Store::new().expect("in-memory store");
    store
        .load_from_reader(oxigraph::io::RdfFormat::Turtle, MATURITY_TTL.as_bytes())
        .expect("parse");

    let query = r#"
        PREFIX mat: <http://ggen.org/maturity#>
        SELECT ?row (COUNT(?d) AS ?dcount) WHERE {
            ?row a mat:ScoreRow .
            ?row ?dp ?d .
            FILTER(STRSTARTS(STR(?dp), STR(mat:d)))
        } GROUP BY ?row
    "#;

    let results = SparqlEvaluator::new()
        .parse_query(query)
        .expect("query parses")
        .on_store(&store)
        .execute()
        .expect("query executes");

    if let QueryResults::Solutions(solutions) = results {
        for sol in solutions {
            let sol = sol.expect("solution");
            let count = sol.get("dcount").expect("dcount bound").to_string();
            // literal encodes as "7"^^xsd:integer -- just check it starts with 7
            assert!(
                count.starts_with("\"7\""),
                "every ScoreRow must have exactly 7 d-dimension cells (d1..d7), got {count} for {:?}",
                sol.get("row")
            );
        }
    } else {
        panic!("expected solutions");
    }
}
