use lsp_types_max::*;
use serde::{Deserialize, Serialize};

/// Defines an unsigned integer in the range of 0 to 2^31 - 1.
pub type Uinteger = u32;

/// An `Id` to identify a vertex or an edge.
pub type Id = NumberOrString;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ElementTypes {
    Vertex,
    Edge,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GraphElement {
    pub id: Id,
    pub r#type: ElementTypes,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Element {
    Vertex(crate::auto_generated::part6::Vertex),
    Edge(crate::auto_generated::part9::Edge),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum VertexLabels {
    MetaData,
    #[serde(rename = "$event")]
    Event,
    Source,
    Capabilities,
    Project,
    Range,
    Location,
    Document,
    Moniker,
    PackageInformation,
    ResultSet,
    DocumentSymbolResult,
    FoldingRangeResult,
    DocumentLinkResult,
    DiagnosticResult,
    DeclarationResult,
    DefinitionResult,
    TypeDefinitionResult,
    HoverResult,
    ReferenceResult,
    ImplementationResult,
}

pub type Uri = String;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct V {
    pub id: Id,
    pub r#type: ElementTypes,
    pub label: VertexLabels,
}
