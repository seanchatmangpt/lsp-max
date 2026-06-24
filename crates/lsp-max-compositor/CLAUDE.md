# lsp-max-compositor

Multi-server fan-out and merge layer. Aggregates diagnostics, hovers, and code actions from N child LSP servers. Contains Van der Aalst process mining primitives.

## Architecture

```
child_process.rs      — spawn/reap server subprocesses; exit watcher
fanout.rs             — broadcast requests to all children in parallel
merge.rs              — ConformanceVector-aware diagnostic dedup; REFUSED_BY_LAW survives
capability_merge.rs   — Primary wins hover/completion; DiagnosticsOnly excluded
diagnostic_buffer.rs  — DashMap per-URI staging; deposit() → flush()
flush_coordinator.rs  — adaptive quorum debounce; OCEL accumulation; Declare+DFG inline
gate_file.rs          — single-byte ANDON gate file; FNV-1a workspace hash
registry.rs           — ChildTier (Primary|Secondary|DiagnosticsOnly) + ExtensionRouter
declare.rs            — Declare constraint model (9 types); compositor() + anti_llm_detection()
dfg.rs                — Directly-Follows Graph; fitness/precision metrics; mermaid/DOT renderers
receipt.rs            — CompositorReceipt; BLAKE3 provenance; to_ocel_event()
receipt_chain.rs      — ChildEvidence cryptographic chain link
merge/                — merge submodules; witness_isolation.rs for L7 speciation tests
```

## Van der Aalst Integration

`FlushCoordinator` runs Declare conformance + DFG fitness after every flush:
```rust
// After each flush:
let traces = extract_traces(&ocel_events);
let violations = DeclareModel::compositor().check(&traces);  // tracing::warn! on violations
let dfg = DirectlyFollowsGraph::from_traces(&traces);
dfg.fitness_against_model(&normative_arcs);                  // tracing::debug! fitness
```

OCEL events accumulate in `FlushCoordinator::ocel_events`. Drain with:
```rust
let events = coordinator.take_ocel_events();  // drains buffer
let count  = coordinator.ocel_event_count();  // snapshot
```

## Key Invariants

- `flush_coordinator` uses adaptive quorum debounce: fires at quorum (all N servers deposited) or at `last_at + clamp(2×spread, 1ms, 30ms)` — whichever comes first
- Gate write happens BEFORE receipt emission — `has_andon_block` propagates correctly
- `merge_diagnostics` uses `is_refused_by_law()` to ensure REFUSED_BY_LAW codes survive dedup
- `DiagnosticBuffer::deposit()` replaces previous entry for same `server_id + uri` — last-write-wins per server

## Routing

```
textDocument/hover | completion | definition   → FirstSuccess (Primary tier only)
textDocument/publishDiagnostics               → FanAll (all tiers; REFUSED_BY_LAW survives)
textDocument/didOpen | didChange | didClose   → Notify (fan all, no response expected)
unknown methods                               → PrimaryOnly
```

## Testing

```bash
cargo test -p lsp-max-compositor
cargo test -p lsp-max-compositor --test <name>
```

Benchmarks:
```bash
cargo bench -p lsp-max-compositor
```

## Law Status

- L7 Speciation: ADMITTED — `MergeContext` routes each diagnostic through its originating
  server's own C_D (`server_automatons` + `server_prefix_overrides`). A configured server's
  diagnostic is never classified by the workspace union. Witnessed by
  `tests/speciation.rs` (concrete production IDs: `wasm4pm-lsp`, `anti-llm-cheat-lsp`,
  `ggen-lsp`) and `src/merge/witness_isolation.rs` (mutation-checked alpha/beta fixture).
- Per-server receipt chain (RFC B): CANDIDATE — `ChildEvidence::from_flush_contribution` wired into
  `FlushCoordinator`; compositor signs each per-server link via ephemeral `Keystore`. OPEN items:
  persistent `prev_hash` chain head (each flush uses zero-hash genesis), persistent compositor
  Keystore (stable key identity), child-server-published receipt file (child side still OPEN).
- OCEL accumulation (RFC C): CANDIDATE — `take_ocel_events()` wired
- Declare/DFG inline conformance: CANDIDATE — runs after every flush
