# lsp-max Chain Theory: A Law-State Runtime Projected Through LSP

## Abstract

lsp-max is a multi-server LSP compositor whose correctness claims are grounded in process-mining conformance rather than test-passing alone. This document states the theoretical basis for three interacting subsystems: the compositor's fan-out/merge pipeline, the Claude Code hook lifecycle that wires LSP server management to agentic sessions, and the MCP bridge that makes routing decisions queryable at hook-call time. The central claim is that an LSP runtime whose admission is backed by Declare constraints, DFG fitness scores, and cryptographic receipt chains can be monitored and governed at the same granularity as any industrial process model. Status throughout is bounded: ADMITTED, CANDIDATE, BLOCKED, REFUSED, UNKNOWN, PARTIAL, OPEN.

---

## 1. Problem Statement: Why Multi-LSP Routing Is Hard

The LSP specification assumes a one-to-one relationship between a workspace extension and a language server. A client opens a `.rs` file; one server answers. This model collapses under three real pressures.

**The one-LSP-per-extension constraint.** Most editors bind a file extension to exactly one active server at a time. When two servers both claim `.rs` — a Rust analyzer for completions and an `anti-llm-cheat-lsp` for policy diagnostics — the editor resolves the conflict silently, usually by discarding the later-registered server. The losing server's diagnostics never reach the user. There is no protocol mechanism to express "this server contributes diagnostics only; defer to another for hover."

**Config drift.** LSP server registrations are declared at editor startup, loaded from static config files, and not re-evaluated during a session. Adding a server requires restarting the editor. In agentic environments — where an agent may spin up a domain-specific server mid-session to analyze a newly encountered file type — the static config cannot reflect the session's actual server population. The routing table and the running server set diverge.

**Lifecycle management.** Servers are spawned once, at startup, and kept alive for the session. There is no standard mechanism for spawning a server in response to a file being opened, for reaping a server when its domain is no longer active, or for promoting a CANDIDATE server to ADMITTED status after it passes a conformance gate. Lifecycle events — spawn, ready, degraded, shutdown — are invisible to the editor.

These three pressures combine into a single failure mode: a workspace that needs N domain-specific servers ends up with M < N actually routing, the routing table cannot be inspected, and no conformance record exists to verify that the running configuration matches the intended one. lsp-max addresses all three by moving routing, lifecycle, and conformance out of the editor and into a compositor layer that the editor treats as a single server.

---

## 2. Architecture: The Compositor as a Law-State Projection

lsp-max interposes between the editor client and N child LSP servers. From the editor's perspective there is one server. From the law-state runtime's perspective there is a compositor that maintains a `ChildTier`-stratified registry, fans out requests in parallel, and merges responses according to capability tier and ANDON gate state.

### 2.1 Tier Stratification

The registry assigns each child server a `ChildTier`:

- **Primary** (`"full"` or `"semantic"` priority): handles hover, completion, and definition. `FirstSuccess` routing — the first ADMITTED Primary server's response wins.
- **Secondary**: participates in hover/completion but at lower priority.
- **DiagnosticsOnly**: receives `textDocument/did*` notifications and publishes diagnostics; excluded from completion/hover fan-out.

The tier structure is static, read from `lsp-max.toml` at compositor startup. It is not runtime-negotiated. Extension-to-server routing is config-driven, not discovered at runtime. This is law boundary one; see Section 6.

### 2.2 Fan-out and Merge

When a `textDocument/didChange` or `textDocument/publishDiagnostics` message arrives, the compositor fans it to all registered servers concurrently. Each server deposits its diagnostics into a per-URI `DiagnosticBuffer`. The `FlushCoordinator` collects deposits and fires a flush when either quorum is reached (all expected servers have deposited for that URI) or an adaptive timeout expires. The adaptive timeout is `last_at + clamp(2 × spread, 1ms, 30ms)`, where spread is the interval between first and last arrival. At quorum the flush fires immediately; no artificial latency accumulates.

The merge step produces a `MergeResult` carrying a deduplicated diagnostic list and a boolean `has_andon_block`. Deduplication preserves `REFUSED_BY_LAW` codes — a diagnostic issued under law authority is never discarded by dedup, even if an identical code appears from multiple servers.

The `CompositorReceipt` records every flush: which URI, how many diagnostics, whether an ANDON block was present, which law-prefix set governed the merge (as a deterministic fingerprint of sorted prefix strings), and the RFC-B per-child `ChildEvidence` links when the `full` feature is enabled. A receipt with `has_andon_block = true` carries status `BLOCKED`; a receipt without it carries status `ADMITTED`. A BLOCKED receipt must not be used as admission evidence. This is enforced structurally — the `status()` method on `CompositorReceipt` returns `ReceiptStatus::Blocked` without a caller override path.

### 2.3 The LSP Surface Is Read-Only

The compositor emits diagnostics, hover responses, code actions, and conformance reports. It does not write files. It does not execute repairs. It does not mutate workspace state. This is law boundary two: the LSP surface is a read-only projection of law state. Repair proposals are emitted as `CodeAction` items; the user or agent accepts them; a separate write-capable tool executes the mutation. The compositor observes; it does not act.

---

## 3. The Auto-Wiring Theory: Hook System as LSP Lifecycle

Claude Code's hook system defines a lifecycle with five named phases: `SessionStart` (with `startup`, `resume`, `clear`, and `compact` matchers), `PreToolUse`, `PostToolUse`, `SubagentStart`, and `SubagentStop`. These phases are not incidental to LSP server management — they map cleanly onto the server lifecycle that the config-drift and lifecycle-management problems require.

### 3.1 Phase Mapping

**SessionStart:startup** fires exactly once on a fresh container session, before any tool is invoked. This is the discovery phase: no workspace state exists yet, so the hook can scan the workspace, identify which LSP servers are available (by checking binary presence, `Cargo.toml` membership, or explicit config), and write `.claude/lsp-max-auto.toml` with the discovered server set. The `startup` matcher does not fire on `resume` or `clear`, which prevents re-discovery from overwriting a session's accumulated state.

**PreToolUse** fires before every Bash, Edit, Write, TaskCreate, and NotebookEdit call. The ANDON gate is wired here: `gate-check.sh` invokes `lsp-max-cli gate check`, which exits 1 if any active WASM4PM-\*, ANTI-LLM-\*, or GGEN-\* diagnostics are present. Exit 1 from a PreToolUse hook (specifically, exit code 2 in the blocking variant) prevents the tool from executing. The gate is therefore not advisory — it is a hard interlock between the law-state runtime and the agentic write surface.

**PostToolUse** fires after every Bash, Edit, or Write call. The snapshot hook runs `lsp-max-cli diagnostic snapshot`, capturing the current diagnostic state as a timestamped artifact. This snapshot is not a receipt — it is an observation. Receipts require boundary markers, digest chains, and negative-control results; snapshots are lightweight audit points.

**SubagentStart / SubagentStop** (CANDIDATE confidence, per the adversarial verification) enable per-subagent LSP server lifecycle management. If a subagent is responsible for a specific domain — say, OCEL conformance analysis — a SubagentStart hook could spawn the corresponding server and register it with the compositor. SubagentStop would reap it. This maps the LSP server lifecycle onto the agent task boundary, eliminating the zombie-server problem (servers that outlive their domain) and the cold-start problem (servers not yet running when the first request arrives).

### 3.2 The Discover-Merge Pipeline

The `CompositorConfig::load_with_auto()` method reads both `lsp-max.toml` (static, user-managed) and `.claude/lsp-max-auto.toml` (dynamic, written by the SessionStart hook) and merges them. The merge rule is: static entries win on `id` collision; auto-discovered entries with novel IDs are appended. This preserves user intent while enabling automatic extension of the server set for domains the user did not anticipate.

The auto-discovery script (`discover-lsp-chains.sh`, OPEN — not yet present in the repository) would scan the workspace for LSP server binaries, test their availability, and write `[[server]]` entries to `.claude/lsp-max-auto.toml`. The `CompositorConfig::find_auto_config()` method walks upward from the working directory to the workspace root looking for this file, matching the same traversal strategy used for `lsp-max.toml`.

Hot-reload is the remaining gap: once the compositor is running, a change to `lsp-max.toml` is not picked up without a restart. A `FileChanged` hook on `lsp-max.toml` calling `lsp_reload_config` (via the MCP bridge, Section 4) would close this gap. Status: OPEN.

---

## 4. The MCP Bridge: Routing as a Queryable Service

The fundamental problem with static LSP routing is that it cannot be interrogated at the moment a routing decision is made. The `lsp-max.toml` file says which servers exist; it says nothing about which servers are currently healthy, which have passed their conformance gates, or what the routing table looks like after auto-discovery merging.

An MCP server (`crates/lsp-max-mcp/`, OPEN — not yet present) makes these questions answerable. MCP servers are first-class hook participants: a hook handler of type `mcp_tool` invokes an MCP server tool directly, at hook-call time, with the tool result available to the hook's logic. This means a PreToolUse hook can ask the routing layer "which server handles `.ocel.json` files?" and receive a structured response before any edit proceeds.

The proposed tool surface:

- `lsp_discover` — scan the workspace and return the current auto-discovery result as structured TOML. Used by the SessionStart hook to generate `.claude/lsp-max-auto.toml`.
- `lsp_route` — given a method and optional file extension, return the `RoutingDecision` (Route, Fanout, or Unroutable) and the law status of the selected server. Used by agentic routing logic when an agent needs to know which server to address.
- `lsp_health` — return the health status of each registered child server. Used by monitoring loops and SubagentStop cleanup hooks.
- `lsp_reload_config` — signal the compositor to re-read its config files and rebuild the `ExtensionRouter`. Used by the FileChanged hook on `lsp-max.toml`.

The routing table (`RoutingTable` in `routing.rs`) already carries law status per server: the `resolve()` method prefers ADMITTED servers over CANDIDATE servers over everything else. Making this table queryable via MCP means that agents and hooks share the same routing view as the compositor, eliminating split-brain between the hook's assumptions and the compositor's actual configuration.

Named subagent definitions (in `.claude/agents/`, OPEN) can declare per-subagent `mcpServers` frontmatter pointing at `lsp-max-mcp`. This enables native fan-out: each subagent has its own compositor view, routed through its own server subset, with `parent_tool_use_id` tracking providing the lineage chain that ties subagent diagnostics back to the parent request. The agent hierarchy becomes the fan-out hierarchy.

---

## 5. Process-Mining Conformance: Verifying the Chain, Not Just Running It

A running multi-LSP chain is not a conformant one. A chain that consistently fails to produce `CompositorFlushAdmitted` after every `CompositorFlush` — perhaps because an ANDON gate fires on every document — is running but violating its normative process model. DFG fitness and Declare constraint checking detect this class of failure that unit tests cannot.

### 5.1 DFG Fitness

The `DirectlyFollowsGraph` is built from OCEL 2.0 events accumulated by the `FlushCoordinator`. Each flush emits a `CompositorReceipt::to_ocel_event()` call that records the flush as a `CompositorFlush` event with URI, status, diagnostic count, and ANDON state. After each flush, the coordinator calls `DirectlyFollowsGraph::from_traces()` on the accumulated event log and computes fitness against the normative arc set.

Fitness is defined as the fraction of observed directly-follows arcs that appear in the normative model. A perfectly fit chain produces only arcs that the normative model specifies. Observed arcs that do not appear in the normative model reduce fitness, indicating unmodeled behavior — paths the process specification did not anticipate. Precision, the complement, measures the fraction of normative arcs that appear in the observed log; low precision indicates under-exercise of the specified process.

Neither metric alone is sufficient. High fitness with low precision means the chain is constrained but the normative model is over-broad — it permits arcs that never occur. High precision with low fitness means the observed behavior includes paths outside the specification — the chain is doing things the model does not sanction.

### 5.2 Declare Constraints

The `DeclareModel::compositor()` normative model encodes five constraints over the compositor flush pipeline:

- `init(CompositorFlush)` — every trace must begin with a flush event.
- `response(CompositorFlush, CompositorFlushAdmitted)` — every flush must eventually be followed by admission.
- `not_co_existence(CompositorFlushAdmitted, CompositorFlushBlocked)` — a flush cannot be both admitted and blocked in the same case.
- `responded_existence(CompositorFlushBlocked, AndonCodePresent)` — if a flush is blocked, an ANDON code must be present in the same trace.
- `precedence(CompositorFlush, AndonCodePresent)` — an ANDON code cannot appear before the flush that generated it.

These five constraints are not aspirational. They encode invariants that the implementation already enforces: a `BLOCKED` receipt implies `has_andon_block = true`, which implies at least one ANDON code was active, which implies a flush occurred first. The Declare model makes these invariants checkable against the event log after the fact, without access to source code. An agent or CI step can verify process conformance by checking the OCEL log against the model — no Rust compilation required.

The anti-llm detection pipeline has its own normative model (`DeclareModel::anti_llm_detection()`), encoding that `ScanComplete` initiates every detection trace, that every `CheatDetected` event must be followed by `FailsetUpdated`, that `DetectionClaim` must be directly followed by `NegativeControlExecuted` (chain-response), and that `VictoryLanguageEmitted` must never appear. The absence constraint on `VictoryLanguageEmitted` is the process-mining encoding of the no-victory-language law.

### 5.3 Conformance Scores as Gate Inputs

Fitness scores are currently logged via `tracing::debug!` after each flush. They are not yet wired into the ANDON gate predicate. A flush pipeline with fitness below a threshold (e.g., 0.7 against the compositor normative model) indicates structural deviation from the specified process and should trigger an ANDON signal. The mechanism for this — a fitness threshold in `CompositorConfig` and a gate-write path in `FlushCoordinator` — is OPEN.

---

## 6. Law-State Boundaries: Three Invariants

Three invariants govern the system. They are not implementation guidelines; they are architectural boundaries enforced by tooling.

**Invariant 1: Routing is config-driven, not runtime-discovered.** The `ExtensionRouter` is built at startup from `CompositorConfig`. It does not accept runtime registration of new servers via LSP or MCP calls. Auto-discovery writes to `.claude/lsp-max-auto.toml`, which is read at the next `load_with_auto()` call — typically at the next compositor startup or after a `lsp_reload_config` call. There is no live-injection path. This prevents a class of attack where a malicious document causes the compositor to route to a server it did not intend to include.

**Invariant 2: `Unknown` never collapses into `Admitted` or `Refused`.** The `ConformanceVector` carries three sets: `admitted`, `refused`, and `unknown` law-axis sets. An axis in `unknown` means the evidence necessary to assign it to either polarity is absent — a transcript is missing, a negative control has not been run, or a receipt chain has not been verified. `Unknown` is not a default ADMITTED state. It is a structural gap that blocks the axis from contributing to composite admission. Routing decisions that prefer ADMITTED servers over CANDIDATE servers (`RoutingDecision::status()`) leave Unroutable as the result when no ADMITTED server exists — they do not silently promote a CANDIDATE to ADMITTED.

**Invariant 3: The LSP surface is read-only.** The compositor's `LanguageServer` implementation handles `textDocument/didOpen`, `textDocument/didChange`, `textDocument/publishDiagnostics`, `textDocument/hover`, `textDocument/completion`, and `textDocument/definition`. It does not implement `workspace/applyEdit`, `textDocument/formatting`, or any mutation-capable method. Code actions are offered but execution requires an explicit user or agent acceptance via a write-capable tool — which passes through the ANDON gate before execution.

---

## 7. Status: Bounded Assessment

The following table records the admission status of each major component as of 26.6.21. Status claims without receipt artifacts are CANDIDATE at most.

**ADMITTED**

- `crates/lsp-max-compositor/src/declare.rs` — Declare constraint model backed by wasm4pm's type vocabulary. 18 constraint types. Normative models for compositor and anti-llm pipelines. Conformance checking against per-case traces. Receipt: `crates/lsp-max-compositor/transcripts/multi_lsp_per_extension.txt`.
- `crates/lsp-max-compositor/src/dfg.rs` — DFG construction from traces and OCEL events. Fitness and precision scoring. Mermaid and DOT rendering. Backed by wasm4pm's `DFG` data model.
- `crates/lsp-max-compositor/src/receipt.rs` — `CompositorReceipt` with law-set fingerprinting and OCEL 2.0 export. Structural BLOCKED/ADMITTED guard. Receipt: `crates/lsp-max-compositor/receipts/multi_lsp_per_extension_receipt.json`.
- `crates/lsp-max-compositor/src/config.rs` — `CompositorConfig::load_with_auto()` and `merge()`. Static config wins on ID collision.
- `crates/lsp-max-compositor/src/routing.rs` — `RoutingTable` with ADMITTED-preferring resolution. `RoutingDecision` enum with law-status propagation.
- L7 Speciation — `MergeContext` routes each diagnostic through its originating server's own `C_D`. Witnessed by `tests/speciation.rs` and `src/merge/witness_isolation.rs`.
- PreToolUse ANDON gate — `.claude/hooks/gate-check.sh` wired in `.claude/settings.json`. Exit-1 behavior confirmed by hook architecture.

**CANDIDATE**

- `FlushCoordinator` Declare + DFG inline conformance — runs after every flush; scores logged but not gated.
- OCEL accumulation (RFC C) — `take_ocel_events()` wired; event log used by conformance analysis.
- Per-server receipt chain (RFC B) — `ChildEvidence::from_flush_contribution` wired; persistent chain head is OPEN (each flush uses zero-hash genesis).
- `SubagentStart / SubagentStop` hooks — hook type confirmed at medium confidence by adversarial verification; per-subagent server lifecycle wiring not yet implemented.
- Auto-discovery script (`discover-lsp-chains.sh`) — `CompositorConfig::find_auto_config()` implemented and ready to read the output; script not yet present.

**OPEN**

- `crates/lsp-max-mcp/` — MCP server with `lsp_discover`, `lsp_route`, `lsp_health`, `lsp_reload_config` tools. Architecture specified; crate not yet created.
- `.claude/agents/` named subagent definitions — per-subagent `mcpServers` frontmatter for native fan-out. Not yet present.
- `FileChanged` hook on `lsp-max.toml` → `lsp_reload_config` hot-reload path — depends on `lsp-max-mcp` being present.
- Fitness threshold as ANDON gate input — scoring is implemented; gate-write on low fitness is not.
- Persistent compositor keystore — `Keystore` is ephemeral per process; stable key identity across restarts is required for verifiable receipt chain continuity.

**REFUSED**

- Runtime server injection via LSP or MCP — excluded by Invariant 1. No registration path is exposed.
- Victory language in process model, code, or receipts — excluded by `Absence { max: 0 }` constraint on `VictoryLanguageEmitted` in the anti-llm normative model.
- Direct file mutation via the LSP surface — excluded by Invariant 3. No write-capable LSP methods are implemented.

---

## 8. Conclusion

lsp-max is a compositor-first LSP architecture whose correctness surface extends beyond compilation and test-passing. The Declare constraint model encodes the normative flush pipeline as a set of LTL-style constraints checkable against the OCEL event log without source access. DFG fitness measures structural alignment between observed behavior and the normative arc set. The PreToolUse ANDON gate enforces that law-state diagnostics are not bypassed by write operations. The hook lifecycle — SessionStart for discovery, PreToolUse for gate enforcement, PostToolUse for snapshot, SubagentStart/Stop for domain-specific server management — maps the LSP server lifecycle onto the agent session boundary.

The chain from workspace file to law-state verdict runs: file open → fan-out to N child servers → merge with REFUSED\_BY\_LAW preservation → receipt emission with law-set fingerprint → OCEL event accumulation → Declare conformance check → DFG fitness computation → ANDON gate update. Each link in this chain is either ADMITTED (implemented and receipted), CANDIDATE (implemented but not receipted), or OPEN (specified but not implemented). No link carries a status stronger than its evidence supports.
