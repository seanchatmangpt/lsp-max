# Post-Human LSP Framework and Machine-Verifiable Substrate Law

This document formally declares and defines the post-human framework and substrate law governing `lsp-max`.

## Paradigm Declaration: A Post-Human Project-State Protocol

The Language Server Protocol (LSP) is hereby formally declared to be a **post-human project-state protocol**. 

Traditionally, LSP has been understood as a developer-utility protocol designed to facilitate text-editor integration. Under the `lsp-max` paradigm, this assumption is rejected. `lsp-max` is **NOT** an IDE helper, autocompletion provider, or developer assistance tool. It operates strictly as a formal state transition gateway and admission controller for autonomous machine agents.

Every interaction over this protocol is a transition attempt on a persistent project-state graph. The server evaluates incoming JSON-RPC calls not to aid human comprehension, but to enforce correctness invariants, verify compliance certificates, and log cryptographic proofs of valid state transitions.

## Docs as Law Projections

Under the post-human paradigm, documentation does not serve to onboard human developers, explain developer environments, or provide step-by-step tutorial guides. Instead, documentation exists as a **formal projection of the state-machine rules** to which clients must mathematically conform. 

Documentation is the human-readable representation of the active law and semantic gates. It represents the contract boundaries, transition rules, and cryptographic obligations that the underlying runtime admits or rejects. Any client attempting to interact with the system must treat these documents as the specification of the state-machine's admission criteria.

---

## Architectural Layers

The `lsp-max` server is constructed of four distinct, decoupled layers, each playing a specific role in the admission, routing, verification, and logging of state transitions.

```
+-----------------------------------------------------------------------+
|                       LAW PLUGINS (lsp-max-agent)               |
|                   Automated Conformance Evaluation / Gates            |
+-----------------------------------------------------------------------+
                                   |
+-----------------------------------------------------------------------+
|                      RUNTIME (lsp-max-runtime)                  |
|               Zero-Cost Typestate Kernel & Deterministic Snapshots    |
+-----------------------------------------------------------------------+
                                   |
+-----------------------------------------------------------------------+
|                          SERVER (lsp-max)                       |
|                       JSON-RPC Message Routing                        |
+-----------------------------------------------------------------------+
                                   |
+-----------------------------------------------------------------------+
|                       PROTOCOL (lsp-max-protocol)               |
|                     Syntactic Boundary & Extended Schema              |
+-----------------------------------------------------------------------+
```

### 1. Protocol (`lsp-max-protocol`)
This layer defines the syntactic boundary and vocabulary of the system. It contains the type definitions for standard LSP 3.18 types (generated directly from the metamodel) and the extended JSON-RPC structures (such as `MaxDiagnostic`, `MaxCodeAction`, `SnapshotId`, `ConformanceVector`, and `Receipt`). It acts as the shared vocabulary schema across all other components.

### 2. Server (`lsp-max`)
This layer serves as the JSON-RPC message router and network boundary. Built on top of the Tower service stack, it decodes incoming network frames, maps them to the appropriate RPC handlers, manages session lifecycles, and dispatches requests and notifications to the underlying runtime kernel.

### 3. Runtime (`lsp-max-runtime`)
The runtime layer houses the zero-cost typestate kernel (`Machine<L, P, D>`) and coordinates project snapshots. It enforces compile-time and runtime guarantees regarding process phases, transition paths, and state logs. It ensures that the project state is updated deterministically and maintains snapshot histories.

### 4. Law Plugins (`lsp-max-agent`)
The law plugins layer executes out-of-band and in-band automated conformance evaluation. It operates as the correctness validator, running semantic linting, structural checks, and running automated correctness gates. It generates the conformance vectors and validates receipt logs before allowing the runtime to admit new state transitions.
