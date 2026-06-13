use lsp_types_max::*;
use serde::{Deserialize, Serialize};

pub type Id = NumberOrString;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct E11<S, T, K> {
    pub id: Id,
    pub r#type: String,
    pub label: K,
    pub out_v: S,
    pub in_v: T,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct E1N<S, T, K> {
    pub id: Id,
    pub r#type: String,
    pub label: K,
    pub out_v: S,
    pub in_vs: Vec<T>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum E<S, T, K> {
    E11(E11<S, T, K>),
    E1N(E1N<S, T, K>),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ItemEdgeProperties {
    Declarations,
    Definitions,
    References,
    ReferenceResults,
    ReferenceLinks,
    ImplementationResults,
    ImplementationLinks,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ItemEdge<S, T> {
    #[serde(flatten)]
    pub e1n: E1N<S, T, String>,
    pub shard: Id,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub property: Option<ItemEdgeProperties>,
}

pub type Contains = E1N<Id, Id, String>;

pub type Next = E11<Id, Id, String>;

pub type Item = ItemEdge<Id, Id>;
