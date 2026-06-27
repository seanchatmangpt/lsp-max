//! LSIF → Oxigraph RDF named-graph import.
//!
//! Reads an LSIF JSONL file and loads a queryable RDF projection into an
//! Oxigraph named graph.  Only three vertex labels are projected:
//!
//! | LSIF label        | RDF type              | Properties             |
//! |-------------------|-----------------------|------------------------|
//! | `document`        | `lsif:document`       | `lsif:uri`             |
//! | `moniker` (vertex)| `lsif:symbol`         | `lsif:identifier`, `lsif:kind` |
//! | `referenceResult` | `lsif:referenceOf`    | (type triple only)     |
//!
//! All other LSIF labels are skipped.
//!
//! # INVARIANT: OXIGRAPH_NOT_ON_HOT_PATH — lsif_import is cold-path only.
//! This module MUST NOT be called from `did_change`, `did_open`, or any LSP
//! notification handler.  It is a batch import step run at admission time.

use std::io::{BufRead, BufReader};
use std::path::Path;

use oxigraph::model::{GraphNameRef, Literal, NamedNode, NamedNodeRef, Quad, Subject, Term};
use oxigraph::store::Store;

// ---------------------------------------------------------------------------
// RDF namespace
// ---------------------------------------------------------------------------

const NS: &str = "https://lsp-max.dev/lsif/";

fn ns(local: &str) -> NamedNode {
    NamedNode::new(format!("{NS}{local}")).expect("static LSIF namespace must be valid")
}

fn vertex_node(id: u64) -> NamedNode {
    NamedNode::new(format!("urn:lsif:v/{id}")).expect("LSIF vertex URN must be valid")
}

// ---------------------------------------------------------------------------
// Stats
// ---------------------------------------------------------------------------

/// Counts of RDF triples loaded per category during an LSIF import.
///
/// Counts reflect triples parsed from the file, not net additions to the store
/// (Oxigraph is a set store — duplicate triples are silently ignored).
/// This ensures `LsifImportStats` is identical across idempotent re-imports.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct LsifImportStats {
    pub document_triples: usize,
    pub moniker_triples: usize,
    pub reference_triples: usize,
    pub total_triples: usize,
}

// ---------------------------------------------------------------------------
// Import entry point
// ---------------------------------------------------------------------------

/// Import an LSIF JSONL file into an Oxigraph named graph.
///
/// # INVARIANT: OXIGRAPH_NOT_ON_HOT_PATH — lsif_import is cold-path only.
/// Call only at admission / receipt generation time, never from an LSP handler.
///
/// Returns counts of triples inserted per category.  The counts reflect the
/// number of triples parsed from the file; Oxigraph deduplicates automatically.
pub fn import_lsif_into_graph(
    store: &Store,
    lsif_path: &Path,
    graph_name: &str,
) -> anyhow::Result<LsifImportStats> {
    let graph_node = NamedNode::new(graph_name)
        .map_err(|e| anyhow::anyhow!("invalid graph name URI {graph_name:?}: {e}"))?;
    let graph_ref = GraphNameRef::NamedNode(graph_node.as_ref());

    let file = std::fs::File::open(lsif_path)
        .map_err(|e| anyhow::anyhow!("cannot open LSIF file {}: {e}", lsif_path.display()))?;
    let reader = BufReader::new(file);

    let mut stats = LsifImportStats::default();

    // Pre-build type predicates.
    let rdf_type = NamedNode::new("http://www.w3.org/1999/02/22-rdf-syntax-ns#type")
        .expect("rdf:type must be valid");
    let lsif_document = ns("document");
    let lsif_symbol = ns("symbol");
    let lsif_reference_of = ns("referenceOf");
    let lsif_uri_pred = ns("uri");
    let lsif_identifier_pred = ns("identifier");
    let lsif_kind_pred = ns("kind");

    for (line_no, line) in reader.lines().enumerate() {
        let line = line.map_err(|e| anyhow::anyhow!("read error at line {line_no}: {e}"))?;
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        let v: serde_json::Value = match serde_json::from_str(line) {
            Ok(v) => v,
            Err(_) => continue, // skip malformed lines
        };

        // Only process vertex entries.
        if v.get("type").and_then(|t| t.as_str()) != Some("vertex") {
            continue;
        }

        let id = match v.get("id").and_then(|i| i.as_u64()) {
            Some(id) => id,
            None => continue,
        };

        let label = match v.get("label").and_then(|l| l.as_str()) {
            Some(l) => l,
            None => continue,
        };

        let subject = Subject::NamedNode(vertex_node(id));

        match label {
            "document" => {
                let uri_val = v
                    .get("uri")
                    .and_then(|u| u.as_str())
                    .unwrap_or("")
                    .to_string();

                // <vertex_node> rdf:type lsif:document
                insert_quad(
                    store,
                    subject.clone(),
                    rdf_type.clone(),
                    Term::NamedNode(lsif_document.clone()),
                    graph_ref,
                );
                stats.document_triples += 1;

                // <vertex_node> lsif:uri "file:///..."
                insert_quad(
                    store,
                    subject,
                    lsif_uri_pred.clone(),
                    Term::Literal(Literal::new_simple_literal(uri_val)),
                    graph_ref,
                );
                stats.document_triples += 1;
            }

            "moniker" => {
                let identifier = v
                    .get("identifier")
                    .and_then(|i| i.as_str())
                    .unwrap_or("")
                    .to_string();
                let kind = v
                    .get("kind")
                    .and_then(|k| k.as_str())
                    .unwrap_or("local")
                    .to_string();

                // <vertex_node> rdf:type lsif:symbol
                insert_quad(
                    store,
                    subject.clone(),
                    rdf_type.clone(),
                    Term::NamedNode(lsif_symbol.clone()),
                    graph_ref,
                );
                stats.moniker_triples += 1;

                // <vertex_node> lsif:identifier "crate::Symbol"
                insert_quad(
                    store,
                    subject.clone(),
                    lsif_identifier_pred.clone(),
                    Term::Literal(Literal::new_simple_literal(identifier)),
                    graph_ref,
                );
                stats.moniker_triples += 1;

                // <vertex_node> lsif:kind "export"
                insert_quad(
                    store,
                    subject,
                    lsif_kind_pred.clone(),
                    Term::Literal(Literal::new_simple_literal(kind)),
                    graph_ref,
                );
                stats.moniker_triples += 1;
            }

            "referenceResult" => {
                // <vertex_node> rdf:type lsif:referenceOf
                insert_quad(
                    store,
                    subject,
                    rdf_type.clone(),
                    Term::NamedNode(lsif_reference_of.clone()),
                    graph_ref,
                );
                stats.reference_triples += 1;
            }

            _ => {} // skip all other labels
        }
    }

    stats.total_triples = stats.document_triples + stats.moniker_triples + stats.reference_triples;
    Ok(stats)
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Insert a single quad into the store, silently ignoring errors
/// (Oxigraph set semantics — duplicate insertions are no-ops).
fn insert_quad(
    store: &Store,
    subject: Subject,
    predicate: NamedNode,
    object: Term,
    graph_name: GraphNameRef<'_>,
) {
    let quad = Quad::new(subject, predicate, object, graph_name);
    let _ = store.insert(quad.as_ref());
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use oxigraph::store::Store;

    fn real_lsif_path() -> std::path::PathBuf {
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../receipts/v26.6.28-lsif.lsif")
    }

    const GRAPH: &str = "https://lsp-max.dev/graphs/v26.6.28";

    // ------------------------------------------------------------------
    // TRUE: document_triples > 0
    // ------------------------------------------------------------------
    #[test]
    fn lsif_import_loads_document_triples() {
        let lsif_path = real_lsif_path();
        if !lsif_path.exists() {
            eprintln!("SKIP: real LSIF file not found at {}", lsif_path.display());
            return;
        }
        let store = Store::new().unwrap();
        let stats = import_lsif_into_graph(&store, &lsif_path, GRAPH).unwrap();
        assert!(
            stats.document_triples > 0,
            "document_triples must be > 0; got {stats:?}"
        );
    }

    // ------------------------------------------------------------------
    // TRUE: moniker_triples > 0
    // ------------------------------------------------------------------
    #[test]
    fn lsif_import_loads_moniker_triples() {
        let lsif_path = real_lsif_path();
        if !lsif_path.exists() {
            eprintln!("SKIP: real LSIF file not found at {}", lsif_path.display());
            return;
        }
        let store = Store::new().unwrap();
        let stats = import_lsif_into_graph(&store, &lsif_path, GRAPH).unwrap();
        assert!(
            stats.moniker_triples > 0,
            "moniker_triples must be > 0; got {stats:?}"
        );
    }

    // ------------------------------------------------------------------
    // TRUE: total_triples > 0
    // ------------------------------------------------------------------
    #[test]
    fn lsif_import_stats_are_nonzero() {
        let lsif_path = real_lsif_path();
        if !lsif_path.exists() {
            eprintln!("SKIP: real LSIF file not found at {}", lsif_path.display());
            return;
        }
        let store = Store::new().unwrap();
        let stats = import_lsif_into_graph(&store, &lsif_path, GRAPH).unwrap();
        assert!(
            stats.total_triples > 0,
            "total_triples must be > 0; got {stats:?}"
        );
        assert_eq!(
            stats.total_triples,
            stats.document_triples + stats.moniker_triples + stats.reference_triples,
            "total_triples must equal sum of categories"
        );
    }

    // ------------------------------------------------------------------
    // COUNTERFACTUAL: importing same file twice produces identical stats
    // (Oxigraph is a set store — duplicate quads are silently ignored)
    // ------------------------------------------------------------------
    #[test]
    fn lsif_import_is_idempotent() {
        let lsif_path = real_lsif_path();
        if !lsif_path.exists() {
            eprintln!("SKIP: real LSIF file not found at {}", lsif_path.display());
            return;
        }
        let store = Store::new().unwrap();
        let stats1 = import_lsif_into_graph(&store, &lsif_path, GRAPH).unwrap();
        let stats2 = import_lsif_into_graph(&store, &lsif_path, GRAPH).unwrap();
        assert_eq!(
            stats1, stats2,
            "importing the same LSIF file twice must produce identical stats"
        );
    }
}
