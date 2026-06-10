use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PowlRange {
    pub start: PowlPosition,
    pub end: PowlPosition,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PowlPosition {
    pub line: u32,
    pub character: u32,
}

/// Rust equivalent of powlv2lsp/src/types.ts PowlNode union
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum PowlNode {
    Activity(Activity),
    PartialOrder(PartialOrder),
    ChoiceGraph(ChoiceGraph),
    Loop(Loop),
}

impl PowlNode {
    pub fn id(&self) -> &str {
        match self {
            PowlNode::Activity(a) => &a.id,
            PowlNode::PartialOrder(p) => &p.id,
            PowlNode::ChoiceGraph(c) => &c.id,
            PowlNode::Loop(l) => &l.id,
        }
    }

    #[allow(dead_code)]
    pub fn range(&self) -> Option<&PowlRange> {
        match self {
            PowlNode::Activity(a) => a.range.as_ref(),
            PowlNode::PartialOrder(p) => p.range.as_ref(),
            PowlNode::ChoiceGraph(c) => c.range.as_ref(),
            PowlNode::Loop(l) => l.range.as_ref(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Activity {
    pub id: String,
    pub label: Option<String>,
    #[serde(default)]
    pub is_silent: bool,
    pub related: Option<String>,
    pub divergent: Option<bool>,
    pub convergent: Option<bool>,
    pub deficient: Option<bool>,
    pub range: Option<PowlRange>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PartialOrder {
    pub id: String,
    #[serde(default)]
    pub nodes: Vec<PowlNode>,
    #[serde(default)]
    pub edges: Vec<[String; 2]>,
    pub range: Option<PowlRange>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChoiceGraph {
    pub id: String,
    #[serde(default)]
    pub nodes: Vec<PowlNode>,
    #[serde(default)]
    pub edges: Vec<[String; 2]>,
    pub start_node: String,
    pub end_node: String,
    pub range: Option<PowlRange>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Loop {
    pub id: String,
    pub do_part: Box<PowlNode>,
    pub redo_part: Box<PowlNode>,
    pub range: Option<PowlRange>,
}
