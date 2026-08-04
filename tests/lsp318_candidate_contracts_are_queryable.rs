//! Gall checkpoint CP6 (see ~/ggen's `80-20-gall-test-refactor-cheerful-quokka.md`
//! plan): proves that `ontology/lsp318.ttl`'s new `lsp:expectsBinding` /
//! `lsp:producesShape` facts (added additively to every still-CANDIDATE
//! individual, per CP4's finding that all 57 individuals are CANDIDATE) are
//! real, queryable RDF -- not just a comment in generated Rust -- and that
//! `queries/lsp-max/candidate_contracts.sparql` (the literal precondition
//! CP9's ggen-mcp introspection tool needs) actually returns them with zero
//! Rust source reads.
//!
//! `oxigraph::*` use here is in bounds per
//! `src/runtime/control_plane/semantic_graph/store.rs`'s own
//! `OXIGRAPH_BOUNDARY_HELD` invariant, which is explicitly scoped to
//! production code ("outside tests").

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use oxigraph::sparql::{QueryResults, SparqlEvaluator};
use oxigraph::store::Store;
use std::collections::BTreeMap;

const LSP318_TTL: &str = include_str!("../ontology/lsp318.ttl");
const CANDIDATE_CONTRACTS_QUERY: &str = include_str!("../queries/lsp-max/candidate_contracts.sparql");

/// Gate-style query (FILTER NOT EXISTS): any CANDIDATE `lsp:Request` /
/// `lsp:Notification` / `lsp:MaxExtension` individual missing
/// `lsp:expectsBinding` or `lsp:producesShape` is a violation row. Zero rows
/// means the CP6 retrofit is complete over every CANDIDATE individual.
const GATE_MISSING_EXPECTS_BINDING: &str = r#"
PREFIX lsp: <https://lsp-max.rs/ontology/>
PREFIX law: <https://lsp-max.rs/law/>

SELECT ?method
WHERE {
    { ?method a lsp:Request } UNION { ?method a lsp:Notification } UNION { ?method a lsp:MaxExtension }
    ?method law:status law:CANDIDATE .
    FILTER NOT EXISTS { ?method lsp:expectsBinding ?b }
}
"#;

const GATE_MISSING_PRODUCES_SHAPE: &str = r#"
PREFIX lsp: <https://lsp-max.rs/ontology/>
PREFIX law: <https://lsp-max.rs/law/>

SELECT ?method
WHERE {
    { ?method a lsp:Request } UNION { ?method a lsp:Notification } UNION { ?method a lsp:MaxExtension }
    ?method law:status law:CANDIDATE .
    FILTER NOT EXISTS { ?method lsp:producesShape ?s }
}
"#;

/// Independent, hand-transcribed EXPECTED list -- the 57 CANDIDATE
/// individuals confirmed live by CP4's own grep
/// (`grep -o "law:status *law:[A-Z]*" ontology/lsp318.ttl | sort | uniq -c`
/// -> `57 law:CANDIDATE`), each with the `(expects_binding, produces_shape)`
/// pair this checkpoint computed by hand from the individual's real
/// `lsp:paramType`/`lsp:returnType` values -- read directly from
/// `ontology/lsp318.ttl`, not derived from the generation query under test.
fn expected_candidate_contracts() -> BTreeMap<&'static str, (&'static str, &'static str)> {
    [
        ("callHierarchy/incomingCalls", ("single-params-struct", "optional-list")),
        ("callHierarchy/outgoingCalls", ("single-params-struct", "optional-list")),
        ("codeLens/resolve", ("single-params-struct", "value")),
        ("completionItem/resolve", ("single-params-struct", "value")),
        ("documentLink/resolve", ("single-params-struct", "value")),
        ("exit", ("no-params", "void")),
        ("initialize", ("single-params-struct", "value")),
        ("initialized", ("single-params-struct", "void")),
        ("inlayHint/resolve", ("single-params-struct", "value")),
        ("shutdown", ("no-params", "unit")),
        ("textDocument/codeAction", ("single-params-struct", "optional-single")),
        ("textDocument/codeLens", ("single-params-struct", "list")),
        ("textDocument/colorPresentation", ("single-params-struct", "list")),
        ("textDocument/completion", ("single-params-struct", "optional-single")),
        ("textDocument/declaration", ("single-params-struct", "optional-single")),
        ("textDocument/definition", ("single-params-struct", "optional-single")),
        ("textDocument/diagnostic", ("single-params-struct", "value")),
        ("textDocument/didChange", ("single-params-struct", "void")),
        ("textDocument/didClose", ("single-params-struct", "void")),
        ("textDocument/didOpen", ("single-params-struct", "void")),
        ("textDocument/didSave", ("single-params-struct", "void")),
        ("textDocument/documentColor", ("single-params-struct", "list")),
        ("textDocument/documentHighlight", ("single-params-struct", "optional-list")),
        ("textDocument/documentLink", ("single-params-struct", "optional-list")),
        ("textDocument/documentSymbol", ("single-params-struct", "optional-single")),
        ("textDocument/foldingRange", ("single-params-struct", "optional-list")),
        ("textDocument/formatting", ("single-params-struct", "optional-list")),
        ("textDocument/hover", ("single-params-struct", "optional-single")),
        ("textDocument/implementation", ("single-params-struct", "optional-single")),
        ("textDocument/inlayHint", ("single-params-struct", "optional-list")),
        ("textDocument/inlineValue", ("single-params-struct", "optional-list")),
        ("textDocument/linkedEditingRange", ("single-params-struct", "optional-single")),
        ("textDocument/moniker", ("single-params-struct", "optional-list")),
        ("textDocument/onTypeFormatting", ("single-params-struct", "list")),
        ("textDocument/prepareCallHierarchy", ("single-params-struct", "optional-list")),
        ("textDocument/prepareRename", ("single-params-struct", "optional-single")),
        ("textDocument/prepareTypeHierarchy", ("single-params-struct", "optional-list")),
        ("textDocument/rangeFormatting", ("single-params-struct", "optional-list")),
        ("textDocument/references", ("single-params-struct", "optional-list")),
        ("textDocument/rename", ("single-params-struct", "optional-single")),
        ("textDocument/selectionRange", ("single-params-struct", "list")),
        ("textDocument/semanticTokens/full", ("single-params-struct", "optional-single")),
        ("textDocument/semanticTokens/full/delta", ("single-params-struct", "optional-single")),
        ("textDocument/semanticTokens/range", ("single-params-struct", "optional-single")),
        ("textDocument/signatureHelp", ("single-params-struct", "optional-single")),
        ("textDocument/typeDefinition", ("single-params-struct", "optional-single")),
        ("typeHierarchy/subtypes", ("single-params-struct", "optional-list")),
        ("typeHierarchy/supertypes", ("single-params-struct", "optional-list")),
        ("workspace/didChangeConfiguration", ("single-params-struct", "void")),
        ("workspace/didChangeWatchedFiles", ("single-params-struct", "void")),
        ("workspace/didChangeWorkspaceFolders", ("single-params-struct", "void")),
        ("workspace/didCreateFiles", ("single-params-struct", "void")),
        ("workspace/didDeleteFiles", ("single-params-struct", "void")),
        ("workspace/didRenameFiles", ("single-params-struct", "void")),
        ("workspace/diagnostic", ("single-params-struct", "value")),
        ("workspace/executeCommand", ("single-params-struct", "optional-single")),
        ("workspace/symbol", ("single-params-struct", "optional-list")),
    ]
    .into_iter()
    .collect()
}

fn store_from_ttl(ttl: &str) -> Store {
    let store = Store::new().expect("in-memory oxigraph store");
    store
        .load_from_reader(oxigraph::io::RdfFormat::Turtle, ttl.as_bytes())
        .expect("ttl must parse");
    store
}

fn run_select(store: &Store, sparql: &str) -> Vec<Vec<(String, String)>> {
    let evaluator = SparqlEvaluator::new();
    let query = evaluator.parse_query(sparql).expect("query must parse");
    let QueryResults::Solutions(solutions) = query.on_store(store).execute().expect("eval")
    else {
        panic!("expected SELECT results");
    };
    solutions
        .map(|s| {
            let s = s.expect("solution binds");
            s.iter()
                .map(|(var, term)| {
                    let raw = term.to_string();
                    // strip xsd:string literal quoting oxigraph's Display adds
                    let val = raw.trim_matches('"').to_string();
                    (var.as_str().to_string(), val)
                })
                .collect()
        })
        .collect()
}

#[test]
fn lsp318_ttl_parses_into_oxigraph() {
    let store = store_from_ttl(LSP318_TTL);
    let count = run_select(
        &store,
        "SELECT ?s WHERE { ?s ?p ?o }",
    )
    .len();
    assert!(count > 0, "lsp318.ttl must parse into at least one triple");
}

#[test]
fn gate_zero_candidate_individuals_missing_expects_binding_or_produces_shape() {
    let store = store_from_ttl(LSP318_TTL);
    let missing_binding = run_select(&store, GATE_MISSING_EXPECTS_BINDING);
    let missing_shape = run_select(&store, GATE_MISSING_PRODUCES_SHAPE);
    assert_eq!(
        missing_binding.len(),
        0,
        "found CANDIDATE individual(s) missing lsp:expectsBinding: {missing_binding:?}"
    );
    assert_eq!(
        missing_shape.len(),
        0,
        "found CANDIDATE individual(s) missing lsp:producesShape: {missing_shape:?}"
    );
}

#[test]
fn candidate_contracts_query_matches_hand_transcribed_expected_pairs() {
    let store = store_from_ttl(LSP318_TTL);
    let rows = run_select(&store, CANDIDATE_CONTRACTS_QUERY);
    let expected = expected_candidate_contracts();

    assert_eq!(
        rows.len(),
        expected.len(),
        "candidate_contracts.sparql returned {} rows, expected {} \
         (one per CANDIDATE individual)",
        rows.len(),
        expected.len()
    );

    let mut seen = std::collections::BTreeSet::new();
    for row in &rows {
        let mut map = BTreeMap::new();
        for (k, v) in row {
            map.insert(k.as_str(), v.as_str());
        }
        let method_name = *map.get("method_name").expect("method_name bound");
        let expects_binding = *map.get("expects_binding").expect("expects_binding bound");
        let produces_shape = *map.get("produces_shape").expect("produces_shape bound");

        let (exp_binding, exp_shape) = expected
            .get(method_name)
            .unwrap_or_else(|| panic!("unexpected method in query results: {method_name}"));

        assert_eq!(
            expects_binding, *exp_binding,
            "{method_name}: expectsBinding mismatch"
        );
        assert_eq!(
            produces_shape, *exp_shape,
            "{method_name}: producesShape mismatch"
        );
        seen.insert(method_name.to_string());
    }

    for method_name in expected.keys() {
        assert!(
            seen.contains(*method_name),
            "expected method {method_name} missing from candidate_contracts.sparql results"
        );
    }
}

#[test]
fn drift_injection_and_revert_gate_catches_a_removed_expects_binding_fact() {
    // Deliberately corrupt in-memory: strip lsp:expectsBinding from
    // textDocument/hover to prove the gate query actually refuses missing
    // facts, not just vacuously passes on a fully-populated graph.
    let corrupted = LSP318_TTL.replacen(
        "    law:status     law:CANDIDATE ;\n    lsp:expectsBinding \"single-params-struct\" ;\n    lsp:producesShape  \"optional-single\" .\n\nlsp:textDocument_completion",
        "    law:status     law:CANDIDATE ;\n    lsp:producesShape  \"optional-single\" .\n\nlsp:textDocument_completion",
        1,
    );
    assert_ne!(
        corrupted, LSP318_TTL,
        "corruption target string not found -- test fixture drifted from real lsp318.ttl layout"
    );

    let corrupted_store = store_from_ttl(&corrupted);
    let corrupted_violations = run_select(&corrupted_store, GATE_MISSING_EXPECTS_BINDING);
    assert_eq!(
        corrupted_violations.len(),
        1,
        "gate must catch exactly the one deliberately-stripped lsp:expectsBinding fact"
    );

    // Revert: the real on-disk file is untouched and still passes cleanly.
    let real_store = store_from_ttl(LSP318_TTL);
    let real_violations = run_select(&real_store, GATE_MISSING_EXPECTS_BINDING);
    assert_eq!(
        real_violations.len(),
        0,
        "real on-disk ontology/lsp318.ttl must still have zero gate violations"
    );
}
