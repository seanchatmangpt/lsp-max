# Changelog

All notable changes to this project are documented here.

Format: [Keep a Changelog](http://keepachangelog.com/en/1.0.0/).
Versioning: **CalVer (YY.M.D)** — `26.6.13` = 2026-06-13.

---

## [Unreleased]

---

## [26.6.18] — 2026-06-21

### anti-llm-cheat-lsp — full LSP 3.18 detection surface

- **`lsp318_coverage.rs`**: 43 handlers promoted from `Absent` → `Wired`;
  `workspace/applyEdit` promoted from `Absent` → `Refuses` (read-only law).
  Only `exit` and `$/cancelRequest` remain `Absent` — these are
  transport-layer entries with no `LanguageServer` trait method to override.
- **`server.rs`**: 29 new handler overrides covering `willSave`,
  `willSaveWaitUntil`, `completionResolve`, `documentLink`,
  `documentLinkResolve`, `documentColor`, `colorPresentation`,
  `codeActionResolve`, `onTypeFormatting`, `prepareTypeHierarchy`,
  `supertypes`, `subtypes`, `symbolResolve`, `didChangeConfiguration`,
  `didChangeWatchedFiles`, `didChangeWorkspaceFolders`,
  `willCreateFiles`, `willRenameFiles`, `willDeleteFiles`,
  `didCreateFiles`, `didRenameFiles`, `didDeleteFiles`,
  `didOpenNotebookDocument`, `didChangeNotebookDocument`,
  `didSaveNotebookDocument`, `didCloseNotebookDocument`,
  `workDoneProgressCancel`, `setTrace`, `progress`.
- **`initialized` handler**: 9 server-to-client wires — `showMessage`,
  `configuration`, `workspaceFolders`, `showMessageRequest`,
  `showDocument`, `workDoneProgressCreate`, `registerCapability`,
  `unregisterCapability`, `logTrace` — exercising the reverse-request
  surface so clients observe the full detection channel.
- **`run_scan_and_publish`**: 5 refresh/telemetry calls added —
  `semanticTokensRefresh`, `inlayHintRefresh`, `inlineValueRefresh`,
  `workspaceDiagnosticRefresh`, `telemetryEvent`.
- **`capabilities.rs`**: matrix-driven capability declarations updated for
  all newly-wired methods; `textDocument/willSave` and `willSaveWaitUntil`
  upgraded from `Kind` to full `TextDocumentSyncOptions`; type hierarchy,
  color, document-link, on-type-formatting, workspace file-ops, and
  notebook-sync capability blocks added.

### LSIF 0.6 — full element surface confirmed

- `src/rules/lsif06.rs` enumerates all 38 LSIF 0.6 elements (20 vertices
  + 18 edges). All have `modeled_in_crate: true`; example-coverage status
  is `OPEN` — no transcripts or receipts produced in this canary crate.
- `anti-llm://lsif06-matrix` virtual document renders the live surface.

### Coverage summary (PARTIAL — bounded)

LSP 3.18: 93/95 methods wired as detection surfaces. LSIF 0.6: 38/38
elements modeled (OPEN — no transcripts yet). Receipt axis: OPEN in this
canary. No victory language; no handler collapses `Unknown` to `Admitted`.

---

## [26.6.13] — 2026-06-13

### lsp-max-compositor — Λ_CD gate + full fan-out hub

- New crate `lsp-max-compositor`: multi-server fan-out hub that spawns N child
  LSP servers, merges their diagnostics, enforces the Λ_CD gate, and emits
  `CompositorReceipt` (BLAKE3 per-flush provenance).
- **ANDON gate** (`gate_file.rs`): single-byte file (`0`=OPEN / `1`=ANDON),
  written only on state transitions with `AcqRel` atomics + O(1) counter.
- **ANDON classification**: daachorse Aho-Corasick prefix matching at O(|code|);
  replaces per-diagnostic regex. `papaya` lock-free maps + `kanal` async channels.
- **Fan-out**: `didOpen` / `didChange` / `didClose` + `initialized()` + clean
  `shutdown` all fan to the child server pool concurrently.
- **L7 Speciation**: `deposit()` uses per-server `C_D` counter;
  `PARTIAL → CANDIDATE → ADMITTED` state machine.
- **Dynamic quorum-based debounce**: adaptive flush window minimizing user-perceived lag.
- **`CompositorReceipt`**: per-flush law-set provenance with BLAKE3 digest.
- **Child exit watcher**: clears URIs automatically on child process crash.
- `max/compositorHealth` — O(1) health endpoint.
- `max/compositorState` — SELECT-model ANDON state snapshot.
- `max/diagnosticAck` — back-channel acknowledgement to child servers.
- `compositor_capabilities()` — merged child `ServerCapabilities` introspection.
- `ServerCapabilities` from children merged into `InitializeResult`.
- E2E subprocess tests (`cat` stand-in validates spawn path).
- Integration tests: `tests/integration.rs`, `tests/speciation.rs`.
- Benchmark suite: micro / fanout / backpressure at N∈{5,50,500}; 4 additional
  80/20 benchmarks; gate write optimized to state-change-only.

### gate / CLI

- `lsp-max-cli gate check` verb added.
- `PreToolUse` hook wires Λ_CD^runtime enforcement before every
  Bash / Edit / Write / TaskCreate / NotebookEdit call.
- `OrderedFanout` and `ObserveOnly` composition strategy arms implemented.
- `conformance` noun registered in CLI; `NounVerbError` coercions fixed.

### anti-llm-cheat-lsp

- Centralized victory vocabulary: all forbidden terms moved to `config.rs`; engine
  and rule modules reference the single list — no rule hand-rolls its own.
- Vec/String `.contains()` misuse detected and diagnosed separately from
  pattern-match paths (different admission semantics).

### Protocol

- `ConformanceDeltaEntry.timestamp`: RFC 3339 field + `rfc3339_now()` helper.

### DX / docs

- `AGENTS.md` RFC backlog: three Λ_CD architectural priorities from 1000x review.
- Justfile shebang recipe; fanout latency test harness.
- `wasm4pm-lsp`: remove `GallVerdict` wrapper; sync `language_server_impl`.
- Tests: replace fake hash literals with `sha256(b"...")` calls.
- `.claude/settings.json` tracked; session artifacts added to `.gitignore`.

---

## [26.6.12] — 2026-06-12

### Features

- **`DiagnosticSink::publish_max`** + `ValidatedRulePackSet` (Phase 2 primitives).
- **`ConformanceVector` bitmask index**: `LawAxisId`, `LawAxisRegistry`,
  `sync_bits_from_vecs` for O(1) axis lookup.
- **dteam pattern back-port**: hash dedup, adaptive debounce, breed gates.
- `OcelProcessHook` export; `ConformanceVector` defaults derived.

### Fixes

- `DiagnosticSeverity` cast: replace invalid `as u64` with `match`.
- `DocumentUri` in `#[instrument]` macros: use `Debug` format.

### Refactor

- `anti-llm-lsp` renamed to `anti-llm-cheat-lsp` throughout.
- ggen scaffold pipeline added; ERRC innovations; THESIS committed.

---

## [26.6.10] — 2026-06-10

### Features

- **POWL v2 process intelligence** integrated into lsp-max core.
- POWL delegates all process intelligence to `wasm4pm-compat`.

### Fixes

- COG-001: derive module location from `dispatch.rs` instead of naming convention.
- COG-001: accept `eliza→frame.rs` and `mycin→production_rules.rs` as alternate
  module locations.

### Chore

- `wasm4pm-compat` switched to crates.io `v26.6.10`.
- Nested `powl-lsp` worktree artifact removed from index.
- `cargo fmt` rewraps across `ocel.rs`, `vertex.rs`, `evidence_extractors.rs`,
  `wasm4pm_admission.rs`, `protocol.rs`, `tex-lsp/main.rs`.

---

## [26.6.9] — 2026-06-09

### Breaking

- **Rename**: `tower-lsp-max` / `tower_lsp_max` → `lsp-max` / `lsp_max` throughout
  codebase, manifests, lockfiles, and docs. No compatibility shims.

### Features

- `wasm4pm-lsp`: `CLAP-PACK-HANDLER-UNBOUND` diagnostic + companion checks.
- LSIF: restored `Lsp318FeatureExercised`, `NegativeControlExecuted`,
  `FailsetUpdated` event types.

### Docs

- Diataxis index for `examples/` with gap analysis (`docs/EXAMPLES.md`).
- All Explanation-quadrant gaps closed.
- README rewritten for `lsp-max`; reference docs moved to `docs/`.
- `docs/project`: LOC exception table updated after gap splits.

### Refactor

- Domain-specific LSPs moved to `examples/`.
- `tests/e2e`: `test_f4_diagnostics` (780 LOC) and `test_blackbox_gate3` (589 LOC)
  split into subdirectories.
- `admission/mapping` and `kernel` split into submodule directories.

### Fixes

- OCEL: circular hash injection removed — serialize once, hash final content.
- Runtime: unused `cfg(test)` re-exports removed (clippy `-D warnings`).
- Stale `tower_lsp_max_*` identifiers and `lsp-types-max` version constraint patched.
- Pre-existing test failures resolved across workspace.
- `tex-lsp`: thesis directory paths updated.

### Chore

- CalVer bumped `26.6.8 → 26.6.9`.
- Publish enabled for internal crates; version constraints fixed.
- Accidental `.backup` / `.bak` artifacts removed.
- 6-phase gap remediation: dead code, hook registry, handler wiring, law table,
  LOC splits, spec-graph provenance.

---

## [26.6.8] — 2026-06-08

### Features

- Strict no-bullshit stub verification; all mocks rewritten.
- Baseline type authority established; `auto-lsp` codegen ported; `autodx`
  implemented in Justfile.

### Docs

- Blue Ocean Innovation thesis synthesizing the architectural leap.
- Blue Ocean thesis extended with combinatorial maximalism + differential calculus
  formalisms.
- Generative Economics thesis quantifying ggen pack time savings.

---

## [26.6.6] — 2026-06-06

### Features

- **Gall Checkpoints GC001–GC008** admitted via dogfood tests:
  - GC005: wasm4pm OCEL authority + lsp-only proof.
  - GC006: Authority Surface Lock (ADMITTED_BY_DOGFOOD).
  - GC007: wasm4pm-lsp Ownership Relocation.
  - GC008: CLAP-Governed Mutation Route.
- `gc005-wasm4pm-adapter`: true wasm4pm conformance authority for GC005.
- `pattern-lsp` example: pattern-detection LSP with `clap-noun-verb`.
- `examples/`: composition and black-box verification gates implemented.
- Playground: tokio runtime panic + timing races in dogfood harness fixed.

### Chore

- `wasm4pm-lsp` stripped of mutation authority (GC008 compliance).
- `.gitignore` updated; missing test artifacts committed.

---

## [26.6.5] — 2026-06-05

### Features

- **1000x rounds 4–14**: ERRC / TPS / DfLSS gap closures across 14 agent rounds.
  Highlights: judge-panel consensus, 15 zero-coverage RPC method gaps closed,
  error-path coverage, proptest expansion, DfLSS phase invariants.
- **LSIF 0.6.0**: exhaustive data structures + `max/lsif` streaming endpoint.
- **`lsp-max-client`**: LSP client framework scaffolded for downstream consumers.
- **`lsp-max-protocol`**: core library modularized into submodules ≤ 500 LOC.
- Round-13: judge-panel ERRC/TPS/DfLSS consensus selection.

### Fixes

- ANDON diagnostics: non-exhaustive `HookEvent` match, `catch_unwind→Result`,
  `code_actions` field sync.
- `Result<Self, ChainError>` on replay trait impls restored (broken by round-12).
- Lock `unwrap()` replaced with graceful error handling in hot paths.
- Completion and diagnostic handlers modularized; test regressions fixed.

### Docs

- v26.6.5 ARD: Calculus of Manufactured Intelligence formalized.
- Hyperdetailed PRD/ARD for oxigraph integration.

---

## [26.6.4] — 2026-06-04

### Features

- **`ConformanceVector`**: doctrine-correct `admitted` / `refused` / `unknown` axes;
  `From<bool>` / `From<AdmissionDecision>` conversions.
- **22 `max/*` handlers**: full `max/` protocol surface wired.
- **`lsp-max-cli`**: all core team CLI capabilities with full validation.
- **Playground crate**: AMI conformance test harness.
- Workspace hardening: 4 innovations, dev-dependency gap resolved.
- `#[derive(Default)]` replacing hand-written `impl Default` blocks.

### Features (LSP)

- LSP v3.18.0 specification implemented.

---

## [0.20.0] — 2023-08-10 (upstream tower-lsp fork point)

This is the last upstream `tower-lsp` release before the fork diverged into
`lsp-max`. The entries below document the upstream state at fork time.

### Added

* Pull-based diagnostics from LSP 3.17.0 (PR #396):
  `textDocument/diagnostic`, `workspace/diagnostic`,
  `workspace/diagnostic/refresh`.
* `std::str::FromStr` for `jsonrpc::{Request,Response}` (PR #379).
* `From<jsonrpc::ErrorCode>` for `i64` (PR #379).
* FEATURES.md LSP support matrix (PR #383).

### Changed

* Minimum supported Rust version: `1.52.0` → `1.64.0`.
* `lsp-types`: `0.94` → `0.94.1`.
* `syn`: `1` → `2`.
* `jsonrpc::Error::message` field: `String` → `Cow<'static, str>`.
* Several `jsonrpc::Error` / `jsonrpc::ErrorCode` methods marked `const fn`.

### Fixed

* Broken Markdown in `LanguageServer::completion()` doc comment.

---

## [0.19.0] — 2023-02-28 (upstream)

### Added

* `LspService::inner()`.
* `window/showDocument` client request (LSP 3.16.0).
* Partial LSP 3.17.0: type hierarchy, inline values, inlay hints,
  `workspaceSymbol/resolve`, pull-based diagnostics.

### Changed

* Edition `2018` → `2021`.
* Clippy lints addressed.

[Unreleased]: https://github.com/seanchatmangpt/lsp-max/compare/v26.6.18...HEAD
[26.6.18]: https://github.com/seanchatmangpt/lsp-max/compare/v26.6.13...v26.6.18
[26.6.13]: https://github.com/seanchatmangpt/lsp-max/compare/v26.6.12...v26.6.13
[26.6.12]: https://github.com/seanchatmangpt/lsp-max/compare/v26.6.10...v26.6.12
[26.6.10]: https://github.com/seanchatmangpt/lsp-max/compare/v26.6.9...v26.6.10
[26.6.9]: https://github.com/seanchatmangpt/lsp-max/compare/v26.6.8...v26.6.9
[26.6.8]: https://github.com/seanchatmangpt/lsp-max/compare/v26.6.6...v26.6.8
[26.6.6]: https://github.com/seanchatmangpt/lsp-max/compare/v26.6.5...v26.6.6
[26.6.5]: https://github.com/seanchatmangpt/lsp-max/compare/v26.6.4...v26.6.5
[26.6.4]: https://github.com/seanchatmangpt/lsp-max/compare/v0.20.0...v26.6.4
[0.20.0]: https://github.com/seanchatmangpt/lsp-max/compare/v0.19.0...v0.20.0
[0.19.0]: https://github.com/seanchatmangpt/lsp-max/releases/tag/v0.19.0
