# The Structural Conformance of wasm4pm-compat: Bridging Formal Process Mining Theory and High-Performance Execution

**Abstract**
This thesis investigates the architectural design, formal grounding, and strategic role of the `wasm4pm-compat` crate within the broader `wasm4pm` process mining ecosystem. It argues that the strict decoupling of event data models (the "compat" layer) from discovery and conformance algorithms (the "engine" layer) is not merely a software engineering preference, but a fundamental requirement for maintaining mathematical rigor in object-centric process mining (OCPM). By enforcing the formal axioms of Object-Centric Event Data (OCED) as defined in contemporary literature (e.g., van der Aalst et al.), `wasm4pm-compat` serves as the authoritative, mathematically sound boundary layer that guarantees algorithmic fidelity across the ecosystem.

## 1. Introduction
The transition from traditional, case-centric process mining to Object-Centric Process Mining (OCPM) introduces profound complexity in data representation. Traditional event logs assume a single case notion, leading to convergence and divergence problems when modeling real-world interactions (e.g., one order containing multiple items). OCED resolves this by treating objects as first-class citizens, linked relationally to events.

In high-performance computational environments like `wasm4pm`, there is a strong temptation to couple data structures intimately with the algorithms that process them to maximize throughput. This thesis demonstrates why that coupling is an architectural anti-pattern and how `wasm4pm-compat` resolves it by acting as a sealed, authoritative type registry.

## 2. The Architectural Mandate: Separation of Concerns
The `wasm4pm` ecosystem is structurally bifurcated:
1.  **`wasm4pm-compat`**: The Canonical Type Authority.
2.  **`wasm4pm`**: The Algorithmic Execution Engine.

This strict separation enforces a critical boundary. Algorithms (such as Directly-Follows Graph (DFG) discovery, alpha mining, or GALL conformance) are transient; they evolve, are optimized, and are frequently replaced. The underlying mathematical shape of the event data, however, is permanent and defined by academic consensus. 

By centralizing types like `OCEL`, `OCELEvent`, `OCELObject`, and `PowlNode` in `wasm4pm-compat`, the ecosystem guarantees that regardless of which algorithmic crate processes the data, the fundamental invariants of the data model are preserved. This prevents "structural fracturing," where different analytical modules drift into incompatible representations of the same log.

## 3. Formal Grounding in Literature
The structural integrity of `wasm4pm-compat` is not arbitrary; it is deeply rooted in the formal definitions of OCED, specifically aligning with the formalizations provided in foundational papers such as *OCPQ: Object-Centric Process Querying & Constraints* (Küsters & van der Aalst, 2025).

The `wasm4pm-compat` crate implements these formalisms through rigorous validation boundaries, encapsulated within the `OcelLog::validate` function and the `OcelRefusal` taxonomy.

### 3.1 Disjoint Universes of Events and Objects
**Formal Definition:** The universe of events ($U_{ev}$) and the universe of objects ($U_{obj}$) are pairwise disjoint.
**Implementation:** `wasm4pm-compat` enforces this mathematically. During log validation, the system asserts that the intersection of the set of event IDs and the set of object IDs is empty. A violation yields an `IntersectingEventAndObjectUniverses` refusal, structurally preventing the injection of logically impossible graphs into the engine.

### 3.2 Mandatory Event-to-Object Cardinality
**Formal Definition:** $\forall e \in E \ \ eaval_e(objects) \neq \emptyset$
**Implementation:** A core tenet of OCED is that events do not exist in a vacuum; they act upon objects. The validation logic systematically scans the Event-to-Object (E2O) relational tables. If an event is discovered that maps to zero objects, the data is formally rejected with an `EventWithNoObjects` refusal. This guarantees that downstream algorithms never encounter dangling operational states.

### 3.3 Temporal Continuity
**Formal Definition:** $\forall e \in E \ \ eaval_e(time) \in \mathcal{T}$
**Implementation:** Process mining relies fundamentally on temporal ordering. `wasm4pm-compat` enforces that the timestamp mapping function is complete for all events, issuing an `EventMissingTimestamp` refusal otherwise. 

## 4. The Dissolution of Intermediary Crates
A critical phase in the maturation of the `wasm4pm` ecosystem was the intentional dissolution of intermediate crates like `ocel-core` and `wasm4pm-algos`. 

The existence of an `ocel-core` crate created an ambiguous middle layer that threatened the single-source-of-truth authority of `wasm4pm-compat`. By physically transplanting the formal OCEL structures directly into `wasm4pm-compat::legacy_ocel` and moving algorithms to the parent `wasm4pm` engine, the architecture achieved "closure." 

This enforces a binary relationship:
*   **Is it a Data Structure or Law?** $\rightarrow$ It belongs in `wasm4pm-compat`.
*   **Is it a Computation or Execution?** $\rightarrow$ It belongs in `wasm4pm`.

## 5. Oracle Class Witness Markers: Formalizing Adversary Detection

### 5.1 Witness Markers as Type-Level Authority Labels

The `wasm4pm-compat` crate includes a `witnesses_anti_cheat` module that formalizes five adversary detection classes (A8–A12) through structure-only witness markers:

```rust
witness_marker!(OracleA8AuditLogTampering, "anti-cheat/oracle-a8-audit-log-tampering", ...);
witness_marker!(OracleA9TemporalAnomaly, "anti-cheat/oracle-a9-temporal-anomaly", ...);
witness_marker!(OracleA10CausalViolation, "anti-cheat/oracle-a10-causal-violation", ...);
witness_marker!(OracleA11UnknownCollapse, "anti-cheat/oracle-a11-unknown-collapse", ...);
witness_marker!(OracleA12CyclicDependency, "anti-cheat/oracle-a12-cyclic-dependency", ...);
```

These markers are:
- **Zero-cost**: Uninhabited enums compiled to no runtime code or data.
- **Type-level authority**: Travel with `Evidence<T, Admitted, W>` as phantom markers.
- **Boundary-tagged**: Key strings ("anti-cheat/oracle-a8-*") name the exact violation class.
- **Queryable by the engine**: `wasm4pm` uses these keys to select which Oracle inference algorithm runs.

### 5.2 Oracle Classes as Formal Adversary Models

Each Oracle class formalizes a specific cheating pattern detectable through OCEL analysis:

| Class | Name | Formal Violation |
|-------|------|------------------|
| A8 | Audit Log Tampering | ∃e ∈ E with no causal predecessor in causality graph |
| A9 | Temporal Anomaly | ∃e₁, e₂ where e₁ causes e₂ but timestamp(e₁) ≥ timestamp(e₂) |
| A10 | Causal Violation | ∃m ∈ Mutations with no preceding Observation in E2O graph |
| A11 | Unknown State Collapse | ConformanceVector axis transitions UNKNOWN → {ADMITTED, REFUSED} without ResolutionEvent |
| A12 | Cyclic Dependency | ∃ cycle in causality DAG (∃e where e is its own ancestor) |

These map directly to academic process mining theory:
- **Causality** (A8, A10, A12) comes from the Event-to-Object relational model.
- **Temporal ordering** (A9) is fundamental to van der Aalst's notion of event precedence.
- **Conformance vectors** (A11) generalize the "burden of proof" notion from formal verification.

### 5.3 CompatDiagnostic Variants for Oracle Classes

The `wasm4pm-compat::diagnostic::CompatDiagnostic` enum includes five new variants:

```rust
pub enum CompatDiagnostic {
    // ... existing variants ...
    
    OracleA8AuditLogTampering,      // Law: every receipt/event has causal ancestor
    OracleA9TemporalAnomaly,        // Law: ancestor events have earlier timestamps
    OracleA10CausalViolation,       // Law: mutations preceded by observations
    OracleA11UnknownCollapse,       // Law: UNKNOWN never collapses without resolution
    OracleA12CyclicDependency,      // Law: causality graph is a DAG
}

impl Display for CompatDiagnostic {
    fn fmt(&self, f: &mut Formatter) -> Result {
        let (severity, message) = match self {
            OracleA8AuditLogTampering => (
                Error,
                "A8 oracle — audit log tampering: receipt/event has no ancestor computation in OCEL causal graph"
            ),
            // ... similarly for A9–A12
        };
    }
}
```

These diagnostics carry zero structural information about *how* the violation was detected — that is the engine's job. They carry only the *formal name* of the law and the *remedy* (what structure would satisfy the law).

### 5.4 The Boundary Between Structure and Engine

This design enforces a sharp architectural boundary:

**`wasm4pm-compat` responsibility:**
- Define the formal shape of OCEL (Event, Object, EventToObject links)
- Register Oracle class witness markers (type-level authority labels)
- Define CompatDiagnostic variants (named laws)
- Validate structural axioms (disjoint universes, mandatory E2O, temporal continuity)

**`wasm4pm` responsibility:**
- Run conformance algorithms to detect which Oracle class is violated
- Map violations to witness keys and confidence scores
- Produce engine-level inference reports
- Train statistical models for false-positive filtering

The boundary is enforced: `wasm4pm-compat` imports no algorithm code from `wasm4pm`. `wasm4pm` imports *only* types and witness markers from `wasm4pm-compat`.

## 6. Conclusion
`wasm4pm-compat` is not merely a utility crate or a collection of shared types. It is the codified, executable manifestation of process mining theory. By rigorously enforcing the mathematical axioms of Object-Centric Event Data at the boundary layer, and by formalizing adversary detection classes through witness markers and diagnostic laws, it ensures that high-performance analytical engines can operate at maximum velocity with absolute mathematical safety.

The Oracle class framework demonstrates that anti-cheat detection can be lifted from ad-hoc pattern matching to formal, auditable, specification-derived analysis. It stands as a prime example of translating the theoretical rigor of academic process mining (as pioneered by researchers like Dr. Wil van der Aalst) and adversarial security theory into a robust, production-grade software architecture.