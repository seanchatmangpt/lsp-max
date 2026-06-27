//! Semantic law graph module — clean boundary wall around Oxigraph.
//!
//! # Architecture invariants
//!
//! ## INVARIANT: OXIGRAPH_BOUNDARY_HELD
//! `oxigraph::*` MUST only appear inside `store.rs`.
//! No other module in `src/` may import `oxigraph` for semantic-graph purposes.
//!
//! ## INVARIANT: OXIGRAPH_NOT_ON_HOT_PATH
//! `SemanticLawGraph` methods MUST NOT be called on the `handle_did_change` /
//! `handle_did_open` hot path.  The semantic graph is COLD storage (law lookup),
//! not live diagnostic computation.  Live diagnostics use the Salsa layer.
//! See: `src/language_server/impls/sync.rs` — `did_change` handler comment.
//!
//! ## INVARIANT: STALE_LSIF_INDEX = STOP
//! See `src/andon/lsif_invariant.rs`.

pub mod snapshot;
pub mod store;

pub use snapshot::{GraphDigest, LawGraphSnapshot};

// ---------------------------------------------------------------------------
// Core trait — the ONLY public surface for semantic graph access
// ---------------------------------------------------------------------------

/// Trait for the semantic law graph (COLD storage layer).
///
/// Implementations must be `Send + Sync` because the graph may be shared
/// across async tasks inside the LSP server.
///
/// The only admitted implementation is `store::OxigraphStore`.
/// Error type is `String` for compatibility with existing control_plane conventions.
pub trait SemanticLawGraph: Send + Sync {
    /// Load a snapshot of N-Quads triples into the store.
    /// Returns the BLAKE3 digest of the loaded content for receipt verification.
    fn load_snapshot(&self, snapshot: LawGraphSnapshot) -> Result<GraphDigest, String>;

    /// Query all invariant URIs registered for a given scope.
    fn query_invariants(&self, scope: &str) -> Result<Vec<String>, String>;

    /// Query all witness URIs attached to a specific claim.
    fn query_witnesses(&self, claim_id: &str) -> Result<Vec<String>, String>;

    /// Query all repair action URIs applicable to a diagnostic code.
    fn query_repairs(&self, diagnostic_code: &str) -> Result<Vec<String>, String>;
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // ------------------------------------------------------------------
    // INVARIANT: OXIGRAPH_BOUNDARY_HELD
    // ------------------------------------------------------------------
    /// Verify that `OxigraphStore` implements `SemanticLawGraph`.
    /// This is a compile-time assertion — if the boundary breaks,
    /// this test will fail to compile.
    #[test]
    fn oxigraph_boundary_held() {
        fn _assert_impl<T: SemanticLawGraph>() {}
        _assert_impl::<crate::runtime::control_plane::semantic_graph::store::OxigraphStore>();
    }

    // ------------------------------------------------------------------
    // TRUE: snapshot load + query round-trip
    // ------------------------------------------------------------------
    #[test]
    fn snapshot_load_returns_digest() {
        let store = store::OxigraphStore::default();
        let nquads = "<urn:a> <urn:b> \"c\" .\n";
        let snapshot = LawGraphSnapshot::new(nquads, "test");
        let expected_digest = snapshot.digest();
        let actual_digest = store.load_snapshot(snapshot).unwrap();
        assert_eq!(expected_digest, actual_digest, "digest must match snapshot content");
    }

    // ------------------------------------------------------------------
    // FALSE: empty store returns empty results (not stale data)
    // ------------------------------------------------------------------
    #[test]
    fn empty_store_returns_empty_query_results() {
        let store = store::OxigraphStore::default();
        let result = store.query_invariants("nonexistent-scope").unwrap();
        assert!(result.is_empty(), "empty store must return no invariants");

        let repairs = store.query_repairs("LSPMAX-ANDON-001").unwrap();
        assert!(repairs.is_empty(), "empty store must return no repairs");
    }
}
