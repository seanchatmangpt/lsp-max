# Examples — Diataxis Index

This document maps every item in `examples/` to a [Diataxis] documentation
quadrant. The goal is zero gaps: every example belongs to exactly one primary
quadrant, and every quadrant has coverage.

[Diataxis]: https://diataxis.fr

## Quadrant map

| Quadrant | Purpose | Contents |
|----------|---------|----------|
| [Tutorial](#tutorials) | Learning-oriented — guided first steps | `stdio.rs`, `tcp.rs`, `websocket.rs` |
| [How-to](#how-to-guides) | Goal-oriented — specific task recipes | `custom_notification.rs`, `repro_lifecycle.rs`, `axum-lsp`, `bevy-lsp`, `tex-lsp`, `wasm/` |
| [Reference](#reference) | Information-oriented — technical specification | `anti-llm-lsp`, `clap-noun-verb-lsp`, `pattern-lsp`, `wasm4pm-lsp`, `wasm4pm-compat-lsp`, `gc005-wasm4pm-adapter` |
| [Explanation](#explanation) | Understanding-oriented — concepts and rationale | `agi-swarm-defense`, `receipt_chain_explained.rs`, `conformance_vector_explained.rs`, `calver_law_explained.rs` |

---

## Tutorials

> Learning-oriented. The reader follows steps and arrives at a working result.
> Success = the reader built something and understood what they built.

### `stdio.rs`

The canonical first server. Implements `initialize`, `initialized`, and
`shutdown` over stdio. Start here before any other example.

**What you build:** a minimal LSP server that accepts connections from any
LSP-compatible editor or test client.

### `tcp.rs`

Same minimal server, transport replaced with TCP. Shows that `lsp-max` is
transport-agnostic: swap `Server::new(stdin, stdout, …)` for
`Server::new(read_half, write_half, …)`.

**What you build:** a TCP-backed LSP server reachable at `127.0.0.1:9257`.

### `websocket.rs`

Same minimal server over WebSocket via `async-tungstenite`. Shows the
runtime-agnostic transport path.

**What you build:** a WebSocket-backed LSP server suitable for browser-side
clients.

---

## How-to guides

> Goal-oriented. The reader has a specific outcome in mind and needs directions.
> Success = the reader accomplished the task.

### `custom_notification.rs`

**How to define and send a custom LSP notification.**

Declares a typed `CustomNotification` with `serde` params, registers it on the
`Client`, and shows round-trip through `LanguageServer::execute_command`.

### `repro_lifecycle.rs`

**How to reproduce and assert LSP lifecycle edge cases.**

Drives the full initialize → initialized → shutdown → exit sequence
programmatically. Use this as a template for writing lifecycle regression tests.

### `axum-lsp`

**How to host an LSP server inside an Axum web application.**

Runs `LspService` alongside Axum routes in a single Tokio runtime. Useful when
the language server must share state with an HTTP API.

### `bevy-lsp`

**How to integrate lsp-max into a Bevy application.**

Spawns the LSP server as a Bevy system, demonstrating how to bridge the ECS
event loop with LSP request/response cycles.

### `tex-lsp`

**How to build a domain-specific LSP for a structured text format (LaTeX).**

Minimal TeX-aware server: hover on `\command` nodes, diagnostics for unknown
packages. Use as a starting point for any text-format LSP.

### `wasm/`

**How to compile lsp-max to WebAssembly.**

Configures `lsp-max` with `default-features = false` and `runtime-agnostic`
for a `wasm32-unknown-unknown` target. See the linked
[tower-lsp-web-demo](https://github.com/silvanshade/tower-lsp-web-demo) for a
complete browser demo.

---

## Reference

> Information-oriented. Accurate, complete description of behaviour. The reader
> already knows what they want — they need to look it up.
> Success = the reader found the fact they needed.

### `anti-llm-lsp`

The admissibility canary. This LSP server runs in CI and detects:

- Reintroduction of forbidden identifiers (`tower_lsp::`, `tower_lsp_max::`)
- Fake receipts (receipt JSON not backed by BLAKE3 hash of actual file content)
- Victory language in code, comments, or commit messages
- Version-law violations (non-CalVer strings)

Diagnostic codes: `ANTI-LLM-*`, `ANTI-LLM-VERSION-*`.
Receipt: `examples/anti-llm-lsp/ocel/anti_llm_lsp_ocel.receipt.json`.

### `clap-noun-verb-lsp`

Reference implementation of a noun/verb CLI surface exposed through LSP.
Registers verbs via `#[verb("subcommand")]` + `linkme` distributed slices.
Demonstrates: `scan workspace`, `graph export`, `command inspect`,
`layout check`, `rules list`, `doctor check`, `receipt show`.

Use this to understand how `clap-noun-verb` and `lsp-max` compose.

### `pattern-lsp`

Reference implementation of a pattern-detection LSP. Scans Rust source for
structural patterns (missing handlers, incorrect routing) and emits
`ANTI-FAKE-*` diagnostics with source attribution.

### `wasm4pm-lsp`

Reference LSP for process-mining (wasm4pm) workspaces. Watches `.ocel.json`
files, runs Gall conformance checks via `gc005-wasm4pm-adapter`, and publishes
`GC-*` diagnostics. The LSP surface is read-only: it never mutates files.

Dogfood tests: `examples/wasm4pm-lsp/tests/dogfood_gc005.rs`.

### `wasm4pm-compat-lsp`

Reference LSP for `wasm4pm-compat` sibling workspace. Validates OCEL files
against the compat type authority. Companion to `wasm4pm-lsp`.

### `gc005-wasm4pm-adapter`

Reference adapter implementing the GC005 gate contract. Calls
`wasm4pm::check_gall_conformance`, translates results to `lsp-max`
`ConformanceVector`, and emits receipts. Not a standalone binary — used as a
library by `wasm4pm-lsp`.

Gate tests: `examples/gc005-wasm4pm-adapter/tests/dogfood_gc006.rs`.

---

## Explanation

> Understanding-oriented. Background and rationale — the *why* behind decisions.
> Success = the reader has a better mental model.

### `agi-swarm-defense`

**Why the law-state runtime exists.**

Demonstrates the threat model: an LSP mesh where multiple agents compete to
claim admissions. Shows how `ConformanceVector` with explicit `Unknown` prevents
false admission, how receipts create an unforgeable audit trail, and why "the
editor is just one client" is the correct frame for understanding lsp-max's
architecture.

Read this after the tutorials and before building a production agent client.

### `receipt_chain_explained.rs`

**Why BLAKE3 content-addressable receipts, not test assertions.**

Explains the circular-hash trap, why receipts must be written *after* the
artifact (not embedded in it), what a receipt does and does not prove, and how
receipts compose with OCEL conformance and `ConformanceVector` in the full
admission model.

### `conformance_vector_explained.rs`

**Why `ConformanceVector` has three axes — and why Unknown must never collapse.**

Explains the failure modes of a boolean `is_admitted` flag, the gate decision
contract for consumers, how axes transition from `unknown` to `admitted` or
`refused` during server startup and conformance checks, and how the delta log
enables incremental admission polling.

### `calver_law_explained.rs`

**Why `YY.M.D` CalVer instead of SemVer.**

Explains why SemVer's compatibility signal is meaningless for lsp-max's
agent-first clients, how CalVer integrates with the receipt/audit trail, the
`ANTI-LLM-VERSION-*` enforcement mechanism, and the sibling-repo version
constraint trap that blanket bump scripts trigger.

---

## Gap analysis

| Quadrant | Examples | Status |
|----------|----------|--------|
| Tutorial | `stdio.rs`, `tcp.rs`, `websocket.rs` | COVERED — three transport variants |
| How-to | `custom_notification.rs`, `repro_lifecycle.rs`, `axum-lsp`, `bevy-lsp`, `tex-lsp`, `wasm/` | COVERED — six distinct tasks |
| Reference | `anti-llm-lsp`, `clap-noun-verb-lsp`, `pattern-lsp`, `wasm4pm-lsp`, `wasm4pm-compat-lsp`, `gc005-wasm4pm-adapter` | COVERED — six reference implementations |
| Explanation | `agi-swarm-defense`, `receipt_chain_explained.rs`, `conformance_vector_explained.rs`, `calver_law_explained.rs` | COVERED — four conceptual examples |

All quadrants covered. No known gaps.
