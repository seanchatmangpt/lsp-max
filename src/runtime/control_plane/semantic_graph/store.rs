//! `OxigraphStore` — the ONLY place `oxigraph::*` appears outside tests.
//!
//! All other modules MUST interact with Oxigraph exclusively through the
//! `SemanticLawGraph` trait defined in `mod.rs`.
//!
//! # INVARIANT: OXIGRAPH_BOUNDARY_HELD
//! `OxigraphStore` is the single implementation of `SemanticLawGraph` that
//! touches `oxigraph::*`.  No other module in `src/` may import `oxigraph`
//! for semantic-graph purposes.

use oxigraph::model::Term;
use oxigraph::sparql::{QueryResults, SparqlEvaluator};
use oxigraph::store::Store;

use super::{SemanticLawGraph, snapshot::{GraphDigest, LawGraphSnapshot}};

/// Oxigraph-backed implementation of `SemanticLawGraph`.
///
/// Uses an in-memory Oxigraph `Store` for SPARQL query evaluation.
/// This struct is the sole owner of all oxigraph imports in production code.
pub struct OxigraphStore {
    store: Store,
}

impl OxigraphStore {
    /// Create a new empty in-memory store.
    pub fn new() -> Result<Self, String> {
        Ok(Self {
            store: Store::new().map_err(|e| e.to_string())?,
        })
    }
}

impl Default for OxigraphStore {
    fn default() -> Self {
        Self::new().expect("in-memory Oxigraph store must not fail")
    }
}

impl SemanticLawGraph for OxigraphStore {
    fn load_snapshot(&self, snapshot: LawGraphSnapshot) -> Result<GraphDigest, String> {
        let digest = snapshot.digest();
        self.store
            .load_from_reader(
                oxigraph::io::RdfFormat::NQuads,
                snapshot.nquads.as_bytes(),
            )
            .map_err(|e| e.to_string())?;
        Ok(digest)
    }

    fn query_invariants(&self, scope: &str) -> Result<Vec<String>, String> {
        let sparql = format!(
            r#"SELECT ?inv WHERE {{
                ?inv a <urn:lsp-max:andon:Invariant> ;
                     <urn:lsp-max:andon:scope> "{scope}" .
            }}"#
        );
        run_select_first_binding(&self.store, &sparql)
    }

    fn query_witnesses(&self, claim_id: &str) -> Result<Vec<String>, String> {
        let sparql = format!(
            r#"SELECT ?w WHERE {{
                <{claim_id}> <urn:lsp-max:andon:hasWitness> ?w .
            }}"#
        );
        run_select_first_binding(&self.store, &sparql)
    }

    fn query_repairs(&self, diagnostic_code: &str) -> Result<Vec<String>, String> {
        let sparql = format!(
            r#"SELECT ?repair WHERE {{
                ?repair a <urn:lsp-max:andon:Repair> ;
                        <urn:lsp-max:andon:forCode> "{diagnostic_code}" .
            }}"#
        );
        run_select_first_binding(&self.store, &sparql)
    }
}

// ---------------------------------------------------------------------------
// Private SPARQL helpers (oxigraph-aware; stay inside this module)
// ---------------------------------------------------------------------------

fn run_select_first_binding(store: &Store, sparql: &str) -> Result<Vec<String>, String> {
    let evaluator = SparqlEvaluator::new();
    let query = evaluator
        .parse_query(sparql)
        .map_err(|e| e.to_string())?;
    let results = query
        .on_store(store)
        .execute()
        .map_err(|e| e.to_string())?;

    match results {
        QueryResults::Solutions(sols) => {
            let mut out = Vec::new();
            for sol in sols {
                let sol = sol.map_err(|e| e.to_string())?;
                if let Some(term) = sol.get(0) {
                    out.push(term_to_string(term));
                }
            }
            Ok(out)
        }
        _ => Ok(Vec::new()),
    }
}

/// Convert an Oxigraph `Term` to a plain `String`.
/// RDF-star `Triple` terms are not supported and return an empty string.
fn term_to_string(term: &Term) -> String {
    match term {
        Term::NamedNode(n) => n.as_str().to_string(),
        Term::BlankNode(b) => b.as_str().to_string(),
        Term::Literal(l) => l.value().to_string(),
        _ => String::new(), // RDF-star Triple terms unsupported — OXIGRAPH_BOUNDARY_HELD
    }
}
