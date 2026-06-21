# CLAUDE.md — examples/wasm4pm-lsp

Local coding agent instructions for the `wasm4pm-lsp` example. This supplements
the root `CLAUDE.md` and `AGENTS.md`, which govern the whole repository. The
project-level laws (no tower-lsp, no victory language, bounded statuses, receipts
over logs, CalVer) all apply here.

## What this is

A standalone Rust package (`wasm4pm-lsp`, `publish = false`) that demonstrates
cognitive-breeds process mining over an LSP surface. It is **not** part of the
workspace crate graph — it builds against `lsp-max` via a path dependency and
requires sibling repos (`../lsp-types-max`, `../wasm4pm-compat`, `../wasm4pm`).

The example houses 10 *cognitive breeds*: self-contained AI reasoning algorithm
implementations that span SymbolicAI, ProbabilisticAI, FormalMethods, MetaCognition,
ReinforcementLearning, and NeuralAI families. Breed admission is governed by COG laws
1–12 (see below).

## Key Types

| Type / Function | File | Purpose |
|-----------------|------|---------|
| `CognitiveBreed` trait | `src/breeds/breed.rs` | Synchronous `run(&self, input: &BreedInput) -> Option<Value>` |
| `BreedInput` | `src/breeds/breed.rs` | Wraps `serde_json::Value`; has `get(key)` |
| `dispatch()` | `src/breeds/dispatch.rs` | Routes `breed_id: &str` → `Option<Value>` |
| `lib.rs` | `src/lib.rs` | `pub mod breeds;` — exposes breeds as library target for runner binary |

## Breed Inventory

| breed_id | Family | module_stem | oracle_value | Blocking COG Laws |
|----------|--------|-------------|-------------|-------------------|
| asp | FormalMethods | asp | 0.0 | COG-003, 006, 007, 011 |
| bayesian_network | ProbabilisticAI | bayesian_network | 0.284 | COG-003, 006, 007, 011 |
| cbr | SymbolicAI | cbr | 0.85 | COG-003, 006, 007, 011 |
| eliza | SymbolicAI | frame | 0.0 | COG-003, 006, 007, 011 |
| frames_inheritance | SymbolicAI | frames_inheritance | 0.0 | COG-003, 006, 007, 011 |
| llm | NeuralAI | llm | 0.0 (non-det.) | COG-003, 006, 007, 011 |
| ltl_monitor | FormalMethods | ltl_monitor | 1.0 | COG-003, 006, 007, 011 |
| meta_reasoning | MetaCognition | meta_reasoning | 0.0 | COG-003, 006, 007, 011 |
| mycin | SymbolicAI | production_rules | 0.693 | COG-003, 006, 007, 011 |
| pomdp | ReinforcementLearning | pomdp | 0.969 | COG-003, 006, 007, 011 |

## COG Laws Quick Reference

| Law | Requirement | Status pattern |
|-----|-------------|----------------|
| COG-001 | Breed module in `src/breeds/{module_stem}.rs` | ADMITTED for all 10 |
| COG-002 | OCPN model at `ocel/models/l1/{breed_id}.ocpn.json` | CANDIDATE (models exist) |
| COG-003 | Conformance runner must execute; fitness populated | OPEN (runner not yet run) |
| COG-004 | Paper fixture at `tests/fixtures/papers/{breed_id}.json` | ADMITTED for all 10 |
| COG-005 | Fixture `expected.value` must not be PENDING | ADMITTED for all 10 |
| COG-006 | fitness = 1.0 required | OPEN (awaiting runner run) |
| COG-007 | `measured_by`, `measured_on`, `run_id` in report populated | OPEN (awaiting runner run) |
| COG-008 | Doc card at `docs/breeds/{breed_id}.md` | ADMITTED for all 10 |
| COG-009 | TS fixture mirror at `packages/cognition/src/__tests__/fixtures/papers/` | ADMITTED for all 10 |
| COG-010 | No oracle literal on non-comment line in breed source | CANDIDATE (scan test exists) |
| COG-011 | All DoD items: fitness=1.0, admitted=true, all artifacts | OPEN (awaiting runner run) |
| COG-012 | Dispatch arm in `src/breeds/dispatch.rs` | ADMITTED for all 10 |

**To advance COG-003/006/007/011 from OPEN → ADMITTED**: run the conformance runner
(see Commands below) and commit the updated `ocel/reports/*.json` files.

## Commands

```sh
# Build the example (requires sibling repos)
cargo build --manifest-path examples/wasm4pm-lsp/Cargo.toml

# Run conformance runner — reads breeds/registry.json, dispatches all 10 breeds
# against paper fixtures, writes measured ocel/reports/{breed_id}.json files.
cargo run --bin conformance-runner --manifest-path examples/wasm4pm-lsp/Cargo.toml

# COG-010 oracle injection scan — scans 5 (module_stem, literal) pairs for oracle
# values on non-comment lines; writes tests/receipts/cog010-scan.json at runtime.
cargo test -p wasm4pm-lsp cog010_no_oracle_injection -- --nocapture

# Run all wasm4pm-lsp tests
cargo test -p wasm4pm-lsp

# Run a specific dogfood test
cargo test -p wasm4pm-lsp --test dogfood_breed_registry
```

## LLM Breed Notes

`src/breeds/llm.rs` calls the Anthropic Messages API via `reqwest::blocking` wrapped
in `std::thread::spawn` (avoids deadlocking the tokio runtime). It reads
`ANTHROPIC_API_KEY` from the environment.

- **Key absent**: `run()` returns `None` — the conformance runner skips the breed
  (logged as SKIP, not counted as FAIL). This is the correct CI behaviour.
- **Key present**: makes a blocking POST to `https://api.anthropic.com/v1/messages`.
- Pass criterion: `response` field is a non-empty string. Output is non-deterministic;
  no fixed oracle value applies.

Do not embed the API key in fixtures, tests, or comments.

## COG-010 Oracle Scan

The scan test (`tests/cog010_oracle_scan.rs`) checks 5 pairs:

| module_stem | oracle_literal | Excluded because |
|-------------|---------------|-----------------|
| bayesian_network | 0.284 | — |
| bayesian_network | 0.2842 | — |
| production_rules | 0.693 | — |
| cbr | 0.85 | — |
| pomdp | 0.969 | — |

Literals 0.0 and 1.0 are excluded — they appear too frequently in algorithm code.
Lines starting with `//` after trim are skipped (doc comments are not violations).

The receipt is written to `tests/receipts/cog010-scan.json` at test runtime.
That path is gitignored (receipts are runtime artifacts). The test panics on any
violation, and asserts the receipt file was written.

## Adding a New Breed

1. Create `src/breeds/{module_stem}.rs` implementing `CognitiveBreed`
2. Add `pub mod {module_stem};` to `src/breeds/mod.rs`
3. Add `"{breed_id}" => run_breed(&MyStruct, input)` arm in `src/breeds/dispatch.rs`
4. Add registry entry to `breeds/registry.json`
5. Create `tests/fixtures/papers/{breed_id}.json` with real inputs + expected.value
6. Create `packages/cognition/src/__tests__/fixtures/papers/{breed_id}.json` (TS mirror)
7. Create `ocel/models/l1/{breed_id}.ocpn.json` with breed-specific places/transitions
8. Create `ocel/reports/{breed_id}.json` stub (fitness=0.0, admitted=false, status="OPEN")
9. Create `docs/breeds/{breed_id}.md` with COG law table

If the oracle value is non-trivial (not 0.0 or 1.0), add a `(module_stem, literal)`
pair to `SCAN_PAIRS` in `tests/cog010_oracle_scan.rs`.

## Anti-Patterns

- **No oracle injection**: Do not hardcode oracle values (0.284, 0.693, 0.85, 0.969)
  on non-comment lines inside breed source files. They belong in fixtures, not implementations.
- **No victory language**: Fitness reports use `OPEN`/`CANDIDATE`/`ADMITTED`/`REFUSED`.
  Never write `"status": "done"` or `"admitted": "yes"` in JSON artifacts.
- **No hardcoded fitness**: The conformance runner writes fitness; do not manually edit
  `ocel/reports/*.json` fitness values to non-zero without a runner measurement backing them.
- **No async in breed implementations**: `CognitiveBreed::run` is synchronous. For I/O
  (e.g., HTTP in the llm breed), use `std::thread::spawn` + `reqwest::blocking`.

## File Layout

```
examples/wasm4pm-lsp/
├── breeds/
│   └── registry.json              # canonical breed registry (10 entries)
├── docs/breeds/                   # COG law tables, algorithm summaries (10 .md files)
├── ocel/
│   ├── models/l1/                 # OCPN models (10 .ocpn.json files)
│   └── reports/                   # fitness reports (10 .json files; updated by runner)
├── packages/cognition/src/__tests__/fixtures/papers/  # TS mirrors (10 .json)
├── src/
│   ├── lib.rs                     # pub mod breeds; (library target for runner)
│   ├── main.rs                    # LSP server binary
│   ├── bin/
│   │   └── conformance_runner.rs  # conformance runner binary
│   └── breeds/
│       ├── breed.rs               # CognitiveBreed trait, BreedInput
│       ├── dispatch.rs            # dispatch() routing function
│       ├── mod.rs                 # module declarations
│       └── {asp,bayesian_network,cbr,frame,frames_inheritance,
│             llm,ltl_monitor,meta_reasoning,pomdp,production_rules}.rs
└── tests/
    ├── cog010_oracle_scan.rs      # COG-010 oracle injection scan (integration test)
    ├── dogfood_*.rs               # gate conformance dogfood tests
    ├── fixtures/papers/           # paper fixtures (10 .json files)
    └── receipts/                  # gitignored; written by tests at runtime
```
