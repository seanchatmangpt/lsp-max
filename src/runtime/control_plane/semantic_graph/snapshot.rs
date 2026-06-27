//! `LawGraphSnapshot` — a portable, serialisable bundle of RDF triples that can be
//! loaded into any `SemanticLawGraph` implementation.
//!
//! The snapshot format is intentionally simple (N-Quads text) so it can be
//! generated offline, stored in a receipt, and replayed deterministically.

/// A BLAKE3 hex digest binding a snapshot to its content.
pub type GraphDigest = String;

/// A portable snapshot of the semantic law graph, encoded as N-Quads text.
///
/// Snapshots are the unit of admission for the semantic layer:
///
/// ```text
/// snapshot loaded → GraphDigest returned
///                 → digest matches receipt → ADMITTED
///                 → digest mismatch        → STOP
/// ```
#[derive(Debug, Clone)]
pub struct LawGraphSnapshot {
    /// N-Quads formatted content (UTF-8).
    pub nquads: String,
    /// Originating source label (e.g. `"src/"` or a receipt path).
    pub source_label: String,
}

impl LawGraphSnapshot {
    pub fn new(nquads: impl Into<String>, source_label: impl Into<String>) -> Self {
        Self {
            nquads: nquads.into(),
            source_label: source_label.into(),
        }
    }

    /// Compute the BLAKE3 digest of the N-Quads content.
    pub fn digest(&self) -> GraphDigest {
        blake3::hash(self.nquads.as_bytes()).to_hex().to_string()
    }
}
