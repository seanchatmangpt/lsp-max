# LSP 3.18 + LSIF 0.6 Combinatorial Coverage

This document records the combinatorial coverage extractor added to
`anti-llm-cheat-lsp` and the evidence basis for its statuses. It is the
artifact `ANTI-LLM-LSP318-COMB-001` asks for: a real spec extractor that
enumerates the full surface, instead of treating the 15-row delta changelog
matrix as full LSP 3.18 coverage.

## Why this exists

`src/rules/lsp318.rs` enumerates a 15-row **delta changelog** of LSP 3.18
features. The example ships 98 transcript fixtures that exercise nearly the
entire method surface, yet the matrix mapped only 15 of them — and declared
`SUPPORTED_WITH_TRANSCRIPT` against receipt paths under a `receipts/`
directory that does not exist on disk. The rule `ANTI-LLM-LSP318-COMB-001`
flags exactly this implication:

> `ChangelogCoverage(15 rows) ⇒ SpecCoverage(LSP 3.18)`

The extractor refutes that implication by enumerating the surface and deriving
each row's status from on-disk evidence.

## What was added

- `src/rules/lsp318_coverage.rs` — the full LSP 3.18 method surface (95 methods
  across lifecycle, text sync, navigation, completion/lens/action/link/color,
  formatting/folding/hints/inline/semantic/symbol, call/type hierarchy, pull
  diagnostics, workspace, file operations, server-to-client refreshes, window,
  and notebook). Status is **computed** from evidence, never declared.
- `src/rules/lsif06.rs` — the full LSIF 0.6 element graph (20 vertices + 18
  edges) with each element's `modeled in crate` flag and an honest
  example-coverage status.
- `src/virtual_docs/lsp318_full_matrix.rs` — rendered at
  `anti-llm://lsp318-full-matrix` (the live extractor output).
- `src/virtual_docs/lsif06_matrix.rs` — rendered at `anti-llm://lsif06-matrix`.
- Code actions to open both matrices; dogfood assertions in `tests/dogfood.rs`.

## Status taxonomy (bounded, tri-state aware)

| Status | Meaning |
| --- | --- |
| `SUPPORTED_WITH_TRANSCRIPT` | Wired handler in `server.rs` **and** a transcript on disk; receipt axis still `OPEN`. |
| `PARTIAL` | Wired handler, no transcript on disk. |
| `UNKNOWN` | Transcript on disk but **no** wired handler. Never collapses to a polarity. |
| `OPEN` | Neither handler nor transcript evidence. |
| `REFUSED` | Handler refuses by law (e.g. `range_formatting` returns `Err`). |
| `BLOCKED` | Declared refusal-by-law contradicted by an implemented no-op handler (notebook family). |

The receipt axis is held `OPEN` for every method: no receipt artifacts exist on
disk, so no row reaches `ADMITTED`. The matrix tells the truth the moment
receipt artifacts (path, digest, boundary, negative-control) land.

## Evidence basis (10-agent scan)

A 10-way partition of the surface was scanned against four real sources: the
spec authority `crates/lsp-max-specgen/fixtures/metaModel-3.18.json` (99 method
declarations), the 98 transcripts under `transcripts/`, the handlers/capabilities
in `src/server.rs`, and the (absent) `receipts/` directory. Headline findings:

- **Capability vector:** `server.rs` advertises 9 of ~34 server capability
  fields; ~20 transcripts exist for capabilities the server never declares.
- **Transcript-without-handler:** the dominant pattern — transcripts present,
  no wired handler. These are `UNKNOWN`, not support.
- **Notebook contradiction:** the delta matrix declares notebook
  `REFUSED_BY_LAW_WITH_RECEIPT`, but `server.rs` ships four empty no-op
  notebook handlers — neither refusal nor support. Marked `BLOCKED`.
- **LSIF 0.6:** the `lsp-max-lsif` crate models 36 of 38 elements (missing
  edges `nextMoniker`, `belongsTo`; the `capabilities` vertex exists only via
  codegen). The example carries zero LSIF transcripts/receipts, so every
  element's example-coverage status is `OPEN`/`PARTIAL`/`UNKNOWN`.

## Verification status

Workspace compile/test is `BLOCKED` in this environment: the required sibling
checkouts (`../lsp-types-max`, `../wasm4pm`, `../wasm4pm-compat`) are absent, so
`cargo check` fails at workspace-manifest loading before any crate compiles.
The dogfood assertions in `tests/dogfood.rs` encode the invariants (surface is
combinatorial, transcript-only never collapses to supported, receipts axis
`OPEN`, LSIF surface enumerated, example LSIF coverage `0`) and run once the
siblings are present.
