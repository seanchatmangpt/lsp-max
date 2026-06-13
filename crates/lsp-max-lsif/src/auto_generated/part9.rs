use lsp_types_max::*;
use serde::{Deserialize, Serialize};

pub type Id = NumberOrString;

// -------------------------------------------------------------------------
// Edges
// -------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Edge11 {
    pub id: Id,
    #[serde(rename = "type")]
    pub type_: String, // "edge"
    pub label: String,
    pub out_v: Id,
    pub in_v: Id,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Edge1N {
    pub id: Id,
    #[serde(rename = "type")]
    pub type_: String, // "edge"
    pub label: String,
    pub out_v: Id,
    pub in_vs: Vec<Id>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ItemEdge {
    pub id: Id,
    #[serde(rename = "type")]
    pub type_: String, // "edge"
    pub label: String, // "item"
    pub out_v: Id,
    pub in_vs: Vec<Id>,
    pub shard: Id,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub property: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Edge {
    E11(Edge11),
    E1N(Edge1N),
    Item(ItemEdge),
}

// -------------------------------------------------------------------------
// Specific Edge Types (Aliases)
// -------------------------------------------------------------------------

pub type TextDocumentDeclarationEdge = Edge;
pub type TextDocumentDefinitionEdge = Edge;
pub type TextDocumentTypeDefinitionEdge = Edge;
pub type TextDocumentHoverEdge = Edge;
pub type TextDocumentReferencesEdge = Edge;
pub type TextDocumentImplementationEdge = Edge;
