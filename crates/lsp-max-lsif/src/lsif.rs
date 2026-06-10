pub use crate::lsif_types::*;
use lsp_types_max::Position;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "label")]
pub enum Vertex {
    #[serde(rename = "metaData")]
    MetaData {
        id: Id,
        #[serde(rename = "type")]
        type_: VertexType,
        version: String,
        #[serde(rename = "projectRoot")]
        project_root: Uri,
        #[serde(rename = "positionEncoding")]
        position_encoding: PositionEncoding,
        #[serde(rename = "toolInfo", skip_serializing_if = "Option::is_none")]
        tool_info: Option<ToolInfo>,
    },
    #[serde(rename = "source")]
    Source {
        id: Id,
        #[serde(rename = "type")]
        type_: VertexType,
        #[serde(rename = "workspaceRoot")]
        workspace_root: Uri,
        #[serde(skip_serializing_if = "Option::is_none")]
        repository: Option<Repository>,
    },
    #[serde(rename = "project")]
    Project {
        id: Id,
        #[serde(rename = "type")]
        type_: VertexType,
        #[serde(skip_serializing_if = "Option::is_none")]
        #[serde(default)]
        kind: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        resource: Option<Uri>,
        #[serde(skip_serializing_if = "Option::is_none")]
        contents: Option<String>,
    },
    #[serde(rename = "document")]
    Document {
        id: Id,
        #[serde(rename = "type")]
        type_: VertexType,
        uri: Uri,
        #[serde(rename = "languageId")]
        language_id: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        contents: Option<String>,
    },
    #[serde(rename = "resultSet")]
    ResultSet {
        id: Id,
        #[serde(rename = "type")]
        type_: VertexType,
    },
    #[serde(rename = "range")]
    Range {
        id: Id,
        #[serde(rename = "type")]
        type_: VertexType,
        start: Position,
        end: Position,
        #[serde(skip_serializing_if = "Option::is_none")]
        tag: Option<RangeTag>,
    },
    #[serde(rename = "resultRange")]
    ResultRange {
        id: Id,
        #[serde(rename = "type")]
        type_: VertexType,
        start: Position,
        end: Position,
    },
    #[serde(rename = "moniker")]
    Moniker {
        id: Id,
        #[serde(rename = "type")]
        type_: VertexType,
        scheme: String,
        identifier: String,
        kind: MonikerKind,
        unique: UniquenessLevel,
    },
    #[serde(rename = "packageInformation")]
    PackageInformation {
        id: Id,
        #[serde(rename = "type")]
        type_: VertexType,
        name: String,
        manager: String,
        version: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        repository: Option<Repository>,
    },
    #[serde(rename = "hoverResult")]
    HoverResult {
        id: Id,
        #[serde(rename = "type")]
        type_: VertexType,
        result: HoverResultData,
    },
    #[serde(rename = "referenceResult")]
    ReferenceResult {
        id: Id,
        #[serde(rename = "type")]
        type_: VertexType,
    },
    #[serde(rename = "declarationResult")]
    DeclarationResult {
        id: Id,
        #[serde(rename = "type")]
        type_: VertexType,
    },
    #[serde(rename = "definitionResult")]
    DefinitionResult {
        id: Id,
        #[serde(rename = "type")]
        type_: VertexType,
    },
    #[serde(rename = "implementationResult")]
    ImplementationResult {
        id: Id,
        #[serde(rename = "type")]
        type_: VertexType,
    },
    #[serde(rename = "typeDefinitionResult")]
    TypeDefinitionResult {
        id: Id,
        #[serde(rename = "type")]
        type_: VertexType,
    },
    #[serde(rename = "callHierarchyResult")]
    CallHierarchyResult {
        id: Id,
        #[serde(rename = "type")]
        type_: VertexType,
    },
    #[serde(rename = "typeHierarchyResult")]
    TypeHierarchyResult {
        id: Id,
        #[serde(rename = "type")]
        type_: VertexType,
    },
    #[serde(rename = "foldingRangeResult")]
    FoldingRangeResult {
        id: Id,
        #[serde(rename = "type")]
        type_: VertexType,
        result: Vec<lsp_types_max::FoldingRange>,
    },
    #[serde(rename = "documentLinkResult")]
    DocumentLinkResult {
        id: Id,
        #[serde(rename = "type")]
        type_: VertexType,
        result: Vec<lsp_types_max::DocumentLink>,
    },
    #[serde(rename = "documentSymbolResult")]
    DocumentSymbolResult {
        id: Id,
        #[serde(rename = "type")]
        type_: VertexType,
        result: DocumentSymbolResultData,
    },
    #[serde(rename = "diagnosticResult")]
    DiagnosticResult {
        id: Id,
        #[serde(rename = "type")]
        type_: VertexType,
        result: Vec<lsp_types_max::Diagnostic>,
    },
    #[serde(rename = "semanticTokensResult")]
    SemanticTokensResult {
        id: Id,
        #[serde(rename = "type")]
        type_: VertexType,
        result: SemanticTokensData,
    },
    #[serde(rename = "$event")]
    Event {
        id: Id,
        #[serde(rename = "type")]
        type_: VertexType,
        kind: EventKind,
        scope: EventScope,
        data: Id,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ItemEdgeProperty {
    Definitions,
    Declarations,
    References,
    ReferenceResults,
    ImplementationResults,
    #[serde(rename = "typeDefinitionResults")]
    TypeDefinitions,
    ReferenceLinks,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "label")]
pub enum Edge {
    #[serde(rename = "contains")]
    Contains {
        id: Id,
        #[serde(rename = "type")]
        type_: EdgeType,
        #[serde(rename = "outV")]
        out_v: Id,
        #[serde(rename = "inVs")]
        in_vs: Vec<Id>,
    },
    #[serde(rename = "next")]
    Next {
        id: Id,
        #[serde(rename = "type")]
        type_: EdgeType,
        #[serde(rename = "outV")]
        out_v: Id,
        #[serde(rename = "inV")]
        in_v: Id,
    },
    #[serde(rename = "moniker")]
    Moniker {
        id: Id,
        #[serde(rename = "type")]
        type_: EdgeType,
        #[serde(rename = "outV")]
        out_v: Id,
        #[serde(rename = "inV")]
        in_v: Id,
    },
    #[serde(rename = "attach")]
    Attach {
        id: Id,
        #[serde(rename = "type")]
        type_: EdgeType,
        #[serde(rename = "outV")]
        out_v: Id,
        #[serde(rename = "inV")]
        in_v: Id,
    },
    #[serde(rename = "packageInformation")]
    PackageInformation {
        id: Id,
        #[serde(rename = "type")]
        type_: EdgeType,
        #[serde(rename = "outV")]
        out_v: Id,
        #[serde(rename = "inV")]
        in_v: Id,
    },
    #[serde(rename = "item")]
    Item {
        id: Id,
        #[serde(rename = "type")]
        type_: EdgeType,
        #[serde(rename = "outV")]
        out_v: Id,
        #[serde(rename = "inVs")]
        in_vs: Vec<Id>,
        document: Id,
        #[serde(skip_serializing_if = "Option::is_none")]
        property: Option<ItemEdgeProperty>,
    },
    #[serde(rename = "textDocument/hover")]
    TextDocumentHover {
        id: Id,
        #[serde(rename = "type")]
        type_: EdgeType,
        #[serde(rename = "outV")]
        out_v: Id,
        #[serde(rename = "inV")]
        in_v: Id,
    },
    #[serde(rename = "textDocument/definition")]
    TextDocumentDefinition {
        id: Id,
        #[serde(rename = "type")]
        type_: EdgeType,
        #[serde(rename = "outV")]
        out_v: Id,
        #[serde(rename = "inV")]
        in_v: Id,
    },
    #[serde(rename = "textDocument/declaration")]
    TextDocumentDeclaration {
        id: Id,
        #[serde(rename = "type")]
        type_: EdgeType,
        #[serde(rename = "outV")]
        out_v: Id,
        #[serde(rename = "inV")]
        in_v: Id,
    },
    #[serde(rename = "textDocument/references")]
    TextDocumentReferences {
        id: Id,
        #[serde(rename = "type")]
        type_: EdgeType,
        #[serde(rename = "outV")]
        out_v: Id,
        #[serde(rename = "inV")]
        in_v: Id,
    },
    #[serde(rename = "textDocument/implementation")]
    TextDocumentImplementation {
        id: Id,
        #[serde(rename = "type")]
        type_: EdgeType,
        #[serde(rename = "outV")]
        out_v: Id,
        #[serde(rename = "inV")]
        in_v: Id,
    },
    #[serde(rename = "textDocument/typeDefinition")]
    TextDocumentTypeDefinition {
        id: Id,
        #[serde(rename = "type")]
        type_: EdgeType,
        #[serde(rename = "outV")]
        out_v: Id,
        #[serde(rename = "inV")]
        in_v: Id,
    },
    #[serde(rename = "textDocument/callHierarchy")]
    TextDocumentCallHierarchy {
        id: Id,
        #[serde(rename = "type")]
        type_: EdgeType,
        #[serde(rename = "outV")]
        out_v: Id,
        #[serde(rename = "inV")]
        in_v: Id,
    },
    #[serde(rename = "textDocument/typeHierarchy")]
    TextDocumentTypeHierarchy {
        id: Id,
        #[serde(rename = "type")]
        type_: EdgeType,
        #[serde(rename = "outV")]
        out_v: Id,
        #[serde(rename = "inV")]
        in_v: Id,
    },
    #[serde(rename = "textDocument/foldingRange")]
    TextDocumentFoldingRange {
        id: Id,
        #[serde(rename = "type")]
        type_: EdgeType,
        #[serde(rename = "outV")]
        out_v: Id,
        #[serde(rename = "inV")]
        in_v: Id,
    },
    #[serde(rename = "textDocument/documentLink")]
    TextDocumentDocumentLink {
        id: Id,
        #[serde(rename = "type")]
        type_: EdgeType,
        #[serde(rename = "outV")]
        out_v: Id,
        #[serde(rename = "inV")]
        in_v: Id,
    },
    #[serde(rename = "textDocument/documentSymbol")]
    TextDocumentDocumentSymbol {
        id: Id,
        #[serde(rename = "type")]
        type_: EdgeType,
        #[serde(rename = "outV")]
        out_v: Id,
        #[serde(rename = "inV")]
        in_v: Id,
    },
    #[serde(rename = "textDocument/diagnostic")]
    TextDocumentDiagnostic {
        id: Id,
        #[serde(rename = "type")]
        type_: EdgeType,
        #[serde(rename = "outV")]
        out_v: Id,
        #[serde(rename = "inV")]
        in_v: Id,
    },
    #[serde(rename = "textDocument/semanticTokens/full")]
    TextDocumentSemanticTokens {
        id: Id,
        #[serde(rename = "type")]
        type_: EdgeType,
        #[serde(rename = "outV")]
        out_v: Id,
        #[serde(rename = "inV")]
        in_v: Id,
    },
}

/// Overarching element type for mixed lists.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Element {
    Vertex(Vertex),
    Edge(Edge),
}

#[cfg(test)]
mod tests {
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
}
