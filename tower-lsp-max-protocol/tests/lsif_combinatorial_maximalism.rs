use tower_lsp_max_protocol::lsif::*;
use lsp_types::{NumberOrString, Position, Range, SymbolKind, MarkupContent, MarkupKind, DocumentSymbol, SemanticToken};
use std::fmt::Debug;
use serde::{Serialize, de::DeserializeOwned};

fn roundtrip<T>(val: T, expected_fragments: &[&str])
where
    T: Serialize + DeserializeOwned + PartialEq + Debug,
{
    let json = serde_json::to_string(&val).unwrap();
    for frag in expected_fragments {
        assert!(json.contains(frag), "JSON did not contain '{}'. Full JSON: {}", frag, json);
    }
    let parsed: T = serde_json::from_str(&json).unwrap();
    assert_eq!(val, parsed);
}

#[test]
fn exhaustive_position_encodings() {
    roundtrip(PositionEncoding::Utf8, &["\"utf-8\""]);
    roundtrip(PositionEncoding::Utf16, &["\"utf-16\""]);
    roundtrip(PositionEncoding::Utf32, &["\"utf-32\""]);
}

#[test]
fn exhaustive_moniker_kinds() {
    roundtrip(MonikerKind::Import, &["\"import\""]);
    roundtrip(MonikerKind::Export, &["\"export\""]);
    roundtrip(MonikerKind::Local, &["\"local\""]);
}

#[test]
fn exhaustive_uniqueness_levels() {
    roundtrip(UniquenessLevel::Document, &["\"document\""]);
    roundtrip(UniquenessLevel::Project, &["\"project\""]);
    roundtrip(UniquenessLevel::Workspace, &["\"workspace\""]);
    roundtrip(UniquenessLevel::Scheme, &["\"scheme\""]);
    roundtrip(UniquenessLevel::Global, &["\"global\""]);
}

#[test]
fn exhaustive_event_kinds_and_scopes() {
    roundtrip(EventKind::Begin, &["\"begin\""]);
    roundtrip(EventKind::End, &["\"end\""]);
    roundtrip(EventScope::Project, &["\"project\""]);
    roundtrip(EventScope::Document, &["\"document\""]);
}

#[test]
fn exhaustive_hover_contents() {
    roundtrip(HoverContents::String("text".to_string()), &["\"text\""]);
    roundtrip(HoverContents::Markup(MarkupContent { kind: MarkupKind::Markdown, value: "md".to_string() }), &["\"markdown\""]);
}

#[test]
fn exhaustive_range_tags() {
    let dummy_range = Range { start: Position { line: 0, character: 0 }, end: Position { line: 0, character: 1 } };
    
    roundtrip(RangeTag::Declaration {
        text: "d".to_string(),
        kind: SymbolKind::FILE,
        full_range: dummy_range.clone(),
        detail: None,
    }, &["\"type\":\"declaration\""]);

    roundtrip(RangeTag::Definition {
        text: "d".to_string(),
        kind: SymbolKind::FILE,
        full_range: dummy_range.clone(),
        detail: Some("detail".to_string()),
    }, &["\"type\":\"definition\"", "\"detail\""]);

    roundtrip(RangeTag::Reference { text: "r".to_string() }, &["\"type\":\"reference\""]);
    roundtrip(RangeTag::Unknown { text: "u".to_string() }, &["\"type\":\"unknown\""]);
}

#[test]
fn exhaustive_vertices() {
    let id = Id::Number(1);
    let v_type = VertexType::Vertex;

    roundtrip(Vertex::MetaData {
        id: id.clone(), type_: v_type.clone(), version: "0.6.0".to_string(), position_encoding: PositionEncoding::Utf16, tool_info: None
    }, &["\"label\":\"metaData\""]);

    roundtrip(Vertex::Source {
        id: id.clone(), type_: v_type.clone(), workspace_root: "w".to_string(), repository: None
    }, &["\"label\":\"source\""]);

    roundtrip(Vertex::Project {
        id: id.clone(), type_: v_type.clone(), kind: "k".to_string(), resource: None, contents: None
    }, &["\"label\":\"project\""]);

    roundtrip(Vertex::Document {
        id: id.clone(), type_: v_type.clone(), uri: "u".to_string(), language_id: "l".to_string(), contents: None
    }, &["\"label\":\"document\""]);

    roundtrip(Vertex::ResultSet { id: id.clone(), type_: v_type.clone() }, &["\"label\":\"resultSet\""]);

    roundtrip(Vertex::Range {
        id: id.clone(), type_: v_type.clone(), start: Position{line:0, character:0}, end: Position{line:0, character:0}, tag: None
    }, &["\"label\":\"range\""]);

    roundtrip(Vertex::ResultRange {
        id: id.clone(), type_: v_type.clone(), start: Position{line:0, character:0}, end: Position{line:0, character:0}
    }, &["\"label\":\"resultRange\""]);

    roundtrip(Vertex::Moniker {
        id: id.clone(), type_: v_type.clone(), scheme: "s".to_string(), identifier: "i".to_string(), kind: MonikerKind::Import, unique: UniquenessLevel::Workspace
    }, &["\"label\":\"moniker\""]);

    roundtrip(Vertex::PackageInformation {
        id: id.clone(), type_: v_type.clone(), name: "n".to_string(), manager: "m".to_string(), version: "v".to_string(), repository: None
    }, &["\"label\":\"packageInformation\""]);

    roundtrip(Vertex::HoverResult {
        id: id.clone(), type_: v_type.clone(), result: HoverResultData { contents: HoverContents::String("s".to_string()), range: None }
    }, &["\"label\":\"hoverResult\""]);

    roundtrip(Vertex::ReferenceResult { id: id.clone(), type_: v_type.clone() }, &["\"label\":\"referenceResult\""]);
    roundtrip(Vertex::DeclarationResult { id: id.clone(), type_: v_type.clone() }, &["\"label\":\"declarationResult\""]);
    roundtrip(Vertex::DefinitionResult { id: id.clone(), type_: v_type.clone() }, &["\"label\":\"definitionResult\""]);
    roundtrip(Vertex::ImplementationResult { id: id.clone(), type_: v_type.clone() }, &["\"label\":\"implementationResult\""]);
    roundtrip(Vertex::TypeDefinitionResult { id: id.clone(), type_: v_type.clone() }, &["\"label\":\"typeDefinitionResult\""]);
    
    roundtrip(Vertex::FoldingRangeResult { id: id.clone(), type_: v_type.clone(), result: vec![] }, &["\"label\":\"foldingRangeResult\""]);
    roundtrip(Vertex::DocumentLinkResult { id: id.clone(), type_: v_type.clone(), result: vec![] }, &["\"label\":\"documentLinkResult\""]);
    
    roundtrip(Vertex::DocumentSymbolResult {
        id: id.clone(), type_: v_type.clone(), result: DocumentSymbolResultData::RangeBased(vec![
            RangeBasedDocumentSymbol { id: Id::Number(2), children: None }
        ])
    }, &["\"label\":\"documentSymbolResult\""]);

    roundtrip(Vertex::DiagnosticResult { id: id.clone(), type_: v_type.clone(), result: vec![] }, &["\"label\":\"diagnosticResult\""]);
    
    roundtrip(Vertex::SemanticTokensResult {
        id: id.clone(), type_: v_type.clone(), result: SemanticTokensData { data: vec![] }
    }, &["\"label\":\"semanticTokensResult\""]);

    roundtrip(Vertex::Event {
        id: id.clone(), type_: v_type.clone(), kind: EventKind::Begin, scope: EventScope::Project, data: Id::Number(2)
    }, &["\"label\":\"$event\""]);
}

#[test]
fn exhaustive_edges() {
    let id = Id::Number(1);
    let e_type = EdgeType::Edge;
    let out_v = Id::Number(2);
    let in_v = Id::Number(3);
    let in_vs = vec![Id::Number(4)];

    roundtrip(Edge::Contains { id: id.clone(), type_: e_type.clone(), out_v: out_v.clone(), in_vs: in_vs.clone() }, &["\"label\":\"contains\""]);
    roundtrip(Edge::Next { id: id.clone(), type_: e_type.clone(), out_v: out_v.clone(), in_v: in_v.clone() }, &["\"label\":\"next\""]);
    roundtrip(Edge::Moniker { id: id.clone(), type_: e_type.clone(), out_v: out_v.clone(), in_v: in_v.clone() }, &["\"label\":\"moniker\""]);
    roundtrip(Edge::Attach { id: id.clone(), type_: e_type.clone(), out_v: out_v.clone(), in_v: in_v.clone() }, &["\"label\":\"attach\""]);
    roundtrip(Edge::PackageInformation { id: id.clone(), type_: e_type.clone(), out_v: out_v.clone(), in_v: in_v.clone() }, &["\"label\":\"packageInformation\""]);
    roundtrip(Edge::Item { id: id.clone(), type_: e_type.clone(), out_v: out_v.clone(), in_vs: in_vs.clone(), document: in_v.clone(), property: Some(ItemEdgeProperty::Definitions) }, &["\"label\":\"item\""]);
    
    roundtrip(Edge::TextDocumentHover { id: id.clone(), type_: e_type.clone(), out_v: out_v.clone(), in_v: in_v.clone() }, &["\"label\":\"textDocument/hover\""]);
    roundtrip(Edge::TextDocumentDefinition { id: id.clone(), type_: e_type.clone(), out_v: out_v.clone(), in_v: in_v.clone() }, &["\"label\":\"textDocument/definition\""]);
    roundtrip(Edge::TextDocumentDeclaration { id: id.clone(), type_: e_type.clone(), out_v: out_v.clone(), in_v: in_v.clone() }, &["\"label\":\"textDocument/declaration\""]);
    roundtrip(Edge::TextDocumentReferences { id: id.clone(), type_: e_type.clone(), out_v: out_v.clone(), in_v: in_v.clone() }, &["\"label\":\"textDocument/references\""]);
    roundtrip(Edge::TextDocumentImplementation { id: id.clone(), type_: e_type.clone(), out_v: out_v.clone(), in_v: in_v.clone() }, &["\"label\":\"textDocument/implementation\""]);
    roundtrip(Edge::TextDocumentTypeDefinition { id: id.clone(), type_: e_type.clone(), out_v: out_v.clone(), in_v: in_v.clone() }, &["\"label\":\"textDocument/typeDefinition\""]);
    roundtrip(Edge::TextDocumentFoldingRange { id: id.clone(), type_: e_type.clone(), out_v: out_v.clone(), in_v: in_v.clone() }, &["\"label\":\"textDocument/foldingRange\""]);
    roundtrip(Edge::TextDocumentDocumentLink { id: id.clone(), type_: e_type.clone(), out_v: out_v.clone(), in_v: in_v.clone() }, &["\"label\":\"textDocument/documentLink\""]);
    roundtrip(Edge::TextDocumentDocumentSymbol { id: id.clone(), type_: e_type.clone(), out_v: out_v.clone(), in_v: in_v.clone() }, &["\"label\":\"textDocument/documentSymbol\""]);
    roundtrip(Edge::TextDocumentDiagnostic { id: id.clone(), type_: e_type.clone(), out_v: out_v.clone(), in_v: in_v.clone() }, &["\"label\":\"textDocument/diagnostic\""]);
    roundtrip(Edge::TextDocumentSemanticTokens { id: id.clone(), type_: e_type.clone(), out_v: out_v.clone(), in_v: in_v.clone() }, &["\"label\":\"textDocument/semanticTokens\""]);
}

#[test]
fn test_mixed_element_wrapper() {
    let v = Element::Vertex(Vertex::ResultSet { id: Id::Number(1), type_: VertexType::Vertex });
    let e = Element::Edge(Edge::Next { id: Id::Number(2), type_: EdgeType::Edge, out_v: Id::Number(1), in_v: Id::Number(3) });

    roundtrip(v, &["\"label\":\"resultSet\"", "\"type\":\"vertex\""]);
    roundtrip(e, &["\"label\":\"next\"", "\"type\":\"edge\""]);
}
