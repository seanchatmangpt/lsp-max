use lsp_types_max::{NumberOrString, Range};
use serde::{Deserialize, Serialize};

/// The identifier of an element.
pub type Id = NumberOrString;
/// A document or project URI.
pub type Uri = String;

/// Always "vertex"
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub enum VertexType {
    #[serde(rename = "vertex")]
    #[default]
    Vertex,
}

/// Always "edge"
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub enum EdgeType {
    #[serde(rename = "edge")]
    #[default]
    Edge,
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
    Group,
    Scheme,
    Global,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum HoverContents {
    Markup(lsp_types_max::MarkupContent),
    String(String),
    MarkedString(lsp_types_max::MarkedString),
    MarkedStringArray(Vec<lsp_types_max::MarkedString>),
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
    DocumentSymbols(Vec<lsp_types_max::DocumentSymbol>),
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
        kind: lsp_types_max::SymbolKind,
        #[serde(rename = "fullRange")]
        full_range: Range,
        #[serde(skip_serializing_if = "Option::is_none")]
        detail: Option<String>,
    },
    #[serde(rename = "definition")]
    Definition {
        text: String,
        kind: lsp_types_max::SymbolKind,
        #[serde(rename = "fullRange")]
        full_range: Range,
        #[serde(skip_serializing_if = "Option::is_none")]
        detail: Option<String>,
    },
    #[serde(rename = "reference")]
    Reference { text: String },
    #[serde(rename = "unknown")]
    Unknown { text: String },
}
