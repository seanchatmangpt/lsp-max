use crate::auto_generated::part3::RepositoryInfo;
use lsp_types_max::*;
use serde::{Deserialize, Serialize};

pub type Id = NumberOrString;
pub type DocumentId = Id;

/// The LSP capabilities a dump supports
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Capabilities {
    pub id: Id,
    #[serde(rename = "type")]
    pub type_: String,
    pub label: String,
    pub hover_provider: bool,
    pub declaration_provider: bool,
    pub definition_provider: bool,
    pub type_definition_provider: bool,
    pub references_provider: bool,
    pub document_symbol_provider: bool,
    pub folding_range_provider: bool,
    pub diagnostic_provider: bool,
}

/// A project vertex.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Project {
    pub id: Id,
    #[serde(rename = "type")]
    pub type_: String,
    pub label: String,
    pub kind: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resource: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub contents: Option<String>,
}

/// A vertex representing a document in the project
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Document {
    pub id: Id,
    #[serde(rename = "type")]
    pub type_: String,
    pub label: String,
    pub uri: String,
    pub language_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub contents: Option<String>,
}

/// The moniker kind.
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
pub struct Moniker {
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
pub struct PackageInformation {
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
