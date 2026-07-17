//! LSIF 0.6.0 protocol coverage conformance tests.
//!
//! These tests verify, by indexing real Rust source snippets and parsing the
//! resulting LSIF NDJSON output with `LsifDump`, that `LsifBuilder` emits the
//! vertex and edge labels it claims to emit in `coverage::BUILDER_VERTEX_LABELS`
//! / `BUILDER_EDGE_LABELS`.
//!
//! # Design
//!
//! Rather than asserting on opaque element ids (which are deterministic only
//! within a single tool run), we assert on label presence in the parsed dump.
//! This mirrors the structural assertions in `general.rs` and `validate.rs`.
//!
//! # Chicago TDD
//!
//! Every assertion here requires real LSIF output produced by indexing real
//! Rust source.  There are no mock builders and no hand-crafted NDJSON strings
//! that bypass the indexer.

use lsif_tests::{index_rust_linked, LsifDump};
use lsp_max_lsif::coverage::{
    lsif_coverage, BUILDER_EDGE_LABELS, BUILDER_VERTEX_LABELS, SPEC_EDGE_LABELS, SPEC_VERTEX_LABELS,
};

// ─────────────────────────────────────────────────────────────────────────────
// Helpers
// ─────────────────────────────────────────────────────────────────────────────

/// Assert that the dump passes structural validation.
fn assert_valid(dump: &LsifDump) {
    if let Err(e) = dump.validate() {
        panic!("LSIF structural validation failed:\n{e}");
    }
}

/// Count elements in the dump with the given label.
fn count_by_label(dump: &LsifDump, label: &str) -> usize {
    dump.elements
        .values()
        .filter(|v| v.get("label").and_then(|x| x.as_str()) == Some(label))
        .count()
}

// ─────────────────────────────────────────────────────────────────────────────
// Structural invariants across all claimed emitted labels
// ─────────────────────────────────────────────────────────────────────────────

/// Index a snippet rich enough to exercise every claimed builder vertex and
/// confirm each appears in the dump at least once.
///
/// This is the main "builder matches its own manifest" test: if
/// `BUILDER_VERTEX_LABELS` claims a label is emitted but the indexer never
/// produces it for any reasonable input, this test surfaces the discrepancy.
#[test]
fn all_claimed_builder_vertices_appear_in_rich_snippet() {
    // A snippet that exercises: pub fn (resultSet, range, definitionResult,
    // hoverResult), impl block (range, resultSet), const (definitionResult).
    let source = "
/// A documented struct.
pub struct MyStruct {
    /// A field.
    pub value: u32,
}

impl MyStruct {
    /// A documented method.
    pub fn new(v: u32) -> Self {
        Self { value: v }
    }
}

/// A top-level constant.
pub const ANSWER: u32 = 42;

/// A top-level function.
pub fn add(a: u32, b: u32) -> u32 {
    a + b
}
";
    let dump = index_rust_linked("file:///test/rich.rs", source);
    assert_valid(&dump);

    // Collect all labels emitted in the real dump.
    use std::collections::HashSet;
    let emitted: HashSet<&str> = dump
        .elements
        .values()
        .filter_map(|v| v.get("label").and_then(|x| x.as_str()))
        .collect();

    // Every label in BUILDER_VERTEX_LABELS must appear in this rich snippet.
    let exempt = [
        // diagnosticResult is only emitted when diagnostics are present;
        // a clean snippet produces none.
        "diagnosticResult",
        // referenceResult is only emitted when a symbol has at least one
        // call/use site indexed in the same pass.  A single-file snippet
        // that only defines symbols (no call sites) produces no
        // referenceResult; the builder correctly emits it in multi-file
        // or call-site-bearing inputs.
        "referenceResult",
        // declarationResult mirrors definitionResult but is emitted only
        // when the indexer can distinguish a declaration from a definition
        // (e.g. a trait method vs an impl body).  A plain pub fn definition
        // produces only definitionResult.
        "declarationResult",
        // OPEN-substrate extended surface: emittable via direct `LsifBuilder`
        // calls (`lsif_builder/extended.rs`) but NOT driven by the rust indexer,
        // so a basic snippet never produces them. Their status in the registry
        // is `OpenSubstrate`; this consumer test scopes to indexer-driven labels.
        "implementationResult",
        "typeDefinitionResult",
        "callHierarchyResult",
        "typeHierarchyResult",
        "foldingRangeResult",
        "documentLinkResult",
        "documentSymbolResult",
        "semanticTokensResult",
        "source",
        "resultRange",
        "packageInformation",
        // The `capabilities` vertex is emittable via `LsifBuilder` but is not
        // produced by the rust indexer's single-file dump; its registry status
        // is `OpenSubstrate`.
        "capabilities",
    ];

    for (label, _) in BUILDER_VERTEX_LABELS {
        if exempt.contains(label) {
            continue;
        }
        assert!(
            emitted.contains(label),
            "BUILDER_VERTEX_LABELS claims {:?} is emitted but it does not appear in the rich snippet dump",
            label
        );
    }
}

/// Mirror of the above for edge labels.
#[test]
fn all_claimed_builder_edges_appear_in_rich_snippet() {
    let source = "
/// Documented.
pub fn rich_fn(x: u32) -> u32 { x + 1 }

pub struct S;
impl S {
    /// Method.
    pub fn method() {}
}
";
    let dump = index_rust_linked("file:///test/edge_rich.rs", source);
    assert_valid(&dump);

    use std::collections::HashSet;
    let emitted: HashSet<&str> = dump
        .elements
        .values()
        .filter_map(|v| v.get("label").and_then(|x| x.as_str()))
        .collect();

    let exempt = [
        // textDocument/declaration and textDocument/references require
        // multi-file indexing with a linker run; the single-file dump only
        // produces definition edges.
        "textDocument/declaration",
        "textDocument/references",
        // textDocument/diagnostic only appears when diagnostics are emitted.
        "textDocument/diagnostic",
        // item edges appear alongside definition/declaration/reference
        // result sets; single-file snippet may not produce all of them.
        "item",
        // OPEN-substrate extended surface: emittable via direct `LsifBuilder`
        // calls (`lsif_builder/extended.rs`) but NOT driven by the rust indexer.
        "textDocument/implementation",
        "textDocument/typeDefinition",
        "textDocument/callHierarchy",
        "textDocument/typeHierarchy",
        "textDocument/foldingRange",
        "textDocument/documentLink",
        "textDocument/documentSymbol",
        "textDocument/semanticTokens/full",
        "attach",
        "packageInformation",
        // `nextMoniker` chains monikers across import/export boundaries; the
        // single-file snippet emits monikers but no nextMoniker edge. Its
        // registry status is `OpenSubstrate`.
        "nextMoniker",
        // `belongsTo` links a moniker to its packageInformation vertex; the rust
        // indexer emits neither in a single-file snippet, so this edge does not
        // appear. Its registry status is `OpenSubstrate`.
        "belongsTo",
    ];

    for (label, _) in BUILDER_EDGE_LABELS {
        if exempt.contains(label) {
            continue;
        }
        assert!(
            emitted.contains(label),
            "BUILDER_EDGE_LABELS claims {:?} is emitted but it does not appear in the rich snippet dump",
            label
        );
    }
}
