# cl-f-eviction — M8 dead-child eviction + no-respawn law (IMPLEMENTATION lane CL-F)

Status: PARTIAL — core lane ADMITTED below; full-suite sweep BLOCKED on one
environment-missing external binary (pre-existing, not session-introduced).

## Subject

- Repo: /Users/sac/lsp-max (canonical checkout, no worktree)
- Owned file: `crates/lsp-max-compositor/src/child_process.rs`
- New court: `crates/lsp-max-compositor/tests/child_eviction_test.rs`
- Minimal unblock (disclosed per SLA-6): `crates/lsp-max-compositor/tests/crash_isolation.rs` —
  flipped ONLY the eviction assert + the C3 header text. The C3 test itself declares the flip
  surface ("THIS assert flips — intended falsifier surface"), and the restart-side asserts
  (no respawn within observe window) are unchanged and still pass. No other content touched.
- Not touched: server_handle.rs (CL-E), config.rs (CL-G), any other src.
- git: never run (lane law).

## Diff summary (child_process.rs)

1. `ChildProcess` gains `pub exited: Arc<AtomicBool>` liveness flag, flipped `true` by the
   real exit-watcher task (the task that owns `child.wait()`), unconditionally — even when the
   oneshot receiver is gone, the pool must observe death.
 2. `ChildProcessPool::get()` now lazy-evicts: if the entry's exit flag is observed true, the
   entry is removed from the DashMap and None returned, so fanout/routing/flush-coordinator
   dispatch paths never re-enter a dead child; dispatch falls back to surviving children per
   the documented FirstSuccess/fan-all routing law. `is_live()` added for health/state surfaces.
3. NO-RESPAWN LAW documented on `ChildProcessPool`: after child exit (crash or clean), the
   compositor evicts and does not respawn — restartOnCrash/maxRestarts/backoff are Claude Code
   FRONT-door concerns (fleet split, finding M8). Receipt: this file.
   Prior behavior (dead entries retained forever; exit watcher only logged + cleared buffer
   entries at the server.rs block) is falsified by tests/child_eviction_test.rs.

## Court (tests/child_eviction_test.rs) — 3 tests, all real child processes, asserts on final state

- E1+E2 `dead_child_is_evicted_and_dispatch_falls_back_typed`: real `sh -c` child answers init
  then exits (exit 0); bounded poll on `!pool.is_live(id)`; `get()` evicts + returns None;
  eviction permanent (no resurrection); dispatch-shaped probe over the pool returns typed Err.
- E3 `stable_sibling_unaffected_by_sibling_eviction`: dying child + real `lsp-echo-server`
  binary sibling; after eviction the sibling still answers within bound and answers again
  after a fallback dispatch to the dead id.
- E4 `dead_child_is_never_respawned_by_the_compositor`:
  2 s observe window; membership stays exactly {stable}; dead id never reappears; sibling serving at close.

## Commands / exits (this session)

- `cargo test -p lsp-max-compositor --test child_eviction_test` → exit 0, 3 passed / 0 failed (2.3 s; 21 s under parallel suite load)
- `cargo test -p lsp-max-compositor --test crash_isolation` → exit 0, 3 passed / 0 failed (2.2 s)
- `cargo test -p lsp-max-compositor` → ran; first sweep stopped at diagnostic_relay_probe FAIL
  (1/2). Classified: the probe hard-depends on
  `/Users/sac/claude-code-config-lsp/target/release/claude-code-config-lsp`
  (tests/diagnostic_relay_probe.rs:19), an external binary that does not exist on this host —
  child can never spawn → diagnostics never relay. Pre-existing environmental BLOCKED,
  not session-introduced.
- `cargo test -p lsp-max-compositor --no-fail-fast` full sweep LANDED (exit: 3 targets failed,
  all classified below):

  | target | result | classification |
  |---|---|---|
  | unittests (52) + 15 integration targets | ok | — |
  | child_eviction_test | ok 3/3 | THIS LANE — ALIVE |
  | crash_isolation (flipped C3) | ok 3/3 | THIS LANE — ALIVE |
  | diagnostic_relay_probe | FAIL 1/2 | pre-existing BLOCKED: hard-depends on external binary `/Users/sac/claude-code-config-lsp/target/release/claude-code-config-lsp` (tests/diagnostic_relay_probe.rs:19) absent on this host — child can never spawn |
  | request_arbitration | FAIL 8/9 (`dead_child_folds_to_none_first_request_slow_then_fast`, :359) | wave-internal flip, NOT this lane: the test pins the OLD Ok(None) laundering; CL-E's landed typed failure in server_handle.rs (`ClientError("child-exited: write path closed before request 2 was sent")`) supersedes it — the pin needs updating by the arbitration lane |
  | server_handle_failure_test | FAIL 2/3 (1 ignored) | CL-E lane surface (server_handle.rs owner); file appeared mid-wave (mtime 15:07) |

## Standing (final, this session)

- child_eviction court: ALIVE — `cargo test -p lsp-max-compositor --test child_eviction_test`
  exit 0, 3/3 (2.3 s standalone, 2.6 s under suite load), real child processes, final-state asserts.
- crash_isolation with flipped C3 pin: ALIVE — exit 0, 3/3 (2.2 s).
- Full `lsp-max-compositor` suite: PARTIAL — every failure classified above as pre-existing
  environmental BLOCKED (relay probe) or other lanes' in-flight surfaces (request_arbitration
  pin vs CL-E typed failure; server_handle_failure_test). Zero failures attributable to this
  lane's diff: the only behavior change is get()-time eviction of children whose exit the
  pool has actually observed, and both crash courts exercising that path pass.

## Known classes noted, not fixed (outside lane scope)

- The 30 s stall-into-null laundering in `ServerHandle::request`
  (src/client/server_handle.rs:67) — filed by crash_isolation C4, owned by CL-E.
- diagnostic_relay_probe hardcodes an absolute external-binary path — env-dependent court
  design; flagged for the coordinator.
- ServerHandle::request signature `Result<Option<Value>, _>` — hover-shaped Option returned
  for arbitrary methods; noted for the CL-E lane.

## Standing

- child_eviction court: ALIVE (exit 0, 3/3 this session, real child processes, final-state asserts)
- crash_isolation (with flipped C3 pin): ALIVE (exit 0, 3/3 this session)
- Full suite: PARTIAL pending background sweep; diagnostic_relay_probe BLOCKED
  (missing external binary, pre-existing).
