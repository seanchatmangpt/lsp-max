# The Axiomatic Foundation: `wasm4pm-compat` as the Baseline Type Authority

**Abstract**
This thesis documents the final, ideal state of the `wasm4pm` process mining ecosystem following the total eradication of transitional facades, deprecated namespaces, and compatibility narratives. It establishes that `wasm4pm-compat` is not, and never was, a backward-compatibility layer. Instead, it is the absolute, axiomatic baseline type authority for admissible process evidence. By enforcing a direct, unmediated dependency graph—where the algorithmic engine (`wasm4pm`) consumes canonical types exclusively from the baseline (`wasm4pm-compat`)—the architecture achieves absolute mathematical and structural purity. This document formalizes the final closure of the ecosystem's type authority graph.

## 1. Introduction: The Eradication of Residue
In the evolution of high-performance analytical systems, technical debt often manifests as transitional architecture: shims, facades, "legacy" namespaces, and museum crates designed to bridge old and new paradigms. 

The `wasm4pm` ecosystem explicitly rejects this paradigm. **There is no deprecation path. There is no transitional facade. There is no compatibility narrative.**

This thesis details the architectural singularity achieved when all such residue was burned away. The ecosystem now operates on a binary, axiomatic truth:
1.  **`wasm4pm-compat`** owns the canonical process-evidence type surface.
2.  **`wasm4pm`** owns execution.

The removal of intermediary type authorities (such as `wasm4pm-types` and `ocel-core`) was not a migration; it was a source-of-truth correction.

## 2. The Semantic Redefinition of `wasm4pm-compat`
The suffix `-compat` in software engineering traditionally implies backward compatibility. Within this ecosystem, that definition is strictly forbidden. 

`wasm4pm-compat` is the **baseline**. It is the immutable bedrock of admissibility. It defines the formal shape of process evidence (e.g., Object-Centric Event Data, Partial-Order Workflows, Petri Nets) before it is permitted to enter the execution engine.

To describe `wasm4pm-compat` as a "compatibility layer" is an architectural violation. It is the sole arbiter of what constitutes mathematically valid process mining data.

## 3. The Ideal Future Graph
The dependency graph of the ecosystem has been collapsed into its most direct, mathematically sound formulation.

```text
wasm4pm (The Execution Engine)
  └── depends on wasm4pm-compat (The Baseline Type Authority)
```

### 3.1 The Purge of `wasm4pm-types`
The `wasm4pm-types` crate was fundamentally incompatible with this pristine state. It existed as an intermediary, diluting the absolute authority of the baseline. 

Under the strict acceptance laws of this ecosystem, `wasm4pm-types` did not survive as a deprecated facade. It was not retained as a wrapper. It was entirely erased from the canonical dependency graph. The execution engine (`wasm4pm`) imports its types *directly* and *exclusively* from `wasm4pm_compat`.

### 3.2 The Eradication of "Legacy"
Language shapes architecture. The persistence of terms like `legacy_ocel` or `legacy_event_log` implicitly sanctioned the existence of dual truths (the "old" way and the "new" way).

In the ideal future state, there is only the *correct* way. Every crate, module, document, test, fixture, and comment containing the word "legacy" (in any case variation) has been purged. The OCEL structures in `wasm4pm-compat` are not legacy structures; they are the canonical, formal OCED definitions.

## 4. The Corrected Acceptance Law
The structural integrity of this ideal state is guarded by an unyielding acceptance law, continuously verified by the agentic machinery:

1.  **Exclusive Ownership:** All canonical process-mining and process-evidence types live exclusively in `wasm4pm-compat`.
2.  **Direct Consumption:** `wasm4pm` imports those types directly from `wasm4pm-compat`.
3.  **No Museum Crates:** `wasm4pm-types` is removed as a type authority, not deprecated.
4.  **Linguistic Purity:** No artifact within the ecosystem contains the string `legacy`, `Legacy`, or `LEGACY`.
5.  **Architectural Clarity:** No public architecture language describes `wasm4pm-compat` as backward compatibility.
6.  **Documented Baseline:** `wasm4pm-compat` is strictly documented as the baseline admissibility/type crate.

## 5. The Oracle Class Witness Hierarchy

The baseline type authority extends beyond mere data shapes to include formal **named laws** that the engine must respect. The `witnesses_anti_cheat` module exemplifies this principle: it registers five Oracle classes (A8–A12) as structure-only witness markers that travel through the admission pipeline.

### 5.1 Witness Markers as Canonical Law Registry

Each witness marker is a zero-cost uninhabited enum tagged with a KEY string:

```rust
witness_marker!(
    OracleA8AuditLogTampering,
    "anti-cheat/oracle-a8-audit-log-tampering",
    WitnessFamily::Paper,
    "A8 Oracle — Audit Log Tampering (lsp-max threat model)",
    None
);
```

The KEY string ("anti-cheat/oracle-a8-*") is the canonical naming authority. When `wasm4pm` detects a violation conforming to A8's pattern (receipt without causal ancestor), it reports:
```json
{
  "oracle_class": "anti-cheat/oracle-a8-audit-log-tampering",
  "confidence": 0.95
}
```

This report is not empirical guesswork. It flows from:
1. A formal definition in `witnesses_anti_cheat` (structure-only).
2. A named law in `CompatDiagnostic::OracleA8AuditLogTampering` (what satisfaction looks like).
3. An algorithm in `wasm4pm` that proves conformance to that law (detection logic).

The **baseline type authority** is the keeper of steps 1 and 2. The engine owns step 3.

### 5.2 No Intermediary Witness Crates

Just as `wasm4pm-types` was eradicated to enforce a single type authority, no intermediary witness registry was created. All Oracle class witnesses live in `wasm4pm-compat::witnesses_anti_cheat`. This prevents:

- Different analytical modules from registering incompatible Oracle class definitions.
- The engine from drifting into using witness KEYs that do not exist in the baseline.
- Users from "rolling their own" adversary classes outside the canonical registry.

## 6. Conclusion
The architecture detailed in this thesis represents the terminal, perfected state of the process mining ecosystem. By violently rejecting the comfort of transitional shims and deprecation narratives, the ecosystem forced a structural reckoning. 

What remains is an architecture that perfectly mirrors the formal mathematics of process mining: a pristine data model (`wasm4pm-compat`) cleanly decoupled from, yet directly consumed by, a high-performance execution engine (`wasm4pm`). The addition of Oracle class witness markers demonstrates that this baseline type authority extends to *formal laws* — named, queryable, mathematically grounded adversary detection classes.

This is not a migration bridge. This is a source-of-truth correction. This is how architecture becomes theory and theory becomes code.