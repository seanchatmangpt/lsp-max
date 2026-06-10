use lsp_types_max::NumberOrString;
use tower_lsp_max_lsif::lsif::*;
use tower_lsp_max_lsif::lsif_builder::LsifBuilder;

#[test]
fn test_metadata_conformance() {
    let mut buf = Vec::new();
    let mut builder = LsifBuilder::new(&mut buf);
    builder
        .emit_metadata(
            "0.6.0",
            "file:///test_project",
            ToolInfo {
                name: "test-conformance".to_string(),
                version: Some("1.0.0".to_string()),
                args: None,
            },
        )
        .unwrap();

    let json_str = String::from_utf8(buf).unwrap();
    assert!(json_str.contains(r#""projectRoot":"file:///test_project""#));
    assert!(json_str.contains(r#""version":"0.6.0""#));
    assert!(json_str.contains(r#""label":"metaData""#));
}

#[test]
fn test_project_optional_kind_conformance() {
    // 1. Project with kind Some
    let proj_some = Element::Vertex(Vertex::Project {
        id: NumberOrString::Number(1),
        type_: VertexType::Vertex,
        kind: Some("rust".to_string()),
        resource: Some("file:///proj".to_string()),
        contents: None,
    });
    let json_some = serde_json::to_string(&proj_some).unwrap();
    assert!(json_some.contains(r#""kind":"rust""#));

    // 2. Project with kind None (verifies it serializes and deserializes without error)
    let proj_none = Element::Vertex(Vertex::Project {
        id: NumberOrString::Number(2),
        type_: VertexType::Vertex,
        kind: None,
        resource: Some("file:///proj".to_string()),
        contents: None,
    });
    let json_none = serde_json::to_string(&proj_none).unwrap();
    assert!(!json_none.contains(r#""kind""#));

    let deserialized: Element = serde_json::from_str(&json_none).unwrap();
    if let Element::Vertex(Vertex::Project { kind, .. }) = deserialized {
        assert!(kind.is_none());
    } else {
        panic!("Expected Project variant");
    }
}

#[test]
fn test_moniker_uniqueness_level_group_conformance() {
    let moniker = Element::Vertex(Vertex::Moniker {
        id: NumberOrString::Number(1),
        type_: VertexType::Vertex,
        scheme: "cargo".to_string(),
        identifier: "test".to_string(),
        kind: MonikerKind::Export,
        unique: UniquenessLevel::Group,
    });
    let json_str = serde_json::to_string(&moniker).unwrap();
    assert!(json_str.contains(r#""unique":"group""#));

    let deserialized: Element = serde_json::from_str(&json_str).unwrap();
    if let Element::Vertex(Vertex::Moniker { unique, .. }) = deserialized {
        assert_eq!(unique, UniquenessLevel::Group);
    } else {
        panic!("Expected Moniker variant");
    }
}

#[test]
fn test_call_and_type_hierarchy_vocabulary_conformance() {
    let call_res = Element::Vertex(Vertex::CallHierarchyResult {
        id: NumberOrString::Number(1),
        type_: VertexType::Vertex,
    });
    let json_res = serde_json::to_string(&call_res).unwrap();
    assert!(json_res.contains(r#""label":"callHierarchyResult""#));

    let call_edge = Element::Edge(Edge::TextDocumentCallHierarchy {
        id: NumberOrString::Number(2),
        type_: EdgeType::Edge,
        out_v: NumberOrString::Number(10),
        in_v: NumberOrString::Number(11),
    });
    let json_edge = serde_json::to_string(&call_edge).unwrap();
    assert!(json_edge.contains(r#""label":"textDocument/callHierarchy""#));

    let type_res = Element::Vertex(Vertex::TypeHierarchyResult {
        id: NumberOrString::Number(3),
        type_: VertexType::Vertex,
    });
    let json_type_res = serde_json::to_string(&type_res).unwrap();
    assert!(json_type_res.contains(r#""label":"typeHierarchyResult""#));

    let type_edge = Element::Edge(Edge::TextDocumentTypeHierarchy {
        id: NumberOrString::Number(4),
        type_: EdgeType::Edge,
        out_v: NumberOrString::Number(12),
        in_v: NumberOrString::Number(13),
    });
    let json_type_edge = serde_json::to_string(&type_edge).unwrap();
    assert!(json_type_edge.contains(r#""label":"textDocument/typeHierarchy""#));
}
