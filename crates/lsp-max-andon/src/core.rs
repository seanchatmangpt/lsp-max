use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ProbeResult {
    Pass,
    Fail,
    Missing,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Witness {
    pub kind: String,
    pub evidence: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RepairAction {
    pub id: String,
    pub title: String,
    pub next_lawful_step: Option<String>,
    pub command: Option<String>,
    pub code_action: Option<String>,
    pub virtual_doc_uri: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum Severity {
    Info,
    Warning,
    Stop,
    Refuse,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AndonInvariant {
    pub id: String,
    pub statement: String,
    pub scope: String,

    pub true_probe: Option<String>,
    pub false_probe: Option<String>,
    pub counterfactual_probe: Option<String>,

    pub witness_rule: Option<String>,
    pub repair_rule: Option<RepairAction>,

    pub severity: Severity,
    pub blocks: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TruthTableRow {
    pub invariant_id: String,
    pub true_case: ProbeResult,
    pub false_case: ProbeResult,
    pub counterfactual_case: ProbeResult,
    pub witness: Option<Witness>,
    pub repair: Option<RepairAction>,
    pub verdict: ProbeResult,
    pub admission_allowed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TruthTable {
    pub rows: Vec<TruthTableRow>,
}

pub struct InvariantRegistry {
    invariants: Vec<AndonInvariant>,
}

impl InvariantRegistry {
    pub fn new() -> Self {
        Self { invariants: Vec::new() }
    }

    pub fn empty() -> Self {
        Self::new()
    }

    pub fn register(&mut self, invariant: AndonInvariant) {
        self.invariants.push(invariant);
    }

    pub fn get_all(&self) -> &[AndonInvariant] {
        &self.invariants
    }
    
    pub fn is_empty(&self) -> bool {
        self.invariants.is_empty()
    }
}
