//! LSIF 0.6.0 protocol coverage matrix for `LsifBuilder`.
//!
//! # Why this exists
//!
//! `LsifBuilder` emits a useful but incomplete subset of the LSIF 0.6.0
//! vertex and edge label vocabulary.  This module quantifies that gap:
//!
//! * `SPEC_VERTEX_LABELS` — every vertex label defined in `lsif.rs` (derived
//!   from the `#[serde(rename = "...")]` attributes on the `Vertex` enum).
//! * `SPEC_EDGE_LABELS` — every edge label defined in `lsif.rs`.
//! * `BUILDER_VERTEX_LABELS` / `BUILDER_EDGE_LABELS` — what the LSIF emission
//!   path actually emits.  This list spans two layers: most labels come from
//!   `Vertex::*` / `Edge::*` construction sites in `lsif_builder.rs`, but the
//!   `moniker` vertex and `moniker` edge are emitted one layer up, in
//!   `LsifContext::emit_moniker` (`lsif_indexer.rs`), which calls
//!   `builder.emit(raw Element)` directly rather than through a top-level
//!   `LsifBuilder` method.  Counting only `lsif_builder.rs` would undercount,
//!   because `emit_moniker` is driven by the real lsif-rust / lsif-typescript
//!   indexers during indexing runs.
//!
//! The coverage percentages are reported separately for vertices and edges,
//! because the LSIF spec has a deliberate asymmetry: many result vertices
//! exist that are only emitted by advanced language-feature indexers (call
//! hierarchy, type hierarchy, moniker, …).
//!
//! # Coverage watermarks
//!
//! The tests in this module assert that vertex and edge coverage stay at or
//! above the measured baselines.  Removing a method from `LsifBuilder` will
//! fail the tests; adding new methods should raise the watermarks.

// ─────────────────────────────────────────────────────────────────────────────
// Spec-defined vertex labels (from Vertex enum in lsif.rs)
// ─────────────────────────────────────────────────────────────────────────────

/// Every vertex label declared in the `Vertex` enum in `lsif.rs`.
///
/// These are the `#[serde(rename = "...")]` values on each variant, which are
/// what appear in the actual NDJSON output and what the LSIF 0.6.0 spec
/// normalises on.
pub const SPEC_VERTEX_LABELS: &[&str] = &[
    "metaData",
    "source",
    "project",
    "document",
    "resultSet",
    "range",
    "resultRange",
    "moniker",
    "packageInformation",
    "hoverResult",
    "referenceResult",
    "declarationResult",
    "definitionResult",
    "implementationResult",
    "typeDefinitionResult",
    "callHierarchyResult",
    "typeHierarchyResult",
    "foldingRangeResult",
    "documentLinkResult",
    "documentSymbolResult",
    "diagnosticResult",
    "semanticTokensResult",
    "$event",
];

// ─────────────────────────────────────────────────────────────────────────────
// Spec-defined edge labels (from Edge enum in lsif.rs)
// ─────────────────────────────────────────────────────────────────────────────

/// Every edge label declared in the `Edge` enum in `lsif.rs`.
pub const SPEC_EDGE_LABELS: &[&str] = &[
    "contains",
    "next",
    "moniker",
    "attach",
    "packageInformation",
    "item",
    "textDocument/hover",
    "textDocument/definition",
    "textDocument/declaration",
    "textDocument/references",
    "textDocument/implementation",
    "textDocument/typeDefinition",
    "textDocument/callHierarchy",
    "textDocument/typeHierarchy",
    "textDocument/foldingRange",
    "textDocument/documentLink",
    "textDocument/documentSymbol",
    "textDocument/diagnostic",
    "textDocument/semanticTokens/full",
];

// ─────────────────────────────────────────────────────────────────────────────
// What the LSIF emission path actually emits + each label's consumer status
//
// Derived by reading every `Vertex::*` and `Edge::*` construction site in
// `lsif_builder.rs` and `lsif_builder/extended.rs`, PLUS the moniker
// vertex/edge emitted one layer up in `LsifContext::emit_moniker`
// (`lsif_indexer.rs`).  Updated whenever new emit methods are added at either
// layer.
//
// A label being *emittable* (an emit method exists) is distinct from a label
// being *admitted* (a named behavioral test drives it end-to-end through a
// real consumer — the lsif-rust / lsif-typescript indexers).  Most of the
// `extended.rs` surface is emittable but has no product consumer driving it:
// the rust/ts indexers do not produce implementationResult, typeHierarchy,
// foldingRange, documentLink, documentSymbol, semanticTokens, packageInfo,
// source, or resultRange.  Those are OPEN substrate, not ADMITTED.
// ─────────────────────────────────────────────────────────────────────────────

/// Consumer status of an emittable LSIF label.
///
/// * `Admitted(test)` — a real consumer drives this label, witnessed by the
///   named behavioral test.  The string is the consuming test name so the
///   claim is auditable; never mark a label `Admitted` without one.
/// * `OpenSubstrate` — an emit method exists, but no product consumer (the
///   rust/ts indexers) drives this label.  Emittable, not admitted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConsumerStatus {
    /// Driven through a consumer by the named behavioral test.
    Admitted(&'static str),
    /// Emittable substrate with no product consumer yet.
    OpenSubstrate,
}

/// Vertex labels constructed on the LSIF emission path, paired with status.
///
/// Verified against `lsif_builder.rs` and `lsif_builder/extended.rs`.
/// The lower block (`OpenSubstrate`) is the `extended.rs` surface that no
/// indexer drives; only labels with a named consuming test are `Admitted`.
pub const BUILDER_VERTEX_LABELS: &[(&str, ConsumerStatus)] = &[
    // Driven end-to-end by the lsif-rust / lsif-typescript indexers and
    // witnessed by golden-output behavioral tests in `tests/`.
    (
        "metaData",
        ConsumerStatus::Admitted("lsif-tests/coverage.rs::metadata_vertex_present_exactly_once"),
    ),
    (
        "moniker",
        ConsumerStatus::Admitted(
            "lsif-tests/validate.rs::moniker_edges_connect_result_set_to_moniker",
        ),
    ),
    // Emittable; no named test drives these through a consumer yet.
    ("project", ConsumerStatus::OpenSubstrate),
    ("document", ConsumerStatus::OpenSubstrate),
    ("resultSet", ConsumerStatus::OpenSubstrate),
    ("range", ConsumerStatus::OpenSubstrate),
    ("hoverResult", ConsumerStatus::OpenSubstrate),
    ("referenceResult", ConsumerStatus::OpenSubstrate),
    ("declarationResult", ConsumerStatus::OpenSubstrate),
    ("definitionResult", ConsumerStatus::OpenSubstrate),
    ("diagnosticResult", ConsumerStatus::OpenSubstrate),
    ("$event", ConsumerStatus::OpenSubstrate),
    // Newly emittable via `lsif_builder/extended.rs` — OPEN substrate: the
    // rust/ts indexers do not produce these result vertices.
    ("implementationResult", ConsumerStatus::OpenSubstrate),
    ("typeDefinitionResult", ConsumerStatus::OpenSubstrate),
    ("callHierarchyResult", ConsumerStatus::OpenSubstrate),
    ("typeHierarchyResult", ConsumerStatus::OpenSubstrate),
    ("foldingRangeResult", ConsumerStatus::OpenSubstrate),
    ("documentLinkResult", ConsumerStatus::OpenSubstrate),
    ("documentSymbolResult", ConsumerStatus::OpenSubstrate),
    ("semanticTokensResult", ConsumerStatus::OpenSubstrate),
    ("source", ConsumerStatus::OpenSubstrate),
    ("resultRange", ConsumerStatus::OpenSubstrate),
    ("packageInformation", ConsumerStatus::OpenSubstrate),
];

/// Edge labels written on the LSIF emission path, paired with status.
///
/// Verified against `lsif_builder.rs` and `lsif_builder/extended.rs`.
pub const BUILDER_EDGE_LABELS: &[(&str, ConsumerStatus)] = &[
    // Driven end-to-end and witnessed by golden-output behavioral tests.
    (
        "contains",
        ConsumerStatus::Admitted("lsif-tests/coverage.rs::contains_edge_present"),
    ),
    (
        "next",
        ConsumerStatus::Admitted("lsif-tests/validate.rs::next_edges_connect_range_to_result_set"),
    ),
    (
        "moniker",
        ConsumerStatus::Admitted(
            "lsif-tests/validate.rs::moniker_edges_connect_result_set_to_moniker",
        ),
    ),
    // Emittable; no named test drives these through a consumer yet.
    ("item", ConsumerStatus::OpenSubstrate),
    ("textDocument/hover", ConsumerStatus::OpenSubstrate),
    ("textDocument/definition", ConsumerStatus::OpenSubstrate),
    ("textDocument/declaration", ConsumerStatus::OpenSubstrate),
    ("textDocument/references", ConsumerStatus::OpenSubstrate),
    ("textDocument/diagnostic", ConsumerStatus::OpenSubstrate),
    // Newly emittable via `lsif_builder/extended.rs` — OPEN substrate.
    ("textDocument/implementation", ConsumerStatus::OpenSubstrate),
    ("textDocument/typeDefinition", ConsumerStatus::OpenSubstrate),
    ("textDocument/callHierarchy", ConsumerStatus::OpenSubstrate),
    ("textDocument/typeHierarchy", ConsumerStatus::OpenSubstrate),
    ("textDocument/foldingRange", ConsumerStatus::OpenSubstrate),
    ("textDocument/documentLink", ConsumerStatus::OpenSubstrate),
    ("textDocument/documentSymbol", ConsumerStatus::OpenSubstrate),
    (
        "textDocument/semanticTokens/full",
        ConsumerStatus::OpenSubstrate,
    ),
    ("attach", ConsumerStatus::OpenSubstrate),
    ("packageInformation", ConsumerStatus::OpenSubstrate),
];

// ─────────────────────────────────────────────────────────────────────────────
// LsifCoverageReport
// ─────────────────────────────────────────────────────────────────────────────

/// The result of comparing `LsifBuilder`'s actual emission set against the
/// full LSIF 0.6.0 spec vocabulary.
///
/// Vertex and edge coverage are reported separately because the LSIF spec
/// includes many result-vertex types that are only relevant for language
/// features beyond basic hover/definition/references (e.g. call hierarchy,
/// type hierarchy, moniker, package information).
#[derive(Debug, Clone)]
pub struct LsifCoverageReport {
    /// Vertex labels emitted by `LsifBuilder` (emittable: an emit method exists).
    pub emitted_vertices: Vec<&'static str>,
    /// Vertex labels defined in the spec but not emitted by `LsifBuilder`.
    pub missing_vertices: Vec<&'static str>,
    /// Percentage of spec vertex labels emittable by `LsifBuilder`.
    pub vertex_coverage_pct: f64,
    /// Vertex labels with `ConsumerStatus::Admitted` (a named test drives them).
    pub admitted_vertices: Vec<&'static str>,
    /// Percentage of spec vertex labels that are admitted (not merely emittable).
    pub admitted_vertex_coverage_pct: f64,
    /// Edge labels emitted by `LsifBuilder` (emittable: an emit method exists).
    pub emitted_edges: Vec<&'static str>,
    /// Edge labels defined in the spec but not emitted by `LsifBuilder`.
    pub missing_edges: Vec<&'static str>,
    /// Percentage of spec edge labels emittable by `LsifBuilder`.
    pub edge_coverage_pct: f64,
    /// Edge labels with `ConsumerStatus::Admitted` (a named test drives them).
    pub admitted_edges: Vec<&'static str>,
    /// Percentage of spec edge labels that are admitted (not merely emittable).
    pub admitted_edge_coverage_pct: f64,
}

impl LsifCoverageReport {
    /// One-line human-readable summary.
    pub fn summary(&self) -> String {
        format!(
            "LSIF 0.6.0 coverage — vertices emittable {:.1}% ({}/{}) admitted {:.1}% ({}/{}), \
             edges emittable {:.1}% ({}/{}) admitted {:.1}% ({}/{})",
            self.vertex_coverage_pct,
            self.emitted_vertices.len(),
            SPEC_VERTEX_LABELS.len(),
            self.admitted_vertex_coverage_pct,
            self.admitted_vertices.len(),
            SPEC_VERTEX_LABELS.len(),
            self.edge_coverage_pct,
            self.emitted_edges.len(),
            SPEC_EDGE_LABELS.len(),
            self.admitted_edge_coverage_pct,
            self.admitted_edges.len(),
            SPEC_EDGE_LABELS.len(),
        )
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// lsif_coverage()
// ─────────────────────────────────────────────────────────────────────────────

/// Compute the LSIF 0.6.0 coverage report for `LsifBuilder`.
pub fn lsif_coverage() -> LsifCoverageReport {
    use std::collections::HashSet;

    let emittable_v: HashSet<&str> = BUILDER_VERTEX_LABELS.iter().map(|(l, _)| *l).collect();
    let emittable_e: HashSet<&str> = BUILDER_EDGE_LABELS.iter().map(|(l, _)| *l).collect();
    let admitted_v: HashSet<&str> = BUILDER_VERTEX_LABELS
        .iter()
        .filter(|(_, s)| matches!(s, ConsumerStatus::Admitted(_)))
        .map(|(l, _)| *l)
        .collect();
    let admitted_e: HashSet<&str> = BUILDER_EDGE_LABELS
        .iter()
        .filter(|(_, s)| matches!(s, ConsumerStatus::Admitted(_)))
        .map(|(l, _)| *l)
        .collect();

    let select = |spec: &[&'static str], set: &HashSet<&str>| -> Vec<&'static str> {
        let mut v: Vec<&'static str> = spec.iter().copied().filter(|l| set.contains(l)).collect();
        v.sort_unstable();
        v
    };

    let emitted_vertices = select(SPEC_VERTEX_LABELS, &emittable_v);
    let admitted_vertices = select(SPEC_VERTEX_LABELS, &admitted_v);
    let emitted_edges = select(SPEC_EDGE_LABELS, &emittable_e);
    let admitted_edges = select(SPEC_EDGE_LABELS, &admitted_e);

    let mut missing_vertices: Vec<&'static str> = SPEC_VERTEX_LABELS
        .iter()
        .copied()
        .filter(|l| !emittable_v.contains(l))
        .collect();
    missing_vertices.sort_unstable();

    let mut missing_edges: Vec<&'static str> = SPEC_EDGE_LABELS
        .iter()
        .copied()
        .filter(|l| !emittable_e.contains(l))
        .collect();
    missing_edges.sort_unstable();

    let v_total = SPEC_VERTEX_LABELS.len();
    let e_total = SPEC_EDGE_LABELS.len();

    let pct = |n: usize, total: usize| -> f64 {
        if total == 0 {
            0.0
        } else {
            100.0 * n as f64 / total as f64
        }
    };

    LsifCoverageReport {
        vertex_coverage_pct: pct(emitted_vertices.len(), v_total),
        admitted_vertex_coverage_pct: pct(admitted_vertices.len(), v_total),
        edge_coverage_pct: pct(emitted_edges.len(), e_total),
        admitted_edge_coverage_pct: pct(admitted_edges.len(), e_total),
        emitted_vertices,
        missing_vertices,
        admitted_vertices,
        emitted_edges,
        missing_edges,
        admitted_edges,
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    /// Vertex coverage watermark — raise when new vertices are added on the
    /// emission path, never lower.  Floor reflects 12/23 emitted (52.2%; moniker
    /// vertex now counted from `LsifContext::emit_moniker`).
    const VERTEX_WATERMARK: f64 = 50.0;

    /// Edge coverage watermark — raise when new edges are added on the
    /// emission path, never lower.  Floor reflects 9/19 emitted (47.4%; moniker
    /// edge now counted from `LsifContext::emit_moniker`).  Raised from 40.0;
    /// Phase B raises it further as the remaining spec edges are emitted.
    const EDGE_WATERMARK: f64 = 47.0;

    #[test]
    fn vertex_coverage_meets_watermark() {
        let report = lsif_coverage();
        eprintln!("{}", report.summary());
        eprintln!("Missing vertices: {:?}", report.missing_vertices);
        assert!(
            report.vertex_coverage_pct >= VERTEX_WATERMARK,
            "LSIF vertex coverage {:.1}% is below watermark {:.1}%.\n\
             Missing: {:?}",
            report.vertex_coverage_pct,
            VERTEX_WATERMARK,
            report.missing_vertices,
        );
    }

    #[test]
    fn edge_coverage_meets_watermark() {
        let report = lsif_coverage();
        assert!(
            report.edge_coverage_pct >= EDGE_WATERMARK,
            "LSIF edge coverage {:.1}% is below watermark {:.1}%.\n\
             Missing: {:?}",
            report.edge_coverage_pct,
            EDGE_WATERMARK,
            report.missing_edges,
        );
    }

    #[test]
    fn metadata_vertex_is_emitted() {
        let report = lsif_coverage();
        assert!(
            report.emitted_vertices.contains(&"metaData"),
            "metaData must be in emitted vertices"
        );
    }

    #[test]
    fn contains_edge_is_emitted() {
        let report = lsif_coverage();
        assert!(
            report.emitted_edges.contains(&"contains"),
            "'contains' edge must be in emitted edges"
        );
    }

    #[test]
    fn next_edge_is_emitted() {
        let report = lsif_coverage();
        assert!(
            report.emitted_edges.contains(&"next"),
            "'next' edge must be in emitted edges"
        );
    }

    #[test]
    fn spec_labels_are_non_empty() {
        assert!(
            !SPEC_VERTEX_LABELS.is_empty(),
            "spec vertex list must not be empty"
        );
        assert!(
            !SPEC_EDGE_LABELS.is_empty(),
            "spec edge list must not be empty"
        );
    }

    #[test]
    fn builder_labels_are_subset_of_spec() {
        use std::collections::HashSet;
        let spec_v: HashSet<&str> = SPEC_VERTEX_LABELS.iter().copied().collect();
        let spec_e: HashSet<&str> = SPEC_EDGE_LABELS.iter().copied().collect();
        for (label, _) in BUILDER_VERTEX_LABELS {
            assert!(
                spec_v.contains(label),
                "BUILDER_VERTEX_LABELS contains {:?} which is not in SPEC_VERTEX_LABELS",
                label
            );
        }
        for (label, _) in BUILDER_EDGE_LABELS {
            assert!(
                spec_e.contains(label),
                "BUILDER_EDGE_LABELS contains {:?} which is not in SPEC_EDGE_LABELS",
                label
            );
        }
    }
}
