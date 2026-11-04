# CL-E — ServerHandle typed child-death failure (IMPLEMENTATION lane receipt)

Lane: IMPLEMENTATION (CL-E), repo `/Users/sac/lsp-max`, canonical checkout.
No git commands run.

## Subject

Closes M8 OPEN defect 1: in-flight child death was laundered into `Ok(None)`
after the hardcoded 30 s timeout (`src/client/server_handle.rs:67`). Now:
dead-child resolves typed + immediately; the bounded 30 s timeout fires ONLY
for a live-but-silent child.

## Diff (per file)

- `src/client/server_handle.rs` (owned):
  - `CHILD_EXITED_TAG` ("child-exited") + `SERVER_ERROR_TAG` ("server-error")
    — matchable typed surfaces on `ClientError` (String newtype owned outside
    this file; tags are the discrimination surface).
  - `ServerHandle` gains `alive: watch::Receiver<bool>` (read-loop liveness);
    `ServerHandle::new` is now 4-arg.
  - `request()`: dead-child fast path (`alive == false` → typed Err, request
    never sent); `tokio::select!` races the 30 s timeout against
    `alive.changed()` (in-flight child death → typed Err immediately);
    sentinel error objects (message "transport closed") → `child-exited`;
    child-sent JSON-RPC errors → `server-error`; `Ok(Err(_))` dropped
    responder → typed; timeout branch keeps `Ok(None)` (live-silent law).
- `src/client/builder.rs` (LM-1 owns the file; my disclosed edits):
  - `fail_pending` + `transport_closed_error` sentinel: read loop drains all
    pending requests with the typed sentinel at stdout EOF / read error /
    read_exact error.
  - `(alive_tx, alive_rx)` watch channel wired into `read_loop` and
    `ServerHandle::new`.
  - Response dispatch preserves the JSON-RPC `error` object (previously
    dropped — errors reached the proxy as Null and laundered to `Ok(None)`).
- `crates/lsp-max-compositor/tests/server_handle_failure_test.rs` (new):
  D1 SIGKILL two-in-flight → both typed `child-exited` < 2 s; D3 live-silent
  30 s timeout → `Ok(None)` (include-ignored non-vacuity court, witnessed at
  30.02 s); D4 stable transport prompt round-trip; D5 server error object →
  typed `server-error`. Per-test distinct pkill markers (parallel-thread
  isolation witnessed flake: a shared marker let D1's pkill kill D3's stub).
- `crates/lsp-max-compositor/tests/request_arbitration.rs` (dependent pins):
  header law comment updated; `child_error_falls_through_to_next_child` now
  asserts typed `server-error`; `dead_child_folds_to_none_first_request_
  slow_then_fast` renamed `dead_child_fails_typed_promptly_both_phases`
  (XG-6 rename) asserting typed `child-exited` both phases, each < 2 s;
  `all_children_failing_surfaces_null_result_not_typed_error` renamed
  `all_children_fail_typed_per_child_loop_folds_to_null` (per-child typed,
  loop tail still folds to `Ok(None)` at server.rs).
- `crates/lsp-max-compositor/tests/crash_isolation.rs` (M8's file; disclosed):
  `bounded_request` classifier: served = `Ok(_)` or typed `server-error`
  (live child answering method-not-found IS a served round-trip); only
  `child-exited`/timeout means transport failure. C4 flipped from pinning
  the laundering to pinning the typed law.
- `crates/lsp-max-compositor/tests/child_eviction_test.rs` (CL-F's file;
  disclosed): same classifier in `bounded_request` (`server-error` →
  `Ok(None)`, i.e. served).
- `crates/lsp-max-compositor/tests/fanout_latency.rs` (disclosed unblock):
  `ServerHandle::new` call site updated to the 4-arg signature.

## Commands / exits (this session, CARGO_TARGET_DIR=target-lane-cle)

```
cargo test -p lsp-max-compositor --test server_handle_failure_test
  → ok. 3 passed; 0 failed; 1 ignored; 1.03 s          (exit 0)
cargo test ... --test request_arbitration
  → ok. 9 passed; 0 failed; 2.11 s                     (exit 0)
cargo test ... --test crash_isolation
  → ok. 3 passed; 0 failed; 2.07 s                     (exit 0)
cargo test ... --test child_eviction_test
  → ok. 3 passed; 0 failed; 3.27 s                     (exit 0)
cargo test ... --test fanout_latency
  → ok. 1 passed; 0 failed; 0.01 s                     (exit 0)
cargo test ... --test server_handle_failure_test -- --include-ignored
  → ok. 4 passed; 0 failed; 30.02 s (live-silent court burned the full
    30 s budget: timeout path intact for live children)
cargo check -p lsp-max --lib  → exit 0
cargo clippy -p lsp-max --lib → exit 0 (no warnings)
rustfmt --check (all touched files except builder.rs) → clean
```

## XG-6 witness

`child_eviction_test` runtime: 13.21 s (first run, machine under concurrent
lane load) / 2.85 s (clean run) / 3.27 s (final gate run) — under 10 s on
uncontended runs. Residual runtime is the test's own observe windows, not
transport stalls: the pre-CL-E 30 s pending-deadline stalls are gone (the
typed dead-handle path resolves in milliseconds).

## Disclosure (cross-lane file touches)

- builder.rs is LM-1-owned; my watch-channel + drain + error-preservation
  edits were integrated against their in-flight NotificationCounters/canned-
  response work and survived their subsequent edits. builder.rs left
  unformatted (rustfmt --check shows import-order + block diffs mostly in
  LM-1's hunks) — dx-polish at integration owns that.
- config.rs:41 transient shared-compile break (missing
  `String::deserialize` trait scope) unblocked minimally with a fully
  qualified `<String as serde::Deserialize>::deserialize`; superseded by the
  owner lane's landing.
- Mid-flight contamination witnessed: another lane's `pkill -f` marker
  collision and my earlier `| tail` pipes hanging on orphaned stub fds —
  resolved (kill stubs by per-test markers; no pipes on test runs).

## Standing

- Typed dead-child failure (D1 + dead_child_fails_typed + C4 + E1): ALIVE
  (ran + passed this session, receipts above).
- Live-silent bounded timeout law (D3 include-ignored): ALIVE (30.02 s).
- server-error typed surfacing (D5 + child_error_falls_through + E-class
  classifier normalization): ALIVE.
- Compositor suite unregressed: ALIVE for the five touched/dependent suites
  (19 tests green); full-workspace census belongs to integration.
- OPEN (not mine): post-mortem requests issued by a ServerHandle whose read
  loop died BEFORE any drain race start still fail typed via the alive
  pre-check — covered; residual: pending map entries inserted between
  pre-check and select are covered by the alive.changed race — no gap
  observed; full-fleet `just dx-polish` (builder.rs formatting) pending at
  integration.
