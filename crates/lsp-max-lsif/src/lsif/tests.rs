use super::*;
use lsp_types_max::NumberOrString;

#[test]
fn test_serialize_metadata() {
    let meta = Element::Vertex(Vertex::MetaData {
        id: NumberOrString::Number(1),
        type_: VertexType::Vertex,
        version: "0.6.0".to_string(),
        project_root: "file:///".to_string(),
        position_encoding: PositionEncoding::Utf16,
        tool_info: Some(ToolInfo {
            name: "lsp-max".to_string(),
            version: Some("1.0.0".to_string()),
            args: None,
        }),
    });

    let json = serde_json::to_string(&meta).unwrap();
    assert!(json.contains(r#""label":"metaData""#));
    assert!(json.contains(r#""type":"vertex""#));
    assert!(json.contains(r#""version":"0.6.0""#));
    assert!(json.contains(r#""projectRoot":"file:///""#));
}

#[test]
fn test_serialize_contains_edge() {
    let edge = Element::Edge(Edge::Contains {
        id: NumberOrString::Number(2),
        type_: EdgeType::Edge,
        out_v: NumberOrString::Number(1),
        in_vs: vec![NumberOrString::Number(3), NumberOrString::Number(4)],
    });

    let json = serde_json::to_string(&edge).unwrap();
    assert!(json.contains(r#""label":"contains""#));
    assert!(json.contains(r#""type":"edge""#));
    assert!(json.contains(r#""inVs":[3,4]"#));
}

#[test]
fn test_serialize_next_moniker_edge() {
    let edge = Element::Edge(Edge::NextMoniker {
        id: NumberOrString::Number(5),
        type_: EdgeType::Edge,
        out_v: NumberOrString::Number(1),
        in_v: NumberOrString::Number(2),
    });

    let json = serde_json::to_string(&edge).unwrap();
    assert!(json.contains(r#""label":"nextMoniker""#));
}

#[test]
fn test_serialize_belongs_to_edge() {
    let edge = Element::Edge(Edge::BelongsTo {
        id: NumberOrString::Number(6),
        type_: EdgeType::Edge,
        out_v: NumberOrString::Number(1),
        in_v: NumberOrString::Number(2),
    });

    let json = serde_json::to_string(&edge).unwrap();
    assert!(json.contains(r#""label":"belongsTo""#));
}

#[test]
fn test_serialize_capabilities_vertex() {
    let vertex = Element::Vertex(Vertex::Capabilities {
        id: NumberOrString::Number(7),
        type_: VertexType::Vertex,
        hover_provider: true,
        declaration_provider: true,
        definition_provider: true,
        type_definition_provider: true,
        references_provider: true,
        document_symbol_provider: true,
        folding_range_provider: true,
        diagnostic_provider: true,
    });

    let json = serde_json::to_string(&vertex).unwrap();
    assert!(json.contains(r#""label":"capabilities""#));
    assert!(json.contains(r#""hoverProvider":true"#));
}
