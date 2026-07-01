# Architecture: Law-State Runtime via LSP

This document describes the core architecture of `lsp-max` and its law-state execution model. It is the authoritative reference for understanding how the system enforces invariants, maintains cryptographic receipts, and coordinates multi-server LSP composition.

## I. Paradigm & Foundation

### I.1. Post-Human Protocol

`lsp-max` is a **post-human project-state protocol** — LSP repurposed for autonomous machine agents, not IDE assistants. It is not an editor helper that shows users visual suggestions; it is a deterministic state-transition controller that logs cryptographic receipts and enforces architectural laws.

Three implications follow:

1. **Documentation is law projection.** Every doc describes formal system behavior, not onboarding guidance. Docs are normative, not pedagogical.
2. **Every JSON-RPC call is a state transition.** Requests are not "queries for suggestions" but formal attempts to mutate project state. Law gates admit or refuse the transition.
3. **Human review is rejected.** Conformance is determined by automated correctness gates only, not code review, testing, or human judgment. A release requires a conformance score of exactly 100.0.

### I.2. Three-Protocol Stack

`lsp-max` sits at the boundary of three protocols, each serving a distinct role:

- **A2A (Agent-to-Agent):** Delegation. Agents communicate intent and request co-execution.
- **MCP (Model Context Protocol):** Tools. Agents request access to resources (files, APIs, databases) via MCP servers.
- **LSP (Language Server Protocol):** Law signals. The LSP server is the ambient law enforcer — it emits diagnostics, accepts state queries, and gates mutations based on conformance.

Why LSP for law, not MCP or A2A? Because **LSP is a push protocol**: the server sends diagnostics (refusals) without being polled. A2A and MCP are pull-based; a law signal embedded in a pull response reaches the requester too late. The law must be ambient in the environment, not buried in tool responses. LSP achieves this by maintaining a persistent connection and pushing violation notifications immediately.

## II. Architecture: Five-Layer Stack

`lsp-max` is decomposed into five layers. Each layer enforces invariants at its boundary; mutations cross boundaries only if a receipt proves lawfulness.

```
┌─────────────────────────────────────────┐
│ Layer 5: Autonomic LSP Mesh             │  sibling repos (wasm4pm, lsp-types-max)
│ (cross-project agent orchestration)     │
└─────────────────────────────────────────┘
          ↑           ↑           ↑
┌─────────────────────────────────────────┐
│ Layer 4: Knowledge Hooks                │  crates/lsp-max-agent, crates/lsp-max-analyzer
│ (analysis bundles, agent integration)   │
└─────────────────────────────────────────┘
          ↑           ↑           ↑
┌─────────────────────────────────────────┐
│ Layer 3: Law-State Runtime              │  crates/lsp-max-runtime
│ (typestate machine, receipts, gates)    │
└─────────────────────────────────────────┘
          ↑           ↑           ↑
┌─────────────────────────────────────────┐
│ Layer 2: LSP State Surface              │  src/ (root crate)
│ (LanguageServer trait, routing, LSP    │
│  typedefs, transport: stdio/TCP)        │
└─────────────────────────────────────────┘
          ↑           ↑           ↑
┌─────────────────────────────────────────┐
│ Layer 1: Actuation Grammar              │  crates/lsp-max-cli, clap-noun-verb
│ (CLI nouns & verbs, endpoint mapping)   │
└─────────────────────────────────────────┘
```

### II.1. Layer 1: Actuation Grammar

The CLI surface (`lsp-max-cli`) maps user commands (noun + verb pairs) to LSP method calls. Example: `lsp-max gate list` invokes the `max/gate` method with the `list` verb. Every entry point is filtered through the post-human doctrine: only actions with valid receipt proofs are executed.

### II.2. Layer 2: LSP State Surface

The root crate's `src/` exports the `LanguageServer` trait (forked from `tower-lsp`, not a composition wrapper). Every LSP method — `didOpen`, `didChange`, `definition`, `hover`, etc. — threads through the law-state runtime below before returning a response. The routing layer (`src/registry.rs`) maps file extensions to child servers and applies tier-stratified dispatch strategies (FirstSuccess for navigation, FanAll for diagnostics).

### II.3. Layer 3: Law-State Runtime

The law-state runtime (`lsp-max-runtime`) is the heart of the system. It provides:

- **Typestate machine:** Five unidirectional phases (Uninitialized → Initializing → Initialized → ShutDown → Exited). Every LSP call is a state-transition attempt. The machine verifies that transitions are lawful before executing.
- **Receipt chains:** Each transition is proved by a cryptographic BLAKE3 hash chain. Receipt integrity is non-negotiable; a claimed transition without a receipt is refused.
- **ConformanceVector:** Three-axis tracking (admitted/refused/unknown) of LSP feature support. Transitions to "admitted" are only valid with receipt artifacts.
- **Gate predicates (Λ_CD):** Formal predicates that block shell actions if law violations are detected. ANDON diagnostics are emitted when a gate refuses.

### II.4. Layer 4: Knowledge Hooks

Agents integrate via hook bundles (`lsp-max-agent`). Hooks are called at five points in the LSP lifecycle: SessionStart (discovery), PreToolUse (gate + snapshot), PostToolUse (analysis), SubagentStart, and SubagentStop. Hook implementations register analysis rules, define custom laws, and update the knowledge base asynchronously.

### II.5. Layer 5: Autonomic Mesh

Sibling repos (`wasm4pm`, `lsp-types-max`, `wasm4pm-compat`) are composed through the LSP protocol. They discover each other via SessionStart hooks and synchronize state via receipts. The mesh is not orchestrated centrally; each node enforces its own laws and verifies signatures on receipts from peers.

## III. Core Mechanisms

### III.1. Typestate Machine

The machine defines five states and legal transitions:

```
Uninitialized
    ↓ (initialize receipt)
Initializing  ← → (gate refuses → remain Initializing or emit Refused)
    ↓ (full init receipt)
Initialized   ← → (gate admits → remain Initialized or transition out)
    ↓ (shutdown receipt)
ShutDown      (one-way; no further mutations)
    ↓ (exit receipt)
Exited        (terminal; no further transitions)
```

Every transition requires a receipt. The exit code (0 = clean ShutDown → Exited; 1 = any other violation) is a deterministic function of the final state.

### III.2. ConformanceVector: Three-Valued Logic

LSP capabilities are tracked on three independent axes:

```rust
pub struct ConformanceVector {
    pub admitted: HashSet<String>,    // Features with receipt artifacts
    pub refused: HashSet<String>,     // Features blocked by law/constraint
    pub unknown: HashSet<String>,     // Features with indeterminate status
}
```

**Invariants:**

1. `admitted ∩ refused = ∅`
2. `admitted ∩ unknown = ∅`
3. `refused ∩ unknown = ∅`

Why three axes? Binary logic (supported/unsupported) collapses "unknown" into one of the other two, destroying information. Unknown means "analysis is incomplete or the question is inapplicable" — a distinct epistemic state that must be preserved. Only transitions backed by receipt artifacts are lawful; unknown can move to admitted *or* refused, but never silently collapse.

### III.3. Receipt Chains & Tamper Evidence

Every transition (§III.1) produces a receipt:

```rust
pub struct CryptographicReceipt {
    pub prev_hash: Blake3Hash,           // digest of prior receipt
    pub discipline_id: Uuid,             // which law governs this transition
    pub law_id: Uuid,                    // specific law that approved/refused
    pub consequence_hash: Blake3Hash,    // commitment to the transition's outcome
    pub sequence: u64,                   // monotonic sequence number
    pub signature: [u8; 64],             // Ed25519 signature (when signed)
}
```

The chain is tamper-evident: if any receipt is modified, `prev_hash` of the next receipt becomes invalid. Replaying or reordering receipts changes the chain topology. Verification tools can detect tampering by recomputing hashes from scratch.

### III.4. Λ_CD Gate Predicate

The gate predicate is a formal boolean function that decides whether a transition is lawful. It evaluates:

- Receipt presence and validity (cryptographic check)
- ConformanceVector invariants (admitted/refused/unknown disjointness)
- Process conformance (DFG fitness against declared Declare constraints)
- Temporal constraints (compliance windows, SLA boundaries)

If the predicate evaluates to false, the gate refuses the transition, emits an ANDON diagnostic, and blocks the action.

## IV. Multi-Server Composition

`lsp-max` acts as a compositor, fanning out LSP events to multiple child servers and merging their responses. This enables:

- Shared language-analysis infrastructure (multiple linters, formatters, type checkers on `.rs`)
- Hierarchical specialization (one server for syntax, another for type checking, another for style)
- Incremental adoption (add servers incrementally without rewriting the root LSP driver)

### IV.1. Tier-Stratified Routing

Each child server is registered with a tier and a list of file extensions:

- **Primary:** Full LSP support (navigation, formatting, refactoring). FirstSuccess strategy: use the first Primary server's response, skip secondaries.
- **Secondary:** Partial support. Included in FanAll (all servers), excluded from FirstSuccess.
- **DiagnosticsOnly:** Emits diagnostics only, cannot serve navigation requests.

Example: `.rs` files routed to `rust-analyzer` (Primary), `clippy-lsp` (DiagnosticsOnly), `test-analyzer` (DiagnosticsOnly). Hover goes to rust-analyzer (FirstSuccess); diagnostics come from all three (FanAll).

### IV.2. Fan-Out / Merge Pipeline

On every LSP notification (`didOpen`, `didChange`, `didClose`, `didSave`):

1. **Fan-out:** Route the notification to all registered servers for the file extension.
2. **Local processing:** Each server updates its internal state.
3. **Merge:** Collect diagnostics from all servers (no dedup by default; REFUSED_BY_LAW diagnostics are preserved, others may be unified).
4. **Flush:** Emit the merged diagnostic set to the client in a `publishDiagnostics` notification.

The compositor itself emits a receipt for each flush (RFC-B: per-server speciation receipt chains), attributing diagnostics to their originating servers and binding them to a moniker join key when a code symbol is involved.

### IV.3. Diagnostic Buffer & Flush Coordinator

Diagnostics are buffered (not emitted immediately) and flushed on a fixed schedule or on explicit `max/flushDiagnostics` requests. The FlushCoordinator ensures:

- **Ordering:** Flushes are serialized; no concurrent publishes.
- **Atomicity:** A flush publishes all diagnostics from a complete round-trip, not partial results.
- **Receipt binding:** Each flush produces a CompositorReceipt, attributing diagnostics to servers and gates.

## V. Law Enforcement & Conformance

### V.1. Diagnostics as Refused Transitions

Every LSP diagnostic is a formal statement: "A transition was attempted; the gate refused it." Diagnostics are not suggestions or IDE hints. They are law violations.

A diagnostic carries:

- `law_id`: Which law blocked the transition.
- `attempted_transition`: What the code tried to do.
- `violated_axes`: Which ConformanceVector invariants were violated (admitted ∩ refused, etc.).
- `doc_routes`: Links to the laws and their formal specifications.
- `repair_actions`: Proposed fixes (as CodeActions).
- `verification_gates`: Which gates must pass for the repair to be admitted.
- `receipt_obligation`: Proof that the repair was attempted and its outcome.

### V.2. max/* Protocol Methods

`lsp-max` extends LSP with custom methods for state querying and repair:

- `max/snapshot`: Emit the current typestate machine snapshot and ConformanceVector.
- `max/conformanceVector`: Query the current admitted/refused/unknown sets.
- `max/explainDiagnostic`: Unpack a diagnostic to show the law, the gate predicate, and the receipt chain.
- `max/repairPlan`: Generate a repair plan (sequence of CodeActions) to transition from refused to admitted.
- `max/applyRepairTransaction`: Atomically apply a CodeAction and emit a receipt.
- `max/gate`: Run a gate manually and emit the verdict.
- `max/receipt`: Query or verify a receipt (check its cryptographic signature).
- `max/releaseActuation`: Request a release (builds the final conformance score and decides admit/refuse).

### V.3. Automated Gates & Conformance Scoring

A **conformance score** is calculated as:

```
score = max(0, 100 - ∑penalties)
```

Where penalties are:

- Error diagnostic: 30 points
- Warning diagnostic: 15 points
- Info/Hint diagnostic: 5 points

A release is admitted only if `score == 100.0` (exactly) — no rounding. This is absolute: 99.99 is not enough.

### V.4. Process Mining: DFG & Declare

The compositor records the sequence of LSP events (didOpen, didChange, didClose, didSave, publishDiagnostics, gate requests, etc.) as a Declare event log. Conformance is measured against two models:

- **Directly-Follows Graph (DFG):** A state machine that tracks which events can follow which. Example: didClose cannot occur without a prior didOpen for the same file.
- **Declare constraints:** Temporal and causal constraints. Example: "existence(gate_passed)" — the gate must run before release.

Violations of DFG or Declare constraints are reported as process-level ANDON diagnostics, not individual code issues.

## VI. Acceleration: Specification-Driven Development

Writing an LSP server by hand is tedious. You must:
1. Define types (150+ structs) matching the LSP spec.
2. Write request/response handlers (100+ LOC per method).
3. Thread receipts through every dispatch path (50+ LOC boilerplate).
4. Write tests to verify protocol compliance (200+ LOC).

`lsp-max` replaces this with specification-driven generation.

### VI.1. RulePackServer & Protocol Overhead Reduction

The `RulePackServer` trait abstracts the common pattern:

```rust
pub trait RulePackServer {
    fn scan_uri(&self, uri: &str) -> Vec<Diagnostic>;
    fn index(&self) -> Arc<WorkspaceIndex>;
}
```

Implementing `RulePackServer` (20 LOC) and placing a TOML rule file (50 LOC) eliminates 400+ LOC of LSP boilerplate. A 20-line struct implementing `RulePackServer` gains all LSP routing, diagnostic publishing, and receipt threading for free.

### VI.2. ggen μ-Pipeline

The `ggen` code generator reads a formal specification (RDF triples or Tera templates) and emits:

- Protocol types from `metaModel.json` (LSP 3.18 spec)
- Trait implementations for your domain laws
- Diagnostic handlers and CodeAction factories
- Receipt schemas and verification stubs

Development time: raw `tower-lsp` → 1–3 days. With `ggen` + `RulePackServer` → ~25 minutes.

### VI.3. Ontologies as Authoritative Law

Rules are stored as RDF triples in a knowledge base, not hardcoded in Rust:

```turtle
:contract-rule-1 a :ContractViolation ;
    :severity :error ;
    :pattern "[public mod][no][impl][Serialize]" ;
    :remediation :auto-derive-serde ;
    :receipt-plan :update-derive-macro-line .
```

Queries into the ontology are SPARQL, not regex loops. This enables:

- Non-developers to author and update rules (domain experts, compliance officers).
- Cross-project rule sharing via RDF federation.
- Formal verification of rule consistency (no conflicting patterns).
- Automated documentation generation.

### VI.4. Oracle Classes A8–A12

Six formal adversary classes are defined for detecting tampering and logical flaws:

- **A8 (Audit Log Tampering):** Receipt chain altered after emission.
- **A9 (Temporal Anomaly):** Event timestamps violate causal ordering.
- **A10 (Causal Violation):** DFG constraint violated (e.g., didClose before didOpen).
- **A11 (Unknown Collapse):** ConformanceVector invariant violated.
- **A12 (Cyclic Dependency):** Declare constraint violated (e.g., circular repair dependencies).

Agents that exercise these oracle classes during testing prove the system's invariants hold under attack.

## VII. Invariants & Guarantees

These invariants are enforced at compile time (via Rust's type system and formal proofs) and at runtime (via gate checks and receipt verification):

1. **Layered isolation:** No layer bypasses the layer below; layer N mutations must be approved by layer N-1.
2. **Receipt binding:** No state mutation without a receipt; no receipt without a lawful transition attempt.
3. **Typestate machine:** No invalid transitions; the machine follows the five-phase model precisely.
4. **Unknown ≠ Admitted:** The ConformanceVector invariants are never violated; unknown cannot be silently conflated with admitted.
5. **LSP is read-only:** LSP requests cannot create side effects outside the law-state runtime; all mutations are logged and reversible.
6. **Composition is non-destructive:** Child server diagnostics are preserved in merges; no diagnostic is silently dropped.
7. **Receipts are immutable:** A receipt cannot be modified after emission; modification would invalidate the hash chain.

## VIII. Practical Integration

### VIII.1. Hook Lifecycle & SessionStart Discovery

When an editor (or agent) opens an LSP session, `lsp-max` calls `SessionStart` hooks. Hooks in this phase:

1. Scan the workspace for `.lsp-max.toml` configuration files.
2. Discover child servers registered locally (e.g., `wasm4pm-lsp` at `/path/to/wasm4pm/target/debug/wasm4pm-lsp`).
3. Spawn child processes and verify protocol compatibility.
4. Build the ExtensionRouter (file extension → child servers mapping).
5. Initialize the knowledge base and load rule ontologies.

### VIII.2. ANDON Gate in PreToolUse

Before a tool is called (e.g., `max/applyRepairTransaction`), the gate predicate (§III.4) is evaluated in the `PreToolUse` hook. If the predicate is false, a refusal diagnostic is emitted and the tool call is blocked. This is the "ANDON" pattern: stop the line before executing.

### VIII.3. Debugging & Gate Inspection

When a gate refuses, use `max/explainDiagnostic` to unpack the refusal:

```bash
$ lsp-max gate explain <diagnostic-id>

Law: contract-public-must-impl-serialize
Gate Predicate: (is_public && is_mod) → has_derive_serde
Attempted Transition: state=ADMITTED, action=SAVE_FILE
Violated Axis: admitted ∩ refused
Receipt Chain:
  Receipt-42: prev=0xabcd..., consequence=0x1234..., signature=valid(ed25519)
  Receipt-41: prev=0x9999..., consequence=0xabcd..., signature=valid(ed25519)
  ...

Repair Actions:
  1. Add `#[derive(Serialize)]` to line 15
  2. Re-run gate to verify

Verification Gate: must_pass(contract-public-must-impl-serialize)
```

This chain of reasoning — gate predicate, state, receipts, repair — is the post-human correctness model. No human judgment; pure logic.

## Further Reading

- **RFC 0001:** Specification Generator (ggen architecture and code generation)
- **RFC 0002:** Law Enforcement via Receipt Chains (cryptographic proof model)
- **RFC 0003:** ConformanceVector Three-Valued Logic (epistemic framework)
- **RFC 0004:** Composition Over tower-lsp Fork (why the architecture is layered)
- **RFC 0005:** CalVer Versioning Over SemVer (deployment semantics)
- **max/* Protocol Specification:** `docs/reference/max-protocol-law.md`
- **Typestate Machine States:** `crates/lsp-max-runtime/src/machine.rs`
- **Five-Layer Crate Dependency Graph:** `CLAUDE.md` Section "Workspace architecture"
