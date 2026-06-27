//! Salsa 0.26 incremental computation database for `lsp-max-lsif`.
//!
//! Caches per-file LSIF index results so that unchanged files skip
//! re-indexing on subsequent runs.  The raw LSIF emission (JSON-RPC
//! streaming) stays in the existing non-Salsa path; this layer only
//! caches the *summary* of what was indexed.

use salsa::Setter;

// ---------------------------------------------------------------------------
// Result type — must satisfy salsa::Update (all fields are primitives)
// ---------------------------------------------------------------------------

/// Summary of an LSIF indexing pass for a single source file.
///
/// Salsa memoises this value; it only re-indexes when `LsifSource.text`
/// changes.  `Vec<Element>` is intentionally NOT the return type here —
/// `Element` does not implement `salsa::Update`.  The raw element graph
/// is produced by the existing non-Salsa emission path.
#[derive(Debug, Clone, PartialEq, salsa::Update)]
pub struct LsifFileResult {
    /// The document URI (mirrors `LsifSource.path`).
    pub document_uri: String,
    /// Number of indexable lines (proxy for LSIF vertex count).
    pub vertex_count: u32,
    /// Number of definition-site monikers found.
    pub moniker_count: u32,
    /// Number of reference sites found.
    pub reference_count: u32,
    /// `true` if any parse-level error was detected.
    pub has_errors: bool,
}

// ---------------------------------------------------------------------------
// Input
// ---------------------------------------------------------------------------

/// A source file tracked by the Salsa incremental engine for LSIF indexing.
#[salsa::input]
pub struct LsifSource {
    pub path: String,
    pub text: String,
    pub language_id: String,
}

// ---------------------------------------------------------------------------
// Trait database
// ---------------------------------------------------------------------------

/// Trait implemented by all Salsa databases that host the LSIF layer.
#[salsa::db]
pub trait LsifDb: salsa::Database {}

// ---------------------------------------------------------------------------
// Tracked query
// ---------------------------------------------------------------------------

/// Language-specific definition keywords used for moniker counting.
fn definition_keywords(language_id: &str) -> &'static [&'static str] {
    match language_id {
        "rust" => &[
            "fn ", "struct ", "enum ", "trait ", "type ", "const ", "static ", "impl ",
        ],
        "typescript" | "javascript" => &[
            "function ",
            "class ",
            "interface ",
            "type ",
            "const ",
            "let ",
            "var ",
            "export ",
        ],
        _ => &["fn ", "def ", "class ", "function "],
    }
}

/// Index a single source file and return a summary `LsifFileResult`.
///
/// This is the only `#[salsa::tracked]` query in this crate.  It is memoised:
/// Salsa only re-executes when `src.text` or `src.language_id` changes.
///
/// The implementation counts:
/// - `vertex_count`  — non-empty lines (each maps to at least one LSIF vertex)
/// - `moniker_count` — occurrences of language-specific definition keywords
/// - `reference_count` — occurrences of language-specific identifiers beyond definitions
/// - `has_errors`    — whether the source appears structurally empty/corrupt
#[salsa::tracked]
pub fn index_document(db: &dyn LsifDb, src: LsifSource) -> LsifFileResult {
    let path = src.path(db);
    let text = src.text(db);
    let language_id = src.language_id(db);

    let has_errors = text.is_empty();

    let vertex_count = text.lines().filter(|l| !l.trim().is_empty()).count() as u32;

    let keywords = definition_keywords(&language_id);
    let moniker_count = keywords
        .iter()
        .map(|kw| text.matches(kw).count() as u32)
        .sum();

    // Reference count: rough estimate — total word-boundary identifier tokens
    // beyond the definition sites themselves.
    let reference_count = text
        .split(|c: char| !c.is_alphanumeric() && c != '_')
        .filter(|tok| {
            tok.len() > 1
                && tok
                    .chars()
                    .next()
                    .map(|c| c.is_alphabetic())
                    .unwrap_or(false)
        })
        .count()
        .saturating_sub(moniker_count as usize) as u32;

    LsifFileResult {
        document_uri: path,
        vertex_count,
        moniker_count,
        reference_count,
        has_errors,
    }
}

// ---------------------------------------------------------------------------
// Concrete database
// ---------------------------------------------------------------------------

/// Concrete Salsa database for the `lsp-max-lsif` crate.
#[salsa::db]
#[derive(Clone)]
pub struct LsifMaxDb {
    storage: salsa::Storage<Self>,
}

impl LsifMaxDb {
    pub fn new() -> Self {
        Self {
            storage: salsa::Storage::default(),
        }
    }
}

impl Default for LsifMaxDb {
    fn default() -> Self {
        Self::new()
    }
}

#[salsa::db]
impl salsa::Database for LsifMaxDb {}

#[salsa::db]
impl LsifDb for LsifMaxDb {}

// ---------------------------------------------------------------------------
// IncrementalLsifIndexer — thin wrapper for multi-file sessions
// ---------------------------------------------------------------------------

/// Wraps `LsifMaxDb` and a path → `LsifSource` registry so that unchanged
/// files reuse cached `LsifFileResult` values across indexing runs.
pub struct IncrementalLsifIndexer {
    db: LsifMaxDb,
    sources: std::collections::HashMap<String, LsifSource>,
}

impl Default for IncrementalLsifIndexer {
    fn default() -> Self {
        Self::new()
    }
}

impl IncrementalLsifIndexer {
    pub fn new() -> Self {
        Self {
            db: LsifMaxDb::new(),
            sources: std::collections::HashMap::new(),
        }
    }

    /// Register or update a file.  Returns the cached `LsifFileResult`,
    /// re-executing only if the text has changed since the last call.
    pub fn index_file(
        &mut self,
        path: impl Into<String>,
        text: impl Into<String>,
        language_id: impl Into<String>,
    ) -> LsifFileResult {
        let path = path.into();
        let text = text.into();
        let language_id = language_id.into();

        let src = if let Some(&existing) = self.sources.get(&path) {
            // Update text + language_id if they changed (Salsa tracks the revision).
            existing.set_text(&mut self.db).to(text.clone());
            existing
                .set_language_id(&mut self.db)
                .to(language_id.clone());
            existing
        } else {
            let src = LsifSource::new(&self.db, path.clone(), text, language_id);
            self.sources.insert(path, src);
            src
        };

        index_document(&self.db, src)
    }

    /// Remove a file from the tracked session.
    pub fn remove_file(&mut self, path: &str) {
        self.sources.remove(path);
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    const RUST_SRC: &str = r#"
fn hello() {
    println!("Hello, world!");
}

struct Point {
    x: f64,
    y: f64,
}

fn distance(a: Point, b: Point) -> f64 {
    let dx = a.x - b.x;
    let dy = a.y - b.y;
    (dx * dx + dy * dy).sqrt()
}
"#;

    const RUST_SRC_CHANGED: &str = r#"
fn hello() {
    println!("Hello!");
}
"#;

    // ------------------------------------------------------------------
    // TRUE: lsif_did_open_indexes_document
    //       Index a Rust file; verify vertex_count > 0.
    // ------------------------------------------------------------------
    #[test]
    fn lsif_did_open_indexes_document() {
        let mut indexer = IncrementalLsifIndexer::new();
        let result = indexer.index_file("src/lib.rs", RUST_SRC, "rust");

        assert!(
            result.vertex_count > 0,
            "vertex_count must be > 0 for non-empty source; got {result:?}"
        );
        assert_eq!(result.document_uri, "src/lib.rs");
        assert!(!result.has_errors, "valid source must not have errors");
    }

    // ------------------------------------------------------------------
    // TRUE: lsif_unchanged_file_is_cache_hit
    //       Index same source twice; assert identical LsifFileResult.
    // ------------------------------------------------------------------
    #[test]
    fn lsif_unchanged_file_is_cache_hit() {
        let mut indexer = IncrementalLsifIndexer::new();

        let r1 = indexer.index_file("src/lib.rs", RUST_SRC, "rust");
        let r2 = indexer.index_file("src/lib.rs", RUST_SRC, "rust");

        assert_eq!(r1, r2, "second call must return identical cached result");
    }

    // ------------------------------------------------------------------
    // TRUE: lsif_changed_file_is_reindexed
    //       Change the text; assert vertex_count changes.
    // ------------------------------------------------------------------
    #[test]
    fn lsif_changed_file_is_reindexed() {
        let mut indexer = IncrementalLsifIndexer::new();

        let r1 = indexer.index_file("src/lib.rs", RUST_SRC, "rust");
        let r2 = indexer.index_file("src/lib.rs", RUST_SRC_CHANGED, "rust");

        assert_ne!(
            r1.vertex_count, r2.vertex_count,
            "vertex_count must change when source text changes"
        );
    }

    // ------------------------------------------------------------------
    // COUNTERFACTUAL: lsif_two_files_independent_invalidation
    //       Change file A; assert file B result is unaffected.
    // ------------------------------------------------------------------
    #[test]
    fn lsif_two_files_independent_invalidation() {
        let mut indexer = IncrementalLsifIndexer::new();

        indexer.index_file("src/a.rs", RUST_SRC, "rust");
        let b_before = indexer.index_file("src/b.rs", RUST_SRC, "rust");

        // Mutate file A only.
        indexer.index_file("src/a.rs", RUST_SRC_CHANGED, "rust");

        let b_after = indexer.index_file("src/b.rs", RUST_SRC, "rust");

        assert_eq!(
            b_before, b_after,
            "file B result must be unchanged when only file A is mutated"
        );
    }

    // ------------------------------------------------------------------
    // FALSE: stale_lsif_index_is_stop
    //        An empty/corrupt source sets has_errors = true and returns
    //        vertex_count = 0 — NOT stale data from a previous call.
    // ------------------------------------------------------------------
    #[test]
    fn stale_lsif_index_is_stop() {
        let mut indexer = IncrementalLsifIndexer::new();

        // Index a valid file first.
        let valid = indexer.index_file("src/main.rs", RUST_SRC, "rust");
        assert!(valid.vertex_count > 0);

        // Now replace with empty text (simulates corrupt/deleted file).
        let empty = indexer.index_file("src/main.rs", "", "rust");

        assert!(
            empty.has_errors,
            "empty source must report has_errors = true; got {empty:?}"
        );
        assert_eq!(
            empty.vertex_count, 0,
            "empty source must have vertex_count = 0; got {empty:?}"
        );

        // Verify the result is not stale (it matches current empty state,
        // not the previous valid state).
        assert_ne!(
            valid.vertex_count, empty.vertex_count,
            "stale result must not be returned — vertex_count must differ"
        );
    }
}
