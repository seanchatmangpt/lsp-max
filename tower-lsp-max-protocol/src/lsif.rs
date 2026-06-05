use lsp_types::{NumberOrString, Position, Range};
use serde::{Deserialize, Serialize};

/// The identifier of an element.
pub type Id = NumberOrString;
/// A document or project URI.
pub type Uri = String;

/// Always "vertex"
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum VertexType {
    #[serde(rename = "vertex")]
    Vertex,
}

impl Default for VertexType {
    fn default() -> Self {
        VertexType::Vertex
    }
}

/// Always "edge"
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum EdgeType {
    #[serde(rename = "edge")]
    Edge,
}

impl Default for EdgeType {
    fn default() -> Self {
        EdgeType::Edge
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolInfo {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub args: Option<Vec<String>>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PositionEncoding {
    #[serde(rename = "utf-8")]
    Utf8,
    #[serde(rename = "utf-16")]
    Utf16,
    #[serde(rename = "utf-32")]
    Utf32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Repository {
    #[serde(rename = "type")]
    pub type_: String,
    pub url: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum MonikerKind {
    Import,
    Export,
    Local,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum UniquenessLevel {
    Document,
    Project,
    Workspace,
    Scheme,
    Global,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum HoverContents {
    Markup(lsp_types::MarkupContent),
    String(String),
    MarkedString(lsp_types::MarkedString),
    MarkedStringArray(Vec<lsp_types::MarkedString>),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HoverResultData {
    pub contents: HoverContents,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub range: Option<Range>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum DocumentSymbolResultData {
    DocumentSymbols(Vec<lsp_types::DocumentSymbol>),
    RangeBased(Vec<RangeBasedDocumentSymbol>),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RangeBasedDocumentSymbol {
    pub id: Id,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub children: Option<Vec<RangeBasedDocumentSymbol>>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SemanticTokensData {
    pub data: Vec<u32>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum EventKind {
    Begin,
    End,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum EventScope {
    Project,
    Document,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum RangeTag {
    #[serde(rename = "declaration")]
    Declaration {
        text: String,
        kind: lsp_types::SymbolKind,
        #[serde(rename = "fullRange")]
        full_range: Range,
        #[serde(skip_serializing_if = "Option::is_none")]
        detail: Option<String>,
    },
    #[serde(rename = "definition")]
    Definition {
        text: String,
        kind: lsp_types::SymbolKind,
        #[serde(rename = "fullRange")]
        full_range: Range,
        #[serde(skip_serializing_if = "Option::is_none")]
        detail: Option<String>,
    },
    #[serde(rename = "reference")]
    Reference {
        text: String,
    },
    #[serde(rename = "unknown")]
    Unknown {
        text: String,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "label")]
pub enum Vertex {
    #[serde(rename = "metaData")]
    MetaData {
        id: Id,
        #[serde(rename = "type")]
        type_: VertexType,
        version: String,
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
        kind: String,
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
    #[serde(rename = "foldingRangeResult")]
    FoldingRangeResult {
        id: Id,
        #[serde(rename = "type")]
        type_: VertexType,
        result: Vec<lsp_types::FoldingRange>,
    },
    #[serde(rename = "documentLinkResult")]
    DocumentLinkResult {
        id: Id,
        #[serde(rename = "type")]
        type_: VertexType,
        result: Vec<lsp_types::DocumentLink>,
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
        result: Vec<lsp_types::Diagnostic>,
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
    #[serde(rename = "textDocument/semanticTokens")]
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
    use lsp_types::NumberOrString;

    #[test]
    fn test_serialize_metadata() {
        let meta = Element::Vertex(Vertex::MetaData {
            id: NumberOrString::Number(1),
            type_: VertexType::Vertex,
            version: "0.6.0".to_string(),
            position_encoding: PositionEncoding::Utf16,
            tool_info: Some(ToolInfo {
                name: "tower-lsp-max".to_string(),
                version: Some("1.0.0".to_string()),
                args: None,
            }),
        });

        let json = serde_json::to_string(&meta).unwrap();
        assert!(json.contains(r#""label":"metaData""#));
        assert!(json.contains(r#""type":"vertex""#));
        assert!(json.contains(r#""version":"0.6.0""#));
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
