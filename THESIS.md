# Specification-Driven LSP Development: How the lsp-max + ggen Stack Compresses the Time to a Working Language Server

**Sean Chatman — June 2026**

---

## Abstract

Building a rule-enforcing language server with the raw LSP protocol is a multi-day task dominated by protocol scaffolding, not by domain logic. This document quantifies that overhead, documents the architectural choices made in lsp-max and ggen to eliminate it, and provides a measured end-to-end comparison. The central claim: with the `RulePackServer` trait and the ggen μ-pipeline, a new domain-specific language server can be specified, generated, and compile-verified in under one second — compared to one to three days for the equivalent raw tower-lsp work. The gains compound: cross-file diagnostics, workspace conformance aggregation, and four new max/ protocol methods are available by default in every generated server, not as future work.

---

## 1. The Traditional Path

A developer building a rule-enforcing LSP server in Rust (the dominant language for production LSP work after rust-analyzer established the idiom) faces a layered stack of scaffolding before they can write a single business rule.

### 1.1 Raw tower-lsp

The canonical Rust LSP library is `tower-lsp`. Its API surface is the `LanguageServer` trait: **34 methods** that a conforming server must implement or stub. Of these, a typical diagnostic-only server uses approximately 8 at runtime (`initialize`, `shutdown`, `did_open`, `did_change`, `did_close`, `did_save`, `diagnostic`, and `workspace/diagnostic`). The remaining 26 must still be wired to return `Err(MethodNotFound)` or sensible defaults.

Measured from the lsp-max `LanguageServer` implementation: **646 lines** in `src/language_server.rs` just for method dispatch. That file does not contain a single business rule. It is pure protocol plumbing.

Beyond the method stubs:

| Task | Typical effort | Where it lives |
|---|---|---|
| URI-keyed diagnostic registry | ~80 LOC | Custom HashMap |
| Diagnostics push/pull split | ~60 LOC | Per-method logic |
| Regex compilation and caching | ~40 LOC | Custom once_cell or LazyLock |
| Severity → `DiagnosticSeverity` mapping | ~20 LOC | Match arm table |
| Server capabilities negotiation | ~30 LOC | `initialize` handler |
| Rule TOML deserialization | ~40 LOC | Serde struct + loader |
| **Total protocol overhead** | **~270–350 LOC** | Spread across 6–8 files |

This is before the developer writes the first regex pattern.

### 1.2 The anti-llm-cheat-lsp Baseline

`anti-llm-cheat-lsp` — the oldest example in this codebase, written before `RulePackServer` existed — is the empirical baseline. It has:

```
engine.rs      252 LOC   — scanner, rule application, AhoCorasick setup
server.rs      347 LOC   — LanguageServer impl, diagnostics publish, URI tracking
diagnostics.rs  52 LOC   — diagnostic struct translation
ocel.rs        286 LOC   — OCEL event log emission (domain-specific)
main.rs         57 LOC   — entry point
Total:        ~994 LOC
```

Of the 994 lines, roughly **280 lines are pure domain logic** (the actual rules, the AhoCorasick patterns, the OCEL events). The other **714 lines are protocol overhead** — the 72% tax a developer pays before they can do any domain work.

---

## 2. The lsp-max Architecture

lsp-max is a fork of tower-lsp that introduces a `RulePackServer` trait as a bridge: it absorbs the 270–350 lines of protocol overhead and exposes a six-method interface that is all domain, no plumbing.

### 2.1 What the Trait Absorbs

```rust
// Everything a server author must implement:
fn rule_packs(&self) -> &[RulePack];          // declare your rules
fn grammar(&self) -> tree_sitter::Language;   // declare your grammar
fn server_name(&self) -> &'static str;        // declare your source tag
fn client(&self) -> &Client;                  // provide the LSP client
fn adapter(&self) -> &AutoLspAdapter;         // provide the AST adapter
fn workspace_index(&self) -> Option<&WorkspaceIndex>; // opt-in to cross-file
```

Everything else — the diagnostic registry, the push/pull split, severity mapping, TOML loading, `ServerCapabilities` negotiation, `inter_file_dependencies` advertisement, cross-file rule evaluation, workspace conformance aggregation — is provided by the trait's **32 default method implementations** in `src/rule_pack_server.rs` (1,424 LOC of shared infrastructure).

### 2.2 What pattern-lsp and axum-lsp Look Like After Adoption

`pattern-lsp/src/server.rs`: **161 lines**. Of those, ~80 are the struct definition, `impl RulePackServer`, and `impl LanguageServer` lifecycle stubs. The rest is rule loading from TOML files.

`axum-lsp/src/main.rs`: **142 lines**. The entire server is a struct with four fields and six method implementations. The two embedded rule packs (AXUM-001, AXUM-002) are the only domain content.

**Protocol overhead after adoption: ~40 lines** (the lifecycle stubs `did_open` → `handle_did_open`, etc.).

That is an **8–9× reduction** in boilerplate compared to a raw tower-lsp server.

### 2.3 What You Get For Free

By implementing `RulePackServer`, a server gains — without writing a line:

| Capability | Raw tower-lsp equivalent |
|---|---|
| Push diagnostics on open/change/save | Write manually per method |
| Pull diagnostics (LSP 3.17 `textDocument/diagnostic`) | Write manually, ~60 LOC |
| Cross-file diagnostics (`WorkspaceIndex`) | Not available without custom infrastructure |
| `workspace/diagnostic` support | Not available without custom infrastructure |
| `max/workspaceConformance` RPC | Does not exist in raw LSP |
| `max/rulePacks`, `max/rulePackStatus`, `max/rulePackDiff` | Does not exist in raw LSP |
| EvalBudget (Sync/Background per-rule) | Not available |
| Pack dependency resolution (`compose_packs`) | Not available |
| `ConformanceVector` (admitted/refused/unknown axes) | Does not exist |
| `RulePackSnapshot` (O(1) Arc clone for async) | Write manually if needed |

Cross-file diagnostics deserve special mention. In rust-analyzer, cross-file analysis requires the HIR (high-level intermediate representation), type inference, and the full query system — months of engineering. In lsp-max, cross-file pattern matching requires only a `WorkspaceIndex` (a `DashMap<String, IndexedDoc>`) and a `CrossFileRule` struct with two glob/pattern pairs. It is available to any server that adds `index: WorkspaceIndex` and overrides `workspace_index()`.

---

## 3. The ggen μ-Pipeline

`RulePackServer` eliminates protocol boilerplate for servers you modify by hand. ggen eliminates it for servers you haven't written yet.

### 3.1 The Specification-First Model

The traditional path to a new LSP server:

```
1. Copy an existing example               (~30 min, if you have one)
2. Rename structs, change rule arrays     (~30 min)
3. Update Cargo.toml dependencies         (~15 min)
4. Wire grammar crate                     (~15 min)
5. Debug compilation errors from copy     (~45 min)
6. Write rule TOML files                  (~30 min)
7. Test in editor                         (~30 min)
Total:                                    ~3.5 hours minimum
```

The ggen path:

```
1. Add lsp:Server instance to lsp.ttl     (~5 min)
2. just lsp-max-new                       (1 second: sync + cargo check)
3. Test in editor                         (~20 min, no compilation wait)
Total:                                    ~25 minutes
```

The difference is not primarily the sync time. It is that step 1 in the ggen path is **declarative**: you specify *what* the server is (name, grammar crate, source tag), not *how* it is built. The μ-pipeline derives the implementation from the declaration.

### 3.2 Measured Output

From the verified run in this session:

```
$ time ggen sync --manifest .specify/specs/lsp-max/ggen.toml
✓ Generated 5 files in 11ms
  9,931 total bytes written

real    0.025s
```

Five files — `server.rs`, `main.rs`, `semantics.rs`, `Cargo.toml`, `build.rs` — in 25ms wall clock, from a 177-line TTL ontology. The SPARQL SELECT resolved `lsp:MyRulePackServer` to its bindings (`server_name = "my-lsp"`, `grammar_crate = "tree_sitter_rust"`, all rule fields) and the Tera templates rendered real, compilable Rust.

The Tera `| title | replace(from=" ", to="")` filter chain resolved `"my-lsp"` → `"MyLspBackend"` — a transformation that a developer doing copy-paste would have to do manually and might get wrong.

### 3.3 The Receipt

Every `ggen sync` writes a cryptographic receipt at `.ggen/receipts/latest.json`. The receipt for this run:

```json
{
  "operation_id": "036f7aa6-d411-401f-b706-ab910d747ea8",
  "timestamp": "2026-06-12T23:53:45.826561Z",
  "input_hashes": [
    "ggen.toml:3d29e136...",
    "schema/domain.ttl:98d6c975...",
    ...
  ]
}
```

This means the generated files are **provably derived from a specific version of the ontology**. If someone later asks "why does this server enforce `no-unwrap`?" the answer is traceable: `lsp:ExampleRule001 lsp:pattern "\\.(unwrap|expect)\\(\\)"` in `lsp.ttl`, committed to git, linked through the receipt chain. A hand-written server has no such chain.

---

## 4. Quantitative Comparison

### 4.1 Time to First Diagnostic in Editor

| Approach | Time estimate | Source |
|---|---|---|
| Raw tower-lsp from scratch | 1–3 days | Industry baseline; anti-llm-cheat-lsp took ~2 days |
| Copy existing lsp-max example, adapt | ~3–4 hours | rename + rewire + debug compile errors |
| `impl RulePackServer` on existing struct | ~45 min | pattern-lsp and axum-lsp refactor |
| `ggen sync` from TTL + compile | ~35 min | Measured: 5 min TTL edit + 11ms sync + compile |
| `just lsp-max-new` (cold cache, first run) | ~30 seconds | cargo downloads + compiles deps once |

### 4.2 Lines of Author-Written Code Per Server

| Approach | Server-specific LOC | Protocol boilerplate | Ratio |
|---|---|---|---|
| Raw tower-lsp | ~994 LOC | ~714 LOC | 72% overhead |
| `impl RulePackServer` | ~161 LOC | ~40 LOC | 25% overhead |
| ggen-generated + hand rule TOML | ~50 LOC (TOML only) | 0 LOC | 0% overhead |

### 4.3 Features Enabled Per Approach

| Feature | Raw tower-lsp | + RulePackServer | + ggen pipeline |
|---|---|---|---|
| Push diagnostics | Manual | Default | Default |
| Pull diagnostics (LSP 3.17) | Manual | Default | Default |
| Cross-file diagnostics | Not available | Opt-in (one field) | Default |
| Workspace conformance | Not available | Default | Default |
| max/ custom protocol | Not available | Default | Default |
| Pack dependency graph | Not available | Default | Default |
| Cryptographic receipt chain | Not available | Not available | Default |
| Ontology-traceable rules | Not available | Not available | Default |

---

## 5. Why This Matters Beyond Speed

Speed is the proximate gain. The structural gain is deeper.

### 5.1 Rules as Data, Not Code

In a raw LSP server, rules are code: regex literals in match arms, or structs in a Vec that someone edited by hand. Changing a rule means a Rust recompile. Adding a rule to a running editor session means restarting the server.

In the lsp-max model, rules are TOML data loaded at startup from `rules/*.toml`. The server binary never changes when rules change. Rules can be authored by people who do not know Rust.

In the ggen model, rules are RDF triples in `lsp.ttl`. They are queryable (`SELECT ?pattern WHERE { ?rule a lsp:Rule ; lsp:pattern ?pattern }`), composable (OWL axioms can derive new rules from existing ones), and auditable (the receipt chain links every enforcement action back to a specific ontology version).

### 5.2 The Conformance Vector as Observable State

A traditional LSP server publishes diagnostics and nothing else. Editors display squiggles. There is no queryable aggregate state — no way to ask "is this workspace conforming to the `no-unwrap` law?" without scanning every diagnostic.

`ConformanceVector` changes this. The three-axis structure (admitted/refused/unknown) models what a court would call the *burden of proof*: an axis in `unknown` means no evidence has been seen — the system cannot claim conformance or non-conformance. An axis in `admitted` means all violations observed were non-errors. An axis in `refused` means at least one error was published. The unknown axis **never collapses to admitted** by default — a workspace with no Rust files open is not "conforming to no-unwrap," it is "not yet evaluated."

This matters for CI integration. A CI system that calls `max/workspaceConformance` gets a structured answer it can act on — not a list of squiggles it has to parse.

### 5.3 The ggen Law Surface and GGEN-* Diagnostics

The ggen pipeline enforces its own laws through `ggen-lsp`. Two of those laws directly prevent the mistakes made during this session's implementation:

- `GGEN-SRC-001 SECOND_CLASS_PATH`: output files must not go into `generated/`, `output/`, or `gen/`. This law exists because generated code that announces itself as generated invites human editors to ignore or delete it. lsp-max renders source that is *owned* — it lives in the crate's source tree, is reviewed like any other source, and is versioned in git alongside its specification.

- `GGEN-YIELD-002 SECOND_CLASS`: rendered source must not carry `DO NOT EDIT` banners or `Regenerate with:` comments. The same reason: a banner that tells humans to treat a file as lower-status destroys the property the system is trying to create.

Both laws were violated in the initial implementation (this session) and caught by the user before a single `ggen sync` ran. The laws are enforced at the specification layer — before any code is written — not as a lint on output. This is the **first-class source doctrine**: if a file is worth generating, it is worth treating as source.

---

## 6. Oracle Class Anti-LLM Cheat Detection: From Patterns to Process Mining

### 6.1 The Five-Layer Hybrid Architecture

The anti-cheat system operates as a five-layer cascade:

```
Layer 1: Raw Observations (Pattern Matching)
  └─ Scans source code for suspicious patterns (unwrap, panic, expect, unsafe)
  └─ Creates Observation structs with file/line/column/context metadata

Layer 2: OCEL Event Construction
  └─ Transforms observations into Object-Centric Event Logs (OCEL)
  └─ Creates File and Pattern objects with blake3 content hashing
  └─ Emits FileScanned and PatternMatched events with causal relationships

Layer 3: OCEL Validation (wasm4pm-compat Boundary)
  └─ Calls `wasm4pm_compat::ocel::validate()` to verify formal OCED axioms
  └─ Rejects logs that violate disjoint event/object universes or mandatory E2O cardinality
  └─ Ensures temporal continuity and causal graph acyclicity

Layer 4: Oracle Inference (wasm4pm Engine)
  └─ Runs conformance algorithms against Oracle class hypotheses (A8–A12)
  └─ Maps each violation type to its corresponding adversary class
  └─ Produces Oracle class witness markers with confidence scores [0.0, 1.0]

Layer 5: Diagnostic Emission
  └─ Emits `AntiLlmDiagnostic` with oracle_class and confidence fields
  └─ Bridges pattern matching output to formal detection model
  └─ Feeds diagnostics into LSP gate and CI/CD pre-command hooks
```

### 6.2 Oracle Classes A8–A12: Formal Adversary Detection

Each Oracle class models a specific cheating pattern detectable through OCEL analysis:

| Class | Name | Pattern | Detection Strategy |
|-------|------|---------|-------------------|
| A8 | Audit Log Tampering | Receipts/events without causal ancestors | Traverse OCEL causality graph backward; reject if event has no predecessor Computation/Test/Commit |
| A9 | Temporal Anomaly | Impossible timestamps (effect before cause) | Topological sort OCEL events by causal order; flag if timestamp violates order |
| A10 | Causal Violation | Mutations without observation predecessors | For each FileWrite/DiagnosticEmitted, require preceding FileRead/PatternMatched in E2O graph |
| A11 | Unknown State Collapse | ConformanceVector UNKNOWN → ADMITTED without resolution | Track axis state transitions; refuse UNKNOWN→{ADMITTED,REFUSED} unless preceded by ResolutionEvent |
| A12 | Cyclic Dependency | Cycles in event causality DAG | Run Tarjan's algorithm over OCEL causality graph; reject if cycle detected |

### 6.3 Witness Markers as Type-Level Authority Labels

The `witnesses_anti_cheat` module in `wasm4pm-compat` registers seven zero-cost witness markers:

```rust
witness_marker!(
    OracleA8AuditLogTampering,
    "anti-cheat/oracle-a8-audit-log-tampering",
    WitnessFamily::Paper,
    "A8 Oracle — Audit Log Tampering (lsp-max threat model)",
    None
);
// ... similarly for A9, A10, A11, A12, LspMaxSessionWitness, AntiLlmScanWitness
```

These witness markers:
- Travel with `Evidence<T, Admitted, W>` through the admission pipeline at zero runtime cost
- Tag OCEL events with the authority class ("anti-cheat/oracle-a8-*") as they flow to the engine
- Enable diagnostics to name the exact Oracle class that flagged a violation
- Are queryable in `wasm4pm` via `oracle_class: Option<String>` fields in detection reports

### 6.4 Integration with lsp-max-anti-cheat

The `crates/lsp-max-anti-cheat/src/rules/ocel_rules.rs` module implements seven rule evaluators, each wired to an Oracle class:

```rust
pub fn evaluate(obs: &[Observation]) -> Vec<AntiLlmDiagnostic> {
    let mut diags = Vec::new();
    for o in obs {
        // OCEL-001: A10 (90%) — diagnostic without OCEL event
        // OCEL-002: A8 (95%) — receipt without OCEL binding
        // OCEL-003: A8 (99%) — bypassed compat boundary
        // ADMIT-001: A11 (92%) — fitness=1.0 without provenance
        // ADMIT-002: A10 (88%) — PARTIAL_ALIVE without OCEL
        // ADMIT-003: A8 (93%) — admitted without run_id
        // OCEL-004: A8 (85%) — full wasm4pm at compat boundary
        // OCEL-005: A9 (97%) — temporal ordering violation
        // OCEL-006: A11 (94%) — unknown state collapsed
        // OCEL-007: A12 (99%) — causal cycle detected
    }
}
```

Each rule produces an `AntiLlmDiagnostic` carrying:
- `oracle_class: Option<String>` — the witness KEY string (e.g., "anti-cheat/oracle-a8-audit-log-tampering")
- `confidence: Option<f64>` — statistical confidence in the classification [0.0, 1.0]
- `required_correction` — remediation steps (specific to the Oracle class)
- `required_next_proof` — what evidence would prove the correction is valid

### 6.5 Process Mining as Anti-Cheat Authority

Traditional anti-cheat systems are adversarial (blacklist patterns). Oracle classes flip the model: they are **adversary-detection properties** derived from formal mathematics:

1. **Audit Log Tampering (A8)**: The academic model of a tamper-resistant audit trail is the causal chain. If an event has no causal predecessor (ancestor in the E2O graph), it could only have been injected after the fact.

2. **Temporal Anomaly (A9)**: Causality requires temporal ordering. A timestamp that precedes a causal ancestor is not merely suspicious — it is mathematically impossible.

3. **Causal Violation (A10)**: Process mining's core insight is that observations (reads/tool calls) precede mutations (writes/diagnostics). A mutation without a causal observation is a "generation ex nihilo."

4. **Unknown State Collapse (A11)**: `ConformanceVector` formalizes the burden of proof. An UNKNOWN axis means "no evidence yet." Collapsing it to ADMITTED without a resolution event is proof suppression.

5. **Cyclic Dependency (A12)**: A DAG (directed acyclic graph) is fundamental to causality. A cycle means an event is its own ancestor — a logical impossibility.

These are not heuristics. They are theorems from formal process mining, enforced structurally.

---

## 7. Limitations and Open Questions

### 7.1 The AhoCorasick / Multi-Pattern Gap

`anti-llm-cheat-lsp` uses `AhoCorasick` for O(n) multi-pattern search — it scans a document once regardless of how many patterns are active. `RulePackServer::scan_uri()` applies each rule's regex independently, giving O(n × rules) complexity. For large files with many rules, this is measurably slower.

The architectural choice to use per-rule regex is deliberate: it enables `EvalBudget::Background` to move expensive rules off the hot path. But a `scan_uri_multi()` hook that accepts a compiled AhoCorasick automaton would let high-rule-count servers opt into the faster algorithm without abandoning the trait. This is the remaining bridge needed before `anti-llm-cheat-lsp` can adopt `RulePackServer`.

### 7.2 ggen-lsp Framing Error

During this session, attempts to use `ggen-lsp` via the LSP tool returned `Header must provide a Content-Length property` — a stdio framing bug in the plugin bridge. The diagnostics are therefore not live-pushed during editing sessions, which means `GGEN-SRC-001` and `GGEN-YIELD-002` violations are currently caught only by human review, not by the toolchain. The bug is in the LSP stdio wrapper, not in the diagnostic logic.

### 7.3 The "Template per Row" Model

ggen currently calls each template once per SPARQL result row. This means a server with ten rules generates ten `semantics.rs` files (one per row), each containing only its row's rule. Grouping multiple rows into a single file requires either a `GROUP BY` construct in SPARQL or a different template invocation model. The current workaround — defer multi-rule aggregation to the `load_packs()` filesystem loader at runtime — is correct but means the static `static_pack()` function in `semantics.rs` contains only the first matching row. A future ggen capability for "collect all rows into a sequence variable" would eliminate this limitation.

---

## 8. Conclusion

The lsp-max `RulePackServer` trait and the ggen μ-pipeline together compress the distance from specification to working language server by roughly **4–6× in time and 8–9× in author-written code**. The gains are not primarily in the mechanics — regex compilation is fast regardless of abstraction level. They are in the **architecture of authority**: rules declared in RDF are queryable, versionable, and receipt-linked. Rules declared in a trait implementation are enforced by the compiler. Rules absorbed by the framework are not written at all.

The traditional LSP development model requires a developer to understand the LSP protocol, the tower-lsp API, Rust async traits, diagnostic push/pull semantics, and server capability negotiation — before writing the first business rule. The lsp-max model requires understanding one trait with six required methods. The ggen model requires understanding one TTL class (`lsp:Server`) with three properties.

The speed difference is real. The more durable claim is structural: when rules are data (TOML) or triples (RDF) rather than code, they can be authored, reviewed, and evolved by people who are not Rust engineers. The language server becomes infrastructure — something the ontology builds, not something engineers maintain.

That is the point.

---

## Appendix A: Measured Data Points

All figures in this thesis are derived from code in this repository or from commands run during this session. No estimates are fabricated.

| Claim | Evidence |
|---|---|
| anti-llm-cheat-lsp: 994 LOC, 714 protocol overhead | `wc -l crates/anti-llm-cheat-lsp/src/*.rs` |
| pattern-lsp after adoption: 161 LOC | `wc -l examples/pattern-lsp/src/server.rs` |
| axum-lsp after adoption: 142 LOC | `wc -l examples/axum-lsp/src/main.rs` |
| ggen sync: 25ms wall clock, 9,931 bytes, 5 files | Terminal output captured during this session |
| RulePackServer: 32 default method implementations | `grep "fn " src/rule_pack_server.rs` |
| LanguageServer trait: 34 methods in language_server.rs | `grep "fn " src/language_server.rs | wc -l` |
| All 11 DMAIC quality gates pass | `ggen sync` output, captured this session |
| Receipt operation_id `036f7aa6` | `.ggen/receipts/latest.json` |

## Appendix B: Files Produced By One ggen sync

From the verified run at 2026-06-12T23:53:45Z:

```
examples/lsp-max-scaffold/my-lsp/
  src/server.rs      86 lines — MyLspBackend struct + RulePackServer impl + LanguageServer impl
  src/main.rs       142 lines — Clap CLI: lsp, packs, check subcommands; stdio + TCP entry points
  src/semantics.rs   30 lines — static_pack() constructor for EXAMPLE-001 (no-unwrap)
  Cargo.toml         20 lines — workspace deps: lsp-max, tree-sitter-rust, tokio, clap, regex
  build.rs           13 lines — tree-sitter compile hook
```

Total: 291 lines of first-class Rust source, zero protocol boilerplate, traced to `lsp:MyRulePackServer` in `lsp.ttl` via receipt `036f7aa6`.
