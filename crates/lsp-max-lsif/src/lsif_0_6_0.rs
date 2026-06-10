//! LSIF 0.6.0 extensions

use lsp_types_max::{NumberOrString, ItemKind};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EdgeDataMultiIn {
    pub label: String,
    pub id: NumberOrString,
    #[serde(rename = "outV")]
    pub out_v: NumberOrString,
    #[serde(rename = "inVs")]
    pub in_vs: Vec<NumberOrString>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EdgeData {
    pub label: String,
    pub id: NumberOrString,
    #[serde(rename = "outV")]
    pub out_v: NumberOrString,
    #[serde(rename = "inV")]
    pub in_v: NumberOrString,
}