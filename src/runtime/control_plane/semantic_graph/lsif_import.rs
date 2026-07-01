//! LSIF → Oxigraph RDF named-graph import.
//!
//! Reads an LSIF JSONL file and loads a queryable RDF projection into an
//! Oxigraph named graph, using the same vocabulary
//! `views::populate_defs_refs`'s SPARQL queries expect (the real LSIF 0.6.0
//! spec namespace plus the `max:` position-property vocabulary), so that
//! imported data is actually reachable via `textDocument/definition` and
//! `textDocument/references` lookups:
//!
//! | LSIF entry                        | RDF triple(s)                                  |
//! |------------------------------------|------------------------------------------------|
//! | `document` vertex                  | `lsif:document` type; `max:uri`                |
//! | `moniker` vertex                   | `lsif:symbol` type; `lsif:identifier`/`lsif:kind` |
//! | `referenceResult` vertex           | `lsif:referenceOf` type                        |
//! | `range` vertex                     | `max:startLine`/`startCharacter`/`endLine`/`endCharacter` |
//! | `contains` edge (outV, inVs)       | `lsif:contains` per id in `inVs`               |
//! | `next` edge (outV, inV)            | `lsif:next`                                    |
//! | `item` edge (outV, inVs)           | `lsif:item` per id in `inVs`                   |
//! | `textDocument/definition` edge     | `lsif:textDocument_definition`                 |
//! | `textDocument/references` edge     | `lsif:textDocument_references`                 |
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
// RDF namespaces
// ---------------------------------------------------------------------------

/// Real LSIF 0.6.0 spec namespace — must match the `PREFIX lsif:` used in
/// `views::populate_defs_refs`'s SPARQL queries.
const NS: &str =
    "https://microsoft.github.io/language-server-protocol/specifications/lsif/0.6.0/specification/";

/// `max:` position/uri vocabulary — must match the `PREFIX max:` used in
/// `views::populate_defs_refs`'s SPARQL queries.
const MAX_NS: &str = "urn:lsp-max:core:";

fn ns(local: &str) -> NamedNode {
    NamedNode::new(format!("{NS}{local}")).expect("static LSIF namespace must be valid")
}

fn max_ns(local: &str) -> NamedNode {
    NamedNode::new(format!("{MAX_NS}{local}")).expect("static max: namespace must be valid")
}

fn vertex_node(id: u64) -> NamedNode {
    NamedNode::new(format!("urn:lsif:v/{id}")).expect("LSIF vertex URN must be valid")
}

fn u64_literal(n: u64) -> Term {
    // `views::helpers::term_to_u32` reads the literal's lexical value via
    // `Literal::value()` regardless of datatype, so a simple literal (same
    // pattern the rest of this file already uses) is sufficient here.
    Term::Literal(Literal::new_simple_literal(n.to_string()))
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
    pub range_triples: usize,
    pub edge_triples: usize,
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

    // Pre-build type/property predicates.
    let rdf_type = NamedNode::new("http://www.w3.org/1999/02/22-rdf-syntax-ns#type")
        .expect("rdf:type must be valid");
    let lsif_document = ns("document");
    let lsif_symbol = ns("symbol");
    let lsif_reference_of = ns("referenceOf");
    let lsif_identifier_pred = ns("identifier");
    let lsif_kind_pred = ns("kind");
    let max_uri_pred = max_ns("uri");
    let max_start_line = max_ns("startLine");
    let max_start_char = max_ns("startCharacter");
    let max_end_line = max_ns("endLine");
    let max_end_char = max_ns("endCharacter");
    let lsif_contains = ns("contains");
    let lsif_next = ns("next");
    let lsif_item = ns("item");
    let lsif_text_document_definition = ns("textDocument_definition");
    let lsif_text_document_references = ns("textDocument_references");

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

        let entry_type = match v.get("type").and_then(|t| t.as_str()) {
            Some(t) => t,
            None => continue,
        };

        let label = match v.get("label").and_then(|l| l.as_str()) {
            Some(l) => l,
            None => continue,
        };

        match entry_type {
            "vertex" => {
                let id = match v.get("id").and_then(|i| i.as_u64()) {
                    Some(id) => id,
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

                        // <vertex_node> max:uri "file:///..." — the property
                        // populate_defs_refs's SPARQL actually queries for.
                        insert_quad(
                            store,
                            subject,
                            max_uri_pred.clone(),
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

                    "range" => {
                        // <vertex_node> max:startLine/startCharacter/endLine/endCharacter — the
                        // exact properties populate_defs_refs's SPARQL projects for both the
                        // source and destination range of a definition/reference.
                        let start = v.get("start");
                        let end = v.get("end");
                        let start_line = start.and_then(|s| s.get("line")).and_then(|n| n.as_u64());
                        let start_char = start
                            .and_then(|s| s.get("character"))
                            .and_then(|n| n.as_u64());
                        let end_line = end.and_then(|e| e.get("line")).and_then(|n| n.as_u64());
                        let end_char = end
                            .and_then(|e| e.get("character"))
                            .and_then(|n| n.as_u64());

                        if let (Some(sl), Some(sc), Some(el), Some(ec)) =
                            (start_line, start_char, end_line, end_char)
                        {
                            insert_quad(
                                store,
                                subject.clone(),
                                max_start_line.clone(),
                                u64_literal(sl),
                                graph_ref,
                            );
                            insert_quad(
                                store,
                                subject.clone(),
                                max_start_char.clone(),
                                u64_literal(sc),
                                graph_ref,
                            );
                            insert_quad(
                                store,
                                subject.clone(),
                                max_end_line.clone(),
                                u64_literal(el),
                                graph_ref,
                            );
                            insert_quad(
                                store,
                                subject,
                                max_end_char.clone(),
                                u64_literal(ec),
                                graph_ref,
                            );
                            stats.range_triples += 4;
                        }
                    }

                    _ => {} // skip all other vertex labels
                }
            }

            "edge" => {
                let out_v = match v.get("outV").and_then(|i| i.as_u64()) {
                    Some(id) => id,
                    None => continue,
                };
                let out_node = Subject::NamedNode(vertex_node(out_v));

                match label {
                    "contains" => {
                        // <outV> lsif:contains <inV> for each id in inVs.
                        if let Some(in_vs) = v.get("inVs").and_then(|a| a.as_array()) {
                            for in_v in in_vs.iter().filter_map(|i| i.as_u64()) {
                                insert_quad(
                                    store,
                                    out_node.clone(),
                                    lsif_contains.clone(),
                                    Term::NamedNode(vertex_node(in_v)),
                                    graph_ref,
                                );
                                stats.edge_triples += 1;
                            }
                        }
                    }

                    "next" => {
                        // <outV> lsif:next <inV>
                        if let Some(in_v) = v.get("inV").and_then(|i| i.as_u64()) {
                            insert_quad(
                                store,
                                out_node,
                                lsif_next.clone(),
                                Term::NamedNode(vertex_node(in_v)),
                                graph_ref,
                            );
                            stats.edge_triples += 1;
                        }
                    }

                    "item" => {
                        // <outV> lsif:item <inV> for each id in inVs.
                        if let Some(in_vs) = v.get("inVs").and_then(|a| a.as_array()) {
                            for in_v in in_vs.iter().filter_map(|i| i.as_u64()) {
                                insert_quad(
                                    store,
                                    out_node.clone(),
                                    lsif_item.clone(),
                                    Term::NamedNode(vertex_node(in_v)),
                                    graph_ref,
                                );
                                stats.edge_triples += 1;
                            }
                        }
                    }

                    "textDocument/definition" => {
                        // <outV> lsif:textDocument_definition <inV> — note the
                        // underscore: the SPARQL prefix concatenation expects
                        // `lsif:textDocument_definition`, not a `/`-containing
                        // local name (which isn't valid in a SPARQL PN_LOCAL
                        // without escaping anyway).
                        if let Some(in_v) = v.get("inV").and_then(|i| i.as_u64()) {
                            insert_quad(
                                store,
                                out_node,
                                lsif_text_document_definition.clone(),
                                Term::NamedNode(vertex_node(in_v)),
                                graph_ref,
                            );
                            stats.edge_triples += 1;
                        }
                    }

                    "textDocument/references" => {
                        if let Some(in_v) = v.get("inV").and_then(|i| i.as_u64()) {
                            insert_quad(
                                store,
                                out_node,
                                lsif_text_document_references.clone(),
                                Term::NamedNode(vertex_node(in_v)),
                                graph_ref,
                            );
                            stats.edge_triples += 1;
                        }
                    }

                    _ => {} // skip all other edge labels
                }
            }

            _ => {} // skip anything that's neither vertex nor edge
        }
    }

    stats.total_triples = stats.document_triples
        + stats.moniker_triples
        + stats.reference_triples
        + stats.range_triples
        + stats.edge_triples;
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
        // CARGO_MANIFEST_DIR for the root lsp-max crate = /Users/sac/lsp-max
        // receipts/ lives directly under the workspace root.
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("receipts/v26.6.28-lsif.lsif")
    }

    const GRAPH: &str = "https://lsp-max.dev/graphs/v26.6.28";

    // ------------------------------------------------------------------
    // TRUE: an imported definition is actually reachable via
    // `views::lookup_definition` — the concrete proof that this importer's
    // vocabulary and `populate_defs_refs`'s SPARQL now agree. Before this
    // fix, this test could not pass anywhere in the codebase: the importer
    // dropped every edge and used a different RDF namespace than the SPARQL
    // queries expect.
    // ------------------------------------------------------------------
    #[test]
    fn imported_definition_is_reachable_via_lookup_definition() {
        use crate::runtime::control_plane::views::{
            lookup_definition, update_views, MaterializedViewStore,
        };
        use lsp_max_lsif::indexer_rust::index_rust_source;
        use lsp_max_lsif::lsif::ToolInfo;
        use lsp_max_lsif::lsif_builder::LsifBuilder;

        let source = "fn helper() {}\nfn run() { helper(); }\n";
        let uri = "file:///run.rs";

        let mut buf = Vec::new();
        {
            let mut builder = LsifBuilder::new(&mut buf);
            builder
                .emit_metadata(
                    "0.6.0",
                    "file:///w",
                    ToolInfo {
                        name: "lsp-max-lsif".to_string(),
                        version: None,
                        args: None,
                    },
                )
                .unwrap();
            index_rust_source(source, uri, &mut builder).unwrap();
        }

        let tmp =
            std::env::temp_dir().join(format!("lsif_import_test_{}.lsif", std::process::id()));
        std::fs::write(&tmp, &buf).unwrap();

        let store = Store::new().unwrap();
        let stats =
            import_lsif_into_graph(&store, &tmp, "https://lsp-max.dev/graphs/test-defs").unwrap();
        std::fs::remove_file(&tmp).ok();

        assert!(
            stats.range_triples > 0,
            "expected range vertices to be imported; got {stats:?}"
        );
        assert!(
            stats.edge_triples > 0,
            "expected edges (next/item/textDocument_definition/contains) to be imported; got {stats:?}"
        );

        let views = MaterializedViewStore::new();
        update_views(&store, &views);

        // The call site `helper()` on line 1 (0-indexed) starts right after
        // "fn run() { " — column 11 lands inside the `helper` identifier.
        let call_site_uri = url::Url::parse(uri).unwrap();
        let call_site_pos = lsp_types_max::Position {
            line: 1,
            character: 11,
        };

        let location = lookup_definition(&views, &call_site_uri, call_site_pos);
        assert!(
            location.is_some(),
            "expected a definition location for the call to `helper()`, got None"
        );
    }

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
