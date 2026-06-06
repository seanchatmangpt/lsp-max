use super::super::invariants::VerificationReport;
use super::super::receipts::{Blake3Hash, CryptographicReceipt};
use super::admitter::*;
use super::mapping::*;
use super::mapping_helpers::*;
use super::types::*;
use super::validation::*;
use crate::Machine;
use oxigraph::store::Store;
use tower_lsp_max_base::abstractions::RelationAdmitter;
use tower_lsp_max_lsif::lsif::{Edge, EdgeType, Element, PositionEncoding, Vertex, VertexType};
use tower_lsp_max_protocol::MaxDiagnostic;

fn make_meta_element() -> Element {
    Element::Vertex(Vertex::MetaData {
        id: lsp_types_max::NumberOrString::Number(1),
        type_: VertexType::Vertex,
        version: "0.6.0".to_string(),
        project_root: "file:///".to_string(),
        position_encoding: PositionEncoding::Utf16,
        tool_info: None,
    })
}

#[test]
fn test_raw_to_candidate_transition() {
    let raw_elements = vec![make_meta_element()];
    let raw_machine = Machine::new(
        RAW,
        RawData {
            elements: raw_elements,
        },
    );

    let candidate_machine = raw_machine.admit_candidate("snap-123").unwrap();
    assert_eq!(candidate_machine.data.elements.len(), 1);
    assert!(!candidate_machine.data.graph_hash.is_empty());
    assert!(!candidate_machine.data.quads.is_empty());
}

#[test]
fn test_candidate_to_refused_transition() {
    let raw_elements = vec![make_meta_element()];
    let raw_machine = Machine::new(
        RAW,
        RawData {
            elements: raw_elements,
        },
    );
    let candidate_machine = raw_machine.admit_candidate("snap-123").unwrap();

    let report = VerificationReport {
        is_success: false,
        diagnostics: vec![],
        execution_time_ms: 5,
    };
    let refused_machine = candidate_machine.admit_refuse(report);
    assert!(!refused_machine.data.report.is_success);
}

#[test]
fn test_candidate_to_quarantined_transition() {
    let raw_elements = vec![make_meta_element()];
    let raw_machine = Machine::new(
        RAW,
        RawData {
            elements: raw_elements,
        },
    );
    let candidate_machine = raw_machine.admit_candidate("snap-123").unwrap();

    let quarantined_machine = candidate_machine.admit_quarantine(vec!["dep1".to_string()]);
    assert_eq!(
        quarantined_machine.data.missing_dependencies,
        vec!["dep1".to_string()]
    );
}

#[test]
fn test_admitted_to_superseded_transition() {
    let _store = Store::new().unwrap();
    let receipt = CryptographicReceipt {
        prev_hash: Blake3Hash([0u8; 32]),
        discipline_id: uuid::Uuid::new_v4(),
        law_id: uuid::Uuid::new_v4(),
        consequence_hash: Blake3Hash([0u8; 32]),
        sequence: 1,
        signature: [0u8; 64],
    };
    let admitted_machine = Machine::new(
        ADMITTED,
        AdmittedData {
            graph_name: oxigraph::model::GraphName::DefaultGraph,
            quad_count: 5,
            receipt,
        },
    );

    let superseded_by = oxigraph::model::GraphName::NamedNode(
        oxigraph::model::NamedNode::new("urn:snap-2").unwrap(),
    );
    let superseded_machine = admitted_machine.admit_supersede(superseded_by.clone());
    assert_eq!(superseded_machine.data.superseded_by, superseded_by);
}

#[test]
fn test_relation_admitter_trait_integration() {
    let store = Store::new().unwrap();
    let active_graph = oxigraph::model::GraphName::NamedNode(
        oxigraph::model::NamedNode::new("urn:project:local:snapshot:snap-1").unwrap(),
    );
    let admitter = AdmittedGraph {
        store,
        active_graph,
    };

    let elements = vec![make_meta_element()];
    let result = admitter.admit(elements);
    assert!(result.is_ok());

    let admitted_graph = result.unwrap();
    assert_eq!(admitted_graph.graph_name, admitter.active_graph);
    assert_ne!(admitted_graph.receipt.signature, [0u8; 64]);
}

#[test]
fn test_ontology_laundering_ingestion_whitelist() {
    let raw_elements = vec![Element::Edge(Edge::Contains {
        id: lsp_types_max::NumberOrString::Number(2),
        type_: EdgeType::Edge,
        out_v: lsp_types_max::NumberOrString::Number(1),
        in_vs: vec![lsp_types_max::NumberOrString::Number(3)],
    })];
    let raw_machine = Machine::new(
        RAW,
        RawData {
            elements: raw_elements,
        },
    );
    assert!(raw_machine.admit_candidate("snap-1").is_ok());

    let diag = MaxDiagnostic {
        diagnostic_id: "diag-1".to_string(),
        law_id: "law-1".to_string(),
        lsp: lsp_types_max::Diagnostic {
            message: "test diagnostic message".to_string(),
            ..Default::default()
        },
        receipt_obligation: Some(tower_lsp_max_protocol::ReceiptObligation {
            required_receipts: vec!["rcpt-123".to_string()],
        }),
        ..Default::default()
    };
    let quads = map_diagnostic_to_quads(&diag, &oxigraph::model::GraphName::DefaultGraph);
    assert!(!quads.is_empty());
    assert_eq!(quads.len(), 5);
}
