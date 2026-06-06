# Original User Request

## Initial Request — 2026-06-05T14:48:32-07:00

Write 8 comprehensive, fully realized PRD/ARD markdown files under `docs/v26.6.5/prd-ard/` in the tower-lsp-max repository, representing the Oxigraph/SPARQL admitted graph control plane.

Working directory: /Users/sac/tower-lsp-max
Integrity mode: benchmark

## Requirements

### R1. Grounding in Official Library Documentation & Specifications
Research and ground all technical definitions, API signatures, and schemas in the official specifications and documentation:
- **Oxigraph v0.5.8**: On-disk `oxigraph::store::Store` utilizing RocksDB, handling of `oxrdf::Quad`, and `SparqlEvaluator`.
- **SPARQL 1.1 & 1.2 (rdf-12)**: Standard syntax for graph pattern queries, `ASK`, `SELECT`, `CONSTRUCT`, path traversal, and filters.
- **LSIF 0.6.0**: Strict specification constraints for vertices, edges, item-edge properties, and ranges.
- **Model Context Protocol (MCP) (2025-06-18)**: Capabilities, tools, resources, and protocol structure.
- **Agent2Agent (A2A) (April 2025)**: Agent cards, JSON-RPC 2.0 communication, and task delegation.
- **W3C Standards**: SHACL, PROV-O, DCTERMS, DCAT, SKOS.
- **Base Protocol 0.9**: LSP base protocol experimental structures.

### R2. Core File Structure (8 Files Total)
Create exactly the following 8 files under `/Users/sac/tower-lsp-max/docs/v26.6.5/prd-ard/`:
1. `README.md`: Overview map, release classification, and navigation index for the PRD/ARD.
2. `prd.md`: Product requirements document covering thesis, customer problem, goals, target users, core user stories, requirements (PRD-R1 to PRD-R7), and non-goals.
3. `logical_architecture.md`: Detail of layers (Observation, Admission, RDF Store, SPARQL Query, SHACL Validation, Materialized Views, and Protocol Projection).
4. `ard_decisions.md`: Principles and five Architectural Decision Records (ARD-001 through ARD-005).
5. `data_model.md`: Data model boundary, public vocabulary preferences, bounded private vocabulary namespaces, and required graph object classes and relations.
6. `invariants.md`: Detailed definitions of Invariants 1-5, including syntactically valid SPARQL queries for invariant checks (orphan LSIF relations, unreceipted graph consequences, etc.).
7. `sequence_flows.md`: Complete sequence flows in Mermaid format for: Verification Flow, LSP Response Flow (Hot-path), and MCP/A2A Projection Flow.
8. `verification_and_gate.md`: Verification Ladder (Unit, Integration, E2E, Chaos, Stress, Benchmark, Verifier Report), Risk Register, and Release Gate criteria.

### R3. Universal Completeness & Quality
No placeholders, stubs, "TODO", "TBD", "unimplemented", "in a production environment", or deferred work are allowed in any file. All text must be fully written, professionally presented, and aligned with standard technical product/architecture requirements.

## Acceptance Criteria

### Documentation Coverage & File Check
- [ ] Exactly 8 files are generated under `/Users/sac/tower-lsp-max/docs/v26.6.5/prd-ard/` matching the names and topics in R2.
- [ ] The files contain no placeholders, `TODO`s, `TBD`s, or stub blocks.
- [ ] Artifact `docs/reports/SPECGEN-001-bootstrap-report.md` exists and contains the requested status and commands table.

## Follow-up — 2026-06-05T21:52:19Z

Implement the Oxigraph/SPARQL Admitted Graph Control Plane and Ostar Generative Pipeline integration in tower-lsp-max, as specified in the PRD/ARD documents inside `docs/reports/` and `docs/v26.6.5/prd-ard/`.

Working directory: /Users/sac/tower-lsp-max
Integrity mode: benchmark

## Requirements

### R1. Admitted RDF Graph State & Oxigraph Integration
- Implement the `RelationAdmitter` trait supporting states: `RAW`, `CANDIDATE`, `ADMITTED`, `REFUSED`, `QUARANTINED`, `SUPERSEDED`, `REPLAYED`.
- Support both in-memory `oxigraph::store::Store` (default) and persistent RocksDB-backed `Store` (via a configurable storage path).
- Successfully translate LSIF 0.6.0 elements (documents, ranges, vertices, edges, item properties) and diagnostic observations into `oxrdf::Quad` triples using standard vocabularies (LSIF, PROV-O, DCTERMS, etc.).

### R2. SPARQL Invariant Verification & Diagnostics
- Enforce the 5 Core Invariants:
  1. *No orphan LSIF relations*: Validate that all LSIF edge targets point to existing vertices using SPARQL `ASK`.
  2. *No unreceipted graph consequences*: Every diagnostic or protocol artifact must have a `prov:wasGeneratedBy` receipt link.
  3. *No hot-path SPARQL dependency*: Ensure interactive LSP query loops do not execute SPARQL queries directly.
  4. *No ontology laundering*: Private terms (`max:`) must not masquerade as public semantics.
  5. *No false ALIVE*: Valid status requires successful cryptographic replay verification.
- Capture and report structural errors as detailed `VerificationReport` diagnostics, refusing invalid fixtures.

### R3. Materialized View & LSP Routing
- Implement asynchronous materialized views (e.g. using `DashMap` or structured indexes) populated by background SPARQL queries.
- Serve standard LSP lookup requests (`textDocument/definition`, `textDocument/references`, `textDocument/hover`, and `textDocument/publishDiagnostics`) directly from these materialized views in `O(1)` time.

### R4. Cryptographic Receipt Chaining
- Implement a robust `CryptographicReceipt` structure in Rust (and a key management mechanism for Ed25519 signing) that records transition metadata: `prev_hash`, `discipline_id`, `law_id`, `consequence_hash`, and `sequence`.
- Compute and chain digests using BLAKE3 to build an immutable, chronological execution chain.

### R5. Deterministic Replay Engine
- Implement a query consequence replay verifier.
- Re-run transitions in isolation: initialize states from genesis parameters in the trace log, mock/stub system clocks and random seeds deterministically, and assert that recomputed state hashes match the signed receipts.

### R6. Ostar Typestate Kernel Integration
- Bind the codebase transitions to the generic `Machine<L, P, D>` container and compile-time checked `TypestateKernel` trait.
- Enforce linear consumption of states using Rust's affine ownership type system (`self` moves).

## Acceptance Criteria

### Compilation & Tests
- [ ] All code compiles cleanly under `cargo check` and contains no warnings under `cargo clippy`.
- [ ] `cargo test` passes 100% across the workspace, including new unit and integration tests for the admitted graph, SPARQL queries, materialized views, receipt chaining, and deterministic replay.
- [ ] Existing LSIF parser baseline tests remain green without regression.

### Graph Admission & Query Verification
- [ ] A sample LSIF fixture is successfully parsed, admitted into the `oxigraph::Store`, and validated against the 5 invariants.
- [ ] Malformed/invalid graph fixtures are successfully detected, quarantined, and refused with corresponding diagnostic explanations.

### Hot-Path Views & Replay
- [ ] LSP queries (e.g. Definition) resolve from the materialized views without calling the Oxigraph store.
- [ ] The replay engine successfully runs a verification against a generated receipt chain, producing a matching cryptographic digest and proving replay determinism.
- [ ] No stubs, placeholders, `TODO`s, or unimplemented sections remain in the active codebase.

### Technical Accuracy & Syntax
- [ ] Every Mermaid diagram in the files parses successfully (no syntax errors).
- [ ] Every SPARQL query provided in `invariants.md` is syntactically valid according to SPARQL 1.1/1.2.
### Objective Verification Mechanism
- [ ] A verification script `scratch/verify_prd_ard.py` is written and executed to validate link sanity, file presence, and absence of placeholders.
