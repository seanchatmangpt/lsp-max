use super::lookups::*;
use super::types::*;
use super::update::*;
use lsp_types_max::{Diagnostic, Position, Range, SymbolKind};
use oxigraph::model::GraphName;
use oxigraph::model::NamedNode;
use oxigraph::store::Store;
use lsp_max_lsif::lsif::{
    Edge, EdgeType, Element, HoverContents, HoverResultData, RangeTag, Vertex, VertexType,
};
use lsp_max_protocol::MaxDiagnostic;
use url::Url;

#[allow(dead_code)]
fn make_meta_element() -> Element {
    Element::Vertex(Vertex::MetaData {
        id: lsp_types_max::NumberOrString::Number(1),
        type_: VertexType::Vertex,
        version: "0.6.0".to_string(),
        project_root: "file:///".to_string(),
        position_encoding: lsp_max_lsif::lsif::PositionEncoding::Utf16,
        tool_info: None,
    })
}

#[test]
fn test_materialized_views_definition_references_hover_diagnostics() {
    let store = Store::new().unwrap();
    let active_graph =
        GraphName::NamedNode(NamedNode::new("urn:project:local:snapshot:snap-1").unwrap());

    let elements = vec![
        Element::Vertex(Vertex::Document {
            id: lsp_types_max::NumberOrString::Number(1),
            type_: VertexType::Vertex,
            uri: "file:///test.rs".to_string(),
            language_id: "rust".to_string(),
            contents: None,
        }),
        Element::Vertex(Vertex::Range {
            id: lsp_types_max::NumberOrString::Number(3),
            type_: VertexType::Vertex,
            start: Position::new(10, 5),
            end: Position::new(10, 10),
            tag: Some(RangeTag::Reference {
                text: "foo".to_string(),
            }),
        }),
        Element::Vertex(Vertex::ResultSet {
            id: lsp_types_max::NumberOrString::Number(2),
            type_: VertexType::Vertex,
        }),
        Element::Vertex(Vertex::DefinitionResult {
            id: lsp_types_max::NumberOrString::Number(4),
            type_: VertexType::Vertex,
        }),
        Element::Vertex(Vertex::Range {
            id: lsp_types_max::NumberOrString::Number(5),
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
            id: lsp_types_max::NumberOrString::Number(6),
            type_: VertexType::Vertex,
            result: HoverResultData {
                contents: HoverContents::String("This is foo function".to_string()),
                range: None,
            },
        }),
        Element::Vertex(Vertex::ReferenceResult {
            id: lsp_types_max::NumberOrString::Number(7),
            type_: VertexType::Vertex,
        }),
        // Edges:
        // Document contains src range
        Element::Edge(Edge::Contains {
            id: lsp_types_max::NumberOrString::Number(10),
            type_: EdgeType::Edge,
            out_v: lsp_types_max::NumberOrString::Number(1),
            in_vs: vec![
                lsp_types_max::NumberOrString::Number(3),
                lsp_types_max::NumberOrString::Number(5),
            ],
        }),
        // src range next resultSet
        Element::Edge(Edge::Next {
            id: lsp_types_max::NumberOrString::Number(11),
            type_: EdgeType::Edge,
            out_v: lsp_types_max::NumberOrString::Number(3),
            in_v: lsp_types_max::NumberOrString::Number(2),
        }),
        // resultSet definition definitionResult
        Element::Edge(Edge::TextDocumentDefinition {
            id: lsp_types_max::NumberOrString::Number(12),
            type_: EdgeType::Edge,
            out_v: lsp_types_max::NumberOrString::Number(2),
            in_v: lsp_types_max::NumberOrString::Number(4),
        }),
        // definitionResult item Range(Def)
        Element::Edge(Edge::Item {
            id: lsp_types_max::NumberOrString::Number(13),
            type_: EdgeType::Edge,
            out_v: lsp_types_max::NumberOrString::Number(4),
            in_vs: vec![lsp_types_max::NumberOrString::Number(5)],
            document: lsp_types_max::NumberOrString::Number(1),
            property: None,
        }),
        // resultSet hover hoverResult
        Element::Edge(Edge::TextDocumentHover {
            id: lsp_types_max::NumberOrString::Number(14),
            type_: EdgeType::Edge,
            out_v: lsp_types_max::NumberOrString::Number(2),
            in_v: lsp_types_max::NumberOrString::Number(6),
        }),
        // resultSet references referenceResult
        Element::Edge(Edge::TextDocumentReferences {
            id: lsp_types_max::NumberOrString::Number(15),
            type_: EdgeType::Edge,
            out_v: lsp_types_max::NumberOrString::Number(2),
            in_v: lsp_types_max::NumberOrString::Number(7),
        }),
        // referenceResult item Range(src)
        Element::Edge(Edge::Item {
            id: lsp_types_max::NumberOrString::Number(16),
            type_: EdgeType::Edge,
            out_v: lsp_types_max::NumberOrString::Number(7),
            in_vs: vec![lsp_types_max::NumberOrString::Number(3)],
            document: lsp_types_max::NumberOrString::Number(1),
            property: None,
        }),
    ];

    // Populate store with quads
    for el in &elements {
        let mut quads = Vec::new();
        crate::control_plane::admission::mapping::map_element_to_quads(
            el,
            &active_graph,
            &mut quads,
        )
        .unwrap();
        for quad in quads {
            store.insert(&quad).unwrap();
        }
    }

    // LiveLSP diagnostics
    let live_diags = vec![MaxDiagnostic {
        diagnostic_id: "diag-1".to_string(),
        law_id: "law-axis-invariant-1".to_string(),
        lsp: Diagnostic {
            message: "Deadlock risk detected in autonomic mesh".to_string(),
            severity: Some(lsp_types_max::DiagnosticSeverity::ERROR),
            ..Default::default()
        },
        doc_routes: vec![lsp_max_protocol::DocRoute {
            path: "file:///test.rs".to_string(),
        }],
        ..Default::default()
    }];

    // Map live diag to store
    for diag in &live_diags {
        let quads = crate::control_plane::admission::mapping_helpers::map_diagnostic_to_quads(
            diag,
            &active_graph,
        );
        for quad in quads {
            store.insert(&quad).unwrap();
        }
    }

    let _views = MaterializedViews::new();
    // Use an internal helper or simulate update_sync
    // For test compatibility, we can verify update_views on MaterializedViewStore
    let store_views = MaterializedViewStore::new();
    update_views(&store, &store_views);

    let test_url = Url::parse("file:///test.rs").unwrap();

    // Query the views!
    // 1. Definition lookup
    let def_opt = lookup_definition(&store_views, &test_url, Position::new(10, 7));
    assert!(def_opt.is_some());
    let loc = def_opt.unwrap();
    assert_eq!(loc.uri.as_str(), "file:///test.rs");
    assert_eq!(
        loc.range,
        Range::new(Position::new(2, 5), Position::new(2, 10))
    );

    // 2. References lookup
    let ref_opt = lookup_references(&store_views, &test_url, Position::new(10, 7));
    assert!(ref_opt.is_some());
    let locs = ref_opt.unwrap();
    assert_eq!(locs.len(), 1);
    assert_eq!(locs[0].uri.as_str(), "file:///test.rs");
    assert_eq!(
        locs[0].range,
        Range::new(Position::new(10, 5), Position::new(10, 10))
    );

    // 3. Hover lookup
    let hover_opt = lookup_hover(&store_views, &test_url, Position::new(10, 7));
    assert!(hover_opt.is_some());
    let hover = hover_opt.unwrap();
    if let lsp_types_max::HoverContents::Scalar(lsp_types_max::MarkedString::String(content)) =
        hover.contents
    {
        assert_eq!(content, "This is foo function");
    } else {
        panic!("Expected marked string hover contents");
    }

    // 4. Diagnostics lookup
    let diags_opt = lookup_diagnostics(&store_views, &test_url);
    assert!(diags_opt.is_some());
    let diags = diags_opt.unwrap();
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].message, "Deadlock risk detected in autonomic mesh");
}

#[test]
fn test_materialized_view_store_lookups() {
    let store = Store::new().unwrap();
    let active_graph =
        GraphName::NamedNode(NamedNode::new("urn:project:local:snapshot:snap-2").unwrap());

    let elements = vec![
        Element::Vertex(Vertex::Document {
            id: lsp_types_max::NumberOrString::Number(1),
            type_: VertexType::Vertex,
            uri: "file:///test.rs".to_string(),
            language_id: "rust".to_string(),
            contents: None,
        }),
        Element::Vertex(Vertex::Range {
            id: lsp_types_max::NumberOrString::Number(3),
            type_: VertexType::Vertex,
            start: Position::new(10, 5),
            end: Position::new(10, 10),
            tag: Some(RangeTag::Reference {
                text: "foo".to_string(),
            }),
        }),
        Element::Vertex(Vertex::ResultSet {
            id: lsp_types_max::NumberOrString::Number(2),
            type_: VertexType::Vertex,
        }),
        Element::Vertex(Vertex::DefinitionResult {
            id: lsp_types_max::NumberOrString::Number(4),
            type_: VertexType::Vertex,
        }),
        Element::Vertex(Vertex::Range {
            id: lsp_types_max::NumberOrString::Number(5),
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
            id: lsp_types_max::NumberOrString::Number(6),
            type_: VertexType::Vertex,
            result: HoverResultData {
                contents: HoverContents::String("This is foo function".to_string()),
                range: None,
            },
        }),
        Element::Vertex(Vertex::ReferenceResult {
            id: lsp_types_max::NumberOrString::Number(7),
            type_: VertexType::Vertex,
        }),
        Element::Edge(Edge::Contains {
            id: lsp_types_max::NumberOrString::Number(10),
            type_: EdgeType::Edge,
            out_v: lsp_types_max::NumberOrString::Number(1),
            in_vs: vec![
                lsp_types_max::NumberOrString::Number(3),
                lsp_types_max::NumberOrString::Number(5),
            ],
        }),
        Element::Edge(Edge::Next {
            id: lsp_types_max::NumberOrString::Number(11),
            type_: EdgeType::Edge,
            out_v: lsp_types_max::NumberOrString::Number(3),
            in_v: lsp_types_max::NumberOrString::Number(2),
        }),
        Element::Edge(Edge::TextDocumentDefinition {
            id: lsp_types_max::NumberOrString::Number(12),
            type_: EdgeType::Edge,
            out_v: lsp_types_max::NumberOrString::Number(2),
            in_v: lsp_types_max::NumberOrString::Number(4),
        }),
        Element::Edge(Edge::Item {
            id: lsp_types_max::NumberOrString::Number(13),
            type_: EdgeType::Edge,
            out_v: lsp_types_max::NumberOrString::Number(4),
            in_vs: vec![lsp_types_max::NumberOrString::Number(5)],
            document: lsp_types_max::NumberOrString::Number(1),
            property: None,
        }),
        Element::Edge(Edge::TextDocumentHover {
            id: lsp_types_max::NumberOrString::Number(14),
            type_: EdgeType::Edge,
            out_v: lsp_types_max::NumberOrString::Number(2),
            in_v: lsp_types_max::NumberOrString::Number(6),
        }),
        Element::Edge(Edge::TextDocumentReferences {
            id: lsp_types_max::NumberOrString::Number(15),
            type_: EdgeType::Edge,
            out_v: lsp_types_max::NumberOrString::Number(2),
            in_v: lsp_types_max::NumberOrString::Number(7),
        }),
        Element::Edge(Edge::Item {
            id: lsp_types_max::NumberOrString::Number(16),
            type_: EdgeType::Edge,
            out_v: lsp_types_max::NumberOrString::Number(7),
            in_vs: vec![lsp_types_max::NumberOrString::Number(3)],
            document: lsp_types_max::NumberOrString::Number(1),
            property: None,
        }),
    ];

    for el in &elements {
        let mut quads = Vec::new();
        crate::control_plane::admission::mapping::map_element_to_quads(
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
        diagnostic_id: "diag-2".to_string(),
        law_id: "law-axis-invariant-1".to_string(),
        lsp: Diagnostic {
            message: "Deadlock risk detected in autonomic mesh".to_string(),
            severity: Some(lsp_types_max::DiagnosticSeverity::ERROR),
            ..Default::default()
        },
        doc_routes: vec![lsp_max_protocol::DocRoute {
            path: "file:///test.rs".to_string(),
        }],
        ..Default::default()
    }];

    for diag in &live_diags {
        let quads = crate::control_plane::admission::mapping_helpers::map_diagnostic_to_quads(
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
    if let lsp_types_max::HoverContents::Scalar(lsp_types_max::MarkedString::String(content)) =
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
