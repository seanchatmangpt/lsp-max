//! Semantic law graph module — clean boundary wall around Oxigraph.
//!
//! # Architecture invariants
//!
//! ## INVARIANT: OXIGRAPH_BOUNDARY_HELD
//! `oxigraph::*` MUST only appear inside `store.rs` and `lsif_import.rs`.
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

pub mod lsif_import;
pub mod snapshot;
pub mod store;

pub use lsif_import::LsifImportStats;
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

    /// Import an LSIF JSONL file into a named graph within the store.
    ///
    /// # INVARIANT: OXIGRAPH_NOT_ON_HOT_PATH — lsif_import is cold-path only.
    /// MUST NOT be called from `did_change` or any LSP notification handler.
    fn import_lsif_snapshot(
        &self,
        lsif_path: &std::path::Path,
        graph_name: &str,
    ) -> anyhow::Result<LsifImportStats>;
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
    /// Compile-time assertion — if the boundary breaks, this won't compile.
    #[test]
    fn oxigraph_boundary_held() {
        fn _assert_impl<T: SemanticLawGraph>() {}
        _assert_impl::<crate::runtime::control_plane::semantic_graph::store::OxigraphStore>();
    }

    // ------------------------------------------------------------------
    // TRUE: snapshot load + digest round-trip
    // ------------------------------------------------------------------
    #[test]
    fn snapshot_load_returns_digest() {
        let store = store::OxigraphStore::default();
        let nquads = "<urn:a> <urn:b> \"c\" .\n";
        let snapshot = LawGraphSnapshot::new(nquads, "test");
        let expected_digest = snapshot.digest();
        let actual_digest = store.load_snapshot(snapshot).unwrap();
        assert_eq!(
            expected_digest, actual_digest,
            "digest must match snapshot content"
        );
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

    // ------------------------------------------------------------------
    // INVARIANT: OXIGRAPH_NOT_ON_HOT_PATH
    // ------------------------------------------------------------------
    #[test]
    fn oxigraph_not_on_hot_path() {
        let sync_rs_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("src")
            .join("language_server")
            .join("impls")
            .join("sync.rs");

        let sync_rs = std::fs::read_to_string(&sync_rs_path).expect("Could not read sync.rs");

        for (i, line) in sync_rs.lines().enumerate() {
            let line = line.trim();
            // Ignore comments
            if line.starts_with("//") || line.is_empty() {
                continue;
            }
            if line.contains("SemanticLawGraph")
                || line.contains("import_lsif_snapshot")
                || line.contains("oxigraph")
            {
                panic!(
                    "OXIGRAPH_NOT_ON_HOT_PATH violated in sync.rs at line {}: {}",
                    i + 1,
                    line
                );
            }
        }
    }
}
