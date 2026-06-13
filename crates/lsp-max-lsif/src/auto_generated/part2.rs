use lsp_types_max::*;
use serde::{Deserialize, Serialize};

pub type Id = NumberOrString;
pub type RangeId = Id;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum DeclarationType {
    #[serde(rename = "declaration")]
    Declaration,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum DefinitionType {
    #[serde(rename = "definition")]
    Definition,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ReferenceType {
    #[serde(rename = "reference")]
    Reference,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum UnknownType {
    #[serde(rename = "unknown")]
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeclarationTag {
    #[serde(rename = "type")]
    pub type_: DeclarationType,
    pub text: String,
    pub kind: SymbolKind,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<SymbolTag>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deprecated: Option<bool>,
    pub full_range: lsp_types_max::Range,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DefinitionTag {
    #[serde(rename = "type")]
    pub type_: DefinitionType,
    pub text: String,
    pub kind: SymbolKind,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<SymbolTag>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deprecated: Option<bool>,
    pub full_range: lsp_types_max::Range,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReferenceTag {
    #[serde(rename = "type")]
    pub type_: ReferenceType,
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UnknownTag {
    #[serde(rename = "type")]
    pub type_: UnknownType,
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum RangeTag {
    Declaration(DeclarationTag),
    Definition(DefinitionTag),
    Reference(ReferenceTag),
    Unknown(UnknownTag),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Range {
    pub id: Id,
    #[serde(rename = "type")]
    pub type_: String,
    pub label: String,
    pub start: Position,
    pub end: Position,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tag: Option<RangeTag>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DefinitionRange {
    pub id: Id,
    #[serde(rename = "type")]
    pub type_: String,
    pub label: String,
    pub start: Position,
    pub end: Position,
    pub tag: DefinitionTag,
}
