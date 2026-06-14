# Execution Sequence Flows: lsp-max v26.6.5

This document maps the runtime interactions and transaction pathways of the Admitted Graph Control Plane using Mermaid diagrams.

---

## 1. Verification Flow

This sequence diagram illustrates the pipeline from LSIF ingestion through syntactic checking, SPARQL invariant validation, SHACL shapes checking, and receipt generation.

```mermaid
sequenceDiagram
    autonumber
    participant Input as LSIF/LiveLSP Input Stream
    participant Boundary as RelationAdmitter (Admission Boundary)
    participant Store as Oxigraph Store (oxigraph::store::Store)
    participant SPARQL as SparqlEvaluator (SPARQL Query Layer)
    participant SHACL as SHACL Engine (Validation Posture)
    participant Report as VerificationReport
    participant Receipt as ReceiptBuilder (Cryptographic Receipt)

    Input->>Boundary: stream_observations(elements)
    Note over Boundary: Validate syntactic structure & LSIF schema
    Boundary->>Store: admit_quads(Vec<oxrdf::Quad>) with GraphName (Snapshot ID)
    Store-->>Boundary: confirm_admission(QuadCount)
    
    Boundary->>SPARQL: run_invariants(GraphName)
    SPARQL->>Store: execute_ask_queries()
    Store-->>SPARQL: return_boolean_results()
    
    Boundary->>SHACL: validate_shapes(GraphName)
    SHACL->>Store: execute_shape_checks()
    Store-->>SHACL: return_shacl_results()
    
    SPARQL->>Report: write_invariant_results()
    SHACL->>Report: write_shape_results()
    
    Report->>Receipt: calculate_proof_hash(InputHash, QueryHash, ResultHash)
    Receipt-->>Report: return_cryptographic_receipt(max:Receipt)
    Report->>Input: return_verification_status(Success/Failure + max:Receipt)
```

---

## 2. LSP Response Flow (Hot-Path)

To maintain a definition lookup latency under `<5ms`, client requests are served from in-memory materialized views. These views are asynchronously synchronized with the persistent Oxigraph store when snapshots change. If strict accuracy is required, a synchronization barrier blocks the reader until the views catch up with the committed transaction epoch.

```mermaid
sequenceDiagram
    autonumber
    participant Client as LSP Client (e.g., Code Editor)
    participant Router as LSP Router
    participant View as Materialized Views (DashMap)
    participant Barrier as Sync Barrier (Epoch Monitor)
    participant ReceiptIdx as Receipt Index (In-Memory)
    participant Store as Oxigraph Store (Asynchronous control plane)

    Client->>Router: request(textDocument/definition, params)
    alt Strict Accuracy Required
        Router->>Barrier: await_sync(target_epoch)
        Note over Barrier, Store: If committed_epoch > applied_epoch, block reader
        Store-->>Barrier: signal_update_complete(applied_epoch)
        Barrier-->>Router: resume_read()
    end
    Router->>View: lookup_definition(URI, position)
    View-->>Router: return_materialized_definition(O(1) lookup)
    
    Router->>ReceiptIdx: get_receipt(definition_element_id)
    ReceiptIdx-->>Router: return_receipt_proof(max:Receipt)
    
    Router-->>Client: response(result, proof_receipt)
    
    Note over Store, View: Asynchronously, when a new snapshot is admitted, SPARQL projections update the Materialized Views & Receipt Index.
```

---

## 3. MCP/A2A Projection Flow

This diagram shows how the control plane projects its graph state to the Model Context Protocol (MCP) and Agent-to-Agent (A2A) protocol interfaces, providing agents with verifiable capabilities.

```mermaid
sequenceDiagram
    autonumber
    participant Store as Oxigraph Store (oxigraph::store::Store)
    participant Projection as ProjectionEngine
    participant MView as Materialized Capability View (DashMap)
    participant MCP as MCP Surface (JSON-RPC)
    participant A2A as A2A Surface (JSON-RPC)
    participant Receipt as ReceiptBuilder

    Note over Store, Projection: Triggered on snapshot admission or periodic update
    Projection->>Store: execute_projection_queries(GraphName)
    Store-->>Projection: return_projection_results (capabilities, tools, tasks)
    
    Projection->>MView: update_capability_view(Vec<Capability>)
    
    %% MCP request
    MCP->>MView: get_tools() / get_resources()
    MView-->>MCP: return_mcp_declarations (JSON-RPC)
    
    %% A2A request
    A2A->>MView: get_agent_cards() / get_tasks()
    MView-->>A2A: return_a2a_cards (JSON-RPC)
    
    Projection->>Receipt: bind_projection(GraphName, QueryHash, ResultHash)
    Receipt-->>Projection: generate_receipt()
```
