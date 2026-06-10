//! Evidence payload types and From/TryFrom converters for wasm4pm compatibility.

use crate::control_plane::receipts::CryptographicReceipt;
use lsp_max_protocol::MaxDiagnostic;
use wasm4pm_compat::evidence::Evidence;
use wasm4pm_compat::state::{Admitted, Raw};
use wasm4pm_compat::witness::Ocel20;

/// Payload representing a Workspace node.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct WorkspaceEvidencePayload {
    pub id: String,
    pub kind: String,
    pub document_uris: Vec<String>,
}

/// Payload representing a Range node.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct RangeEvidencePayload {
    pub start_line: u32,
    pub start_character: u32,
    pub end_line: u32,
    pub end_character: u32,
}

/// Payload representing a Diagnostic node.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct DiagnosticEvidencePayload {
    pub message: String,
    pub severity: Option<String>,
    pub code: Option<String>,
    pub source: Option<String>,
    pub range: Option<RangeEvidencePayload>,
}

/// Payload representing a CryptographicReceipt.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct CryptographicReceiptEvidencePayload {
    pub prev_hash: String,
    pub discipline_id: String,
    pub law_id: String,
    pub consequence_hash: String,
    pub sequence: u64,
    pub signature: String,
}

// ==========================================
// From / TryFrom Implementations
// ==========================================

impl From<&lsp_types_max::Range> for RangeEvidencePayload {
    fn from(r: &lsp_types_max::Range) -> Self {
        Self {
            start_line: r.start.line,
            start_character: r.start.character,
            end_line: r.end.line,
            end_character: r.end.character,
        }
    }
}

impl TryFrom<&lsp_max_lsif::lsif::Vertex> for RangeEvidencePayload {
    type Error = &'static str;

    fn try_from(v: &lsp_max_lsif::lsif::Vertex) -> Result<Self, Self::Error> {
        match v {
            lsp_max_lsif::lsif::Vertex::Range { start, end, .. }
            | lsp_max_lsif::lsif::Vertex::ResultRange { start, end, .. } => Ok(Self {
                start_line: start.line,
                start_character: start.character,
                end_line: end.line,
                end_character: end.character,
            }),
            _ => Err("Vertex is not a Range or ResultRange"),
        }
    }
}

impl TryFrom<&lsp_max_lsif::lsif::Vertex> for WorkspaceEvidencePayload {
    type Error = &'static str;

    fn try_from(v: &lsp_max_lsif::lsif::Vertex) -> Result<Self, Self::Error> {
        match v {
            lsp_max_lsif::lsif::Vertex::Project { id, kind, .. } => {
                let id_str = match id {
                    lsp_types_max::NumberOrString::Number(n) => n.to_string(),
                    lsp_types_max::NumberOrString::String(s) => s.clone(),
                };
                Ok(Self {
                    id: id_str,
                    kind: kind.clone().unwrap_or_default(),
                    document_uris: Vec::new(),
                })
            }
            _ => Err("Vertex is not a Project"),
        }
    }
}

impl From<&MaxDiagnostic> for DiagnosticEvidencePayload {
    fn from(d: &MaxDiagnostic) -> Self {
        Self {
            message: d.lsp.message.clone(),
            severity: d.lsp.severity.map(|s| format!("{:?}", s)),
            code: d.lsp.code.as_ref().map(|c| match c {
                lsp_types_max::NumberOrString::Number(n) => n.to_string(),
                lsp_types_max::NumberOrString::String(s) => s.clone(),
            }),
            source: d.lsp.source.clone(),
            range: Some(RangeEvidencePayload::from(&d.lsp.range)),
        }
    }
}

impl From<&CryptographicReceipt> for CryptographicReceiptEvidencePayload {
    fn from(r: &CryptographicReceipt) -> Self {
        Self {
            prev_hash: crate::control_plane::receipts::to_hex(&r.prev_hash.0),
            discipline_id: r.discipline_id.to_string(),
            law_id: r.law_id.to_string(),
            consequence_hash: crate::control_plane::receipts::to_hex(&r.consequence_hash.0),
            sequence: r.sequence,
            signature: crate::control_plane::receipts::to_hex(&r.signature),
        }
    }
}

impl From<&lsp_max_protocol::Receipt> for CryptographicReceiptEvidencePayload {
    fn from(r: &lsp_max_protocol::Receipt) -> Self {
        Self {
            prev_hash: r.prev_receipt_hash.clone().unwrap_or_default(),
            discipline_id: String::new(),
            law_id: String::new(),
            consequence_hash: r.hash.clone(),
            sequence: 0,
            signature: String::new(),
        }
    }
}

// ==========================================
// Evidence Construction Helpers
// ==========================================

/// Converts a payload into Raw Evidence.
pub fn to_raw_evidence<T, W>(value: T) -> Evidence<T, Raw, W> {
    Evidence::raw(value)
}

/// Converts a payload into Admitted Evidence.
pub fn to_admitted_evidence<T, W>(value: T) -> Evidence<T, Admitted, W> {
    wasm4pm_compat::admission::Admission::new(value).into_evidence()
}

/// Helper to convert a Workspace payload to Admitted evidence with Ocel20 witness.
pub fn workspace_to_admitted_evidence(
    payload: WorkspaceEvidencePayload,
) -> Evidence<WorkspaceEvidencePayload, Admitted, Ocel20> {
    to_admitted_evidence(payload)
}

/// Helper to convert a Range payload to Admitted evidence with Ocel20 witness.
pub fn range_to_admitted_evidence(
    payload: RangeEvidencePayload,
) -> Evidence<RangeEvidencePayload, Admitted, Ocel20> {
    to_admitted_evidence(payload)
}

/// Helper to convert a Diagnostic payload to Admitted evidence with Ocel20 witness.
pub fn diagnostic_to_admitted_evidence(
    payload: DiagnosticEvidencePayload,
) -> Evidence<DiagnosticEvidencePayload, Admitted, Ocel20> {
    to_admitted_evidence(payload)
}

/// Helper to convert a CryptographicReceipt payload to Admitted evidence with Ocel20 witness.
pub fn receipt_to_admitted_evidence(
    payload: CryptographicReceiptEvidencePayload,
) -> Evidence<CryptographicReceiptEvidencePayload, Admitted, Ocel20> {
    to_admitted_evidence(payload)
}
