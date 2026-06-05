use tower_lsp_max_protocol::lsif::*;
use lsp_types::{NumberOrString, Position, Range, SymbolKind};

#[test]
fn validate_moniker_and_package_info() {
    let moniker = Vertex::Moniker {
        id: Id::Number(1),
        type_: VertexType::Vertex,
        scheme: "npm".to_string(),
        identifier: "lodash".to_string(),
        kind: MonikerKind::Import,
        unique: UniquenessLevel::Workspace,
    };
    let json = serde_json::to_string(&moniker).unwrap();
    assert!(json.contains(r#""label":"moniker""#));
    assert!(json.contains(r#""kind":"import""#));

    let pkg = Vertex::PackageInformation {
        id: Id::Number(2),
        type_: VertexType::Vertex,
        name: "lodash".to_string(),
        manager: "npm".to_string(),
        version: "4.17.21".to_string(),
        repository: None,
    };
    let json = serde_json::to_string(&pkg).unwrap();
    assert!(json.contains(r#""label":"packageInformation""#));

    let pkg_edge = Edge::PackageInformation {
        id: Id::Number(3),
        type_: EdgeType::Edge,
        out_v: Id::Number(1),
        in_v: Id::Number(2),
    };
    let json = serde_json::to_string(&pkg_edge).unwrap();
    assert!(json.contains(r#""label":"packageInformation""#));
}

#[test]
fn validate_document_symbols() {
    let sym = Vertex::DocumentSymbolResult {
        id: Id::Number(1),
        type_: VertexType::Vertex,
        result: DocumentSymbolResultData::RangeBased(vec![
            RangeBasedDocumentSymbol {
                id: Id::Number(2),
                children: None,
            }
        ]),
    };
    let json = serde_json::to_string(&sym).unwrap();
    assert!(json.contains(r#""label":"documentSymbolResult""#));
}

#[test]
fn validate_semantic_tokens() {
    let tokens = Vertex::SemanticTokensResult {
        id: Id::Number(1),
        type_: VertexType::Vertex,
        result: SemanticTokensData {
            data: vec![0, 5, 4, 1, 0],
        },
    };
    let json = serde_json::to_string(&tokens).unwrap();
    assert!(json.contains(r#""label":"semanticTokensResult""#));
}

#[test]
fn validate_events() {
    let event = Vertex::Event {
        id: Id::Number(1),
        type_: VertexType::Vertex,
        kind: EventKind::Begin,
        scope: EventScope::Document,
        data: Id::Number(5),
    };
    let json = serde_json::to_string(&event).unwrap();
    assert!(json.contains(r#""label":"$event""#));
}
