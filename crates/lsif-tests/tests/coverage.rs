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
// Coverage matrix correctness
// ─────────────────────────────────────────────────────────────────────────────

/// The static `BUILDER_VERTEX_LABELS` list must be a subset of `SPEC_VERTEX_LABELS`.
/// This is a compile-time property verified at test time.
#[test]
fn builder_vertex_labels_are_subset_of_spec() {
    use std::collections::HashSet;
    let spec: HashSet<&str> = SPEC_VERTEX_LABELS.iter().copied().collect();
    for (label, _) in BUILDER_VERTEX_LABELS {
        assert!(
            spec.contains(label),
            "BUILDER_VERTEX_LABELS contains {:?} which is not in SPEC_VERTEX_LABELS",
            label
        );
    }
}

/// The static `BUILDER_EDGE_LABELS` list must be a subset of `SPEC_EDGE_LABELS`.
#[test]
fn builder_edge_labels_are_subset_of_spec() {
    use std::collections::HashSet;
    let spec: HashSet<&str> = SPEC_EDGE_LABELS.iter().copied().collect();
    for (label, _) in BUILDER_EDGE_LABELS {
        assert!(
            spec.contains(label),
            "BUILDER_EDGE_LABELS contains {:?} which is not in SPEC_EDGE_LABELS",
            label
        );
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Vertex emission — verified against real LSIF output
// ─────────────────────────────────────────────────────────────────────────────

/// `metaData` vertex: every valid LSIF graph MUST begin with exactly one
/// `metaData` vertex.
#[test]
fn metadata_vertex_present_exactly_once() {
    let dump = index_rust_linked("file:///test/meta.rs", "pub fn f() {}");
    assert_valid(&dump);
    let count = count_by_label(&dump, "metaData");
    assert_eq!(
        count, 1,
        "expected exactly one metaData vertex, got {count}"
    );
}

/// `project` vertex: emitted by `emit_project`.
#[test]
fn project_vertex_present() {
    let dump = index_rust_linked("file:///test/proj.rs", "pub fn g() {}");
    assert_valid(&dump);
    let count = count_by_label(&dump, "project");
    assert!(
        count >= 1,
        "expected at least one project vertex, got {count}"
    );
}

/// `document` vertex: emitted by `emit_document`.
#[test]
fn document_vertex_present() {
    let dump = index_rust_linked("file:///test/doc.rs", "pub const N: u32 = 1;");
    assert_valid(&dump);
    let count = count_by_label(&dump, "document");
    assert!(
        count >= 1,
        "expected at least one document vertex, got {count}"
    );
}

/// `resultSet` vertex: emitted for every symbol with semantic information.
#[test]
fn result_set_vertex_present() {
    // A pub function should produce at least one resultSet.
    let dump = index_rust_linked("file:///test/rs.rs", "pub fn exported() -> u32 { 0 }");
    assert_valid(&dump);
    let count = count_by_label(&dump, "resultSet");
    assert!(
        count >= 1,
        "expected at least one resultSet vertex, got {count}"
    );
}

/// `range` vertex: emitted for every symbol occurrence.
#[test]
fn range_vertex_present() {
    let dump = index_rust_linked("file:///test/rng.rs", "pub fn exported() -> u32 { 0 }");
    assert_valid(&dump);
    let count = count_by_label(&dump, "range");
    assert!(
        count >= 1,
        "expected at least one range vertex, got {count}"
    );
}

/// `hoverResult` vertex: emitted by `bind_hover`.
#[test]
fn hover_result_vertex_present_for_documented_fn() {
    // A function with a doc comment should produce a hover result vertex.
    let source = "/// Returns zero.\npub fn zero() -> u32 { 0 }";
    let dump = index_rust_linked("file:///test/hover.rs", source);
    assert_valid(&dump);
    let count = count_by_label(&dump, "hoverResult");
    assert!(
        count >= 1,
        "expected at least one hoverResult vertex, got {count}"
    );
}

/// `definitionResult` vertex: emitted by `bind_definition`.
#[test]
fn definition_result_vertex_present() {
    let dump = index_rust_linked("file:///test/def.rs", "pub fn defined() {}");
    assert_valid(&dump);
    let count = count_by_label(&dump, "definitionResult");
    assert!(
        count >= 1,
        "expected at least one definitionResult vertex, got {count}"
    );
}

/// `$event` vertex: emitted by `begin_document` / `end_document`.
#[test]
fn event_vertex_present() {
    let dump = index_rust_linked("file:///test/evt.rs", "pub fn f() {}");
    assert_valid(&dump);
    let count = count_by_label(&dump, "$event");
    assert!(
        count >= 2,
        "expected at least 2 $event vertices (begin+end document), got {count}"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Edge emission — verified against real LSIF output
// ─────────────────────────────────────────────────────────────────────────────

/// `contains` edge: links project→document and document→range.
#[test]
fn contains_edge_present() {
    let dump = index_rust_linked("file:///test/c.rs", "pub fn f() {}");
    assert_valid(&dump);
    let count = count_by_label(&dump, "contains");
    assert!(
        count >= 1,
        "expected at least one 'contains' edge, got {count}"
    );
}

/// `next` edge: links range→resultSet.
#[test]
fn next_edge_present() {
    let dump = index_rust_linked("file:///test/nx.rs", "pub fn f() {}");
    assert_valid(&dump);
    let count = count_by_label(&dump, "next");
    assert!(count >= 1, "expected at least one 'next' edge, got {count}");
}

/// `textDocument/hover` edge: links resultSet→hoverResult.
#[test]
fn hover_edge_present_for_documented_fn() {
    let source = "/// A documented function.\npub fn documented() {}";
    let dump = index_rust_linked("file:///test/hv.rs", source);
    assert_valid(&dump);
    let count = count_by_label(&dump, "textDocument/hover");
    assert!(
        count >= 1,
        "expected at least one 'textDocument/hover' edge, got {count}"
    );
}

/// `textDocument/definition` edge: links resultSet→definitionResult.
#[test]
fn definition_edge_present() {
    let dump = index_rust_linked("file:///test/td.rs", "pub fn f() {}");
    assert_valid(&dump);
    let count = count_by_label(&dump, "textDocument/definition");
    assert!(
        count >= 1,
        "expected at least one 'textDocument/definition' edge, got {count}"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Coverage watermark — mirrors lsp_coverage watermark contract
// ─────────────────────────────────────────────────────────────────────────────

/// Vertex coverage must stay at or above the baseline.
/// Adjust upward when new vertex types are added to `LsifBuilder`.
#[test]
fn vertex_coverage_meets_watermark() {
    const WATERMARK: f64 = 45.0;
    let report = lsif_coverage();
    eprintln!("{}", report.summary());
    assert!(
        report.vertex_coverage_pct >= WATERMARK,
        "LSIF vertex coverage {:.1}% is below watermark {:.1}%\n\
         Missing: {:?}",
        report.vertex_coverage_pct,
        WATERMARK,
        report.missing_vertices,
    );
}

/// Edge coverage must stay at or above the baseline.
#[test]
fn edge_coverage_meets_watermark() {
    const WATERMARK: f64 = 40.0;
    let report = lsif_coverage();
    assert!(
        report.edge_coverage_pct >= WATERMARK,
        "LSIF edge coverage {:.1}% is below watermark {:.1}%\n\
         Missing: {:?}",
        report.edge_coverage_pct,
        WATERMARK,
        report.missing_edges,
    );
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
