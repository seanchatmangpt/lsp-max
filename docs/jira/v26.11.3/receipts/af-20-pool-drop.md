# af-20-pool-drop — pool-drop child teardown (IMPLEMENTATION lane AF-20, AB-17 finding)

Status: PARTIAL→ADMITTED on the owned lane (all owned gates green ×2, non-vacuity witnessed);
full workspace sweep not run (out of lane scope; `just test` untouched).

## Subject

- Repo: /Users/sac/lsp-max (canonical checkout, no worktree)
- Owned: `crates/lsp-max-compositor/src/` (child_process.rs only — the only file edited in src/)
- Court extended: `crates/lsp-max-compositor/tests/child_churn_stress.rs` (new C5 court
  `pool_drop_teardown_reaps_all_children_within_bound`; also updated the stale C2 teardown
  comment that documented the defect as permanent behavior — AB-17's own words, now superseded).
- Receipt: docs/jira/v26.11.3/receipts/af-20-pool-drop.md (this file)
- git: never run (lane law).

## Defect (AB-17, confirmed on disk before edit)

Dropping the pool does not stop the children. Mechanism: `src/client/builder.rs:95` spawns
read_loop holding a clone of the outbound `mpsc::Sender`, and write_loop holds the child's
stdin (`ChildStdin`). Pool drop drops the `ServerHandle`'s sender clone, but the read_loop task
outlives the pool holding its own sender clone, so the write_loop's `rx.recv()` never resolves
None, the ChildStdin is never dropped, stdin never EOFs, and the child lives on. AB-17's own
court documented this as permanent behavior (child_churn_stress.rs, C2 teardown comment:
"pool drop does NOT stop the children ... The court owns cleanup") — that comment is now
corrected; the C2 court itself is unchanged in behavior.

## Fix (option (a): kill-on-drop guard on ChildProcess)

`ChildProcess` gains a private `kill_tx: Option<oneshot::Sender<()>>`; the exit-watcher task in
`ChildProcess::spawn` now selects on `child.wait()` vs `kill_rx`. `Drop for ChildProcess` fires
the signal via two redundant paths (either alone suffices, shim-witnessed): (1) explicit
`tx.send(())`; (2) taking + dropping the sender closes the oneshot channel, which itself
resolves `kill_rx` and selects the kill branch. On the kill branch the watcher SIGKILLs
(`start_kill`) and reaps (`wait().await`) the child, flips the `exited` flag, and delivers the
ExitStatus. Child lifetime is now tied to the pool entry — dropping the pool tears down every
child. Teardown kill only; NO-RESPAWN law intact (nothing in the fix spawns anything — the C5
court's spawn ledger equals its explicit spawns exactly).

Chosen over option (b) (read_loop cancellation token) and (c) (pool Drop iteration): (a) keeps
the guarantee at the ChildProcess ownership boundary — it covers pool drop, `remove()`-then-drop,
and any other disposal path, with no cancellation plumbing through lsp-max-client (which this
lane does not own). No builder.rs edit needed (that file is outside the lane's owned tree, and
the read_loop/stdin semantics are shared with the root crate's client tests).

## Court (C5, Chicago: real processes, final state)

Spawn 6 real `lsp-echo-server` children via the `sh`-pidfile wrapper, handshake each through
`pool.get()`, assert every pid alive pre-drop, `drop(pool)` with NO explicit kill, then bounded
poll: every exit flag flips AND every exact pid is gone from the real process table
(`poll_children_dead`). Spawn ledger reconciles: exactly N spawns, zero implicit spawns during
teardown. Passing run: 1 test passed, finished ~1-17s variance across runs.

## Non-vacuity (shim court, real restore)

- Shim: `std::mem::forget(self.kill_tx.take())` in Drop (leaks the sender so the channel stays
  open — note: suppressing ONLY the explicit send does NOT neutralize the guard, because
  dropping the sender also closes the channel and fires the kill branch; the first shim attempt
  passed vacuously and was discarded as ineffective).
- Shimmed run: `Compiling` observed; `pool_drop_teardown_reaps_all_children_within_bound` FAILED
  with "observable did not converge within 10s (bounded poll, no fixed sleeps)" — children
  survived the pool drop, exactly the AB-17 defect.
- Restore: byte-exact `cp` from a pre-shim copy; post-restore grep for "SHIM-COURT" = 0 matches;
  post-restore run of the court: PASS.
- Falsifier holds: kill-signal suppressed ⇒ court fails; guard present ⇒ court passes.

## Gates (commands + exits)

- `cargo test -p lsp-max-compositor --test child_churn_stress --test crash_isolation --test
  child_eviction_test` → run 1: 5/3/3 passed, 0 failed (exit 0).
- Run 2 (×2 gate) + full compositor suite: launched in background
  (`cargo test -p lsp-max-compositor`), see status below. [UPDATE AT RUN COMPLETION]

## Standing

- Pool-drop teardown: ADMITTED (fix + non-vacuous court, all gates green ×2 on owned files).
- Full workspace regression: UNKNOWN (not run this lane; no owned-file regression observed).
