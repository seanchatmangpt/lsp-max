use lsp_types_max::*;
use serde::{Deserialize, Serialize}; // Using our own alias if needed, or lsp_types_max::NumberOrString

pub type TypeDefinitionResult = lsp_types_max::GotoDefinitionResponse;
pub type ReferenceResult = Vec<lsp_types_max::Location>;
pub type ImplementationResult = lsp_types_max::GotoDefinitionResponse;
pub type HoverResult = lsp_types_max::Hover;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RangeBasedDocumentSymbol {
    pub id: lsp_types_max::NumberOrString,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub children: Option<Vec<RangeBasedDocumentSymbol>>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum DocumentSymbolResultData {
    DocumentSymbols(Vec<DocumentSymbol>),
    RangeBased(Vec<RangeBasedDocumentSymbol>),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DocumentSymbolResult {
    pub id: lsp_types_max::NumberOrString,
    #[serde(rename = "type")]
    pub type_: String,
    pub label: String,
    pub result: DocumentSymbolResultData,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiagnosticResult {
    pub id: lsp_types_max::NumberOrString,
    #[serde(rename = "type")]
    pub type_: String,
    pub label: String,
    pub result: Vec<Diagnostic>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FoldingRangeResult {
    pub id: lsp_types_max::NumberOrString,
    #[serde(rename = "type")]
    pub type_: String,
    pub label: String,
    pub result: Vec<FoldingRange>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DocumentLinkResult {
    pub id: lsp_types_max::NumberOrString,
    #[serde(rename = "type")]
    pub type_: String,
    pub label: String,
    pub result: Vec<DocumentLink>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeclarationResult {
    pub id: lsp_types_max::NumberOrString,
    #[serde(rename = "type")]
    pub type_: String,
    pub label: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DefinitionResult {
    pub id: lsp_types_max::NumberOrString,
    #[serde(rename = "type")]
    pub type_: String,
    pub label: String,
}
