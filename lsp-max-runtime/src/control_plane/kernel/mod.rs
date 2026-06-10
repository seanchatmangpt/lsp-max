use crate::control_plane::receipts::CryptographicReceipt;

mod admitted_superseded;
mod raw_candidate;
mod refused_quarantined_replayed;

#[cfg(test)]
mod tests;

// Re-exports needed by tests via `use super::*`
#[cfg(test)]
pub(crate) use crate::control_plane::admission::{
    AdmittedData, CandidateData, GraphAdmissionLaw, RawData, ADMITTED, CANDIDATE, RAW, SUPERSEDED,
};
#[cfg(test)]
pub(crate) use crate::control_plane::receipts::Blake3Hash;
#[cfg(test)]
pub(crate) use crate::{Machine, TypestateKernel};
#[cfg(test)]
pub(crate) use ed25519_dalek::Signer;

// ==========================================
// Input payload for CANDIDATE -> ADMITTED
// ==========================================
pub struct CandidateAdmitInput {
    pub store: oxigraph::store::Store,
    pub graph_name: oxigraph::model::GraphName,
    pub receipt: CryptographicReceipt,
}

/// Helper function to convert bytes to hex string (zero-dependency).
pub(super) fn bytes_to_hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}
