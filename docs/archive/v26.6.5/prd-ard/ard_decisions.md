# Architectural Decision Records (ADRs)

This document contains decisions ARD-001 through ARD-005 governing the design of the Oxigraph & SPARQL Admitted Graph Control Plane for `lsp-max` v26.6.5.

---

## ARD-001 — Use Oxigraph v0.5.8 as Embedded RDF Store

### Status
Accepted

### Context
`lsp-max` requires a highly consistent, persistent graph database to store and query workspace files, compilation metadata, diagnostics, rules, and execution receipts. A standard relational database or an external graph server daemon introduces deployment complexity, licensing conflicts, and communication overhead.

### Decision
Integrate `oxigraph` version 0.5.8 as the embedded RDF graph engine.
- For persistent, on-disk deployments, instantiate via `Store::open`.
- For unit testing and CI verification gates, instantiate in-memory via `Store::new` to bypass RocksDB filesystem overhead.

### Rationale
- **Rust-Native**: Integrates directly with the `lsp-max` compilation unit, avoiding external runtime or network requirements.
- **Standards-Compliant**: Fully implements W3C RDF and SPARQL 1.1/1.2 specifications.
- **Embedded Persistence**: Backed by RocksDB, guaranteeing atomic writes and high-performance querying.
- **Thread Safety**: The `Store` structure is `Send + Sync`, enabling concurrent query evaluations without blocking the LSP router.

### Consequences
- **Disk Overhead**: RocksDB databases consume additional disk space; snapshots must be pruned periodically.
- **Batching Writes**: Triple ingestion must be batched and executed on background worker threads to avoid stalling the main LSP server.

---

## ARD-002 — Use SPARQL as Query/Projection Language

### Status
Accepted

### Context
Traversing structural relations, LSIF ranges, and diagnostic dependency graphs in raw Rust code results in fragile, verbose, and difficult-to-maintain code. We need a declarative, standard language to perform invariant checks and extract protocol projections.

### Decision
Declare SPARQL 1.1/1.2 as the standard query language for checking graph invariants, analyzing diagnostics provenance, and projecting data views.

### Rationale
- **Standards Alignment**: SPARQL is the W3C standard for querying RDF graphs.
- **Declarative Power**: Expresses complex multi-hop path traversals (e.g. following LSIF transitive namespaces) in compact, readable query blocks.
- **Interoperability**: Promotes verification reuse across different compliance auditing tools.

### Consequences
- **Query Validation**: All SPARQL queries must be parsed and validated at server startup to prevent runtime syntax crashes.
- **No Dynamic Compilation**: Dynamic user-input SPARQL execution is strictly forbidden in production to prevent query injection attacks.

---

## ARD-003 — Preserve LSIF as Static Language-Intelligence Artifact

### Status
Accepted

### Context
Using a full RDF graph store might suggest that standard LSIF exports are obsolete. However, LSIF 0.6.0 is widely integrated with existing code-hosting platforms (e.g., GitHub, GitLab) and editors.

### Decision
Keep LSIF 0.6.0 as the canonical static code intelligence format. The RDF graph store acts as an active validation wrapper and provenance tracking plane, not a replacement.

### Rationale
- **Tool Compatibility**: Retaining standard LSIF outputs ensures compatibility with existing editors.
- **Role Separation**: LSIF represents the physical, static index; RDF/SPARQL adds semantic verification and compliance tracking.

### Consequences
- **Drift Prevention**: Changes to the LSIF format parser must be automatically mapped to RDF vocabulary schemas to avoid semantic drift.

---

## ARD-004 — Use Materialized Views for Latency-Sensitive Responses

### Status
Accepted (Amended with Epoch Sync Barrier)

### Context
LSP requests (like go-to-definition, hover, find-references) require sub-5ms responses. Direct RocksDB-backed SPARQL queries over large graph databases can exceed 50ms, violating performance budgets. However, asynchronous propagation introduces a window of eventual consistency (<10ms) where rapid read-after-write operations (e.g., from refactoring agents or automated scripts) could receive stale or missing definition data.

### Decision
Serve active LSP requests from in-memory materialized views (`DashMap` tables) rather than calling the Oxigraph store. The graph store updates the materialized views asynchronously upon committing transactions.

To prevent race conditions during write commits when strict consistency is required, we introduce a **Monotonic Epoch Sync Barrier**:
1. **Epoch Registers**: The transaction coordinator maintains two global lock-free atomic counters:
   - `committed_epoch: AtomicU64` — incremented atomically immediately after a write transaction successfully commits to the Oxigraph store.
   - `applied_epoch: AtomicU64` — updated to match the commit epoch by the background projection worker after it has completed updating the materialized `DashMap` tables.
2. **Synchronization Channel (`EpochSignal`)**: A condition variable or event-notification channel is used by the projection worker to notify waiting threads when `applied_epoch` is updated.
3. **Execution Gate / Read Path**:
   - Standard LSP read requests (e.g., manual user hover/definition lookups) bypass synchronization and read directly from `DashMap` for maximum performance (sub-5ms O(1) latency).
   - Read requests requiring **strict accuracy** (e.g., automated agent refactoring scripts, or transactional validation requests) specify a target epoch or check if `committed_epoch > applied_epoch`. If a discrepancy is found, the read thread blocks on the `EpochSignal` condition variable until `applied_epoch >= committed_epoch`, ensuring read-your-writes / linearizable consistency.

### Rationale
- **Isolation**: Keeps the performance-critical data plane completely isolated from control plane database query latency.
- **Concurrency**: `DashMap` provides fast, thread-safe, concurrent read access.
- **Selective Consistency**: Solves eventual consistency races for automated code generators and refactoring loops without penalizing normal editor interactive lookups.

### Consequences
- **Eventual Consistency**: A brief, harmless window (typically `<10ms`) exists for manual edits.
- **Synchronous Overhead for Agents**: Strict accuracy reads will experience a block-and-wait latency equal to the projection propagation time (typically `<10ms`) only when a concurrent write is in progress.

---

## ARD-005 — Support Future MCP/A2A Projections

### Status
Accepted

### Context
Autonomous coding agents require interfaces to discover tools, view code context, and delegate tasks. The Model Context Protocol (MCP) and Agent-to-Agent (A2A) protocol are rapidly emerging as standard communication patterns.

### Decision
Design the RDF control plane schema to support future projections into MCP resource/tool lists and A2A task cards.

### Rationale
- **Extensibility**: Future-proofs `lsp-max` for integration with autonomous agent workflows.
- **Abstract Adapter**: Decoupling the data layer from the transport protocol allows the control plane to remain stable even if MCP or A2A specifications evolve.

### Consequences
- **Stability Class**: MCP and A2A interfaces are marked as experimental until specifications stabilize.
