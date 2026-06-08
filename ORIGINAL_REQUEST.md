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

## Follow-up — 2026-06-05T22:22:27Z

Implement a production-grade 'ALIVE' release candidate for tower-lsp-max v26.6.5, integrating the Oxigraph & SPARQL Admitted Graph Control Plane and completing the remaining planned milestones (M3–M7) for library modularization.

Working directory: /Users/sac/tower-lsp-max
Integrity mode: benchmark

## Requirements

### R1. Admitted RDF Graph State
Ingest workspace files, ranges, LSIF constructs (vertices/edges), LiveLSP diagnostics, and receipts into an embedded Oxigraph (v0.5.8) database as RDF triples. Enforce strict namespace mapping using standard prefixes (`rdf:`, `rdfs:`, `prov:`, `lsif:`) and bounded private prefixes (`max:`, `rcpt:`).

### R2. SPARQL Invariant & SHACL Shapes Gate
Enforce SHACL shape constraints on ingested triples to reject structurally malformed data. Run 5 core SPARQL validation queries (ASK/SELECT) at transaction commit to check for orphans, unregistered namespaces, unreceipted diagnostics, and lack of projections, blocking snapshot updates if any invariant is violated.

### R3. Materialized Views & Epoch Sync Barrier
Decouple live LSP definition, references, and hover requests from SPARQL execution by projecting query results asynchronously into in-memory `DashMap` structures to keep hot-path latencies below 5ms. To prevent race conditions, implement a Monotonic Epoch Sync Barrier that blocks strict-accuracy read requests (from agents/verifiers) when `committed_epoch > applied_epoch` until projection sync completes.

### R4. Cryptographic Receipt Functor
Ensure every admitted diagnostic or projection produces a BLAKE3 cryptographic receipt functor (`max:Receipt`) linking the input graph, query, and result hashes. Maintain functoriality ($\rho(g \circ f) = \rho(g) \circ \rho(f)$) and verify replay determinism by checking that replay query outputs match the receipt's expected result hash.

### R5. Protocol Projection Surface
Provide projection interfaces transforming the admitted graph and materialized views into standard JSON-RPC LSP, LSIF 0.6.0 NDJSON exports, Model Context Protocol (MCP) tool/resource lists, and Agent-to-Agent (A2A) task/agent capability cards.

### R6. Workspace Refactoring & Decoupling (M3–M7)
Refactor and modularize `tower-lsp-max-protocol/src/lib.rs`, `tower-lsp-max-runtime/src/lib.rs`, and the core server modules (`src/lib.rs`, `src/service.rs`, `src/service/client.rs`), splitting large inline files so that all primary source files are under 500 lines of code (LOC).

## Verification Resources
- Use the existing tests inside `tests/` directory (e.g., `test_rocksdb_admission.rs`, `test_materialized_views_integration.rs`, `test_challenger_m3_verification.rs`) as a reference verification harness.
- Add comprehensive integration and unit tests for the Oxigraph control plane, SPARQL invariants, materialized views, and BLAKE3 receipts.

## Acceptance Criteria

### Build & Quality Gates
- [ ] Workspace compiles cleanly on stable Rust channel.
- [ ] `cargo fmt --check` succeeds across all workspace crates.
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` passes without warning/error.
- [ ] All tests in `cargo test --workspace` pass (minimum of 400+ passing tests).

### Functional Correctness
- [ ] Structurally invalid or laundered triples are rejected at the ingestion boundary.
- [ ] 5 core SPARQL invariants are correctly checked at transaction commit.
- [ ] Interactive definition lookups serve from in-memory materialized views with a latency under 5ms.
- [ ] Monotonic Epoch Sync Barrier blocks reads under write contention when strict accuracy is requested.
- [ ] All diagnostics/projections contain valid BLAKE3 receipts, and independent verifier replays confirm 100% hash determinism.

### Code Style & Decoupling
- [ ] Target refactored source files are modularized and stay under 500 LOC.
- [ ] All existing comments and docstrings unrelated to code changes are preserved.

## Follow-up — 2026-06-05T23:13:04Z

Verify and complete the entire `tower-lsp-max` framework implementation using combinatorial maximalism. Resolve all compilation errors resulting from the recent LSIF struct changes, ensure absolute conformance with LSIF 0.6.0, LSP Base 0.9, and LSP 3.18 specifications, and implement rigorous verification via Oxigraph, SPARQL, and BLAKE3 receipts.

Working directory: /Users/sac/tower-lsp-max
Integrity mode: benchmark

## Requirements

### R1. Fix Compilation & Align Structs
Resolve all compilation errors across the workspace (particularly in `tower-lsp-max-runtime/src/control_plane/kernel.rs` and `wasm4pm_graduation.rs`) caused by adding the `project_root` field to `Vertex::MetaData` and updating the `kind` field to `Option<String>` in `Vertex::Project`. Ensure all test fixtures and database initializations construct these types correctly.

### R2. LSIF 0.6.0 & LSP 3.18 Combinatorial Maximalism
Complete all vertex and edge typings according to the LSIF 0.6.0 specification, including new LSIF elements such as `CallHierarchyResult`, `TypeHierarchyResult`, `textDocument/callHierarchy`, and `textDocument/typeHierarchy`. Ensure the `oxigraph` mapping and SPARQL queries correctly process and validate all combinations of these structures.

### R3. SPARQL Invariants & SHACL Shape Gates
Ensure the SHACL-style property validation gates reject any malformed metadata, documents, and invalid severity values or line/character properties. Verify that the 5 core SPARQL invariants (orphans, unregistered namespaces, unreceipted diagnostics, lack of projections, false alive) block state transitions upon commit.

### R4. Caching Materialized Views & Epoch Sync Barrier
Ensure all definition, reference, hover, call/type hierarchy, and diagnostic requests are served from DashMap materialized views with latencies under 5ms. Validate that the Monotonic Epoch Sync Barrier blocks reads during write contention when strict accuracy is requested.

### R5. Cryptographic Receipts & Replay Verification
Ensure every state transition, diagnostic, or projection produces a BLAKE3 Merkle-DAG receipt functor (`max:Receipt`) linking inputs, queries, laws, and outcomes. Verify that functoriality and query replay determinism are programmatically proven.

### R6. Code Quality & Modularity (M3-M7)
Verify that all workspace crates (`tower-lsp-max`, `tower-lsp-max-protocol`, `tower-lsp-max-runtime`, `tower-lsp-max-base`, etc.) are modularized, compile cleanly, have zero warning/clippy issues, and keep every primary source file strictly under 500 lines of code (LOC).

## Acceptance Criteria

### Build & Compilation
- [ ] The entire workspace compiles cleanly without any errors or warnings.
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` passes without issue.

### Verification & Correctness
- [ ] Add programmatic tests in the test suite that verify edge-case combinations of vertices and edges (including Call/Type hierarchies) and confirm they successfully round-trip and map to Oxigraph.
- [ ] 100% of the tests in `cargo test --workspace` pass successfully.

### Code Style & Decoupling
- [ ] Every source file is under 500 LOC (with unit tests extracted where necessary).
- [ ] Module-level and item-level docstrings are preserved.

## Follow-up — 2026-06-06T17:50:34Z

A production-grade verification audit on `tower-lsp-max` and `lsp-types-max` to enforce the 10 gates of the Anti-fake Law, ensuring every requirement R1-R7 has a corresponding bypass-kill proof, fresh-clone verification, and black-box protocol test.

Working directory: /Users/sac/tower-lsp-max
Integrity mode: benchmark

## Requirements

### R1. Fresh-Clone Receipts (Gate 1)
Clone both `tower-lsp-max` and `lsp-types-max` into fresh environments, run complete test suites and clippy checks, and record exact command outputs with the commit SHAs.

### R2. Bypass-Kill Matrix Verification (Gate 2, 4, 6, 7)
Perform deliberate sabotage/bypass tests on each of the core requirements (R1–R7) and components (e.g. `CapabilityTracker`, `method_strategy`, `DocumentVersionTracker`, `TransactionEditGate`, `SourceHealth`, `StaticGraphSource`). Prove that disabling or bypassing these components causes at least one test to fail.

### R3. Exhaustive Checks and Fraud Scan (Gate 5, 8)
Run a comprehensive fraud scan searching for stubs (`todo!`, `unimplemented!`), raw JSON fallbacks, or panics in the hot path, and justify all hits. Verify that all LSP method families map to explicit routing strategies.

### R4. Black-Box Protocol Transcript Tests (Gate 3)
Verify at least one behavior for every requirement (R1–R7) using JSON-RPC/LSP request/response transcript tests, bypassing direct internal function calls to prevent mock-only successes.

### R5. Complete Verification Report (Gate 9, 10)
Produce a report receipt matching the required final wording and report receipt format for all requirements, confirming the status of each.

## Acceptance Criteria

### Execution & Verification
- [ ] Fresh-clone tests and clippy pass with zero errors on both repositories.
- [ ] Sabotage/bypass proof is provided for R1-R7 (at least one failing test per bypass).
- [ ] Fraud scan output is compiled and every match is fully justified.
- [ ] Workspace edit paths are proven to fail if `TransactionEditGate` is bypassed.
- [ ] All 10 acceptance gates are satisfied and documented in the final report.
- [ ] Final receipt uses the exact format required by Gate 9 and final wording required by Gate 10.

## Follow-up — 2026-06-06T17:51:01Z

Hello! The user has added a new hard requirement to the audit scope: R8 — `playground/` Composite LSP Dogfood Harness. Please update your audit, implementation, and verification tasks to include R8.

Here are the requirements and acceptance criteria for R8:
- R8: `playground/` must launch a composite LSP using `tower-lsp-max` and operate against real `tower-lsp-max` / `lsp-types-max` source workspaces. It must compose at least two upstream sources (e.g. static graph + upstream/mock), exercise read-only and mutation LSP methods, show source attribution, show staleness outcomes, show TransactionEditGate behavior, demonstrate upstream failure degradation, and write transcript receipts into `playground/receipts/<timestamp>.jsonl`.
- Sabotage checks: Replacing composite router, capability tracker, doc version tracker, edit gate, static graph, or failure handling with a bypass/no-op must cause named tests or playground transcript checks to fail.
- Please update the final acceptance receipt format to include R8 and satisfy the acceptance criteria.

## Follow-up — 2026-06-06T18:22:22Z

The user has explicitly requested: 'launch 5 subagents to help fill the gaps by writing/updating code'. Please ensure that the orchestrator spawns the 5 specialized coding subagents to implement, edit, and verify the R8 dogfood harness, transaction gates, version tracks, and routing matrix.

## Follow-up — 2026-06-07T07:26:19Z

# Goal

Downgrade the claims of full LSP 3.18 maximalism to reflect Delta Changelog Coverage only. Implement the new `ANTI-LLM-LSP318-COMB-001` diagnostic checking for changelog-only laundering. Implement a spec extractor tool/canary that parses/extracts the complete LSP 3.18 protocol graph from the spec or metamodel to define and generate the combinatorial coverage matrix.

Working directory: `/Users/sac/tower-lsp-max/examples/anti-llm-lsp`
Project root: `/Users/sac/tower-lsp-max`
Integrity mode: development with receipt discipline

## Governing Statuses

```yaml
LSP318_CHANGELOG_DELTA_STATUS:
  status: REPORTED_15_OF_15_WITH_TRANSCRIPTS_AND_RECEIPTS
  scope: changelog_only

LSP318_SPEC_GRAPH_STATUS:
  status: BLOCKED_PENDING_SPEC_GRAPH_EXTRACTION
  scope: full_3_18_protocol_surface

LSP318_COMBINATORIAL_MAXIMALISM_STATUS:
  status: MATRIX_INCOMPLETE
  reason: "15 changelog rows do not cover the full LSP 3.18 request/capability/type/field/state-space graph."
```

## Requirements

### R1. Implement New Diagnostic `ANTI-LLM-LSP318-COMB-001`
- **Code:** `ANTI-LLM-LSP318-COMB-001`
- **Category:** `protocol_surface`
- **Message:** "15-row changelog matrix is being treated as full LSP 3.18 combinatorial coverage."
- **Forbidden Implication:** `ChangelogCoverage(15 rows) ⇒ SpecCoverage(LSP 3.18)`
- **Severity:** `ERROR`
- **Checkpoint Impacted:** `LSP318_COMBINATORIAL_MAXIMALISM`
- **Required Next Proof:**
  - Extract full LSP 3.18 spec graph.
  - Generate message/capability/type/field inventories.
  - Generate combinatorial coverage matrix.
  - Reclassify current 15 rows as changelog delta coverage only.
- **Receipt Required:** true

### R2. Spec Graph and Inventories Extraction
Extract the actual protocol graph from the LSP 3.18 specification/metamodel. The graph is modeled as `G_318 = (M, C, T, F, R, E, N, D)` containing messages, capabilities, protocol types, fields, result types, error paths, nullability/default constraints, dynamic registration and lifecycle constraints.

Generate the following output files under `examples/anti-llm-lsp/generated/` (create this directory if it doesn't exist):
- `examples/anti-llm-lsp/generated/lsp318_spec_graph.json`
- `examples/anti-llm-lsp/generated/lsp318_message_inventory.json`
- `examples/anti-llm-lsp/generated/lsp318_capability_inventory.json`
- `examples/anti-llm-lsp/generated/lsp318_type_field_inventory.json`
- `examples/anti-llm-lsp/generated/lsp318_combinatorial_matrix.json`
- `examples/anti-llm-lsp/generated/lsp318_gap_report.md`

### R3. Comprehensive Spec Inventories Details

#### 1. Message Inventory
Classify and output every LSP method (request, notification, response, server-to-client, client-to-server, both-direction, custom/extension).
Include: method name, direction, params type, result type, partial result type, workDoneProgress support, partialResult support, cancellation behavior, required capabilities, dynamic registration support, error behavior, negative controls, transcript path, receipt path, digest.

#### 2. Capability Inventory
Define every client/server capability path and registration option.
Include: capability_path, owner (client|server|registration), type, status, default behavior, introduced_since, related methods, positive transcript, negative transcript, receipt.

#### 3. Type and Field Inventory
Deconstruct protocol structures (e.g. `InitializeParams`, `ClientCapabilities`, `ServerCapabilities`, `TextDocumentSyncOptions`, `WorkspaceEdit`, `Diagnostic`, `CompletionList`, etc.).
Include: type_name, field_name, required|optional, nullable|non-null, data type, enum values, since/deprecated tags, default behavior, validation tests, negative controls.

#### 4. Lifecycle State Machine
Define and cover protocol state transitions: request before initialize, initialize twice, initialized before initialize, shutdown then request, exit before shutdown, cancel request, cancel unknown request, content modified, server/request cancelled, out-of-order responses, stale request handling.

#### 5. Dynamic Registration Matrix
For every dynamically registerable capability: supports dynamicRegistration, registerCapability accepted, unregisterCapability accepted, request works after registration, request refused after unregistration, bad registration refused, duplicate registration behavior, receipt.

#### 6. Error-path Matrix
Provide negative controls: invalid params, method not found, server not initialized, request cancelled, content modified, unsupported capability, malformed URI, bad range, missing required field, wrong nullability, wrong enum value.

#### 7. Base Transport and JSON-RPC Matrix
Validate: Content-Length correct, Content-Length missing, Content-Length wrong, Content-Type default, utf-8/utf8 compatibility, batch messages refused, invalid JSON refused, request-response/notification-no-response pairing, $/ request/notification handling.

---

## Acceptance Criteria

- [ ] `ANTI-LLM-LSP318-COMB-001` diagnostic is fully implemented in rules and evaluates correctly.
- [ ] Spec graph and inventories are successfully extracted and generated under `examples/anti-llm-lsp/generated/`.
- [ ] No stubs or hand-written summaries replace the generated `lsp318_spec_graph.json`, `lsp318_message_inventory.json`, `lsp318_capability_inventory.json`, `lsp318_type_field_inventory.json`, `lsp318_combinatorial_matrix.json`, and `lsp318_gap_report.md`.
- [ ] All 30 dogfood tests continue to pass and are reclassified as Delta Changelog Delta coverage proof.
- [ ] The final report implements the new two-level status framing correctly.




## Follow-up — 2026-06-07T08:09:14Z

# Goal

anti-llm-lsp must start using wasm4pm-compat for OCEL process evidence.

Working directory: `/Users/sac/tower-lsp-max/examples/anti-llm-lsp`
Project root: `/Users/sac/tower-lsp-max`
Integrity mode: development with receipt discipline

## Mission
anti-llm-lsp must become an OCEL evidence producer using wasm4pm-compat. It must not become a full wasm4pm mining/conformance engine.

## Boundary Law
- anti-llm-lsp observes and emits diagnostics.
- wasm4pm-compat supplies typed process-evidence / OCEL-compatible structures.
- full wasm4pm remains outside this checkpoint.
- no mining, replay, conformance checking, or process discovery is required here.
- do not launder diagnostics into process evidence without OCEL events.

## Forbidden Implication
DiagnosticEmitted ⇒ ProcessEvidenceRecorded

## Required Statuses

```yaml
OCEL_COMPAT_STATUS:
  status: BLOCKED_PENDING_COMPAT_EVENT_LOG
  scope: examples/anti-llm-lsp
```

Do not claim process-evidence completeness until OCEL events are emitted, exported, validated, and receipted.

---

## Required OCEL Evidence Model

Add typed OCEL event emission for the actual anti-LLM lifecycle.

### Event Types
`RepositoryScanned`, `FileObserved`, `RawTextObservationDetected`, `TreeSitterObservationDetected`, `CargoGraphObservationDetected`, `MarkdownClaimDetected`, `JsonRpcTranscriptParsed`, `ReceiptFileParsed`, `RouteEvidenceChecked`, `DiagnosticEmitted`, `ForbiddenImplicationDetected`, `NegativeControlExecuted`, `Lsp318FeatureExercised`, `Lsp318FeatureRefusedByLaw`, `VirtualDocumentServed`, `FailsetUpdated`, `ClaimStatusReported`, `ReceiptValidated`, `AuditReportScanned`.

### Object Types
`Repository`, `Crate`, `File`, `FileRange`, `Diagnostic`, `DiagnosticCode`, `ForbiddenImplication`, `Checkpoint`, `Receipt`, `Digest`, `JsonRpcTranscript`, `LspMethod`, `LspCapabilityPath`, `Lsp318FeatureRow`, `NegativeControlFixture`, `TestCase`, `Failset`, `AgentReport`, `RouteStage`, `OcelEventLog`.

### Core Relations
- `DiagnosticEmitted` -> `FileRange`, `DiagnosticCode`, `ForbiddenImplication`, `Checkpoint`
- `ReceiptValidated` -> `Receipt`, `Digest`, `Checkpoint`
- `Lsp318FeatureExercised` -> `Lsp318FeatureRow`, `JsonRpcTranscript`, `Receipt`
- `NegativeControlExecuted` -> `NegativeControlFixture`, `DiagnosticCode`, `Receipt`
- `FailsetUpdated` -> `Diagnostic`, `Checkpoint`, `AgentReport`

---

## Required Outputs

Add these files under `examples/anti-llm-lsp/ocel/` (create directory if it doesn't exist):
- `examples/anti-llm-lsp/ocel/anti_llm_lsp_ocel.json`
- `examples/anti-llm-lsp/ocel/anti_llm_lsp_ocel.receipt.json`
- `examples/anti-llm-lsp/ocel/ocel_event_inventory.json`
- `examples/anti-llm-lsp/ocel/ocel_object_inventory.json`
- `examples/anti-llm-lsp/ocel/ocel_relation_inventory.json`
- `examples/anti-llm-lsp/ocel/ocel_gap_report.md`

If JSON is used, it is a boundary/export format only.
Internal code should use typed structures through `wasm4pm-compat`, not untyped `serde_json::Value` authority.

---

## New Diagnostics

### ANTI-LLM-OCEL-001
- **Category:** `process_evidence`
- **Message:** "Diagnostic emitted without corresponding OCEL process event."
- **Forbidden Implication:** `DiagnosticEmitted ⇒ ProcessEvidenceRecorded`
- **Severity:** `ERROR`
- **Checkpoint Impacted:** `OCEL-COMPAT-001`
- **Receipt Required:** `true`

### ANTI-LLM-OCEL-002
- **Category:** `process_evidence`
- **Message:** "Receipt claim exists without OCEL object/event binding."
- **Forbidden Implication:** `ReceiptExists ⇒ ReceiptBoundToProcess`
- **Severity:** `ERROR`
- **Checkpoint Impacted:** `OCEL-COMPAT-001`
- **Receipt Required:** `true`

### ANTI-LLM-OCEL-003
- **Category:** `process_evidence`
- **Message:** "OCEL export produced without wasm4pm-compat typed boundary."
- **Forbidden Implication:** `JSONShape(OCEL) ⇒ CompatAdmittedOCEL`
- **Severity:** `ERROR`
- **Checkpoint Impacted:** `OCEL-COMPAT-001`
- **Receipt Required:** `true`

### ANTI-LLM-OCEL-004
- **Category:** `process_evidence`
- **Message:** "Full wasm4pm authority used where wasm4pm-compat boundary was required."
- **Forbidden Implication:** `CompatEvidenceBoundary ⇒ FullMiningAuthority`
- **Severity:** `ERROR`
- **Checkpoint Impacted:** `OCEL-COMPAT-001`
- **Receipt Required:** `true`

---

## Acceptance Criteria

- [ ] `wasm4pm-compat` dependency added.
- [ ] Full `wasm4pm` not introduced.
- [ ] Typed OCEL event/object model exists.
- [ ] Diagnostics, receipts, LSP318 feature rows, negative controls, and failset updates bind to OCEL events.
- [ ] OCEL export, receipt, inventories, and gap report are generated.
- [ ] Implement and pass these 10 tests:
  - `ocel_001_diagnostic_emits_compat_event`
  - `ocel_002_receipt_binds_to_ocel_object`
  - `ocel_003_lsp318_feature_row_binds_to_ocel_event`
  - `ocel_004_negative_control_binds_to_ocel_event`
  - `ocel_005_failset_update_binds_to_ocel_event`
  - `ocel_006_ocel_export_uses_wasm4pm_compat_boundary`
  - `ocel_007_rejects_json_shape_without_compat_boundary`
  - `ocel_008_rejects_full_wasm4pm_authority_in_compat_checkpoint`
  - `ocel_009_generates_ocel_gap_report`
  - `ocel_010_receipts_ocel_export_digest`
- [ ] Final report implements the new OCEL status schema.





