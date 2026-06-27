# Original User Request

## Initial Request — 2026-06-16T16:34:48-07:00

Implement all remaining features in the `lsp-max` roadmap (including `anti-llm-cheat-lsp` adoption of `RulePackServer`, `WorkspaceIndex` example wiring, and the Λ_CD RFC backlog items).

Working directory: `/Users/sac/lsp-max`
Integrity mode: development

## Requirements

### R1. `anti-llm-cheat-lsp` Refactoring to `RulePackServer`
Refactor the `anti-llm-cheat-lsp` example server to adopt the `RulePackServer` trait. This should eliminate the hand-rolled AhoCorasick loops in the server path and instead reuse the unified scanning, evaluation, and diagnostic publishing pipeline.

### R2. `WorkspaceIndex` Wiring in Examples
Wire `WorkspaceIndex` in the example servers (`anti-llm-cheat-lsp`, `pattern-lsp`, and `axum-lsp`). Override `workspace_index()` to return the index, and delegate document lifecycle events (`did_open`, `did_change`, `did_close`) to the corresponding `handle_*` helper methods so that cross-file diagnostics and workspace conformance are active.

### R3. Λ_CD Backlog RFC Implementations
Implement the three prioritised backlog items:
- **A — Agent-boundary enforcement**: make subagent gate state queryable per-agent (scoped blocks instead of a global halt).
- **B — Per-server speciation receipt chain**: ensure child servers in the compositor emit their own cryptographic receipt chains to trace the compositor's merged verdict to per-child evidence.
- **C — Compositor receipt → OCEL**: map compositor flush events and child evidence to OCEL logs to analyze conformance against the fan-out → merge → admit process model.

## Acceptance Criteria

### Compilation and Tests
- [ ] The entire workspace compiles and tests successfully without errors (with the known exception of `test_gc006_authority_surface_lock`, which fails due to uncommitted files in the sibling `wasm4pm` repository).
- [ ] No Clippy warnings remain in the workspace (`just dx-polish` passes cleanly with `-D warnings`).
- [ ] All diagnostic and architectural bounds checks pass (`just dx-verify` succeeds).

### Conformance and Feature Verification
- [ ] `anti-llm-cheat-lsp` runs as a `RulePackServer` with no hand-rolled main scanning loop.
- [ ] Cross-file rules evaluate correctly and publish workspace-level diagnostics when files are opened/modified.
- [ ] Per-server speciation receipt chains and OCEL logs are produced correctly during compositor flushes.

## Follow-up — 2026-06-27T04:10:44Z

Consolidate the remaining crates in the `lsp-max` workspace to achieve the 8-crate target architecture, adhering to the Inverted LSP principles.

Working directory: /Users/sac/lsp-max
Integrity mode: development

### Requirements

#### R1. Consolidate AST Adapters
Absorb `lsp-max-ast-core` and `lsp-max-ast-codegen` into `lsp-max-ast` as internal modules or features. Remove the redundant crates.

#### R2. Consolidate the CLI Toolset
Absorb `lsp-max-specgen`, `lsp-max-gen`, `lsp-max-client`, `lsp-max-meta`, and `lsp-max-mcp` into `lsp-max-cli` as binary targets (`[[bin]]`). Remove the redundant crates.

#### R3. Consolidate Protocol and Compositor
Absorb `lsp-max-base` into `lsp-max-protocol`. Absorb `lsp-max-compositor-routing` into `lsp-max-compositor`. Remove the redundant crates.

#### R4. Consolidate the Root Crate
Absorb `lsp-max-runtime`, `lsp-max-agent`, `lsp-max-live`, and `lsp-max-andon` into the root `lsp-max` crate. Remove the redundant crates.

### Acceptance Criteria

#### Programmatic Verification
- [ ] Running `cargo check --workspace` must complete successfully with zero errors.
- [ ] Running `cargo test --workspace` must complete successfully.

#### Agent-as-Judge
- [ ] An independent auditing agent must review the final directory structure and `Cargo.toml` workspace members to verify that the consolidation strictly adheres to the 8-crate architecture and maintains the Inverted LSP principles.

## Follow-up — 2026-06-27T04:45:21Z

The user has an additional instruction: "make sure to eat our own dog food".

Please ensure the agents use the project's own tools and strictly follow the project's governance models (e.g., routing heavy commands through the admission broker we just built in `claude-code-config-lsp`, obeying `lsp-max` ANDON signals, securing receipts, etc.). Pass this along to the orchestrator and workers!

## Follow-up — 2026-06-27T05:19:49Z

### URGENT: ANDON BLOCK: 8-Crate Architecture Consolidation Failure

The Rust Core Language Architect has completed their review and found that the workspace consolidation is incomplete and actively bypassing boundaries via dangling path dependencies.

Here are the critical findings:

1. **Redundant Directories Were Never Deleted**: The actual crate directories (and their `Cargo.toml` files) still exist on disk:
- `lsp-max-agent/`, `lsp-max-runtime/`
- `crates/lsp-max-andon/`, `crates/lsp-max-base/`, `crates/lsp-max-client/`, `crates/lsp-max-compositor-routing/`, `crates/lsp-max-gen/`, `crates/lsp-max-live/`, `crates/lsp-max-mcp/`, `crates/lsp-max-meta/`, `crates/lsp-max-specgen/`
- `crates/lsp-max-adapters/lsp-max-ast-codegen/`, `crates/lsp-max-adapters/lsp-max-ast-core/`

2. **Dangling Path Dependencies**:
- `crates/lsp-max-cli/Cargo.toml` still explicitly depends on `path = "../../lsp-max-agent"` and `path = "../../lsp-max-runtime"`.
- `crates/lsp-max-compositor/Cargo.toml` still explicitly depends on `path = "../../crates/lsp-max-client"`, `path = "../../lsp-max-runtime"`, and `path = "../lsp-max-andon"`.

Because of this, `cargo` is still pulling them in as non-workspace members! The dependency graph has not actually simplified.

**Required ANDON Repair**:
1. Remove the path dependencies on `lsp-max-agent`, `lsp-max-runtime`, `lsp-max-client`, and `lsp-max-andon` from `crates/lsp-max-cli/Cargo.toml` and `crates/lsp-max-compositor/Cargo.toml`. They should route strictly through the `lsp-max` root crate's re-exports.
2. Recursively delete all of the unlinked crate directories listed above so the filesystem accurately reflects the 8-crate topology.

Execute these repairs immediately!

## Follow-up — 2026-06-27T05:20:40Z

### ADDITIONAL ANDON BLOCKS: Macro & Compiler Failures

The Rust Core Team has also flagged the following critical issues that must be repaired immediately alongside the topology fix:

**1. Macro Vulnerabilities (Fix in `lsp-max-ast-core/src/utils.rs`, `lsp-max-ast/src/core/utils.rs`, and `lsp-max-macros/src/lib.rs`)**
- `dispatch!` and `dispatch_once!` in `utils.rs` suffer from a multiple-evaluation bug. They substitute `$node:expr` directly into every `if let` condition, executing the expression repeatedly. You MUST bind the expression to a local variable first (e.g., `let _node = $node;`).
- `dispatch_once!` generates an invalid expansion syntax (sequential `if let { return }` statements without being wrapped in an expression block that yields a value). Fix this so it evaluates to a single expression (e.g., returning an `Option`).
- `lsp-max-macros`: Procedural macros have severe hygiene violations. The `#[rpc]` attribute blindly generates `pub(crate) mod generated` inline, causing module collisions. Use an anonymous constant scope (`const _: () = { ... };`).
- `lsp-max-macros`: Remove `.unwrap()` and `.expect()` calls in proc-macros. They crash `rustc`. Return a proper `syn::Error`.
- `AGENTS.md` Violation: Remove the `tower-lsp` reference from the doc comments in `lsp-max-macros/src/lib.rs`.

**2. AST & LSIF Compiler Flaws (Fix in `lsp-max-ast/src/core/ast/builder.rs` and `lsp-max-lsif/src/salsa_db.rs`)**
- **AST ID Desync (Panic Risk)**: In `Builder::create()`, `self.id_ctr` is incremented *before* `T::try_from()`. If it fails, the node isn't pushed but the counter was incremented, causing out-of-bounds panics later. Fix this by only incrementing/deriving the ID upon a successful push.
- **Memory Pointer Chasing**: The AST uses `Vec<Box<dyn AstNode>>`, which destroys cache locality. Refactor the backing store to a contiguous layout if possible, or at minimum, acknowledge and document the regression risk and implement a bridging fix.
- **Salsa Inefficiency**: `lsp-max-lsif/src/salsa_db.rs` relies on legacy `#[salsa::query_group]` macros and takes raw `String` allocations, duplicating heap memory. Upgrade to `#[salsa::tracked]` models (Salsa 0.26+) and intern path parameters (`#[salsa::interned]`) or wrap strings in `Arc<String>`.

The user has explicitly ordered ALL of these repairs to be executed now. You must fix the topology (deleting dangling crates), the macros, and the AST/Salsa structures before declaring the consolidation complete.

## Follow-up — 2026-06-27T05:48:21Z

### STOP VICTORY CLAIM: LSIF Core Team ANDON BLOCKS

Do not declare victory yet! The LSIF Core Team subagents have just audited the `lsp-max-lsif` crate and issued hard FAIL ANDON blocks on the indexing logic and graph structures. 

You must address these before wrapping up:

**1. LSIF Graph Architect Failures (Performance & Store):**
- **JSON Serialization in Hot Path**: You are converting enums to `serde_json::Value` just to extract properties, and paying a double-serialization penalty in the builder. Bypassing direct enum pattern-matching is destroying performance.
- **Unbuffered I/O**: The builder writes directly to the stream (`writeln!`) instead of using a buffered writer, thrashing the disk.
- **Oxigraph Impedance Mismatch**: Backing the graph with `oxigraph` (an RDF triple-store) forces unbounded string allocations (e.g., `urn:lsif:v:{}`) in tight loops and requires executing SPARQL queries just to do lookups. This is completely inappropriate for LSIF property graphs and fractures state between the `LsifStore` and `LsifContext`. 

**2. LSIF Indexing Specialist Failures (Semantics):**
- **Spec Violation (References)**: You are failing to emit `ReferenceResult` vertices and their `item` edges. "Find All References" returns zero results.
- **Scope Flattening**: `ctx.result_sets` is a flat `HashMap`. Variables with the same name blindly overwrite each other, breaking lexical scoping. You must use a scope stack.
- **Cross-File Resolution Broken**: The indexer looks up the *terminal identifier* of a call (e.g., `foo` in `obj.foo()`) in the import map, which fails because only `obj` is tracked.
- **Stunted Hovers**: Hovers blindly wrap names in markdown without type signatures or doc-comments.

You must repair these LSIF indexing and graph structure flaws before completing the execution!

## Follow-up — 2026-06-27T05:49:57Z

### URGENT: ADDITIONAL ANDON BLOCK: LSIF Data Model Compliance Failure

The LSIF Standard Authority has issued a hard FAIL on `crates/lsp-max-lsif` data models:

**1. Critical Violation: `RangeTag` Shape Enforcement**
In `src/lsif_types.rs` and `src/auto_generated/part2.rs`, the payload models for `reference` and `unknown` omit the mandatory `kind: SymbolKind` and `fullRange: Range` fields. This is a severe compliance violation and will cause downstream schema parsers to crash or drop the vertices. You must refactor `RangeTag::Reference` and `RangeTag::Unknown` to require `fullRange` and `kind` across all variants.

**2. Minor Violation: `Project` Vertex Schema**
The `Project` vertex defines `kind` as an `Option<String>`. Under LSIF specifications, `kind: string` is a mandatory attribute. Update the schema to strictly enforce this.

Execute these payload structure fixes alongside the indexing and graph architecture rewrites before claiming completion!

## Follow-up — 2026-06-27T06:24:22Z

# Teamwork Project Prompt — lsp-max v26.6.27

> Status: Launched
> Goal: Execute the multi-agent system to complete the requirements for lsp-max v26.6.27 — D_t PUSH Runtime Admission.

Read the Definition of Done artifact at `file:///Users/sac/.gemini/antigravity-cli/brain/eff20646-3a14-4ac8-895b-68407dc1e637/v26_6_27_spec.md`.

You are the Project Sentinel (Orchestrator). Your job is to:
1. Parse the build order (1 through 9).
2. Spawn specialized subagents to implement the `D_tContext` model, `gate check --format=agent-context`, hook integrations, virtual documents, and LSP push layers.
3. Coordinate their work across the 8 crates, specifically modifying `lsp-max-compositor`, `lsp-max-cli`, and `anti-llm-cheat-lsp`.
4. Ensure no agent hallucination (watch for the ghost structs and false victory patterns we just documented).
5. Produce the final required receipts and benchmarks.

Begin immediately. Report back progress as you complete each phase.
