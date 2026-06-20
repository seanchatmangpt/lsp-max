//! Evidence payload types and From/TryFrom converters for wasm4pm compatibility.
//!
//! The Evidence/Admitted/Raw/Ocel20 types from wasm4pm_compat are unavailable
//! in this stub build. Functions that depend on them panic at runtime with a
//! clear BLOCKED message; the public payload types and From/TryFrom impls remain
//! fully usable.

use crate::control_plane::receipts::CryptographicReceipt;
use lsp_max_protocol::MaxDiagnostic;

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
        // lsp_max_protocol::lsp_3_18::Diagnostic.message is MarkupContentOrString;
        // extract the string content from either variant.
        use lsp_max_protocol::lsp_3_18::MarkupContentOrString;
        let message = match &d.lsp.message {
            MarkupContentOrString::String(s) => s.clone(),
            MarkupContentOrString::MarkupContent(mc) => mc.value.clone(),
        };
        // lsp_max_protocol::lsp_3_18::Diagnostic.code is Option<IntegerOrString>;
        // extract as string from either variant.
        use lsp_max_protocol::lsp_3_18::IntegerOrString;
        let code = d.lsp.code.as_ref().map(|c| match c {
            IntegerOrString::Integer(n) => n.to_string(),
            IntegerOrString::String(s) => s.clone(),
        });
        // lsp_max_protocol::lsp_3_18::Range.start/end each have .line/.character u32 fields.
        let range = Some(RangeEvidencePayload {
            start_line: d.lsp.range.start.line,
            start_character: d.lsp.range.start.character,
            end_line: d.lsp.range.end.line,
            end_character: d.lsp.range.end.character,
        });
        Self {
            message,
            severity: d.lsp.severity.map(|s| format!("{:?}", s)),
            code,
            source: d.lsp.source.clone(),
            range,
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
//
// These functions bridge into wasm4pm_compat types that are absent from the
// current stub build.  They panic at call-time with a BLOCKED status so that
// callers can see the gap clearly rather than hitting a cryptic linker error.

/// Converts a payload into Raw Evidence.
///
/// BLOCKED: wasm4pm_compat::evidence not available in stub mode.
pub fn to_raw_evidence<T, W>(_value: T) -> ! {
    panic!("BLOCKED: wasm4pm_compat::evidence unavailable in stub build — to_raw_evidence not reachable");
}

/// Converts a payload into Admitted Evidence.
///
/// BLOCKED: wasm4pm_compat::admission not available in stub mode.
pub fn to_admitted_evidence<T, W>(_value: T) -> ! {
    panic!("BLOCKED: wasm4pm_compat::admission unavailable in stub build — to_admitted_evidence not reachable");
}

/// Helper to convert a Workspace payload to Admitted evidence with Ocel20 witness.
///
/// BLOCKED: wasm4pm_compat not available in stub mode.
pub fn workspace_to_admitted_evidence(_payload: WorkspaceEvidencePayload) -> ! {
    panic!("BLOCKED: wasm4pm_compat unavailable in stub build — workspace_to_admitted_evidence not reachable");
}

/// Helper to convert a Range payload to Admitted evidence with Ocel20 witness.
///
/// BLOCKED: wasm4pm_compat not available in stub mode.
pub fn range_to_admitted_evidence(_payload: RangeEvidencePayload) -> ! {
    panic!("BLOCKED: wasm4pm_compat unavailable in stub build — range_to_admitted_evidence not reachable");
}

/// Helper to convert a Diagnostic payload to Admitted evidence with Ocel20 witness.
///
/// BLOCKED: wasm4pm_compat not available in stub mode.
pub fn diagnostic_to_admitted_evidence(_payload: DiagnosticEvidencePayload) -> ! {
    panic!("BLOCKED: wasm4pm_compat unavailable in stub build — diagnostic_to_admitted_evidence not reachable");
}

/// Helper to convert a CryptographicReceipt payload to Admitted evidence with Ocel20 witness.
///
/// BLOCKED: wasm4pm_compat not available in stub mode.
pub fn receipt_to_admitted_evidence(_payload: CryptographicReceiptEvidencePayload) -> ! {
    panic!("BLOCKED: wasm4pm_compat unavailable in stub build — receipt_to_admitted_evidence not reachable");
}
