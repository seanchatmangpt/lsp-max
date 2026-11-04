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

## Wire evidence (before → after) — shim court, real restore

Shim-court protocol (`with_shim` shape): BASELINE → SHIM (marked) → AFTER →
RESTORE, all against the real compositor release binary and the real
front-door court.

- BASELINE (fix in, 2026-11-03): `cargo build --release -p
  lsp-max-compositor` clean (1m07s); court
  `clh_end_position_survives_front_door`
  (`/Users/sac/claude-code-config-lsp/tests/compositor_relay_stress.rs`)
  GREEN: `1 passed`, asserting `assert_eq!(got, (1, 2, 1, 9))` on the
  front-door wire range — child probe (1,2)→(1,9) survives verbatim.
- SHIM (revert `d.end_line`/`d.end_character` → `d.line`/`d.character`,
  rebuild release, 6m00s): court **FAILED** — `clh_end_position_survives_front_door`
  red; the court flips, so it is non-vacuous (a court that cannot flip is
  FAIL). This is the wire-level before: collapsed end reaches the editor.
- RESTORE (fix re-applied, byte-identical to BASELINE fix — verified by
  grep: `d.end_line` at flush_coordinator.rs:237, `d.line` only at the
  start position line 233; release rebuilt 1m06s): court GREEN again
  (1 passed, 1.27s). Note: first post-restore run FAILED spuriously —
  the CHILD release binary
  `/Users/sac/claude-code-config-lsp/target/release/claude-code-config-lsp`
  had been removed from the shared tree by concurrent lane activity
  (`release binary missing` panic at compositor_relay_stress.rs:230);
  rebuilt child release (3m25s), court green. Environmental, not the fix.
- Court un-ignored per its registered close condition (`#[ignore]`
  attribute removed; the court now runs in the normal suite path).

## Gates

- CL-H court ×2: `cargo test -p lsp-max-compositor --test
  diagnostic_end_position_test` → `4 passed; 0 failed` both runs.
- Full compositor suite: all 21 targets green, 141 passed / 0 failed
  (andon_prefixes 11, cc_007_lsif_routing 8, child_eviction_test 3,
  crash_isolation 3, diagnostic_end_position_test 4, diagnostic_relay_probe
  2, diagnostics_merge 9, e2e 2, fanout_latency 1,
  gate_path_roundtrip_witness 1, integration 47, priority_semantics 10,
  priority_validation_test 5, registry_ontology_test 6, request_arbitration
  9, routing_dispatch 6, routing_integration 21, server_handle_failure_test
  3 (+1 pre-existing #[ignore]), speciation 12, v26_6_28_gate_tests 5).
- `child_churn_stress` target: COMPILE-BROKEN, pre-existing, session-
  independent (E0308/E0277 at tests/child_churn_stress.rs:622, `Option<u32>`
  vs `u32` in test-local code; unrelated to flush_coordinator). Classified
  AB-17-adjacent churn-court resource contention — NOT fixed by this lane
  (not owned). Compile-abort is not a failure count (fanout law 3).

## Standing

- flush_coordinator.rs fix: ALIVE (built, court-flipped on real wire).
- AB-18 F1 registered divergence: CLOSED (court active and green).
- child_churn_stress compile break: BLOCKED (AB-17 lane).
