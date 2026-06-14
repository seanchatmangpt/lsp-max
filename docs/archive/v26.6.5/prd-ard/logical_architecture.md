# Logical Architecture: lsp-max v26.6.5

This document details the multi-layered pipeline, data flow interfaces, and boundaries of the Oxigraph v0.5.8 control plane in `lsp-max`.

---

## 1. Architectural Layers Blueprint

The control plane is organized into seven distinct layers, moving data from raw workspace changes into persistent storage and out to low-latency protocol projection surfaces.

```
+------------------------------------------------------------+
|                  7. Protocol Projection                    |
|             (LSP JSON-RPC, LSIF, MCP, A2A, OCEL)           |
+------------------------------------------------------------+
                             ^
                             |
+------------------------------------------------------------+
|                  6. Materialized Views                     |
|                 (DashMap In-Memory Index)                  |
+------------------------------------------------------------+
                             ^
                             | (Asynchronous Projections)
+------------------------------------------------------------+
|                  5. SHACL Validation Layer                 |
|                   (W3C Shapes Graph Check)                 |
+------------------------------------------------------------+
                             ^
                             |
+------------------------------------------------------------+
|                  4. SPARQL Query Layer                     |
|                (Invariant & Projection Engine)             |
+------------------------------------------------------------+
                             ^
                             |
+------------------------------------------------------------+
|                 3. RDF Graph Store Layer                   |
|              (RocksDB-backed oxigraph::Store)              |
+------------------------------------------------------------+
                             ^
                             | (Transactional Commits)
+------------------------------------------------------------+
|                    2. Admission Layer                      |
|             (RelationAdmitter & Lifecycle State)           |
+------------------------------------------------------------+
                             ^
                             |
+------------------------------------------------------------+
|                   1. Observation Layer                     |
|          (SourceObservation & ObservationReader)           |
+------------------------------------------------------------+
```

---

## 2. Layer Definitions & Interfaces

### A. Observation Layer
- **Role**: Collects workspace events, file changes, compilation outputs, LSIF outputs, diagnostics, and receipt notifications.
- **Interfaces**:
  - `SourceObservation`: Enum representing the kind of incoming observation (e.g. `FileChanged`, `LsifGenerated`, `DiagnosticEmitted`).
  - `ObservationReader`: Trait defining how the control plane streams observations from workspace watchers or LSP client notifications.
- **Output**: A raw stream of candidate observations and facts.

### B. Admission Layer
- **Role**: Parses and validates raw facts before committing them to the database. It enforces the namespace boundaries and manages lifecycle transitions.
- **Interfaces**:
  - `RelationAdmitter`: Trait checking syntactic correctness and namespace validity of incoming triples/`oxrdf::Quad`s.
- **Lifecycle States**:
  - `RAW`: Raw, unparsed input stream data.
  - `CANDIDATE`: Parsed triple/`oxrdf::Quad` awaiting structural validation.
  - `ADMITTED`: Validated fact successfully inserted into the `Store`.
  - `REFUSED`: Fact rejected due to schema or security constraint violation.
  - `QUARANTINED`: Fact suspended due to unresolved dependencies.
  - `SUPERSEDED`: Fact replaced by a newer code change version.
  - `REPLAYED`: Fact matched a previously recorded execution receipt.

### C. RDF Graph Store Layer
- **Role**: Handles persistent, transactionally safe storage of admitted triples/`oxrdf::Quad`s.
- **Implementation**: RocksDB-backed `oxigraph::store::Store`.
- **Constructs**: 
  - Graph snapshots are isolated using `GraphName::NamedNode` containing the snapshot identifier IRI.
  - Multi-threaded read access is supported natively by Oxigraph.

### D. SPARQL Query Layer
- **Role**: Evaluates structural, semantic, and provenance query patterns.
- **Implementation**: Evaluated via `store.query()`.
- **Query Types**:
  - Invariant Checks: ASK queries verifying graph integrity.
  - Diagnostic Explanation: SELECT queries explaining why a diagnostic was triggered.
  - View Projection: CONSTRUCT queries mapping raw triples to user-facing protocol representations.
  - Replay Confirmation: Validating the output of a query against its cryptographic receipt.

### E. SHACL Validation Layer
- **Role**: Validates data shape constraints (e.g., property range datatype, minimum property counts, valid node references) before indexing, without the overhead of complex SPARQL paths.
- **Implementation**: Matches the RDF store against standard SHACL shape graphs (`sh:NodeShape`, `sh:PropertyShape`).

### F. Materialized Views Layer
- **Role**: Aggregates SPARQL projection results into fast Rust data structures to avoid RocksDB queries on the hot path.
- **Implementation**: Concurrent hash maps (`DashMap` tables) holding pre-computed definition, reference, and hover lookups.
- **Boundary**: Ensures LSP router remains completely isolated from Oxigraph RocksDB query latency.

### G. Protocol Projection Layer
- **Role**: Adapts the in-memory materialized views to standard output protocols.
- **Output Interfaces**:
  - **LSP JSON-RPC**: Serves standard definition/hover requests under `<5ms`.
  - **LSIF NDJSON**: Exports static code intelligence graphs.
  - **Model Context Protocol (MCP)**: Exposes workspace resources and tools to AI models.
  - **Agent-to-Agent (A2A)**: Provides agent capabilities, policy cards, and task delegation tracking.
