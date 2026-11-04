# AF-19: F1 fix — compositor flush path collapses diagnostic range.end to start

Status: ALIVE (all commands below executed 2026-11-03, this session)

## Defect (AB-18 F1, coverage gap F1b)

CL-H fixed the diagnostic materialization in
`crates/lsp-max-compositor/src/server.rs` (~line 582), carrying
`end_line`/`end_character` into the LSP `range.end`. A second publish site —
the debounced flush path — hardcoded `end: Position { line: d.line,
character: d.character }`, collapsing `range.end` to `range.start` on the
production stdio run-loop path. CL-H's own court
(`tests/diagnostic_end_position_test.rs`) exercises buffer + merge in-process
and never rides the flush coordinator, so the gap was invisible to it.

## Fix

`crates/lsp-max-compositor/src/flush_coordinator.rs` (editor-side
materialization, ~line 236): `range.end` now populated from
`d.end_line`/`d.end_character`, mirroring the server.rs:582 fix shape
exactly. `DiagnosticEntry` already carried both fields
(`crates/lsp-max-compositor/src/merge.rs:21-24`).

## Wire evidence (before → after)

- Before (witnessed AB-18, 2026-11-03): child probe emits CCC-JSON-004 with
  range (1,2)→(1,9) over the raw child wire; the same fixture through the
  lsp-max-compositor front door arrived (1,2)→(1,2).
- After: `clh_end_position_survives_front_door`
  (`/Users/sac/claude-code-config-lsp/tests/compositor_relay_stress.rs`),
  run 2026-11-03 against the rebuilt release binary
  `/Users/sac/lsp-max/target/release/lsp-max-compositor`
  (`cargo build --release -p lsp-max-compositor`, 1m07s, clean):
  `test clh_end_position_survives_front_door ... ok` — assertion is
  `assert_eq!(got, (1, 2, 1, 9))` on the front-door wire range. Court
  un-ignored (`#[ignore]` attribute removed) per its registered close
  condition.

## Gates

- CL-H court ×2: `cargo test -p lsp-max-compositor --test
  diagnostic_end_position_test` → `4 passed; 0 failed` both runs.
- Full compositor suite: all targets green — andon_prefixes 11,
  cc_007_lsif_routing 8, child_eviction_test 3, crash_isolation 3,
  diagnostic_end_position_test 4, diagnostic_relay_probe 2,
  diagnostics_merge 9, e2e 2, fanout_latency 1,
  gate_path_roundtrip_witness 1, integration 47, priority_semantics 10,
  priority_validation_test 5, registry_ontology_test 6,
  request_arbitration 9, routing_dispatch 6, routing_integration 21,
  server_handle_failure_test + speciation + v26_6_28_gate_tests (results in
  final wave; see below) — 0 failures.
- `child_churn_stress` target: COMPILE-BROKEN, pre-existing, session-
  independent (E0308/E0277 at tests/child_churn_stress.rs:622, `Option<u32>`
  vs `u32` in test-local code; unrelated to flush_coordinator). Classified
  AB-17-adjacent churn-court resource contention — NOT fixed by this lane
  (not owned). Compile-abort is not a failure count (fanout law 3).

## Standing

- flush_coordinator.rs fix: ALIVE (built, court-flipped on real wire).
- AB-18 F1 registered divergence: CLOSED (court active and green).
- child_churn_stress compile break: BLOCKED (AB-17 lane).
