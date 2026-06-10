use super::*;
use oxigraph::model::{NamedNode, NamedOrBlankNode, Quad, Term};
use oxigraph::store::Store;

fn node(uri: &str) -> NamedOrBlankNode {
    NamedOrBlankNode::NamedNode(NamedNode::new(uri).unwrap())
}

fn term(uri: &str) -> Term {
    Term::NamedNode(NamedNode::new(uri).unwrap())
}

#[test]
fn test_invariant_1_orphan() {
    let store = Store::new().unwrap();
    // Insert an edge pointing to a non-existent node
    store.insert(&Quad::new(
        node("urn:range1"),
        NamedNode::new("https://microsoft.github.io/language-server-protocol/specifications/lsif/0.6.0/specification/contains").unwrap(),
        term("urn:nonexistent"),
        oxigraph::model::GraphName::DefaultGraph,
    )).unwrap();

    let report = verify_invariants(&store);
    assert!(!report.is_success);
    assert!(report
        .diagnostics
        .iter()
        .any(|d| d.violated_invariant == "INVARIANT_1"));
}

#[test]
fn test_invariant_2_unreceipted() {
    let store = Store::new().unwrap();
    // Insert an unreceipted diagnostic
    store
        .insert(&Quad::new(
            node("urn:diagnostic1"),
            NamedNode::new("http://www.w3.org/1999/02/22-rdf-syntax-ns#type").unwrap(),
            term("urn:lsp-max:core:Diagnostic"),
            oxigraph::model::GraphName::DefaultGraph,
        ))
        .unwrap();

    let report = verify_invariants(&store);
    assert!(!report.is_success);
    assert!(report
        .diagnostics
        .iter()
        .any(|d| d.violated_invariant == "INVARIANT_2"));
}

#[test]
fn test_invariant_3_missing_projection() {
    let store = Store::new().unwrap();
    // Insert a range and definitionResult but no projection
    store.insert(&Quad::new(
        node("urn:range1"),
        NamedNode::new("http://www.w3.org/1999/02/22-rdf-syntax-ns#type").unwrap(),
        term("https://microsoft.github.io/language-server-protocol/specifications/lsif/0.6.0/specification/Range"),
        oxigraph::model::GraphName::DefaultGraph,
    )).unwrap();
    store.insert(&Quad::new(
        node("urn:range1"),
        NamedNode::new("https://microsoft.github.io/language-server-protocol/specifications/lsif/0.6.0/specification/textDocument_definition").unwrap(),
        term("urn:def1"),
        oxigraph::model::GraphName::DefaultGraph,
    )).unwrap();

    let report = verify_invariants(&store);
    assert!(!report.is_success);
    assert!(report
        .diagnostics
        .iter()
        .any(|d| d.violated_invariant == "INVARIANT_3"));
}

#[test]
fn test_invariant_4_ontology_laundering() {
    let store = Store::new().unwrap();
    // Insert an unwhitelisted LSIF predicate
    store.insert(&Quad::new(
        node("urn:range1"),
        NamedNode::new("https://microsoft.github.io/language-server-protocol/specifications/lsif/0.6.0/specification/laundering_pred").unwrap(),
        term("urn:something"),
        oxigraph::model::GraphName::DefaultGraph,
    )).unwrap();

    let report = verify_invariants(&store);
    assert!(!report.is_success);
    assert!(report
        .diagnostics
        .iter()
        .any(|d| d.violated_invariant == "INVARIANT_4"));
}

#[test]
fn test_invariant_5_false_alive() {
    let store = Store::new().unwrap();
    // Insert a receipt and a mismatching replay
    store
        .insert(&Quad::new(
            node("urn:rcpt1"),
            NamedNode::new("http://www.w3.org/1999/02/22-rdf-syntax-ns#type").unwrap(),
            term("urn:lsp-max:core:Receipt"),
            oxigraph::model::GraphName::DefaultGraph,
        ))
        .unwrap();
    store
        .insert(&Quad::new(
            node("urn:rcpt1"),
            NamedNode::new("urn:lsp-max:core:resultHash").unwrap(),
            Term::Literal(oxigraph::model::Literal::new_simple_literal("expected")),
            oxigraph::model::GraphName::DefaultGraph,
        ))
        .unwrap();
    store
        .insert(&Quad::new(
            node("urn:rcpt1"),
            NamedNode::new("urn:lsp-max:core:queryHash").unwrap(),
            Term::Literal(oxigraph::model::Literal::new_simple_literal("q1")),
            oxigraph::model::GraphName::DefaultGraph,
        ))
        .unwrap();
    store
        .insert(&Quad::new(
            node("urn:rcpt1"),
            NamedNode::new("urn:lsp-max:core:graphHash").unwrap(),
            Term::Literal(oxigraph::model::Literal::new_simple_literal("g1")),
            oxigraph::model::GraphName::DefaultGraph,
        ))
        .unwrap();

    store
        .insert(&Quad::new(
            node("urn:replay1"),
            NamedNode::new("http://www.w3.org/1999/02/22-rdf-syntax-ns#type").unwrap(),
            term("urn:lsp-max:core:Replay"),
            oxigraph::model::GraphName::DefaultGraph,
        ))
        .unwrap();
    store
        .insert(&Quad::new(
            node("urn:replay1"),
            NamedNode::new("urn:lsp-max:core:resultHash").unwrap(),
            Term::Literal(oxigraph::model::Literal::new_simple_literal(
                "actual_different",
            )),
            oxigraph::model::GraphName::DefaultGraph,
        ))
        .unwrap();
    store
        .insert(&Quad::new(
            node("urn:replay1"),
            NamedNode::new("urn:lsp-max:core:queryHash").unwrap(),
            Term::Literal(oxigraph::model::Literal::new_simple_literal("q1")),
            oxigraph::model::GraphName::DefaultGraph,
        ))
        .unwrap();
    store
        .insert(&Quad::new(
            node("urn:replay1"),
            NamedNode::new("urn:lsp-max:core:graphHash").unwrap(),
            Term::Literal(oxigraph::model::Literal::new_simple_literal("g1")),
            oxigraph::model::GraphName::DefaultGraph,
        ))
        .unwrap();

    let report = verify_invariants(&store);
    assert!(!report.is_success);
    assert!(report
        .diagnostics
        .iter()
        .any(|d| d.violated_invariant == "INVARIANT_5"));
}

#[test]
fn test_detailed_diagnostic_contents() {
    let store = Store::new().unwrap();
    // Insert an edge pointing to a non-existent node (orphan relation)
    store.insert(&Quad::new(
        node("urn:range1"),
        NamedNode::new("https://microsoft.github.io/language-server-protocol/specifications/lsif/0.6.0/specification/contains").unwrap(),
        term("urn:nonexistent"),
        oxigraph::model::GraphName::DefaultGraph,
    )).unwrap();

    // Insert unwhitelisted predicate
    store.insert(&Quad::new(
        node("urn:range1"),
        NamedNode::new("https://microsoft.github.io/language-server-protocol/specifications/lsif/0.6.0/specification/laundering_pred").unwrap(),
        term("urn:something"),
        oxigraph::model::GraphName::DefaultGraph,
    )).unwrap();

    let report = verify_invariants(&store);
    assert!(!report.is_success);

    // Check Invariant 1 (Orphan LSIF) detailed fields
    let diag1 = report
        .diagnostics
        .iter()
        .find(|d| d.violated_invariant == "INVARIANT_1" && d.observed_state["predicate"] == "https://microsoft.github.io/language-server-protocol/specifications/lsif/0.6.0/specification/contains")
        .unwrap();
    assert_eq!(diag1.law_axis, LawAxis::Protocol);
    assert!(matches!(diag1.repairability, Repairability::Repairable));
    assert!(matches!(diag1.terminality, Terminality::NonTerminal));

    let obs1 = &diag1.observed_state;
    assert_eq!(obs1["subject"], "urn:range1");
    assert_eq!(obs1["predicate"], "https://microsoft.github.io/language-server-protocol/specifications/lsif/0.6.0/specification/contains");
    assert_eq!(obs1["object"], "urn:nonexistent");

    let exp1 = &diag1.expected_state;
    assert_eq!(exp1["object_exists"], true);

    // Check Invariant 4 (Ontology Laundering) detailed fields
    let diag4 = report
        .diagnostics
        .iter()
        .find(|d| d.violated_invariant == "INVARIANT_4")
        .unwrap();
    assert_eq!(diag4.law_axis, LawAxis::Security);
    assert!(matches!(diag4.repairability, Repairability::NotRepairable));
    assert!(matches!(diag4.terminality, Terminality::Terminal));

    let obs4 = &diag4.observed_state;
    assert_eq!(obs4["subject"], "urn:range1");
    assert_eq!(obs4["predicate"], "https://microsoft.github.io/language-server-protocol/specifications/lsif/0.6.0/specification/laundering_pred");
    assert_eq!(obs4["object"], "urn:something");

    let exp4 = &diag4.expected_state;
    assert_eq!(exp4["whitelisted_predicate"], true);
}
