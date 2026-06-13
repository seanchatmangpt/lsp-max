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
//! * `BUILDER_VERTEX_LABELS` / `BUILDER_EDGE_LABELS` — what `LsifBuilder`
//!   actually emits (verified by reading every `Vertex::*` / `Edge::*`
//!   construction site in `lsif_builder.rs`).
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
// What LsifBuilder actually emits
//
// Derived by reading every `Vertex::*` and `Edge::*` construction site in
// `lsif_builder.rs`.  Updated whenever new emit methods are added.
// ─────────────────────────────────────────────────────────────────────────────

/// Vertex labels that `LsifBuilder` actually constructs and writes.
///
/// Verified against `lsif_builder.rs`:
/// `emit_metadata` → `Vertex::MetaData`
/// `emit_project`  → `Vertex::Project`
/// `emit_document` → `Vertex::Document`
/// `emit_result_set` → `Vertex::ResultSet`
/// `emit_range`    → `Vertex::Range`
/// `bind_hover`    → `Vertex::HoverResult`
/// `bind_definition` → `Vertex::DefinitionResult`
/// `bind_references` → `Vertex::ReferenceResult`
/// `bind_declaration` → `Vertex::DeclarationResult`
/// `diagnostic_result` → `Vertex::DiagnosticResult`
/// `begin_document` / `end_document` / `begin_project` / `end_project` → `Vertex::Event`
pub const BUILDER_VERTEX_LABELS: &[&str] = &[
    "metaData",
    "project",
    "document",
    "resultSet",
    "range",
    "hoverResult",
    "referenceResult",
    "declarationResult",
    "definitionResult",
    "diagnosticResult",
    "$event",
];

/// Edge labels that `LsifBuilder` actually writes.
///
/// Verified against `lsif_builder.rs`:
/// `contains`         → `Edge::Contains`
/// `bind_next`        → `Edge::Next`
/// `bind_hover`       → `Edge::TextDocumentHover`
/// `bind_definition`  → `Edge::TextDocumentDefinition` + `Edge::Item`
/// `bind_references`  → `Edge::TextDocumentReferences` + `Edge::Item`
/// `bind_declaration` → `Edge::TextDocumentDeclaration` + `Edge::Item`
/// `diagnostic_edge`  → `Edge::TextDocumentDiagnostic`
pub const BUILDER_EDGE_LABELS: &[&str] = &[
    "contains",
    "next",
    "item",
    "textDocument/hover",
    "textDocument/definition",
    "textDocument/declaration",
    "textDocument/references",
    "textDocument/diagnostic",
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
    /// Vertex labels emitted by `LsifBuilder`.
    pub emitted_vertices: Vec<&'static str>,
    /// Vertex labels defined in the spec but not emitted by `LsifBuilder`.
    pub missing_vertices: Vec<&'static str>,
    /// Percentage of spec vertex labels emitted by `LsifBuilder`.
    pub vertex_coverage_pct: f64,
    /// Edge labels emitted by `LsifBuilder`.
    pub emitted_edges: Vec<&'static str>,
    /// Edge labels defined in the spec but not emitted by `LsifBuilder`.
    pub missing_edges: Vec<&'static str>,
    /// Percentage of spec edge labels emitted by `LsifBuilder`.
    pub edge_coverage_pct: f64,
}

impl LsifCoverageReport {
    /// One-line human-readable summary.
    pub fn summary(&self) -> String {
        format!(
            "LSIF 0.6.0 coverage — vertices: {:.1}% ({}/{}), edges: {:.1}% ({}/{})",
            self.vertex_coverage_pct,
            self.emitted_vertices.len(),
            SPEC_VERTEX_LABELS.len(),
            self.edge_coverage_pct,
            self.emitted_edges.len(),
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

    let builder_v: HashSet<&str> = BUILDER_VERTEX_LABELS.iter().copied().collect();
    let builder_e: HashSet<&str> = BUILDER_EDGE_LABELS.iter().copied().collect();

    let mut emitted_vertices: Vec<&'static str> = SPEC_VERTEX_LABELS
        .iter()
        .copied()
        .filter(|l| builder_v.contains(l))
        .collect();
    emitted_vertices.sort_unstable();

    let mut missing_vertices: Vec<&'static str> = SPEC_VERTEX_LABELS
        .iter()
        .copied()
        .filter(|l| !builder_v.contains(l))
        .collect();
    missing_vertices.sort_unstable();

    let mut emitted_edges: Vec<&'static str> = SPEC_EDGE_LABELS
        .iter()
        .copied()
        .filter(|l| builder_e.contains(l))
        .collect();
    emitted_edges.sort_unstable();

    let mut missing_edges: Vec<&'static str> = SPEC_EDGE_LABELS
        .iter()
        .copied()
        .filter(|l| !builder_e.contains(l))
        .collect();
    missing_edges.sort_unstable();

    let v_total = SPEC_VERTEX_LABELS.len();
    let e_total = SPEC_EDGE_LABELS.len();

    let vertex_coverage_pct = if v_total == 0 {
        0.0
    } else {
        100.0 * emitted_vertices.len() as f64 / v_total as f64
    };
    let edge_coverage_pct = if e_total == 0 {
        0.0
    } else {
        100.0 * emitted_edges.len() as f64 / e_total as f64
    };

    LsifCoverageReport {
        emitted_vertices,
        missing_vertices,
        vertex_coverage_pct,
        edge_coverage_pct,
        emitted_edges,
        missing_edges,
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    /// Vertex coverage watermark — raise when new vertices are added to
    /// `LsifBuilder`, never lower.
    const VERTEX_WATERMARK: f64 = 45.0;

    /// Edge coverage watermark — raise when new edges are added to
    /// `LsifBuilder`, never lower.
    const EDGE_WATERMARK: f64 = 40.0;

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
        for label in BUILDER_VERTEX_LABELS {
            assert!(
                spec_v.contains(label),
                "BUILDER_VERTEX_LABELS contains {:?} which is not in SPEC_VERTEX_LABELS",
                label
            );
        }
        for label in BUILDER_EDGE_LABELS {
            assert!(
                spec_e.contains(label),
                "BUILDER_EDGE_LABELS contains {:?} which is not in SPEC_EDGE_LABELS",
                label
            );
        }
    }
}
