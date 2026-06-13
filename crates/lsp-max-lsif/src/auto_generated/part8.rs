use lsp_types_max::*;
use serde::{Deserialize, Serialize};

pub type Id = NumberOrString;

// -------------------------------------------------------------------------
// Vertices
// -------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum MonikerKind {
    #[serde(rename = "import")]
    Import,
    #[serde(rename = "export")]
    Export,
    #[serde(rename = "local")]
    Local,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum UniquenessLevel {
    #[serde(rename = "document")]
    Document,
    #[serde(rename = "project")]
    Project,
    #[serde(rename = "workspace")]
    Workspace,
    #[serde(rename = "scheme")]
    Scheme,
    #[serde(rename = "global")]
    Global,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MonikerVertex {
    pub id: Id,
    #[serde(rename = "type")]
    pub type_: String,
    pub label: String,
    pub scheme: String,
    pub identifier: String,
    pub unique: UniquenessLevel,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kind: Option<MonikerKind>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RepositoryInfo {
    #[serde(rename = "type")]
    pub type_: String,
    pub url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub commit_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PackageInformationVertex {
    pub id: Id,
    #[serde(rename = "type")]
    pub type_: String,
    pub label: String,
    pub name: String,
    pub manager: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub uri: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub contents: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub repository: Option<RepositoryInfo>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RangeBasedDocumentSymbol {
    pub id: Id,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub children: Option<Vec<RangeBasedDocumentSymbol>>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum DocumentSymbolResultType {
    DocumentSymbols(Vec<DocumentSymbol>),
    RangeBased(Vec<RangeBasedDocumentSymbol>),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DocumentSymbolResultVertex {
    pub id: Id,
    #[serde(rename = "type")]
    pub type_: String,
    pub label: String,
    pub result: DocumentSymbolResultType,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiagnosticResultVertex {
    pub id: Id,
    #[serde(rename = "type")]
    pub type_: String,
    pub label: String,
    pub result: Vec<Diagnostic>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FoldingRangeResultVertex {
    pub id: Id,
    #[serde(rename = "type")]
    pub type_: String,
    pub label: String,
    pub result: Vec<FoldingRange>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DocumentLinkResultVertex {
    pub id: Id,
    #[serde(rename = "type")]
    pub type_: String,
    pub label: String,
    pub result: Vec<DocumentLink>,
}

// -------------------------------------------------------------------------
// Edges
// -------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MonikerEdge {
    pub id: Id,
    #[serde(rename = "type")]
    pub type_: String,
    pub label: String,
    pub out_v: Id,
    pub in_v: Id,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AttachEdge {
    pub id: Id,
    #[serde(rename = "type")]
    pub type_: String,
    pub label: String,
    pub out_v: Id,
    pub in_v: Id,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PackageInformationEdge {
    pub id: Id,
    #[serde(rename = "type")]
    pub type_: String,
    pub label: String,
    pub out_v: Id,
    pub in_v: Id,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TextDocumentDocumentSymbolEdge {
    pub id: Id,
    #[serde(rename = "type")]
    pub type_: String,
    #[serde(rename = "label")]
    pub label: String,
    pub out_v: Id,
    pub in_v: Id,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TextDocumentFoldingRangeEdge {
    pub id: Id,
    #[serde(rename = "type")]
    pub type_: String,
    #[serde(rename = "label")]
    pub label: String,
    pub out_v: Id,
    pub in_v: Id,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TextDocumentDocumentLinkEdge {
    pub id: Id,
    #[serde(rename = "type")]
    pub type_: String,
    #[serde(rename = "label")]
    pub label: String,
    pub out_v: Id,
    pub in_v: Id,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TextDocumentDiagnosticEdge {
    pub id: Id,
    #[serde(rename = "type")]
    pub type_: String,
    #[serde(rename = "label")]
    pub label: String,
    pub out_v: Id,
    pub in_v: Id,
}
