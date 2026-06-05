# PRD/ARD: tower-lsp-max v26.6.5 — Oxigraph/SPARQL Admitted Graph Control Plane

## 1. Executive Product Thesis & The 80/20 Combinatorial Mandate

**Release Classification:** `Status: PARTIAL_ALIVE → ALIVE candidate | Release: v26.6.5`

The integration of the Language Server Index Format (LSIF 0.6.0) into `tower-lsp-max` transformed the framework from a dynamic JIT analyzer to a static graph emitter. However, a static graph without a verifiable control plane is merely a cache. **tower-lsp-max v26.6.5 introduces an Oxigraph/SPARQL-backed admitted graph control plane.** 

By applying **80/20 Combinatorial Maximalism**, this release does not boil the ocean by rewriting the LSP hot-path in SPARQL. Instead, it targets the 20% architectural layer—the *Control Plane*—that unlocks 80% of the systemic value: cryptographic verifiability, provenance tracking, capability projection (MCP/A2A), and structural validation. 

**The Epistemological Shift:**
*   **LSIF** answers *what* the language server knows (Definitions, Hovers).
*   **Oxigraph/RDF** stores the *admitted state* of that knowledge.
*   **SPARQL** answers *why* that knowledge is lawful and *who/what* generated it.
*   **Receipts** prove the answers cryptographically.

---

## 2. Combinatorial Ontological Blueprint (The Data Boundary)

To prevent "ontology laundering," `tower-lsp-max` strictly maps its combinatorial state space into public RDF vocabularies, retreating to the `max:` namespace only for proprietary lifecycle/receipt mechanisms.

### 2.1 Namespace Matrix
| Prefix | URI | Combinatorial Scope |
| :--- | :--- | :--- |
| `rdf:` | `http://www.w3.org/1999/02/22-rdf-syntax-ns#` | Foundational graph types. |
| `rdfs:` | `http://www.w3.org/2000/01/rdf-schema#` | Hierarchy and relations. |
| `prov:` | `http://www.w3.org/ns/prov#` | Exhaustive provenance (`wasGeneratedBy`, `wasDerivedFrom`). |
| `sh:` | `http://www.w3.org/ns/shacl#` | Structural constraint shapes. |
| `lsif:` | `https://microsoft.github.io/language-server-protocol/specifications/lsif/0.6.0/specification/` | The 21 Vertices and 17 Edges of the LSIF 0.6.0 spec. |
| `max:` | `urn:tower-lsp-max:core:` | LiveLSP laws, diagnostics, autonomic mesh phases. |
| `rcpt:` | `urn:tower-lsp-max:receipt:` | BLAKE3/SHA256 receipt hashes. |
| `proj:` | `urn:project:local:` | The user's specific workspace footprint. |

### 2.2 Exhaustive Node/Edge Translation Map (LSIF -> RDF)
The `tower_lsp_max_lsif::lsif::Element` enum is translated into `oxrdf::Quad` structures. Every LSIF ID `N` becomes `proj:lsif:N`.

| LSIF Construct | LSIF Field | RDF Predicate (Edge) | RDF Object Type |
| :--- | :--- | :--- | :--- |
| `Vertex::Document` | `uri` | `lsif:uri` | `xsd:anyURI` |
| `Vertex::Range` | `start` | `lsif:start` | `xsd:string` (Serialized Position) |
| `Edge::Contains` | `inVs` | `lsif:contains` | `proj:lsif:{inV}` |
| `Edge::Item` | `property` | `lsif:itemProperty` | `lsif:ItemEdgeProperty` enum mapping |
| `Edge::TextDocumentHover`| `inV` | `lsif:textDocument_hover` | `lsif:HoverResult` |

### 2.3 Diagnostic & Provenance Combinatorics
Every diagnostic emitted by the framework must satisfy a combinatorial proof chain in the RDF store:

```turtle
proj:diag:123 a max:Diagnostic ;
    lsif:range proj:lsif:range:45 ;
    max:severity "error" ;
    max:conformsTo max:Law:Refusal ;
    prov:wasGeneratedBy rcpt:hash_xyz .

rcpt:hash_xyz a prov:Activity, max:Receipt ;
    prov:startedAtTime "2026-06-05T14:10:13Z"^^xsd:dateTime ;
    prov:wasAssociatedWith max:Agent:LSIF_Indexer .
```

---

## 3. SPARQL Combinatorial Invariant Verification

The Control Plane uses `oxigraph::sparql::SparqlEvaluator` to execute strict law verification. If any of these queries return violations, the graph state transitions to `REFUSED` and a validation report is generated.

### 3.1 Structural Invariant: The No-Orphan Law
Every LSIF relational edge must point to an admitted, instantiated vertex. 
*Combinatorial Coverage:* Validates `outV`, `inV`, and `inVs` across all 17 LSIF edge types.

```sparql
PREFIX lsif: <https://microsoft.github.io/language-server-protocol/specifications/lsif/0.6.0/specification/>

SELECT ?edge ?target WHERE {
  ?edge ?p ?target .
  # Match all LSIF topological properties
  FILTER(?p IN (lsif:contains, lsif:next, lsif:item, lsif:textDocument_hover, lsif:textDocument_definition))
  # Fail if the target does not exist as a subject
  FILTER NOT EXISTS { ?target a ?anyType }
}
```

### 3.2 Provenance Invariant: The No-Unreceipted-Consequence Law
Every diagnostic and capability projection MUST map to a cryptographic receipt.

```sparql
PREFIX max:  <urn:tower-lsp-max:core:>
PREFIX prov: <http://www.w3.org/ns/prov#>

SELECT ?artifact WHERE {
  { ?artifact a max:Diagnostic } UNION { ?artifact a max:CapabilityProjection }
  FILTER NOT EXISTS {
    ?artifact prov:wasGeneratedBy ?receipt .
    ?receipt a max:Receipt .
  }
}
```

### 3.3 Projection Readiness: The MCP/A2A Authority Query
Agents querying the graph for capabilities must receive proven, receipt-backed tools.

```sparql
PREFIX max: <urn:tower-lsp-max:core:>
PREFIX mcp: <urn:model-context-protocol:v1:>

SELECT ?tool ?authority ?receipt WHERE {
  ?tool a mcp:Tool ;
        max:requiresAuthority ?authority ;
        prov:wasGeneratedBy ?receipt .
}
```

---

## 4. Architectural Topography (C4 Topography)

The architecture deliberately isolates the synchronous LSP hot-path from the asynchronous RDF/SPARQL control plane.

### 4.1 System Topography
1. **Source / Live Observations:** Feeds `tower-lsp-max-live`.
2. **Admission Boundary:** Filters raw observations. Valid data enters the **Autonomic Registry**.
3. **Control Plane Fork:** 
   - **Hot Path:** The Registry updates **Materialized Views** (HashMaps in memory). The LSP Router serves user requests `O(1)` directly from these views.
   - **Control Path:** The Registry streams admitted states to the **`oxigraph::Store`**.
4. **SPARQL Engine:** Operates on the `oxigraph` store to generate `Validation Reports`, `Receipt Bundles`, and `MCP/A2A Capability Indexes`.

### 4.2 Rust Trait & Struct Boundaries
Implemented in `tower-lsp-max-lsif/src/control_plane.rs`:

```rust
use oxigraph::store::Store;
use oxigraph::sparql::{SparqlEvaluator, QueryResults};
use oxrdf::GraphName;

/// Represents the mathematically strict, receipt-bound Control Plane.
pub struct AdmittedGraph {
    store: Store,
}

impl AdmittedGraph {
    /// Instantiates an in-memory store for isolated CI tests, or RocksDB for production.
    pub fn new_in_memory() -> Self { ... }
    pub fn open_rocksdb(path: &std::path::Path) -> Self { ... }

    /// Transforms and admits LSIF vertices/edges into RDF Quads bounded by a Snapshot GraphName.
    pub fn admit_lsif_element(&self, element: &Element, snapshot_id: &GraphName) -> Result<(), max::Error> { ... }

    /// Executes the combinatorial maximalist SPARQL invariant suite.
    pub fn verify_invariants(&self, snapshot_id: &GraphName) -> Result<VerificationReport, max::Error> { ... }
}

pub struct VerificationReport {
    pub is_valid: bool,
    pub orphan_edges: usize,
    pub unreceipted_artifacts: usize,
    pub report_hash: String, // BLAKE3 hash of the report itself.
}
```

---

## 5. Implementation Trajectory (Execution Strategy)

To achieve the 80/20 impact:
1. **Dependency Injection:** Add `oxigraph = "0.3"` to `crates/tower-lsp-max-lsif`.
2. **RDF Mapping Layer:** Implement `Element -> Quad` translators covering the 80% most critical LSIF elements (`Document`, `Range`, `HoverResult`, `DefinitionResult`, `Contains`, `Item`, `Next`).
3. **SPARQL Test Suite:** Implement the `test_sparql_invariants.rs` suite that boots an in-memory `oxigraph::Store`, populates it with a synthetic LSIF graph, and runs the Provenance and Orphan queries.
4. **Receipt Binding:** Ensure the output of `verify_invariants` generates a `Receipt` object that can be queried in future lifecycles.

---

## 6. Release Gate: `ALIVE` Acceptance Criteria

To transition v26.6.5 from `PARTIAL_ALIVE` to `ALIVE`, the following combinatorial proof must pass automatically in the CI suite:

1. **Graph Admission:** A fixture LSIF workspace (minimum 1 Document, 1 Range, 1 Hover, 1 Definition) successfully translates into `oxrdf::Quad`s and is inserted into an `oxigraph::Store`.
2. **Invariant Query Execution:** The SPARQL Structural Invariant query executes and correctly identifies a maliciously injected orphan edge (Negative Test) and returns cleanly on the intact graph (Positive Test).
3. **Provenance Query Execution:** The SPARQL Provenance query successfully identifies a diagnostic injected without a `prov:wasGeneratedBy` receipt link.
4. **Hot-Path Isolation:** The execution of the `AdmittedGraph` does not block, lock, or regress the existing `LanguageServer` `max_lsif` materialized stream export.
5. **Replay Determinism:** Running the exact same SPARQL query against the exact same GraphName yields identical BLAKE3 hashes for the `VerificationReport`.