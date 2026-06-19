# Project: lsp-max Roadmap Enhancements

## Architecture
The `lsp-max` workspace provides a law-state LSP runtime with process-mining conformance and cryptographic receipt-chain admission.
- **`crates/lsp-max-compositor`**: Multi-server fan-out and merge layer. Lifecycle of child LSP processes is managed here. Merged diagnostics check against ANDON gate rules.
- **`crates/lsp-max-cli`**: CLI frontend, including the `gate check` command that queries active ANDON gates.
- **`examples/anti-llm-cheat-lsp`**: Reference detector for compliance violations. Uses an AhoCorasick-based scanning engine.
- **`examples/pattern-lsp` & `examples/axum-lsp`**: Core examples showcasing pattern matching and rule validation.

## Code Layout
- `crates/lsp-max-compositor/src/`: Compositor source files
  - `gate_file.rs`: Coordinates gate state writing
  - `flush_coordinator.rs`: Debounces and pushes merged diagnostic logs & compositor receipts
  - `child_process.rs`: Manages lifecycle of child LSP subprocesses
- `crates/lsp-max-cli/src/nouns/gate.rs`: CLI gate validator
- `examples/anti-llm-cheat-lsp/src/`: Anti-LLM server code
- `examples/pattern-lsp/src/`: Pattern server code
- `examples/axum-lsp/src/`: Axum server code

## Milestones
| # | Name | Scope | Dependencies | Status |
|---|------|-------|-------------|--------|
| 1 | M1: Refactor `anti-llm-cheat-lsp` | Adopt `RulePackServer` & eliminate hand-rolled loop | None | DONE |
| 2 | M2: Wire `WorkspaceIndex` in examples | Wire `WorkspaceIndex` & delegate did_open/did_change/did_close in anti-llm-cheat-lsp, pattern-lsp, and axum-lsp | M1 | DONE |
| 3 | M3: Item A (Agent-scoped gate check) | Support `LSP_MAX_AGENT_ID` in gate file path to isolate gates per-agent | None | DONE |
| 4 | M4: Item B (Child receipt chains) | Replace `NoopClient` in compositor, query child receipts during flush, map to child evidence | M2 | DONE |
| 5 | M5: Item C (OCEL Logging) | Map compositor flushes and child evidence to OCEL JSONL files | M4 | DONE |

## Interface Contracts

### Custom JSON-RPC Method: `max/exportAnalysisBundle`
- **Request Parameters**: `{ "uri": String }`
- **Response Shape**: `AnalysisBundle { receipts: Vec<Receipt> }`
- Used by the compositor to pull receipts from child servers during flush coordination.

### Environment-Based Agent Scopes
- **Variable**: `LSP_MAX_AGENT_ID`
- If set, gate paths resolve to `lsp-max-gate-{hash}-agent-{LSP_MAX_AGENT_ID}`.
- If unset, fallback to global `lsp-max-gate-{hash}`.
