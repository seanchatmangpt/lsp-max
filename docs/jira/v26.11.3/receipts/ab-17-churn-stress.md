# AB-17 — Child-Churn Kill-Stress Court (lsp-max-compositor)

- **Lane**: STRESS lane AB-17, wave v26.11.3
- **Subject**: `crates/lsp-max-compositor/src/child_process.rs` (READ-ONLY — no
  source changes; the only file touched is the new test + this receipt)
- **Pattern source**: cgap-beam `tests/kill_stress.rs` (real collaborator,
  exact-count accounting, bounded convergence barriers — adapted from BEAM
  process kills to OS SIGKILL children)
- **Status**: ALIVE — commands + exits below run this session.

## Deliverable

`crates/lsp-max-compositor/tests/child_churn_stress.rs` (new file, ~570 lines).
Four courts, all over REAL `lsp-echo-server` OS children (`CARGO_BIN_EXE_lsp-echo-server`),
no mocks, bounded polls on observables (no fixed sleeps as coordination;
poll-interval 25–50 ms inside bounded helpers only).

## Courts

### C1 — 200-cycle spawn/kill churn (`churn_200_cycles_spawn_exchange_sigkill_evict_respawn`)

200 cycles: spawn a real child through a `sh -c 'echo $$ > pidfile; exec echo-server'`
wrapper (exec keeps the pid — the SIGKILL lands on the LSP server, not the
shell) → initialize handshake → 3 live requests → SIGKILL mid-flight →
bounded poll: exit flag flips + exact pid reaped from the process table →
`pool.get(id)` evicts → id never reappears → next cycle.

Leak witnesses at cycles 50/100/150/200 (measured, this session):

| checkpoint | descendants | fds | RSS |
|---|---|---|--- | 
| baseline | 4 | 34 | 12144 KiB |
| 50 | 1 | 10 | 13040 KiB |
| 100/150/200/final | 1 | 10 | 13088 KiB (plateau, 0 growth) |

Measured: fd table DROPS 34→10 and plateaus; RSS plateaus at 13088 KiB from
cycle 100 on; every killed pid reaped per cycle. No fd/RSS/process leak
across 200 cycles. Asserted bounds: fds ≤ baseline+16, RSS ≤ 2×baseline+16 MiB.

### C2 — 8×500 concurrent-fanout soak (`fanout_soak_8_children_500_requests_no_cross_bleed`)

8 children × 500 requests interleaved (JoinSet, one task per child). Witness:
per-child attribution intact (`compositor/secondary:fan-N` prefixes), 500/500
served per child on ITS OWN handle (per-handle pending-map correlation; any
cross-child bleed would surface as mid-soak `child-exited`/framing failure),
4000/4000 total, 0 child deaths, 0 transport failures, spawn ledger 8 == 8.

### C3 — Buffer consistency under kill (`buffer_stays_consistent_when_children_are_sigkilled_mid_buffering`)

4 children deposit ANDON diagnostics; a concurrent deposit/flush writer runs
DURING the SIGKILLs (exact pids from pidfiles — never a process-table sweep,
which would hit sibling courts' children under the concurrent harness);
post-kill: exit flags flip + pids reaped, pool eviction observed, every URI
still merges to exactly its own `GGEN-CHURN-00N` code (last-write-wins per
server_id, no phantom entries from dead children), ANDON counter reconciles
to zero exactly once, buffered-URI count drains to 0, no panic.

### C4 — NO-RESPAWN law court (`no_respawn_law_killed_children_are_never_auto_respawned`)

20-cycle kill burst; per-court `SpawnLedger` (Arc<AtomicUsize>, per-court
NOT process-global — the harness runs the courts concurrently in one process,
so a global counter would catch sibling spawns) reconciles delta == explicit
spawns exactly. Pool membership stays empty through a 2 s observe window,
evicted ids never reappear. If the compositor ever gains a respawn policy,
this court flips — the intended falsifier surface.

## Gates (run this session)

- `cargo test -p lsp-max-compositor --test child_churn_stress` — **RUN 1:
  `ok. 4 passed; 0 failed` in 12.14s**
- Same command **RUN 2 (with `--nocapture`): `ok. 4 passed; 0 failed` in
  13.54s** (witness lines above captured from this run)
- Regression: `cargo test -p lsp-max-compositor` (full crate, 24 test
  binaries): **all `ok`, 0 failed everywhere**; child_churn_stress passed a
  third time inside the crate run (12.60s).

## Findings (pre-existing, read-only subjects — not session-introduced)

1. **Pool drop does NOT stop children** (`src/client/builder.rs:96`): the
   original mpsc sender is moved into `read_loop`, which holds it until the
   child's stdout EOFs — so dropping `ChildProcess`/pool never produces stdin
   EOF; children must be torn down by kill. Not a leak (fd/RSS courts show
   bounded state), but teardown is the caller's job. Witnessed by the probe
   reproduction during AB-17 (child alive 5s after pool drop) and confirmed
   by source read (`builder.rs` `build()`), then honored in C2's explicit
   SIGKILL teardown.
2. **Court-design note**: process-global witnesses (descendant count == 0,
   global spawn counter) are invalid under the concurrent test harness (all
   courts in one process); the courts use per-pid death checks and per-court
   ledgers instead. Bounded-ceiling witnesses (fd/RSS) remain process-global
   but tolerate sibling noise (tolerance covers sibling courts' bounded
   in-flight children).
