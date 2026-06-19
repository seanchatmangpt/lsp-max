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
