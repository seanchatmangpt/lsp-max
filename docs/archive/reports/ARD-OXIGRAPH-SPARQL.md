# ARD Addendum: Oxigraph v0.5.8 & SPARQL 1.2 Control Plane

## 1. Context and Objective
This document outlines the architectural blueprints for integrating `oxigraph` v0.5.8 as the Admitted Graph Control Plane for `lsp-max` v26.6.5. It translates the LSIF and LiveLSP states into formally defined RDF graphs and specifies the SPARQL queries required to enforce protocol invariants, provenance, and capability projection.

## 2. Vocabulary & Namespace Blueprint

To integrate LSIF and LiveLSP into the RDF ecosystem, we establish the following namespace prefixes mapping `lsp-max` constructs to standard public ontologies and bounded private namespaces.

### 2.1 Prefixes
```turtle
@prefix rdf:   <http://www.w3.org/1999/02/22-rdf-syntax-ns#> .
@prefix rdfs:  <http://www.w3.org/2000/01/rdf-schema#> .
@prefix xsd:   <http://www.w3.org/2001/XMLSchema#> .
@prefix prov:  <http://www.w3.org/ns/prov#> .
@prefix lsif:  <https://microsoft.github.io/language-server-protocol/lsif/0.6.0/> .
@prefix max:   <urn:lsp-max:core:> .
@prefix rcpt:  <urn:lsp-max:receipt:> .
@prefix proj:  <urn:project:local:> .
```

### 2.2 Core LSIF -> RDF Mapping Examples
Using `oxrdf` structures, a standard LSIF `contains` edge mapping:

**LSIF JSON:**
`{"id": 3, "type": "edge", "label": "contains", "outV": 1, "inVs": [2]}`

**RDF Quads (oxrdf):**
```rust
let graph_name = GraphName::NamedNode(NamedNode::new("urn:project:local:snapshot:1").unwrap());
// Subject: The container (outV)
let subject = NamedNode::new("urn:project:local:lsif:1").unwrap();
// Predicate: lsif:contains
let predicate = NamedNode::new("https://microsoft.github.io/language-server-protocol/lsif/0.6.0/contains").unwrap();
// Object: The contained item (inV)
let object = NamedNode::new("urn:project:local:lsif:2").unwrap();

let quad = Quad::new(subject, predicate, object, graph_name);
// store.insert(&quad);
```

### 2.3 Diagnostic Provenance Mapping
**LiveLSP Diagnostic:**
```turtle
proj:diag:123 a max:Diagnostic ;
    lsif:range proj:lsif:range:45 ;
    max:severity "error" ;
    max:conformsTo max:Law:Refusal ;
    prov:wasGeneratedBy rcpt:hash_xyz .

rcpt:hash_xyz a prov:Activity, max:Receipt ;
    prov:startedAtTime "2026-06-05T14:10:13Z"^^xsd:dateTime .
```

## 3. The Admission Boundary (Rust Abstraction)

The boundary between the hot-path Registry and the Oxigraph store is managed via an adapter trait:

```rust
use oxigraph::store::Store;
use oxrdf::{Quad, NamedNode, Subject, Term, GraphName};
use lsp_max_lsif::lsif::Element;

pub struct AdmittedGraph {
    /// The underlying Oxigraph RocksDB store.
    store: Store,
    /// The active snapshot ID defining the current GraphName.
    active_graph: GraphName,
}

impl AdmittedGraph {
    /// Opens the persistent RocksDB store.
    pub fn open(path: &str, snapshot_id: &str) -> Self {
        let store = Store::open(path).expect("Failed to open Oxigraph store");
        let active_graph = GraphName::NamedNode(
            NamedNode::new(format!("urn:project:local:snapshot:{}", snapshot_id)).unwrap()
        );
        Self { store, active_graph }
    }

    /// Admits an LSIF stream into the RDF store.
    pub fn admit_lsif(&self, element: &Element) {
        // Match on Vertex/Edge, construct Subject/Predicate/Object, and self.store.insert(&quad)
        // Implementation omitted for blueprint.
    }
}
```

## 4. SPARQL Projection Formulation

These queries will be executed via `oxigraph::sparql::SparqlEvaluator`.

### 4.1 Invariant: No Orphan LSIF Relations
Verifies that every edge has a destination vertex within the admitted graph.

```sparql
PREFIX lsif: <https://microsoft.github.io/language-server-protocol/lsif/0.6.0/>

ASK {
  # Find an edge where the object does not exist as a subject in the graph
  ?s ?p ?o .
  FILTER(STRSTARTS(STR(?p), STR(lsif:)))
  FILTER NOT EXISTS { ?o ?any_p ?any_o }
  # Optionally filter to ensure ?o is a NamedNode representing an LSIF ID
}
# Expected Result: false (Boolean)
```

### 4.2 Invariant: Diagnostic Provenance
Finds all diagnostics missing a cryptographic receipt.

```sparql
PREFIX max:  <urn:lsp-max:core:>
PREFIX prov: <http://www.w3.org/ns/prov#>

SELECT ?diagnostic WHERE {
  ?diagnostic a max:Diagnostic .
  FILTER NOT EXISTS {
    ?diagnostic prov:wasGeneratedBy ?receipt .
    ?receipt a max:Receipt .
  }
}
# Expected Result: 0 Solutions
```

### 4.3 Materialized View: Hover Resolution
While the primary LSP handles this natively, the SPARQL projection to cross-verify the materialized view:

```sparql
PREFIX lsif: <https://microsoft.github.io/language-server-protocol/lsif/0.6.0/>
PREFIX proj: <urn:project:local:>

SELECT ?contents WHERE {
  # Given a specific Range ID (e.g., 42)
  proj:lsif:42 lsif:next ?resultSet .
  ?resultSet lsif:textDocument_hover ?hoverResult .
  ?hoverResult lsif:contents ?contents .
}
```

## 5. Verification Artifacts (Test Infrastructure)

For CI and test pipelines, the `lsp-max` framework will leverage Oxigraph's in-memory mode to prevent RocksDB disk overhead:

```rust
#[cfg(test)]
mod tests {
    use oxigraph::store::Store;
    use oxigraph::sparql::SparqlEvaluator;
    
    #[test]
    fn verify_orphan_edge_invariant() {
        // 1. Create in-memory store
        let store = Store::new().unwrap();
        
        // 2. Insert valid quads simulating LSIF...
        
        // 3. Execute SPARQL ASK query
        let query = "ASK { ?s <http://example.com/edge> ?o . FILTER NOT EXISTS { ?o ?p ?o2 } }";
        let result = SparqlEvaluator::new()
            .parse_query(query).unwrap()
            .on_store(&store)
            .execute().unwrap();
            
        if let oxigraph::sparql::QueryResults::Boolean(has_orphans) = result {
            assert!(!has_orphans, "Graph contains orphan edges!");
        }
    }
}
```

## 6. Conclusion
The integration of Oxigraph v0.5.8 provides a mathematically robust, standards-compliant (SPARQL 1.1/1.2) control plane. By strictly separating the `AdmittedGraph` (RDF/SPARQL) from the hot-path `LsifReader` and LSP router, `lsp-max` achieves verifiable provenance and structural integrity without sacrificing interactive latency.