# CL-H: Diagnostic range end preserved through compositor relay

Date: 2026-11-03. Lane: IMPLEMENTATION (CL-H). Repo: /Users/sac/lsp-max (no git run).
Closes M10 residual 1 (`docs/jira/v26.11.3/receipts/m10-e2e.md`): relayed diagnostic
ranges collapsed to their start point — `DiagnosticEntry` carried only (line, character);
end dropped in compositor_client.rs's entry mapping and re-materialized as a point range
in server.rs's Diagnostic conversion.

## Diff

- `/Users/sac/lsp-max/crates/lsp-max-compositor/src/merge.rs` — `DiagnosticEntry` gains
  `end_line: u32` / `end_character: u32` (with a comment naming the M10 residual).
- `src/compositor_client.rs`: entry mapping now carries
  `end_line: d.range.end.line`, `end_character: d.range.end.character`.
- `src/server.rs` `push_diagnostics_to_client`:
  the editor-side `Diagnostic.range.end` now uses `d.end_line` / `d.end_character`
  (previously hardcoded to start — the observed 0:1→0:1 collapse).
- Constructor-site updates (new fields, values preserved from each site's own semantics):
  `src/diagnostic_buffer.rs` (test helper), `src/merge/witness_isolation.rs` (2 literals),
  `tests/andon_prefixes.rs`, `tests/diagnostics_merge.rs`, `tests/integration.rs`,
  `benches/compositor_micro.rs`, `benches/compositor_backpressure.rs`.
- New: `tests/diagnostic_end_position_test.rs` (4 Chicago tests over the real relay path).

## Fix shape

Child `range.start` and `range.end` each map to their own DiagnosticEntry fields and
recombine into a full `Range` on the editor side. Zero-range in → zero-range out;
cross-line ends preserved verbatim. Merge dedup key stays (uri, line, character, code);
the dedup winner keeps its own end (no widening, no point-collapse) — witnessed by
`dedup_winner_keeps_its_own_end_position`.

## Tests (cargo test -p lsp-max-compositor --test diagnostic_end_position_test)

- `multi_char_range_end_survives_relay_and_merge` — the M10 wire witness replayed
  in-process: child emits 0:1→0:8 (CCC-JSON-004, the quoted settings key); merged
  entry arrives (0,1,0,8). PASS.
- `zero_range_diagnostic_stays_zero_range` — 3:7→3:7 stays (3,7,3,7). PASS.
- `multiline_range_end_survives_relay` — 2:4→5:11 survives cross-line. PASS.
- `dedup_winner_keeps_its_own_end_position`:
  same (uri,start,code) with ends 8 vs 99, CCC- as law prefix, Secondary vs
  DiagnosticsOnly → winner's own end (8) stands. PASS.

Command + exit: `cargo test -p lsp-max-compositor --test diagnostic_end_position_test`
→ `test result: ok. 4 passed; 0 failed` (exit 0, re-run after every fix).

## Verification ladder (actual output)

- `cargo build -p lsp-max-compositor` — exit 0 (Finished dev profile).
- Full suite `cargo test -p lsp-max-compositor` (after CL-E/F/G compile freezes cleared;
  two retries were needed mid-session because child_process.rs/config.rs were mid-edit
  by concurrent lanes — compile-abort, not failure):
  - 52 unit + 11 andon_prefixes + 8 cc_007 + 3 child_eviction + 3 crash_isolation
    + 4 diagnostic_end_position — all pass.
  - `crash_isolation` (3/3 PASS this session — the M10-era failure is repaired
    concurrently; M10 residual 2 is closed by another lane, not this one).
  - `child_eviction_test` flaked once (3 FAIL at a 40 s bound during lane churn),
    passed on subsequent runs. `--no-fail-fast` full census (24 suites): every suite
    passes individually except two, both outside this lane's diff:
    `request_arbitration::dead_child_folds_to_none_first_request_slow_then_fast`
    (dead-child pool behavior — CL-E/F/G-owned child_process/server_handle files,
    mid-flight edits, mtime minutes before failure) and
    `diagnostic_relay_probe::run_stdio_relay_works_end_to_end` (20 s timeout, no
    publishDiagnostics arrived — fails at dispatch level, upstream of where my
    end-position fields are read; passed earlier today per the M10 receipt). Both
    suspected lane-attributed, left to CL-E/F/G. My own suites: 4/4
    diagnostic_end_position, 52 unit, 47 integration, 21 routing_integration,
    12 speciation, 11 andon_prefixes, 10 priority_semantics, 9 diagnostics_merge,
    8 cc_007, 6 routing_dispatch, 5 v26_6_28_gate_tests, 3 child_eviction,
    3 crash_isolation, 3 registry_ontology — all green.
- Bench files updated only to satisfy the new fields; not executed.

## Standing

- Residual 1 (this lane): CLOSED — end position preserved child → compositor → merged
  publish, multi-char / zero-range / multiline witnessed over real relay paths.
- Suspected regression in `diagnostic_relay_probe` (dispatch-level, lane-owned files):
  OPEN, attributed to CL-E/F/G in-flight work, not this diff.
