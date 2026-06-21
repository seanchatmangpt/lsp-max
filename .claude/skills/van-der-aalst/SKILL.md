---
name: van-der-aalst
description: Use lsp-max process mining primitives — Directly-Follows Graph (DFG), Declare constraint model, OCEL accumulation. Reference for building or debugging process-conformance features.
tools: [Read, Grep]
---

# Van der Aalst Process Mining — lsp-max Primitives

Source: W.M.P. van der Aalst, "Process Mining: Data Science in Action" (2nd ed., 2016).

## Module Locations

```
crates/lsp-max-compositor/src/declare.rs   — Declare constraint model
crates/lsp-max-compositor/src/dfg.rs       — Directly-Follows Graph
crates/anti-llm-cheat-lsp/src/virtual_docs/process_model.rs — virtual doc (inline DFG)
```

## Declare Constraint Model (`declare.rs`)

9 constraint types enforcing LTL-style declarative process specifications:

```rust
pub enum Constraint {
    Init(String),                           // first activity must be this
    End(String),                            // last activity must be this
    Response(String, String),               // A occurs → B must occur after
    Precedence(String, String),             // B occurs → A must have occurred before
    ExactlyOne(String),                     // activity occurs exactly once
    NotCoExistence(String, String),         // A and B cannot both occur
    RespondedExistence(String, String),     // A occurs → B occurs (anywhere)
    Absence(String),                        // activity must never occur
    ChainResponse(String, String),          // A occurs → B occurs immediately after
}
```

Normative models:
```rust
DeclareModel::compositor()          // FlushCoordinator: Init, Response, NotCoExistence, etc.
DeclareModel::anti_llm_detection()  // detection pipeline: Absence(VictoryLanguageEmitted), etc.
```

Usage:
```rust
let traces = extract_traces(&ocel_events);  // HashMap<case_id, Vec<activity>>
let model = DeclareModel::compositor();
let violations = model.check(&traces);      // Vec<ConstraintViolation>
let fitness = model.fitness(&traces);       // f64 ∈ [0.0, 1.0]
```

## Directly-Follows Graph (`dfg.rs`)

Core discovery primitive: records (A→B) transition frequencies across cases.

```rust
let dfg = DirectlyFollowsGraph::from_traces(&traces);
// or
let dfg = DirectlyFollowsGraph::from_events(&ocel_events);

dfg.node_count()       // number of activities
dfg.edge_count()       // number of arcs
dfg.total_transitions() // sum of arc frequencies

// Conformance metrics
let normative = [
    ("CompositorFlush".to_string(), "CompositorFlushAdmitted".to_string()),
    ("CompositorFlush".to_string(), "CompositorFlushBlocked".to_string()),
];
dfg.fitness_against_model(&normative)    // fraction of normative arcs present
dfg.precision_against_model(&normative)  // fraction of DFG arcs in normative model

// Renderers
dfg.to_mermaid()  // Mermaid flowchart LR
dfg.to_dot()      // Graphviz DOT
```

## OCEL Accumulation (`FlushCoordinator`)

The compositor accumulates OCEL 2.0 events after every flush:

```rust
let eid = event_counter.fetch_add(1, Ordering::Relaxed);
let ocel_event = receipt.to_ocel_event(&format!("cf-{eid}"), &ts);
ocel_events.lock().unwrap().push(ocel_event);
```

Drain the buffer:
```rust
let events: Vec<serde_json::Value> = coordinator.take_ocel_events(); // drains
let count = coordinator.ocel_event_count();                          // snapshot
```

## Virtual Document — `anti-llm://process-model`

Served from `anti-llm-cheat-lsp` via `workspace/textDocumentContent`. Renders:

1. **DFG summary** — node count, edge count, transitions, case count
2. **Mermaid flowchart** — `flowchart LR` with activity nodes and arc frequencies
3. **Declare conformance report** — violation table (constraint, case, detail)
4. **Fitness score** — conformant cases / total cases
5. **Activity legend** — diagnostic code prefixes → activity names

Activity map:
```
ANTI-LLM-VICTORY-* / ANTI-LLM-CLAIMS-* → VictoryLanguageDetected
ANTI-LLM-RECEIPT-*                      → FakeReceiptDetected
ANTI-LLM-ROUTE-*                        → FakeRouteDetected
ANTI-LLM-VERSION-*                      → VersionViolationDetected
ANTI-LLM-TOWER-* / ANTI-LLM-LSP-*      → ForbiddenRefDetected
WASM4PM-*                               → ProcessViolationDetected
GGEN-*                                  → GgenViolationDetected
(all other codes)                       → CheatDetected
(synthetic terminal, always appended)   → ScanComplete
```

## Conformance Statuses

| Status | Meaning |
|--------|---------|
| CANDIDATE | No violations in any trace |
| PARTIAL | Some traces violate at least one constraint |

Never use "all clean", "fully conformant", or other victory language for conformance results.
