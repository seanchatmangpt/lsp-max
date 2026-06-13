use lsp_types_max::*;
use serde::{Deserialize, Serialize};

pub type Id = NumberOrString;
pub type Uri = String;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeclarationTag {
    #[serde(rename = "type")]
    pub type_: String,
    pub text: String,
    pub kind: SymbolKind,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<SymbolTag>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deprecated: Option<bool>,
    pub full_range: Range,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeclarationRange {
    pub id: Id,
    #[serde(rename = "type")]
    pub type_: String,
    pub label: String,
    #[serde(flatten)]
    pub range: Range,
    pub tag: DeclarationTag,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReferenceTag {
    #[serde(rename = "type")]
    pub type_: String,
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReferenceRange {
    pub id: Id,
    #[serde(rename = "type")]
    pub type_: String,
    pub label: String,
    #[serde(flatten)]
    pub range: Range,
    pub tag: ReferenceTag,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Location {
    pub id: Id,
    #[serde(rename = "type")]
    pub type_: String,
    pub label: String,
    pub range: Range,
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
#[serde(rename_all = "camelCase")]
pub struct ToolState {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MetaData {
    pub id: Id,
    #[serde(rename = "type")]
    pub type_: String,
    pub label: String,
    pub version: String,
    pub position_encoding: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_info: Option<ToolInfo>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_state: Option<ToolState>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RepositoryInfo {
    #[serde(rename = "type")]
    pub type_: String,
    pub url: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Source {
    pub id: Id,
    #[serde(rename = "type")]
    pub type_: String,
    pub label: String,
    pub workspace_root: Uri,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub repository: Option<RepositoryInfo>,
}
