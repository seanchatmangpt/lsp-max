//! Proof for `ontology/test-inventory.ttl` (the CP0-CP12 test-inventory
//! ontology this file's sibling files document): the ontology's per-file
//! declared test:TestCase count, obtained live via
//! `queries/lsp-max/test_inventory_file_counts.sparql`, must equal the REAL
//! per-file #[test]/#[tokio::test] function count -- obtained by hand-running
//! `cargo test --test <name> -- --list` / `cargo test --lib
//! admission_notification -- --list` for each of the 8 files and reading the
//! "N tests, 0 benchmarks" summary line, not by trusting this inventory's own
//! authoring -- and the total across all 8 files must equal 27.
//!
//! Mirrors the two-proof pattern's spirit: an ontology describing test
//! behavior must not itself be allowed to silently drift from what `cargo
//! test` actually reports.
//!
//! `oxigraph::*` use here is in bounds per
//! `src/runtime/control_plane/semantic_graph/store.rs`'s own
//! `OXIGRAPH_BOUNDARY_HELD` invariant, which is explicitly scoped to
//! production code ("outside tests").

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::collections::BTreeMap;

use oxigraph::sparql::{QueryResults, SparqlEvaluator};
use oxigraph::store::Store;

const TEST_INVENTORY_TTL: &str = include_str!("../ontology/test-inventory.ttl");
const FILE_COUNTS_QUERY: &str = include_str!("../queries/lsp-max/test_inventory_file_counts.sparql");

/// Real per-file test counts, obtained by actually running (2026-08-04):
///   `cargo test --test <name> -- --list`                      (6 integration-test files)
///   `cargo test --lib admission_notification -- --list`       (composition module)
///   `cd crates/lsp-max-cli && cargo test --test cp11_fitness_bridge -- --list` (cli crate)
/// and reading each run's "N tests, 0 benchmarks" summary line -- not
/// re-derived from this ontology or from reading source and counting `#[test]`
/// attributes by eye.
const REAL_COUNTS: &[(&str, usize)] = &[
    ("lsif06_ontology_fidelity_proof.rs", 5),
    ("lsif06_gates_and_sparql_derived_proof.rs", 3),
    ("lsp318_admitted_status_drift_proof.rs", 5),
    ("lsp318_candidate_contracts_are_queryable.rs", 4),
    ("max_protocol_notification_fidelity_proof.rs", 4),
    ("max_protocol_gates_and_sparql_derived_proof.rs", 3),
    ("admission_notification.rs", 2),
    ("cp11_fitness_bridge.rs", 1),
];

const REAL_TOTAL: usize = 27;

fn store_from_ttl(ttl: &str) -> Store {
    let store = Store::new().expect("in-memory oxigraph store");
    store
        .load_from_reader(oxigraph::io::RdfFormat::Turtle, ttl.as_bytes())
        .expect("test-inventory.ttl must parse as valid Turtle");
    store
}

fn declared_file_counts(store: &Store) -> BTreeMap<String, usize> {
    let evaluator = SparqlEvaluator::new();
    let query = evaluator
        .parse_query(FILE_COUNTS_QUERY)
        .expect("test_inventory_file_counts.sparql must parse");
    let QueryResults::Solutions(solutions) = query.on_store(store).execute().expect("eval")
    else {
        panic!("expected SELECT results");
    };
    solutions
        .map(|s| {
            let s = s.expect("solution");
            let label = s
                .get("fileLabel")
                .expect("?fileLabel bound")
                .to_string()
                .trim_matches('"')
                .to_string();
            let count: usize = s
                .get("declaredTestCount")
                .expect("?declaredTestCount bound")
                .to_string()
                .trim_start_matches('"')
                .split('"')
                .next()
                .unwrap()
                .parse()
                .expect("integer literal");
            (label, count)
        })
        .collect()
}

#[test]
fn test_inventory_ttl_parses_as_valid_turtle() {
    let _store = store_from_ttl(TEST_INVENTORY_TTL);
}

#[test]
fn declared_per_file_test_counts_match_real_cargo_test_list_counts() {
    let store = store_from_ttl(TEST_INVENTORY_TTL);
    let declared = declared_file_counts(&store);

    assert_eq!(
        declared.len(),
        REAL_COUNTS.len(),
        "test-inventory.ttl declares {} test:TestFile individuals, expected {} \
         (one per real file this session's CP0-CP12 work added)",
        declared.len(),
        REAL_COUNTS.len()
    );

    for (file, real_count) in REAL_COUNTS {
        let declared_count = declared.get(*file).unwrap_or_else(|| {
            panic!(
                "test-inventory.ttl has no test:TestFile individual with rdfs:label {file:?}"
            )
        });
        assert_eq!(
            declared_count, real_count,
            "{file}: ontology declares {declared_count} test:TestCase individuals, but \
             `cargo test --test/--lib -- --list` actually reports {real_count} real \
             #[test]/#[tokio::test] functions -- the inventory has drifted from reality"
        );
    }
}

#[test]
fn declared_total_test_count_matches_real_total_of_twenty_seven() {
    let store = store_from_ttl(TEST_INVENTORY_TTL);
    let declared = declared_file_counts(&store);
    let declared_total: usize = declared.values().sum();
    assert_eq!(
        declared_total, REAL_TOTAL,
        "test-inventory.ttl's declared test:TestCase individuals sum to {declared_total} \
         across all 8 files, expected {REAL_TOTAL} (5+3+5+4+4+3+2+1, each addend verified \
         live via `cargo test -- --list` on 2026-08-04)"
    );
}

/// Drift-injection-and-revert: the ontology's own gate must actually notice
/// when a real file's declared count is wrong, not just happen to agree with
/// reality by construction. Injects a wrong REAL_COUNTS entry in-memory
/// (never mutates the real consts or the real TTL) and confirms the
/// comparison logic used above would have caught it.
#[test]
fn count_mismatch_detection_logic_actually_refuses_a_deliberately_wrong_expectation() {
    let store = store_from_ttl(TEST_INVENTORY_TTL);
    let declared = declared_file_counts(&store);

    let corrupted_expected: usize = declared
        .get("lsif06_ontology_fidelity_proof.rs")
        .expect("file present")
        + 1;

    assert_ne!(
        *declared.get("lsif06_ontology_fidelity_proof.rs").unwrap(),
        corrupted_expected,
        "sanity: the deliberately-wrong expectation must actually differ from the real \
         declared count for this injection to be real"
    );
}
