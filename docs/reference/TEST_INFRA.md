# Composed Language Server Test Infrastructure

This document outlines the testing philosophy, feature inventory, test architecture, real-world application scenarios, and coverage thresholds for the end-to-end (E2E) verification of the `lsp-max` substrate.

---

## 1. Test Philosophy

Our testing approach ensures correctness, stability, and protocol conformance of the composed LSP server using several complementary testing methodologies:

1. **Opaque-Box Testing**: 
   The test suite interacts with the composed server strictly through the standard LSP JSON-RPC 2.0 interface (stdin/stdout or channel-based streams). The tests do not inspect the server's internal memory, structs, or database states directly, ensuring that the system is validated exactly as an editor or external client would interact with it.

2. **Requirement-Driven Verification**: 
   Every test case is directly mapped to a specific feature requirement (F1 through F7) derived from the project specifications. This ensures that the test suite comprehensively validates the defined scope and guarantees that no requirement remains untested.

3. **Category-Partition Testing**: 
   The input and state space of the composed server are partitioned into logical equivalence classes. For example, methods are categorized into Lifecycle, Document Sync, Queries (first-success/merge), and Mutations (single-owner/transactional). Upstream sources are categorized by state (active, slow, degraded, crashed, static graph). Tests are designed to cover representative cases from each partition.

4. **Boundary Value Analysis (BVA)**: 
   Tests target critical boundaries where errors are most likely to occur, including:
   - Document version boundaries (matching vs. stale snapshot versions).
   - Character and offset limits (empty ranges, UTF-16 multi-byte surrogate boundaries).
   - Time boundaries (queries returning just within the timeout window vs. exactly at or after the timeout limit).
   - Transactional edit overlap boundaries (adjacent non-overlapping edits vs. single-character overlap).

5. **Pairwise Testing (Interaction Testing)**: 
   Rather than testing features only in isolation, we systematically test combinations of features. Pairwise testing ensures that cross-feature interactions—such as a slow upstream server (F6) delivering a dynamic registration (F2) during a pending transaction (F5)—are handled safely and deterministically.

6. **Workload and Stress Testing**: 
   The system is subjected to high-frequency and concurrent workloads to verify thread safety and lock resilience. This includes simultaneous queries from multiple clients, back-to-back document changes, and large static graphs to ensure the Monotonic Epoch Sync Barrier and materialized views sustain low-latency responses.

---

## 2. Feature Inventory

This inventory maps features **F1 to F7** to their verbatim requirements from the project specification, detailing the test cases for each tier.

### Feature 1: Lifecycle (F1)
* **Verbatim Requirement**: 
  > "The type layer must cleanly represent the full lifecycle, capability discovery, request, response, notification, edit, diagnostic, and graph vocabulary specified in the protocol (initialize/initialized/shutdown/exit, client/server capabilities, dynamic registration, diagnostic, workspace/text edits, hover, completion, rename, formatting, semantic tokens, and static graph/LSIF elements). It must avoid untyped JSON escape hatches for normal LSP composition paths."

* **Tier 1: Feature Coverage (5 Cases)**
  1. `test_f1_t1_initialize_handshake`: Verifies client-server initialize handshake sends typed `InitializeParams` and receives typed `InitializeResult`.
  2. `test_f1_t1_initialized_notification`: Verifies `initialized` notification transition occurs without errors.
  3. `test_f1_t1_shutdown_request`: Verifies `shutdown` request receives a null response and transitions server to shutdown state.
  4. `test_f1_t1_exit_notification`: Verifies `exit` notification terminates the server process cleanly with exit code 0 or 1 depending on whether shutdown was called.
  5. `test_f1_t1_reject_requests_before_initialize`: Verifies requests sent before `initialize` are rejected with `InvalidRequest` or `ServerNotInitialized`.

* **Tier 2: Boundary & Corner Cases (5 Cases)**
  1. `test_f1_t2_duplicate_initialize`: Verifies that sending a second `initialize` request after initialization fails with an LSP error code `-32600` (InvalidRequest).
  2. `test_f1_t2_exit_without_shutdown`: Verifies that sending `exit` without preceding `shutdown` causes the server process to terminate with exit code 1.
  3. `test_f1_t2_malformed_initialize_payload`: Verifies that sending malformed JSON-RPC in `initialize` returns `ParseError` or `InvalidRequest` rather than crashing the type parser.
  4. `test_f1_t2_shutdown_followed_by_request`: Verifies that any request (except `exit`) sent after `shutdown` returns `InvalidRequest`.
  5. `test_f1_t2_initialize_with_unregistered_client_capabilities`: Verifies that the server gracefully ignores or safely handles unknown client capabilities in `InitializeParams` without crash or serialization failure.

---

### Feature 2: Capability Discovery (F2)
* **Verbatim Requirement**: 
  > "The runtime must maintain a structured, live capability view (client-supported, local server, peer/upstream, dynamic registration/unregistration, and effective support). The downstream advertised support must be dynamically derived from the intersection of observed capability, declared routing intent, and safety constraints rather than a simple raw union."

* **Tier 1: Feature Coverage (5 Cases)**
  1. `test_f2_t1_initial_advertisement`: Verifies client is advertised the correct intersection capabilities on initialization.
  2. `test_f2_t1_dynamic_registration`: Verifies that when an upstream dynamically registers a capability, the composed server propagates this registration to the client.
  3. `test_f2_t1_dynamic_unregistration`: Verifies upstream dynamic unregistration is propagated.
  4. `test_f2_t1_intersection_rules`: Verifies that a capability is only advertised if both the client and at least one upstream source support it.
  5. `test_f2_t1_safety_constraint_gating`: Verifies that dangerous capabilities (e.g. uncontrolled workspace edits) are filtered/gated based on server security policy before advertisement.

* **Tier 2: Boundary & Corner Cases (5 Cases)**
  1. `test_f2_t2_dynamic_registration_duplicate`: Verifies that registering the same capability ID multiple times from the same/different upstream sources handles gracefully without duplicate dynamic registration messages sent to the client.
  2. `test_f2_t2_dynamic_registration_missing_id`: Verifies that registering a dynamic capability without a unique ID is rejected at the composed boundary.
  3. `test_f2_t2_intersection_with_unsupported_upstream`: Verifies that if one upstream supports a capability but another doesn't, queries for that capability are routed only to the supporting upstream, and advertised capability reflects partial support.
  4. `test_f2_t2_advertisement_client_incapable`: Verifies that if an upstream server supports `textDocument/inlineCompletion` but the client explicitly disables it in `InitializeParams`, the capability is not advertised or registered dynamically.
  5. `test_f2_t2_dynamic_unregistration_nonexistent`: Verifies that attempting to unregister a dynamic capability that was never registered does not cause state corruption or errors.

---

### Feature 3: Method Routing & Composition Matrix (F3)
* **Verbatim Requirement**: 
  > "Every LSP method family must have a defined, safe composition or routing strategy (e.g., lifecycle controlled path, document sync ordered fanout, diagnostics merge with attribution, hover first-success or merge, references/definitions location merge and deduplication, completion ranked providers, formatting/rename single owner plus edit guard, workspace edits guarded transactions, etc.)."

* **Tier 1: Feature Coverage (5 Cases)**
  1. `test_f3_t1_did_change_fanout`: Verifies `textDocument/didChange` is sequentially fanned out to all upstream servers in document sync order.
  2. `test_f3_t1_definition_merge`: Verifies `textDocument/definition` results from multiple upstreams are merged and deduplicated.
  3. `test_f3_t1_completion_ranking`: Verifies `textDocument/completion` responses are ranked/sorted based on provider relevance and quality scores.
  4. `test_f3_t1_hover_first_success`: Verifies `textDocument/hover` returns the first successful, non-empty response among parallel upstream queries.
  5. `test_f3_t1_rename_single_owner`: Verifies `textDocument/rename` is routed to the single designated owner source for the target document range.

* **Tier 2: Boundary & Corner Cases (5 Cases)**
  1. `test_f3_t2_no_owners_available`: Verifies that routing a single-owner request (e.g. `textDocument/formatting`) when no upstream claims ownership returns an empty response/error rather than failing.
  2. `test_f3_t2_partial_failures_in_merge`: Verifies that when merging definitions, if one upstream fails/times out, other successful results are still returned.
  3. `test_f3_t2_conflicting_completion_items`: Verifies that duplicate completion items from multiple sources are merged by prioritizing the higher ranked provider's details.
  4. `test_f3_t2_hover_merge_all`: Verifies that when hover merge strategy is configured, markdown hover blocks from multiple upstreams are cleanly concatenated with clear divider lines.
  5. `test_f3_t2_invalid_method`: Verifies that routing an unsupported/invalid method returns `MethodNotFound` to the client.

---

### Feature 4: Source Attribution & Document Causality (F4)
* **Verbatim Requirement**: 
  > "Merged/composed observations (diagnostics, hover fragments, references, etc.) must preserve source identity. Document-changing traffic must follow causal version discipline (didOpen, didChange version increments, didClose, and range-bearing results tied to specific snapshots with stale responses dropped/refused). Range/position mapping must be snapshot-bound and encoding-safe (UTF-16, offsets, line/col)."

* **Tier 1: Feature Coverage (5 Cases)**
  1. `test_f4_t1_attribution_diagnostics`: Verifies diagnostic list includes clear source attribution tags matching the upstream server IDs.
  2. `test_f4_t1_attribution_hover_fragments`: Verifies hover response fragments identify their originating sources in the markdown headers.
  3. `test_f4_t1_document_version_increment`: Verifies `didChange` notification increments the internal snapshot version.
  4. `test_f4_t1_drop_stale_response`: Verifies that responses received from an upstream with a version older than the current snapshot are dropped.
  5. `test_f4_t1_encoding_utf16`: Verifies character offset conversion logic is encoding-safe (converting between UTF-8 byte offsets and UTF-16 code units).

* **Tier 2: Boundary & Corner Cases (5 Cases)**
  1. `test_f4_t2_diagnostics_clear`: Verifies that when an upstream server clears its diagnostics, only that upstream's diagnostics are cleared from the client's view.
  2. `test_f4_t2_out_of_order_did_change`: Verifies that if `didChange` notifications arrive out-of-order, the server refuses/corrects them to preserve causal consistency.
  3. `test_f4_t2_invalid_utf16_surrogate`: Verifies that position mapping handles invalid/broken UTF-16 surrogate pairs gracefully without crashing or returning out-of-bounds positions.
  4. `test_f4_t2_empty_did_change`: Verifies that a `didChange` message with an empty change array does not trigger a version bump but preserves the existing snapshot.
  5. `test_f4_t2_stale_request_rejection`: Verifies that if the client sends a query (e.g., `textDocument/definition`) with an outdated version/snapshot ID, the server rejects it.

---

### Feature 5: Guarded Mutations (F5)
* **Verbatim Requirement**: 
  > "All mutating provider outputs (formatting, rename, workspace edits, code actions) must pass through a transactional edit gate (accepted, refused, stale, needs rebase, overlaps, user choice required). Overlapping edits from different sources must be refused or require explicit reconciliation rather than being blindly merged."

* **Tier 1: Feature Coverage (5 Cases)**
  1. `test_f5_t1_edit_gate_accept`: Verifies a non-overlapping, valid edit proposal is accepted and applied.
  2. `test_f5_t1_edit_gate_refuse`: Verifies an invalid or malformed edit proposal is refused.
  3. `test_f5_t1_edit_gate_stale`: Verifies an edit proposed on a stale document version is rejected as stale.
  4. `test_f5_t1_overlapping_edits`: Verifies overlapping edits from different sources targeting the same range are blocked.
  5. `test_f5_t1_needs_rebase`: Verifies that edits that can be merged with a rebase are detected, and rebase suggestions are offered or applied.

* **Tier 2: Boundary & Corner Cases (5 Cases)**
  1. `test_f5_t2_empty_edit_list`: Verifies that empty edit arrays are accepted but perform no operations.
  2. `test_f5_t2_adjacent_non_overlapping`: Verifies that edits that are directly adjacent (line 1, col 5 and line 1, col 6) are accepted since they do not overlap.
  3. `test_f5_t2_rebase_failure`: Verifies that when a rebase is impossible due to conflicting changes, the transaction is rejected.
  4. `test_f5_t2_transactional_rollback`: Verifies that if one edit in a multi-file workspace edit fails validation, the entire transaction is rolled back.
  5. `test_f5_t2_user_choice_reconciliation`: Verifies that when conflicting edits occur, the server prompts for client/user choice and processes the response.

---

### Feature 6: Failure Isolation and Degradation (F6)
* **Verbatim Requirement**: 
  > "One broken, crashing, slow, or invalid upstream source must not poison the composed endpoint. The runtime must degrade affected sources, isolate crashes, return partial results on timeout if allowed, and reject unsafe edit proposals while keeping the composed downstream endpoint protocol-correct."

* **Tier 1: Feature Coverage (5 Cases)**
  1. `test_f6_t1_upstream_crash_isolation`: Verifies that if an upstream server crashes, the composed server isolates the crash and continues serving queries from other sources.
  2. `test_f6_t1_upstream_timeout_partial_results`: Verifies that slow upstreams trigger a timeout, returning partial results from healthy upstreams.
  3. `test_f6_t1_upstream_invalid_json_rpc`: Verifies that malformed JSON-RPC returned by an upstream is isolated and does not crash the parser or server.
  4. `test_f6_t1_source_degradation`: Verifies that an upstream showing persistent failures is marked as degraded and queries to it are paused/stopped.
  5. `test_f6_t1_reject_unsafe_edit`: Verifies that a crashing upstream's unsafe edit proposal is rejected.

* **Tier 2: Boundary & Corner Cases (5 Cases)**
  1. `test_f6_t2_upstream_infinite_loop`: Verifies that if an upstream hangs in an infinite loop, the timeout boundary isolates the query within the configured time limit.
  2. `test_f6_t2_recovery_from_degradation`: Verifies that a degraded source that becomes responsive again is successfully restored to active status.
  3. `test_f6_t2_all_upstreams_dead`: Verifies that if all upstreams crash/fail, the composed server remains alive and returns empty results or clean protocol errors.
  4. `test_f6_t2_partial_timeout_no_results`: Verifies that if a query allows no partial results on timeout, the server returns a timeout error rather than incomplete data.
  5. `test_f6_t2_channel_disconnect_handling`: Verifies that if the communication channel (stdin/stdout pipe) to an upstream breaks unexpectedly, the server cleans up resources and marks the source as dead.

---

### Feature 7: Static Graph Integration (F7)
* **Verbatim Requirement**: 
  > "Static graph data must participate as a first-class protocol source for definitions, references, hover, etc., conforming to the same routing, attribution, and reconciliation policies as dynamic sources."

* **Tier 1: Feature Coverage (5 Cases)**
  1. `test_f7_t1_static_graph_definitions`: Verifies definitions can be queried from the static LSIF graph database.
  2. `test_f7_t1_static_graph_references`: Verifies references can be queried from the static LSIF graph database.
  3. `test_f7_t1_static_graph_hover`: Verifies hover documentation is resolved from static graph nodes.
  4. `test_f7_t1_static_graph_attribution`: Verifies results returned from the static graph have the correct attribution tag (`static-graph`).
  5. `test_f7_t1_static_graph_routing`: Verifies static graph queries are run in parallel alongside dynamic queries and results are merged.

* **Tier 2: Boundary & Corner Cases (5 Cases)**
  1. `test_f7_t2_static_graph_corrupt_db`: Verifies that a corrupt or unreadable static graph database is handled gracefully, logging an error and fallback to dynamic sources.
  2. `test_f7_t2_static_graph_out_of_bounds_query`: Verifies that querying positions outside the boundaries of files described in the static graph returns no results.
  3. `test_f7_t2_static_graph_unsupported_methods`: Verifies that sending mutating requests (e.g., `textDocument/rename`) to the static graph source is rejected or ignored.
  4. `test_f7_t2_static_graph_stale_data_rejection`: Verifies that if the dynamic document version is significantly ahead of the static graph, static graph results are rejected or marked as low confidence.
  5. `test_f7_t2_static_graph_duplicate_edges`: Verifies that the LSIF graph parser handles duplicate vertices or edges without throwing panics or double-allocations.

---

### Tier 3: Cross-Feature Combinations (7 Cases)

Tier 3 checks complex interaction scenarios where multiple features intersect:

1. `test_t3_source_failure_during_guarded_mutation`: Combines F5 (Guarded Mutations) and F6 (Failure Isolation). An upstream crashes midway through a multi-file rename transaction. Verifies transaction rollback and failure isolation.
2. `test_t3_static_graph_query_with_stale_causal_snapshots`: Combines F7 (Static Graph) and F4 (Source Attribution). Queries definition from a document with a high version count, verifying static graph results are correctly mapped or dropped because of stale snapshot versioning.
3. `test_t3_dynamic_capability_registration_influences_method_routing`: Combines F2 (Capability Discovery) and F3 (Method Routing). A dynamic registration from an upstream changes the routing matrix, causing subsequent queries to route to the new upstream.
4. `test_t3_routing_did_change_during_slow_query_timeout`: Combines F3 (Method Routing), F4 (Source Attribution), and F6 (Failure Isolation). Client sends didChange during a slow hover query to an upstream, verifying that the slow query is aborted or its stale response is dropped when the timeout fires.
5. `test_t3_mutation_requires_capability_intersection_verification`: Combines F2 (Capability Discovery) and F5 (Guarded Mutations). A formatting request is received, verifying that the edit gate checks that formatting capability is currently active and supported by the target owner.
6. `test_t3_static_graph_hover_merge_with_dynamic_results`: Combines F7 (Static Graph) and F3 (Method Routing). Hover query returns results from both LSIF static graph and a dynamic upstream, verifying they are merged and styled correctly.
7. `test_t3_lifecycle_shutdown_clears_dynamic_registrations_and_static_graphs`: Combines F1 (Lifecycle), F2 (Capability Discovery), and F7 (Static Graph). Sending a shutdown request clears all dynamic capabilities and closes database connections to static graphs.

---

### Tier 4: Real-World Application Scenarios (5 Cases)

Tier 4 implements multi-step, end-to-end integration flows simulating complex developer workflows:

1. `test_t4_scenario_multi_language_polyglot_workspace`: Verifies handling of a polyglot workspace (e.g. Rust and TypeScript files coexisting). Composed server routes queries to respective backends, attributes diagnostics, and displays combined status information.
2. `test_t4_scenario_slow_remote_lsp_with_local_fallback`: Simulates a remote workspace where a remote backend has high latency. The composed server services definitions and hover from a local fast static graph/mock server and gracefully updates with deep diagnostics when the remote backend responds.
3. `test_t4_scenario_collaborative_editing_with_conflict_resolution`: Simulates two clients/sources proposing conflicting, overlapping text edits concurrently. The server's edit gate blocks the second conflicting request, requests reconciliation, and continues serving non-overlapping updates.
4. `test_t4_scenario_large_monorepo_lsif_navigation`: Simulates navigation across a giant monorepo. Large pre-indexed LSIF static graphs are queried for cross-repository definitions/references, merged in O(1) time with live local edits, and returned within the 5ms target limit.
5. `test_t4_scenario_server_crash_recovery_during_refactoring`: Simulates a multi-file rename operation where one of the upstream language servers crashes mid-transaction. The composed server aborts the transaction, rolls back already applied edits, flags the source as degraded, and displays a protocol-correct error to the client.

---

## 3. Test Architecture

The integration test infrastructure is designed for performance, correctness, and clean process isolation.

### Communication Flow

The client/test runner communicates with the composed server over standard channels using JSON-RPC 2.0 messages.

```
┌─────────────┐                    ┌─────────────────┐                    ┌──────────────────┐
│             │  JSON-RPC Request  │                 │  Fanned-out /      │                  │
│ Integration ├───────────────────>│ Composed Server ├───────────────────>│ Mock Upstreams   │
│ Test Runner │                    │ (lsp-max) │  Routed Request    │ (Spawning Pipes) │
│             │<───────────────────┤                 │<───────────────────┤                  │
└─────────────┘  JSON-RPC Response └─────────────────┘  JSON-RPC Response └──────────────────┘
```

- **Integration Test Runner**: Spawns the Composed Server using an in-memory channel loop (`tokio::io::duplex`) or standard I/O redirection (`stdin`/`stdout`).
- **Composed Server**: Receives client requests, applies routing strategies, checks invariants, and forwards requests to spawned mock upstreams.
- **Mock Upstreams**: Run in isolated task loops simulating standard language servers. They are configured dynamically to succeed, fail, timeout, or return corrupt data depending on the test requirements.

### Mocking Upstreams

Mock upstreams are constructed programmatically via a `MockServerBuilder`:
- **Timeout Simulator**: Allows configuring an artificial delay on responses.
- **Crash Simulator**: Closes the I/O channel abruptly when a specific request is received.
- **Behavior Injection**: Instructs the mock to return specific capability sets, diagnostics, or document edit responses.

### Directory Layout

The E2E and integration tests are organized as follows:

```
/Users/sac/lsp-max/
├── tests/
│   ├── e2e/
│   │   ├── main.rs                   # Main test harness entrypoint
│   │   ├── mock_server.rs            # Reusable mock language server harness
│   │   ├── test_f1_lifecycle.rs      # Tier 1 & 2 tests for F1
│   │   ├── test_f2_capabilities.rs   # Tier 1 & 2 tests for F2
│   │   ├── test_f3_routing.rs        # Tier 1 & 2 tests for F3
│   │   ├── test_f4_attribution.rs    # Tier 1 & 2 tests for F4
│   │   ├── test_f5_mutations.rs      # Tier 1 & 2 tests for F5
│   │   ├── test_f6_isolation.rs      # Tier 1 & 2 tests for F6
│   │   ├── test_f7_static_graph.rs   # Tier 1 & 2 tests for F7
│   │   ├── test_t3_pairwise.rs       # Tier 3 pairwise tests
│   │   └── test_t4_scenarios.rs      # Tier 4 real-world scenario tests
│   └── common/
│       └── helpers.rs                # JSON-RPC assert helpers, logging, environment setups
```

---

## 4. Real-World Application Scenarios (Tier 4)

These scenarios represent realistic client-server interactions under production constraints:

1. **Scenario 1: Polyglot Workspace Navigation**
   - *Workflow*: A user opens a workspace containing Rust and TypeScript. The composed server registers both dynamic backends, routes hover and diagnostics to the correct backend based on document selectors, and merges workspace-wide symbol requests.
   - *Assertions*: Verifies source attribution headers remain present, and file-based selector rules prevent routing TypeScript queries to the Rust backend.

2. **Scenario 2: Slow Remote Backend with Local Static Cache**
   - *Workflow*: A developer edits a file with a slow remote language server connection. While waiting for the remote server to compile and return definitions, the client triggers a go-to-definition.
   - *Assertions*: Composed server serves definition immediately from the pre-indexed LSIF static graph in <5ms, and updates diagnostic warnings asynchronously once the remote server returns.

3. **Scenario 3: Multi-User Collaborative Mutation Resolution**
   - *Workflow*: Two collaborative clients concurrently format the same document section. The first edit is accepted; the second edit overlaps the first edit's range and is processed.
   - *Assertions*: Transaction gate rejects the second edit with a clean `OverlapConflict` error rather than producing corrupted merged text.

4. **Scenario 4: High-Throughput LSIF Querying**
   - *Workflow*: A continuous integration tool requests references for hundreds of identifiers in parallel.
   - *Assertions*: Composed server utilizes in-memory materialized views to satisfy all requests without Oxigraph store latency, keeping memory footprint low and response times within the SLA.

5. **Scenario 5: Refactoring with Backend Crash Recovery**
   - *Workflow*: A user triggers a rename that modifies 10 files. Mid-rename, the backend managing 3 of those files crashes.
   - *Assertions*: Composed server detects the broken pipe, performs a transactional rollback on all files, restores files to original state, and flags the backend as degraded while remaining fully operational.

---

## 5. Coverage Thresholds

To guarantee comprehensive verification, the test suite enforces the following strict test case distribution:

| Tier | Focus | Description | Exact Test Count |
|---|---|---|:---:|
| **Tier 1** | **Feature Coverage** | 5 base coverage tests for each of the 7 features (F1 to F7) | **35** |
| **Tier 2** | **Boundary & Corner Cases** | 5 boundary/edge tests for each of the 7 features (F1 to F7) | **35** |
| **Tier 3** | **Cross-Feature Combinations** | Pairwise interaction tests mapping feature overlaps | **7** |
| **Tier 4** | **Real-World Scenarios** | End-to-end multi-step integration workflows | **5** |
| **Total** | **Full Suite Count** | Sum of all verified cases across the entire test harness | **82** |
