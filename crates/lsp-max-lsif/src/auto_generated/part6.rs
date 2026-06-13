use lsp_types_max::*;
use serde::{Deserialize, Serialize};

pub type Id = NumberOrString;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TypeDefinitionResult {
    pub id: Id,
    #[serde(rename = "type")]
    pub type_: String,
    pub label: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReferenceResult {
    pub id: Id,
    #[serde(rename = "type")]
    pub type_: String,
    pub label: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImplementationResult {
    pub id: Id,
    #[serde(rename = "type")]
    pub type_: String,
    pub label: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HoverResult {
    pub id: Id,
    #[serde(rename = "type")]
    pub type_: String,
    pub label: String,
    pub result: Hover,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum EdgeLabels {
    #[serde(rename = "contains")]
    Contains,
    #[serde(rename = "item")]
    Item,
    #[serde(rename = "next")]
    Next,
    #[serde(rename = "moniker")]
    Moniker,
    #[serde(rename = "attach")]
    Attach,
    #[serde(rename = "packageInformation")]
    PackageInformation,
    #[serde(rename = "textDocument/documentSymbol")]
    TextDocumentDocumentSymbol,
    #[serde(rename = "textDocument/foldingRange")]
    TextDocumentFoldingRange,
    #[serde(rename = "textDocument/documentLink")]
    TextDocumentDocumentLink,
    #[serde(rename = "textDocument/diagnostic")]
    TextDocumentDiagnostic,
    #[serde(rename = "textDocument/definition")]
    TextDocumentDefinition,
    #[serde(rename = "textDocument/declaration")]
    TextDocumentDeclaration,
    #[serde(rename = "textDocument/typeDefinition")]
    TextDocumentTypeDefinition,
    #[serde(rename = "textDocument/hover")]
    TextDocumentHover,
    #[serde(rename = "textDocument/references")]
    TextDocumentReferences,
    #[serde(rename = "textDocument/implementation")]
    TextDocumentImplementation,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Cardinality {
    #[serde(rename = "1:1")]
    One2One,
    #[serde(rename = "1:N")]
    One2Many,
    #[serde(rename = "N:N")]
    Many2Many,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Vertex {
    MetaData(crate::auto_generated::part3::MetaData),
    Event(crate::auto_generated::part1::Event),
    Source(crate::auto_generated::part3::Source),
    Capabilities(crate::auto_generated::part4::Capabilities),
    Project(crate::auto_generated::part4::Project),
    Document(crate::auto_generated::part4::Document),
    Moniker(crate::auto_generated::part4::Moniker),
    PackageInformation(crate::auto_generated::part4::PackageInformation),
    ResultSet(crate::auto_generated::part1::ResultSet),
    Range(crate::auto_generated::part2::Range),
    DocumentSymbolResult(crate::auto_generated::part5::DocumentSymbolResult),
    FoldingRangeResult(crate::auto_generated::part5::FoldingRangeResult),
    DocumentLinkResult(crate::auto_generated::part5::DocumentLinkResult),
    DiagnosticResult(crate::auto_generated::part5::DiagnosticResult),
    DefinitionResult(crate::auto_generated::part5::DefinitionResult),
    DeclarationResult(crate::auto_generated::part5::DeclarationResult),
    TypeDefinitionResult(TypeDefinitionResult),
    HoverResult(HoverResult),
    ReferenceResult(ReferenceResult),
    ImplementationResult(ImplementationResult),
}
