use lsp_types::{Diagnostic, Position, Range, SymbolKind};
use lsp_types_max as lsp_types;
use oxigraph::model::GraphName;
use oxigraph::model::NamedNode;
use oxigraph::store::Store;
use tower_lsp_max_lsif::lsif::{
    Edge, EdgeType, Element, HoverResultData, RangeTag, Vertex, VertexType,
};
use tower_lsp_max_protocol::MaxDiagnostic;
use tower_lsp_max_runtime::control_plane::views::{
    lookup_definition, lookup_diagnostics, lookup_hover, lookup_references, update_views,
    MaterializedViewStore,
};
use url::Url;

#[test]
fn test_integration_materialized_views_flow() {
    let store = Store::new().unwrap();
    let active_graph = GraphName::NamedNode(
        NamedNode::new("urn:project:local:snapshot:snap-integration").unwrap(),
    );

    let elements = vec![
        Element::Vertex(Vertex::Document {
            id: lsp_types::NumberOrString::Number(1),
            type_: VertexType::Vertex,
            uri: "file:///test.rs".to_string(),
            language_id: "rust".to_string(),
            contents: None,
        }),
        Element::Vertex(Vertex::Range {
            id: lsp_types::NumberOrString::Number(3),
            type_: VertexType::Vertex,
            start: Position::new(10, 5),
            end: Position::new(10, 10),
            tag: Some(RangeTag::Reference {
                text: "foo".to_string(),
            }),
        }),
        Element::Vertex(Vertex::ResultSet {
            id: lsp_types::NumberOrString::Number(2),
            type_: VertexType::Vertex,
        }),
        Element::Vertex(Vertex::DefinitionResult {
            id: lsp_types::NumberOrString::Number(4),
            type_: VertexType::Vertex,
        }),
        Element::Vertex(Vertex::Range {
            id: lsp_types::NumberOrString::Number(5),
            type_: VertexType::Vertex,
            start: Position::new(2, 5),
            end: Position::new(2, 10),
            tag: Some(RangeTag::Definition {
                text: "foo".to_string(),
                kind: SymbolKind::FUNCTION,
                full_range: Range::new(Position::new(2, 0), Position::new(4, 0)),
                detail: None,
            }),
        }),
        Element::Vertex(Vertex::HoverResult {
            id: lsp_types::NumberOrString::Number(6),
            type_: VertexType::Vertex,
            result: HoverResultData {
                contents: tower_lsp_max_lsif::lsif::HoverContents::String(
                    "This is foo function".to_string(),
                ),
                range: None,
            },
        }),
        Element::Vertex(Vertex::ReferenceResult {
            id: lsp_types::NumberOrString::Number(7),
            type_: VertexType::Vertex,
        }),
        Element::Edge(Edge::Contains {
            id: lsp_types::NumberOrString::Number(10),
            type_: EdgeType::Edge,
            out_v: lsp_types::NumberOrString::Number(1),
            in_vs: vec![
                lsp_types::NumberOrString::Number(3),
                lsp_types::NumberOrString::Number(5),
            ],
        }),
        Element::Edge(Edge::Next {
            id: lsp_types::NumberOrString::Number(11),
            type_: EdgeType::Edge,
            out_v: lsp_types::NumberOrString::Number(3),
            in_v: lsp_types::NumberOrString::Number(2),
        }),
        Element::Edge(Edge::TextDocumentDefinition {
            id: lsp_types::NumberOrString::Number(12),
            type_: EdgeType::Edge,
            out_v: lsp_types::NumberOrString::Number(2),
            in_v: lsp_types::NumberOrString::Number(4),
        }),
        Element::Edge(Edge::Item {
            id: lsp_types::NumberOrString::Number(13),
            type_: EdgeType::Edge,
            out_v: lsp_types::NumberOrString::Number(4),
            in_vs: vec![lsp_types::NumberOrString::Number(5)],
            document: lsp_types::NumberOrString::Number(1),
            property: None,
        }),
        Element::Edge(Edge::TextDocumentHover {
            id: lsp_types::NumberOrString::Number(14),
            type_: EdgeType::Edge,
            out_v: lsp_types::NumberOrString::Number(2),
            in_v: lsp_types::NumberOrString::Number(6),
        }),
        Element::Edge(Edge::TextDocumentReferences {
            id: lsp_types::NumberOrString::Number(15),
            type_: EdgeType::Edge,
            out_v: lsp_types::NumberOrString::Number(2),
            in_v: lsp_types::NumberOrString::Number(7),
        }),
        Element::Edge(Edge::Item {
            id: lsp_types::NumberOrString::Number(16),
            type_: EdgeType::Edge,
            out_v: lsp_types::NumberOrString::Number(7),
            in_vs: vec![lsp_types::NumberOrString::Number(3)],
            document: lsp_types::NumberOrString::Number(1),
            property: None,
        }),
    ];

    for el in &elements {
        let mut quads = Vec::new();
        tower_lsp_max_runtime::control_plane::admission::map_element_to_quads(
            el,
            &active_graph,
            &mut quads,
        )
        .unwrap();
        for quad in quads {
            store.insert(&quad).unwrap();
        }
    }

    let live_diags = vec![MaxDiagnostic {
        diagnostic_id: "diag-integration-1".to_string(),
        law_id: "law-axis-invariant-1".to_string(),
        lsp: Diagnostic {
            message: "Deadlock risk detected in autonomic mesh".to_string(),
            severity: Some(lsp_types::DiagnosticSeverity::ERROR),
            ..Default::default()
        },
        doc_routes: vec![tower_lsp_max_protocol::DocRoute {
            path: "file:///test.rs".to_string(),
        }],
        ..Default::default()
    }];

    for diag in &live_diags {
        let quads = tower_lsp_max_runtime::control_plane::admission::map_diagnostic_to_quads(
            diag,
            &active_graph,
        );
        for quad in quads {
            store.insert(&quad).unwrap();
        }
    }

    let views = MaterializedViewStore::new();
    update_views(&store, &views);

    let test_url = Url::parse("file:///test.rs").unwrap();

    // 1. Definition lookup
    let def_opt = lookup_definition(&views, &test_url, Position::new(10, 7));
    assert!(def_opt.is_some());
    let loc = def_opt.unwrap();
    assert_eq!(loc.uri.as_str(), "file:///test.rs");
    assert_eq!(
        loc.range,
        Range::new(Position::new(2, 5), Position::new(2, 10))
    );

    // 2. References lookup
    let ref_opt = lookup_references(&views, &test_url, Position::new(10, 7));
    assert!(ref_opt.is_some());
    let locs = ref_opt.unwrap();
    assert_eq!(locs.len(), 1);
    assert_eq!(locs[0].uri.as_str(), "file:///test.rs");
    assert_eq!(
        locs[0].range,
        Range::new(Position::new(10, 5), Position::new(10, 10))
    );

    // 3. Hover lookup
    let hover_opt = lookup_hover(&views, &test_url, Position::new(10, 7));
    assert!(hover_opt.is_some());
    let hover = hover_opt.unwrap();
    if let lsp_types::HoverContents::Scalar(lsp_types::MarkedString::String(content)) =
        hover.contents
    {
        assert_eq!(content, "This is foo function");
    } else {
        panic!("Expected marked string hover contents");
    }

    // 4. Diagnostics lookup
    let diags_opt = lookup_diagnostics(&views, &test_url);
    assert!(diags_opt.is_some());
    let diags = diags_opt.unwrap();
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].message, "Deadlock risk detected in autonomic mesh");
}

#[test]
fn test_integration_verify_replay_flow() {
    use ed25519_dalek::{Signer, SigningKey};
    use tower_lsp_max_runtime::control_plane::receipts::{Blake3Hash, CryptographicReceipt};
    use tower_lsp_max_runtime::control_plane::replay::verify_replay;
    use uuid::Uuid;

    let seed = [0u8; 32];
    let signing_key = SigningKey::from_bytes(&seed);
    let verifying_key = signing_key.verifying_key();
    let genesis_hash = Blake3Hash([0u8; 32]);

    let disc_id = Uuid::nil();
    let law_id = Uuid::nil();

    // 1. Genesis receipt (sequence 0)
    let mut r0 = CryptographicReceipt {
        prev_hash: genesis_hash,
        discipline_id: disc_id,
        law_id,
        consequence_hash: Blake3Hash([0u8; 32]),
        sequence: 0,
        signature: [0u8; 64],
    };
    let payload_hash_0 = r0.compute_payload_hash();
    r0.signature = signing_key.sign(&payload_hash_0.0).to_bytes();

    // 2. Next receipt (sequence 1)
    let mut r1 = CryptographicReceipt {
        prev_hash: payload_hash_0,
        discipline_id: disc_id,
        law_id,
        consequence_hash: Blake3Hash([42u8; 32]),
        sequence: 1,
        signature: [0u8; 64],
    };
    let payload_hash_1 = r1.compute_payload_hash();
    r1.signature = signing_key.sign(&payload_hash_1.0).to_bytes();

    let chain = vec![r0, r1];
    let res = verify_replay(&chain, &verifying_key, &genesis_hash);
    assert!(res.is_ok(), "Replay verification failed: {:?}", res);

    // Test failure on invalid hash
    let mut invalid_chain = chain.clone();
    invalid_chain[1].prev_hash = Blake3Hash([99u8; 32]);
    let res_invalid = verify_replay(&invalid_chain, &verifying_key, &genesis_hash);
    assert!(res_invalid.is_err());
}
