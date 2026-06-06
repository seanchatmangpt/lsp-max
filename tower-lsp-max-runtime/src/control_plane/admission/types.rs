use super::super::invariants::VerificationReport;
use super::super::receipts::{Blake3Hash, CryptographicReceipt};
use crate::{Data, Law, Machine, Phase};
use tower_lsp_max_lsif::lsif::Element;

#[derive(Debug, Clone)]
pub enum GraphAdmissionError {
    ParsingFailed(String),
    NamespaceViolation(String),
    InvariantViolated(VerificationReport),
    DependencyMissing(Vec<String>),
    ReplayMismatch(String),
}

impl std::fmt::Display for GraphAdmissionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GraphAdmissionError::ParsingFailed(s) => write!(f, "Parsing failed: {}", s),
            GraphAdmissionError::NamespaceViolation(s) => write!(f, "Namespace violation: {}", s),
            GraphAdmissionError::InvariantViolated(_) => write!(f, "Invariant violated"),
            GraphAdmissionError::DependencyMissing(deps) => {
                write!(f, "Missing dependencies: {:?}", deps)
            }
            GraphAdmissionError::ReplayMismatch(s) => write!(f, "Replay mismatch: {}", s),
        }
    }
}

impl std::error::Error for GraphAdmissionError {}

pub struct GraphAdmissionLaw;
impl Law for GraphAdmissionLaw {
    type Error = GraphAdmissionError;
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct RAW;
impl Phase for RAW {}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct CANDIDATE;
impl Phase for CANDIDATE {}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ADMITTED;
impl Phase for ADMITTED {}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct REFUSED;
impl Phase for REFUSED {}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct QUARANTINED;
impl Phase for QUARANTINED {}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SUPERSEDED;
impl Phase for SUPERSEDED {}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct REPLAYED;
impl Phase for REPLAYED {}

pub struct RawData {
    pub elements: Vec<Element>,
}
impl Data for RawData {}

pub struct CandidateData {
    pub elements: Vec<Element>,
    pub quads: Vec<oxigraph::model::Quad>,
    pub graph_hash: String,
}
impl Data for CandidateData {}

pub struct AdmittedData {
    pub graph_name: oxigraph::model::GraphName,
    pub quad_count: usize,
    pub receipt: CryptographicReceipt,
}
impl Data for AdmittedData {}

pub struct RefusedData {
    pub report: VerificationReport,
}
impl Data for RefusedData {}

pub struct QuarantinedData {
    pub elements: Vec<Element>,
    pub quads: Vec<oxigraph::model::Quad>,
    pub missing_dependencies: Vec<String>,
}
impl Data for QuarantinedData {}

pub struct SupersededData {
    pub graph_name: oxigraph::model::GraphName,
    pub superseded_by: oxigraph::model::GraphName,
}
impl Data for SupersededData {}

pub struct ReplayedData {
    pub graph_name: oxigraph::model::GraphName,
    pub receipt: CryptographicReceipt,
}
impl Data for ReplayedData {}

impl Machine<GraphAdmissionLaw, RAW, RawData> {
    pub fn admit_candidate(
        self,
        snapshot_id: &str,
    ) -> Result<Machine<GraphAdmissionLaw, CANDIDATE, CandidateData>, GraphAdmissionError> {
        let graph_name = oxigraph::model::GraphName::NamedNode(
            oxigraph::model::NamedNode::new(format!("urn:project:local:snapshot:{}", snapshot_id))
                .map_err(|e| GraphAdmissionError::ParsingFailed(e.to_string()))?,
        );

        let mut quads = Vec::new();
        let mut byte_accumulator = Vec::new();

        for element in &self.data.elements {
            if let Ok(serialized) = serde_json::to_vec(element) {
                byte_accumulator.extend_from_slice(&serialized);
            }
            super::mapping::map_element_to_quads(element, &graph_name, &mut quads)?;
        }

        let graph_hash = crate::sha256(&byte_accumulator);

        Ok(Machine::new(
            CANDIDATE,
            CandidateData {
                elements: self.data.elements,
                quads,
                graph_hash,
            },
        ))
    }
}

impl Machine<GraphAdmissionLaw, CANDIDATE, CandidateData> {
    pub fn admit_admitted(
        self,
        _store: &oxigraph::store::Store,
        graph_name: oxigraph::model::GraphName,
        receipt: CryptographicReceipt,
    ) -> Result<Machine<GraphAdmissionLaw, ADMITTED, AdmittedData>, GraphAdmissionError> {
        Ok(Machine::new(
            ADMITTED,
            AdmittedData {
                graph_name,
                quad_count: self.data.quads.len(),
                receipt,
            },
        ))
    }

    pub fn admit_refuse(
        self,
        report: VerificationReport,
    ) -> Machine<GraphAdmissionLaw, REFUSED, RefusedData> {
        Machine::new(REFUSED, RefusedData { report })
    }

    pub fn admit_quarantine(
        self,
        missing_dependencies: Vec<String>,
    ) -> Machine<GraphAdmissionLaw, QUARANTINED, QuarantinedData> {
        Machine::new(
            QUARANTINED,
            QuarantinedData {
                elements: self.data.elements,
                quads: self.data.quads,
                missing_dependencies,
            },
        )
    }

    pub fn admit_replay(
        self,
        graph_name: oxigraph::model::GraphName,
        receipt: CryptographicReceipt,
    ) -> Machine<GraphAdmissionLaw, REPLAYED, ReplayedData> {
        Machine::new(
            REPLAYED,
            ReplayedData {
                graph_name,
                receipt,
            },
        )
    }
}

impl Machine<GraphAdmissionLaw, ADMITTED, AdmittedData> {
    pub fn admit_supersede(
        self,
        superseded_by: oxigraph::model::GraphName,
    ) -> Machine<GraphAdmissionLaw, SUPERSEDED, SupersededData> {
        Machine::new(
            SUPERSEDED,
            SupersededData {
                graph_name: self.data.graph_name,
                superseded_by,
            },
        )
    }
}

impl Machine<GraphAdmissionLaw, QUARANTINED, QuarantinedData> {
    pub fn into_candidate(
        self,
        graph_hash: String,
    ) -> Machine<GraphAdmissionLaw, CANDIDATE, CandidateData> {
        Machine::new(
            CANDIDATE,
            CandidateData {
                elements: self.data.elements,
                quads: self.data.quads,
                graph_hash,
            },
        )
    }
}
