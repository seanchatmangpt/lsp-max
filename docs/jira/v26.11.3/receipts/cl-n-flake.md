# Receipt: cl-n-flake — crash_isolation load flake triage

- **Subject**: `crates/lsp-max-compositor/tests/crash_isolation.rs`
- **Lane**: IMPLEMENTATION (M3/M10/M8 flake court)
- **Date**: 2026-11-03
- **Standing**: PARTIAL — court green under load; one open pin added

## Triage finding

The named "load flake" (2/5 failures under parallel-suite load, 3/3 in
isolation) was not a timing race in my baseline. Root cause: **test/src drift
against the landed M8 work**. The src had already landed (a) lazy eviction of
dead pool entries in `ChildProcessPool::get`
(`crates/lsp-max-compositor/src/child_process.rs:158-177`) and (b) the typed
EOF failure path — `read_loop` EOF drain
(`src/client/builder.rs:124`) → `ServerHandle::request` error-object branch
(`src/client/server_handle.rs:82-87`) → `Err(ClientError("child-exited: …"))`.
The court still pinned the pre-M8 law (entry retained forever; in-flight
request laundered into `Ok(None)` after the 30 s timeout). Any build against
the new src failed 2 of 3 tests deterministically; the observed load-vs-
isolation split was that drift being witnessed at different times, not a sleep
racing suite load.

At my baseline the file was being co-edited in flight by the M8 lane: my 3
baseline green runs (30.05–30.10s each) ran against the then-current lib;
mid-baseline two shared-build breaks from other lanes' in-flight files
(`src/client/server_handle.rs` closing-delimiter, then
`crates/lsp-max-compositor/src/config.rs:41` E0599) blocked the suite for
~5 and ~10 min respectively; both cleared within the compile-freeze SLA and are
disclosed per lane law (the second break surfaced the E0599 in config.rs, not
in any file I own).

By the time I could run, the M8 lane had already aligned the court to the new
src: C3 asserts lazy eviction on `get` + no respawn; C4 asserts the typed
`child-exited` Err. My contribution on top: **C4 fast-path pin** — measured
resolution time now asserted `< EOF_FAST_BOUND` (20 s), so a regression to
timeout-based laundering (the old 30 s stall) is loud even though
`INFLIGHT_BOUND` (40 s) would still absorb it. All waits in the file are
deadline-bounded polls (pkill argv-visibility poll 50 ms steps; C3 observe
window 100 ms steps) — no fixed-duration coordination sleep. Fixed sleeps
found: 0 (`std::thread::sleep`: 0 matches).

## Verification ladder

| run | command | result | wall |
|---|---|---|exit|
| baseline 1-3 | `cargo test -p lsp-max-compositor --test crash_isolation` | 3 passed | 30.05–30.10 s |
| baseline 4 | same | compile-blocked (other lane, server_handle.rs) | — |
| baseline 5 | same | compile-blocked (other lane, config.rs E0599) | ~10 min blocked, cleared in poll |
| under-load 1-5 | same, with `cargo test -p lsp-max-compositor --lib` running concurrently | 3 passed, exit 0 | 2.25–20.83 s |

After-evidence (5 consecutive green under parallel load): `/tmp/ci_1.log` …
`/tmp/ci_5.log`, all `test result: ok. 3 passed; 0 failed`, exits 0. Suite
time dropped 30.05 s → 2.25–20.83 s (the 30 s laundering stall is gone from
the happy path).

## Diff (mine only)

- `crates/lsp-max-compositor/tests/crash_isolation.rs` — added `EOF_FAST_BOUND`
  (20 s) + measured-elapsed assert pinning the fast typed-EOF resolution in C4;
  updated the stale "slow path is inherently bounded by the 30 s timeout"
  header comment. No src changes; src diffs in the tree belong to concurrent
  lanes (child_process.rs, config.rs, server.rs, registry.rs, … — M8/M10
  owners' work).

## Falsifiers

- C4 regression to Ok(None)-laundering → typed-Err assert fails.
- C4 regression to slow typed path (>20 s) → `EOF_FAST_BOUND` assert fails.
- Any restart/respawn policy appearance → C3 no-respawn asserts fail.

## Open

- Baseline 4/5 runs were compile-blocked by concurrent lanes, so a clean
  pre-fix 5× before-rate could not be measured this session; the 2/5
  under-load failure rate is cited from the lane contract, marked as-of-report
  (stale-by-default per receipt law).
