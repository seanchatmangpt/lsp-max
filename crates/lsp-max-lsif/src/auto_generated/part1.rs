use lsp_types_max::*;
use serde::{Deserialize, Serialize};

pub type Id = NumberOrString;

/// The event kinds
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum EventKind {
    #[serde(rename = "begin")]
    Begin,
    #[serde(rename = "end")]
    End,
}

/// The event scopes
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum EventScope {
    Project,
    Document,
    MonikerAttach,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Event {
    pub id: Id,
    #[serde(rename = "type")]
    pub type_: String,
    pub label: String,
    pub scope: EventScope,
    pub kind: EventKind,
    pub data: Id,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectEvent {
    pub id: Id,
    #[serde(rename = "type")]
    pub type_: String,
    pub label: String,
    pub scope: EventScope,
    pub kind: EventKind,
    pub data: Id,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DocumentEvent {
    pub id: Id,
    #[serde(rename = "type")]
    pub type_: String,
    pub label: String,
    pub scope: EventScope,
    pub kind: EventKind,
    pub data: Id,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MonikerAttachEvent {
    pub id: Id,
    #[serde(rename = "type")]
    pub type_: String,
    pub label: String,
    pub scope: EventScope,
    pub kind: EventKind,
    pub data: Id,
}

/// A result set acts as a hub to share n LSP request results
/// between different ranges.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResultSet {
    pub id: Id,
    #[serde(rename = "type")]
    pub type_: String,
    pub label: String,
}

/// All know range tag literal types.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum RangeTagTypes {
    Declaration,
    Definition,
    Reference,
    Unknown,
}
