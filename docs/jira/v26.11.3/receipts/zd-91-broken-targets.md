# ZD-91: Broken test targets (ZD-34 pre-existing compile breaks) — receipt

- **Repo**: /Users/sac/lsp-max @ bd97e5c
- **Date**: 2026-11-04
- **Status**: ALIVE (both targets compile and pass; `cargo check --workspace --all-targets` exit 0)

## Subject

Two test targets failed to compile at HEAD:

1. `lsp-max` test `test_autonomic_mesh` — E0432: `runtime::{CustomerRequestClassifierHook, PolicyEvaluationHook, ReceiptRoutingHook}` unresolved.
2. `lsp-max-cli` test `cp11_fitness_bridge` — E0061 (7 vs 6 args to `FlushCoordinator::spawn`) + E0063 (missing `end_line`/`end_character` in `DiagnosticEntry` initializer).

## Typed diagnosis + fix

### 1. test_autonomic_mesh

**Diagnosis: hooks are neither deleted nor missing — they are QUARANTINED** behind the
`refund-demo-hooks` feature (default off; Gall checkpoint CP14, see
`src/runtime/mesh_hooks.rs:300-317` and `src/runtime/mod.rs:17-20`). The quarantine
comment states the hooks model an unrelated customer-refund workflow whose
`PolicyEvaluationHook::trigger` → `AutonomicMesh::execute_action` chain performs a real,
unconfirmed `std::fs::write`, and a 2026-08-04 safety audit confirmed the chain is dead
on every live path (only these tests register the hooks). Given that quarantine was a
deliberate safety decision, the lawful fix-forward was **not** to un-quarantine or
re-implement — it was to bind the test target to the quarantined feature, mirroring the
existing `#[cfg(all(test, feature = "refund-demo-hooks"))]` pattern in
`src/runtime/mod.rs:37`.

**Fix**: explicit `[[test]] name = "test_autonomic_mesh"` entry in `Cargo.toml` with
`required-features = ["refund-demo-hooks"]` (with an in-place comment citing ZD-34 and
the quarantine). Effect: `cargo test --workspace --all-targets` (the justfile gate)
skips the target when the feature is off instead of failing the whole workspace build;
the test runs under `--features refund-demo-hooks`.

**Honest caveat**: with the feature off, cargo skips the target quietly (visible only in
Cargo.toml). This mirrors the repo's existing CP14 quarantine pattern for the same hooks
and is documented here rather than hidden.

 Loud-skip mitigation: the comment in Cargo.toml makes the skip auditable in the build
manifest itself.

### 2. cp11_fitness_bridge

**Diagnosis: AF-19 field additions never propagated to the test.** `FlushCoordinator::spawn`
now takes a 7th arg `Arc<AndonSnapshot>` (flush_coordinator.rs:107; production caller
`server.rs:691-699` passes `andon_snapshot_for_coord`), and `DiagnosticEntry` gained
`end_line`/`end_character` (merge.rs:21-22). The test predated both.

**Fix** (test call sites only, current API):
- 7th arg `Arc::new(AndonSnapshot::new())` + import `lsp_max_compositor::andon_snapshot::AndonSnapshot`.
- `end_line: 7, end_character: 3` added to the deposited `DiagnosticEntry`.
- Both with comments citing AF-19.

## Verification (commands + exits, run twice for each target)

```
cargo test -p lsp-max --features refund-demo-hooks --test test_autonomic_mesh   # ok. 11 passed; 0 failed  (×2)
cargo test -p lsp-max-cli --test cp11_fitness_bridge                            # ok. 1 passed; 0 failed    (×2)
cargo check --workspace --all-targets                                           # exit 0 (1m 10s)
cargo test -p lsp-max-compositor                                                # 18 targets, all ok except registry_ontology_test (see below)
```

## Compositor suite: unregressed (one PRE-EXISTING failure, not mine)

`crates/lsp-max-compositor/tests/registry_ontology_test.rs` — 2 tests fail
(`deleting_an_individual_drops_it_from_regeneration`, `determinism_double_run_byte_identical`),
both at `run_ggen` (line 43): ambient `ggen` 26.11.3 now enforces the **EA admission gate
(FM-LAW-015)** — "`.cache/tmp/.tmpXXXX/ea.graph.json` is absent and no admission opt-out was
given... add ea.graph.json, or pass `--skip-ea-admission`". The test spawns bare
`ggen sync run` in a tempdir. My diff touches only `Cargo.toml` (lsp-max test target) and
`crates/lsp-max-cli/tests/cp11_fitness_bridge.rs` — neither is compiled into
`lsp-max-compositor`, so this cannot be a regression from this session's diff. PRE-EXISTING,
left for its own order (ZD-91 scope is the two broken targets; filing as observed
environmental drift).

## Diff (owned files)

- `Cargo.toml` — [[test]] test_autonomic_mesh + required-features (new explicit target entry)
- `crates/lsp-max-cli/tests/cp11_fitness_bridge.rs` — AF-19 call sites updated to current API
- `docs/jira/v26.11.3/receipts/zd-91-broken-targets.md` — this receipt

## Standing

ALIVE. Both broken targets green ×2, workspace check exit 0, compositor suite otherwise
green. Pre-existing `registry_ontology_test` ggen-EA-gate failure observed and documented,
not repaired (outside lane ownership).
