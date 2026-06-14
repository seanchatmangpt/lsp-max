# Product Requirements Document: lsp-max v26.6.5

- **Status**: ALIVE Candidate
- **Version**: v26.6.5
- **Tracked Milestone**: M-PRD-ARD-SPARQL
- **Release Owner**: Worker

---

## 1. Product Thesis

The core thesis of this release is the transition of language server intelligence from an ephemeral, session-bound dynamic query model (standard Language Server Protocol) into a mathematically queryable, formally verifiable, and cryptographically replayable RDF graph database. By implementing the **Oxigraph & SPARQL Admitted Graph Control Plane**, `lsp-max` enables verifiers, autonomous software agents, and compilation engines to query and validate codebase relationships, diagnostic reasoning, and tool executions using unassailable W3C semantic web standards (RDF, SPARQL, SHACL).

---

## 2. Customer Problem: The Crisis of Dynamic Epistemology

Interactive code intelligence today operates under a "dynamic epistemology." When a human developer requests a symbol definition, the Language Server Protocol (LSP) queries an active in-memory Abstract Syntax Tree (AST) to compute and return a dynamic response. This JIT model suffers from major limitations:
- **No Provenance**: There is no persistent record of why a particular definition, diagnostic warning, or hover payload was constructed.
- **Undeterministic Replay**: Codebases change constantly; without a temporal, snapshot-isolated graph, automated security auditors and compliance verifiers cannot verify if a previous compilation or audit was correct.
- **Agent Incompatibility**: Autonomous coding agents cannot query code recursively. They need structured graph query APIs (SPARQL) to reason about cross-module code paths without running heavy server processes.
- **Ontology Laundering**: Tools introduce custom metadata format extensions that masquerade as standard LSP structures, breaking tool interoperability.

---

## 3. Target Users & Value Delivery

The Admitted Graph Control Plane serves six key personas:

1. **LSP Framework Authors**: Validate LSP response correctness using automated SPARQL assertion queries before shipping.
2. **Static-Analysis Builders**: Query recursive code paths (e.g. inheritance hierarchies, symbol references) via declarative SPARQL instead of parsing raw code.
3. **Agent Infrastructure Builders**: Expose code discovery and refactoring tools as semantically bounded resources using the Model Context Protocol (MCP).
4. **Verifiers & Auditors**: Inspect cryptographically signed diagnostic execution receipts to certify that a compilation contains no undeclared vulnerabilities.
5. **Protocol Designers**: Project a single graph dataset to multiple endpoint protocols (JSON-RPC LSP, NDJSON LSIF, MCP, A2A).
6. **Future Coding Agents**: Directly query semantic code databases to obtain clean, structural contexts instead of dealing with flat text.

---

## 4. Core Requirements

This release implements seven core product requirements:

### PRD-R1 — Admitted RDF Graph State
The control plane must represent all workspace files, ranges, LSIF constructs (vertices and edges), diagnostic errors, diagnostic rules, queries, and cryptographic receipts as triples in the RDF dataset.
- *Rationale*: Establishes a single source of semantic truth.
- *Verification*: A query over the graph must successfully return the complete topology of the project.

### PRD-R2 — SPARQL Verification Queries
The system must support built-in SPARQL queries to verify system-level and codebase-level invariants (e.g., detecting disconnected reference edges or unreceipted diagnostics).
- *Rationale*: Guarantees semantic and structural validity.
- *Verification*: Invariant queries are compiled and run on the RocksDB store at transaction commit times.

### PRD-R3 — SHACL-Compatible Validation Posture
The architecture must bifurcate structural validation (shapes constraint checks) from logical projections (SPARQL queries).
- *Rationale*: Separation of concerns prevents performance degradation and keeps queries focused on business logic.
- *Verification*: Ingestion pipelines run SHACL shape validations to refuse structurally malformed triples.

### PRD-R4 — LSIF Remains the Static LSP Answer Format
RDF representation does not replace LSIF but wraps it. Static exports must still conform to LSIF 0.6.0.
- *Rationale*: Leverages the broad industry compatibility of LSIF.
- *Verification*: The system must export valid LSIF NDJSON from the admitted graph.

### PRD-R5 — Materialized Response Views
Forbid live SPARQL query evaluation on the LSP client hot path. The system must asynchronously project SPARQL results into fast, concurrent in-memory indexes (`DashMap`) for LSP definition/reference/hover queries.
- *Rationale*: Keeps user interaction latency under the `<5ms` threshold.
- *Verification*: Performance tests must confirm that definition lookups do not query RocksDB directly.

### PRD-R6 — Receipt Binding
Every admitted diagnostic or projection must generate a cryptographic receipt linking input graph hash + query hash to result hash.
- *Rationale*: Provides mathematical proof of deterministic compilation and analysis.
- *Verification*: Verification script checks that the `max:Receipt` structures are valid and matches original hashes.

### PRD-R7 — Protocol Projection Readiness
The RDF schema must support projections into LSP (JSON-RPC), LSIF (NDJSON), MCP (tools/resources), A2A (agent tasks), and OCEL (object-centric event logs).
- *Rationale*: Facilitates broad toolchain and agent ecosystem integration.
- *Verification*: Schema contains the necessary classes and predicates for all projection surfaces.

---

## 5. Non-Goals

The following areas are explicitly excluded from this release:
- Replacing LSIF or LSP with RDF as a wire protocol.
- Performing live, un-cached SPARQL evaluations on every human editor keystroke.
- Creating custom namespaces for terms that are already covered by standard ontologies (RDF, RDFS, PROV-O, SKOS, DCTERMS).
- Classifying this release as ALIVE if any cryptographic replay verification fails.
