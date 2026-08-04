//! Gall checkpoint CP1 (see ~/ggen's
//! `80-20-gall-test-refactor-cheerful-quokka.md` plan): proves
//! `ontology/lsif06.ttl` is fidelity-correct against the real, serde-tagged
//! `Vertex`/`Edge` enums in `crates/lsp-max-lsif/src/lsif.rs`.
//!
//! Deliberately hand-transcribed, NOT SPARQL-derived from the ontology
//! itself (that would make every assertion a tautology — the CP2 checkpoint
//! adds the SPARQL-derived second proof, re-deriving from a live query
//! instead). The `EXPECTED_*` consts below were transcribed directly from
//! reading `lsif.rs`'s `#[serde(rename = "...")]` attributes, not from this
//! ontology file.
//!
//! `oxigraph::*` normally may only appear in
//! `src/runtime/control_plane/semantic_graph/store.rs` per that module's own
//! `OXIGRAPH_BOUNDARY_HELD` invariant comment — that invariant is explicitly
//! scoped to production code ("outside tests"), so a plain `oxigraph::store`
//! use here is in bounds.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::collections::BTreeSet;

use oxigraph::sparql::{QueryResults, SparqlEvaluator};
use oxigraph::store::Store;

const LSIF06_TTL: &str = include_str!("../ontology/lsif06.ttl");

/// The real wire labels of `lsif.rs`'s `enum Vertex` (24 variants), hand-transcribed
/// from its `#[serde(rename = "...")]` attributes.
const EXPECTED_VERTEX_WIRE_LABELS: &[&str] = &[
    "metaData",
    "source",
    "capabilities",
    "project",
    "document",
    "resultSet",
    "range",
    "resultRange",
    "moniker",
    "packageInformation",
    "hoverResult",
    "referenceResult",
    "declarationResult",
    "definitionResult",
    "implementationResult",
    "typeDefinitionResult",
    "callHierarchyResult",
    "typeHierarchyResult",
    "foldingRangeResult",
    "documentLinkResult",
    "documentSymbolResult",
    "diagnosticResult",
    "semanticTokensResult",
    "$event",
];

/// The real wire labels of `lsif.rs`'s `enum Edge` (21 variants), hand-transcribed
/// from its `#[serde(rename = "...")]` attributes. Note `packageInformation`
/// intentionally collides on wire label with the vertex kind of the same
/// name — this is real ambiguity present in the upstream Rust enum (the edge
/// and vertex variants both rename to the bare string `"packageInformation"`,
/// disambiguated only by the surrounding `label`-tagged JSON shape), not an
/// artifact of this test.
const EXPECTED_EDGE_WIRE_LABELS: &[&str] = &[
    "contains",
    "next",
    "moniker",
    "nextMoniker",
    "belongsTo",
    "attach",
    "packageInformation",
    "item",
    "textDocument/hover",
    "textDocument/definition",
    "textDocument/declaration",
    "textDocument/references",
    "textDocument/implementation",
    "textDocument/typeDefinition",
    "textDocument/callHierarchy",
    "textDocument/typeHierarchy",
    "textDocument/foldingRange",
    "textDocument/documentLink",
    "textDocument/documentSymbol",
    "textDocument/diagnostic",
    "textDocument/semanticTokens/full",
];

/// The real wire values of `lsif.rs`'s `ItemEdgeProperty` enum (7 variants),
/// hand-transcribed. Note `TypeDefinitions` renames to `"typeDefinitionResults"`,
/// not `"typeDefinitions"` -- an intentional Rust-name/wire-value mismatch
/// this test must catch if the ontology ever gets it wrong.
const EXPECTED_ITEM_EDGE_PROPERTIES: &[&str] = &[
    "definitions",
    "declarations",
    "references",
    "referenceResults",
    "implementationResults",
    "typeDefinitionResults",
    "referenceLinks",
];

fn load_store() -> Store {
    let store = Store::new().expect("in-memory oxigraph store");
    store
        .load_from_reader(oxigraph::io::RdfFormat::Turtle, LSIF06_TTL.as_bytes())
        .expect("lsif06.ttl must parse as valid Turtle");
    store
}

fn select_wire_labels(store: &Store, superclass: &str) -> BTreeSet<String> {
    let sparql = format!(
        r#"
        PREFIX rdfs: <http://www.w3.org/2000/01/rdf-schema#>
        PREFIX lsif: <https://lsp-max.rs/ontology/lsif/>
        SELECT ?wireLabel WHERE {{
            ?class rdfs:subClassOf {superclass} ;
                   lsif:wireLabel ?wireLabel .
        }}
        "#
    );
    let evaluator = SparqlEvaluator::new();
    let query = evaluator.parse_query(&sparql).expect("query must parse");
    let QueryResults::Solutions(solutions) = query.on_store(store).execute().expect("eval")
    else {
        panic!("expected SELECT results");
    };
    solutions
        .map(|s| {
            let s = s.expect("solution");
            s.get("wireLabel")
                .expect("?wireLabel bound")
                .to_string()
                .trim_matches('"')
                .to_string()
        })
        .collect()
}

fn select_item_edge_properties(store: &Store) -> BTreeSet<String> {
    let sparql = r#"
        PREFIX rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#>
        PREFIX lsif: <https://lsp-max.rs/ontology/lsif/>
        SELECT ?wireLabel WHERE {
            ?individual rdf:type lsif:ItemEdgeProperty ;
                        lsif:wireLabel ?wireLabel .
        }
    "#;
    let evaluator = SparqlEvaluator::new();
    let query = evaluator.parse_query(sparql).expect("query must parse");
    let QueryResults::Solutions(solutions) = query.on_store(&store).execute().expect("eval")
    else {
        panic!("expected SELECT results");
    };
    solutions
        .map(|s| {
            let s = s.expect("solution");
            s.get("wireLabel")
                .expect("?wireLabel bound")
                .to_string()
                .trim_matches('"')
                .to_string()
        })
        .collect()
}

#[test]
fn lsif06_ttl_parses_as_valid_turtle() {
    let _store = load_store();
}

#[test]
fn vertex_wire_labels_match_the_real_lsif_rs_enum_exactly() {
    let store = load_store();
    let declared = select_wire_labels(&store, "lsif:Vertex");
    let expected: BTreeSet<String> = EXPECTED_VERTEX_WIRE_LABELS
        .iter()
        .map(|s| s.to_string())
        .collect();
    assert_eq!(
        declared, expected,
        "lsif06.ttl's Vertex subclasses' lsif:wireLabel set has drifted from lsif.rs's real \
         `enum Vertex` variants (hand-transcribed EXPECTED_VERTEX_WIRE_LABELS)"
    );
}

#[test]
fn edge_wire_labels_match_the_real_lsif_rs_enum_exactly() {
    let store = load_store();
    let declared = select_wire_labels(&store, "lsif:Edge");
    let expected: BTreeSet<String> = EXPECTED_EDGE_WIRE_LABELS
        .iter()
        .map(|s| s.to_string())
        .collect();
    assert_eq!(
        declared, expected,
        "lsif06.ttl's Edge subclasses' lsif:wireLabel set has drifted from lsif.rs's real \
         `enum Edge` variants (hand-transcribed EXPECTED_EDGE_WIRE_LABELS)"
    );
}

#[test]
fn item_edge_property_values_match_the_real_lsif_rs_enum_exactly() {
    let store = load_store();
    let declared = select_item_edge_properties(&store);
    let expected: BTreeSet<String> = EXPECTED_ITEM_EDGE_PROPERTIES
        .iter()
        .map(|s| s.to_string())
        .collect();
    assert_eq!(
        declared, expected,
        "lsif06.ttl's lsif:ItemEdgeProperty individuals have drifted from lsif.rs's real \
         `ItemEdgeProperty` enum (hand-transcribed EXPECTED_ITEM_EDGE_PROPERTIES)"
    );
}

#[test]
fn vertex_and_edge_wire_label_counts_match_lsif_rs_variant_counts() {
    let store = load_store();
    assert_eq!(
        select_wire_labels(&store, "lsif:Vertex").len(),
        24,
        "lsif.rs's enum Vertex has 24 variants"
    );
    assert_eq!(
        select_wire_labels(&store, "lsif:Edge").len(),
        21,
        "lsif.rs's enum Edge has 21 variants"
    );
}
