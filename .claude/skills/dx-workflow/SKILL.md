---
name: dx-workflow
description: Run the full lsp-max developer experience pipeline (format, lint, verify, test). Invoke before any PR or merge.
tools: [Bash, Read]
---

# DX Workflow

The complete validation pipeline for lsp-max. Run these in order.

## Step 1 — Gate check
```bash
lsp-max-cli gate check
```
Exit 0 = clear. Exit 1 = ANDON blocked (resolve WASM4PM-* / GGEN-* diagnostics first).

## Step 2 — Format + lint
```bash
just dx-polish
```
Runs `cargo fmt --all` then `cargo clippy --workspace --all-targets --all-features -- -D warnings`.
Zero warnings is mandatory. Fix every Clippy warning before proceeding.

## Step 3 — Boundary scan
```bash
just dx-verify
```
Greps sibling repos for forbidden patterns: `tower-lsp`, `tower_lsp`, legacy/deprecated/shim/facade, forbidden type crates.
Must pass before any merge. Failures require fixing in sibling repos (outside this workspace).

## Step 4 — Tests
```bash
just test
```
Runs `cargo test --workspace`. All tests must pass.

## Step 5 — Single crate or single test (when scoped)
```bash
cargo test -p <crate-name> <test_name>
cargo test -p anti-llm-cheat-lsp --test dogfood
cargo test --test test_lsp318_capabilities
```

## Step 6 — Full pre-publish gate
```bash
just test-pre-publish
```
Runs dx-verify + dx-polish + tests with `--include-ignored`. Run this before tagging a release.

## Law Compliance Check
```bash
scripts/check-law-compliance.sh
```
Greps for: plain `tower-lsp`, victory language, fake receipt markers. Run when dx-verify passes but `anti-llm-cheat-lsp` emits `ANTI-LLM-*` diagnostics.

## What Each Step Validates

| Step | What Fails | Fix |
|------|-----------|-----|
| gate check | ANDON active | Resolve WASM4PM-*/GGEN-* diagnostics |
| dx-polish | fmt diff or Clippy warning | `cargo fmt` then fix Clippy output |
| dx-verify | Forbidden pattern in sibling repo | Fix in ../wasm4pm or ../wasm4pm-compat |
| test | Test failure | Fix test or fix code |
| test-pre-publish | Any of the above + ignored tests | Full sweep |

## Parallel DX (when touching multiple independent crates)

Spawn independent agents for each crate, each running:
```bash
cargo test -p <crate-name> && cargo clippy -p <crate-name> -- -D warnings
```
Then merge results in parent session before `just test-pre-publish`.
