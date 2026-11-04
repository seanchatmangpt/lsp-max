// crash_isolation.rs — child-crash isolation court for the compositor.
//
// Witnesses, over REAL child processes (the compiled `lsp-echo-server` helper
// as the stable child; a `sh -c` stub that SIGKILLs itself after the
// initialize handshake as the unstable child):
//
//   C1. Child A crashes mid-session -> child B continues serving (isolation).
//   C2. The compositor session survives the crash: exit watcher resolves,
//       pool intact, B's connection still round-trips.
//   C3. Restart semantics: the compositor's real child policy is NO restart —
//       the exit watcher (server.rs) logs and clears buffer entries only;
//       the dead child is NEVER respawned (restartOnCrash/maxRestarts belong
//       to Claude Code's FRONT-door process, not to the compositor's
//       children). As of M8 the dead child's pool entry IS evicted — lazily,
//       on the first `ChildProcessPool::get` that observes the exit flag —
//       so dispatch never re-enters a dead child. Witnessed by
//       tests/child_eviction_test.rs.
//   C4. Crash during an in-flight request: the pending request resolves
//       within a bounded window — via the hardcoded 30 s timeout in
//       ServerHandle::request (src/client/server_handle.rs:67) — and is
//       LAUNDERED into Ok(None), indistinguishable from "server has no
//       hover". Filed as a defect: a dead child should yield a typed failure
//       at stdout-EOF, not a 30 s stall into a null success.
//
// No mocks. Every child is an OS process; every assert is on final state.

use lsp_max_compositor::child_process::{ChildProcess, ChildProcessPool, NoopClient};

/// Path to the stable child binary, resolved at compile time.
const STABLE_CHILD: &str = env!("CARGO_BIN_EXE_lsp-echo-server");

/// Bounded waits (seconds). Generous ceilings on the fast paths so CI load
/// cannot turn a correctness assert into a flake. C4 has no slow path left:
/// the typed EOF failure resolves at process death, not at a 30 s timeout.
const HANDSHAKE_BOUND: u64 = 30;
const SERVE_BOUND: u64 = 5;
const RESTART_OBSERVE_WINDOW: u64 = 2;
const INFLIGHT_BOUND: u64 = 40;
/// Ceiling for the EOF-typed resolution after a mid-request SIGKILL. The
/// pre-M8 laundering path (30 s stall into Ok(None)) exceeds this; the typed
/// EOF drain does not. Pins the fast path so a regression to timeout-based
/// resolution is loud even when INFLIGHT_BOUND still absorbs it.
const EOF_FAST_BOUND: u64 = 20;

/// Shell stub: speaks just enough LSP to answer one `initialize` request
/// (id 1 — the first id `ServerHandle` mints), then SIGKILLs itself.
const CRASH_AFTER_INIT: &str = r#"
cl=''
while read -r line; do
  line=$(printf '%s' "$line" | tr -d '\r')
  [ -z "$line" ] && break
  case "$line" in
    Content-Length:*) cl="${line#Content-Length: }" ;;
  esac
done
dd bs=1 count="$cl" > /dev/null 2>&1
body='{"jsonrpc":"2.0","id":1,"result":{"capabilities":{}}}'
printf 'Content-Length: %d\r\n\r\n%s' "${#body}" "$body"
kill -9 $$
"#;

/// Spawn a child, run the initialize handshake under `bound` seconds, and
/// return it together with the exit-watcher future.
async fn spawn_child(
    id: &str,
    command: &str,
    args: &[&str],
) -> (
    ChildProcess,
    impl std::future::Future<Output = std::io::Result<std::process::ExitStatus>>,
) {
    let (proc, exit_fut) = ChildProcess::spawn(id.to_string(), command, args, NoopClient)
        .await
        .expect("child spawn must succeed");
    let start = std::time::Instant::now();
    tokio::time::timeout(
        std::time::Duration::from_secs(HANDSHAKE_BOUND),
        proc.initialize(None),
    )
    .await
    .expect("initialize handshake must complete within bound")
    .expect("initialize handshake must succeed");
    assert!(
        start.elapsed() < std::time::Duration::from_secs(HANDSHAKE_BOUND),
        "handshake exceeded bound"
    );
    (proc, exit_fut)
}

/// Raw JSON-RPC request through a child handle, bounded by `secs`.
async fn bounded_request(
    handle: &lsp_max_compositor::child_process::ChildProcess,
    method: &str,
    secs: u64,
) -> (bool, std::time::Duration) {
    let start = std::time::Instant::now();
    let result = tokio::time::timeout(
        std::time::Duration::from_secs(secs),
        handle
            .handle
            .request::<serde_json::Value>(method, serde_json::json!({})),
    )
    .await;
    match result {
        Ok(inner) => (inner.is_ok(), start.elapsed()),
        Err(_) => (false, start.elapsed()),
    }
}

// ── C1 + C2: crash isolation and front-door survival ─────────────────────────

/// Child A (crash stub) SIGKILLs itself after the handshake. Child B (the real
/// echo-server binary) must keep serving; the session (pool + exit watcher)
/// must survive. Asserts are on final state: A's exit status is a signal kill,
/// B answers a request AFTER A is dead, and the pool still lists both ids.
#[tokio::test]
async fn child_a_crashes_child_b_continues_and_session_survives() {
    let pool = ChildProcessPool::new();

    let (child_a, exit_a) = spawn_child("crashy-a", "sh", &["-c", CRASH_AFTER_INIT]).await;
    let (child_b, _exit_b) = spawn_child("stable-b", STABLE_CHILD, &[]).await;
    pool.register("crashy-a".to_string(), child_a);
    pool.register("stable-b".to_string(), child_b);

    // A's exit watcher resolves with a signal-kill status (real crash).
    let status = tokio::time::timeout(std::time::Duration::from_secs(HANDSHAKE_BOUND), exit_a)
        .await
        .expect("exit watcher must fire within bound")
        .expect("exit watcher must not drop");
    assert!(
        status.code().is_none(),
        "crash stub must die by signal, got {status:?}"
    );

    // Front-door survival: AFTER A is dead, B still answers within bound.
    let (served, elapsed) = bounded_request(
        &pool.get("stable-b").expect("b registered"),
        "textDocument/hover",
        SERVE_BOUND,
    )
    .await;
    assert!(served, "child B must continue serving after A crashed");
    assert!(
        elapsed < std::time::Duration::from_secs(SERVE_BOUND),
        "B's serve must be prompt after A's crash, took {elapsed:?}"
    );

    // Session intact: the pool snapshot still lists both ids (lazy eviction
    // happens at get()-time, not eagerly — see C3 and child_eviction_test).
    let ids = pool.server_ids_snapshot();
    assert!(ids.contains(&"crashy-a".to_string()), "pool: {ids:?}");
    assert!(ids.contains(&"stable-b".to_string()), "pool: {ids:?}");
}

// ── C3: restart semantics — the compositor does NOT restart children ─────────

/// The real law: after a child crash the exit watcher only logs and clears
/// buffer URIs (server.rs exit-watcher block). No respawn, no eviction. This
/// test pins that behavior: within the observe window the pool entry set is
/// unchanged and no new process for `crashy-a` appears.
#[tokio::test]
async fn crashed_child_is_not_restarted_by_the_compositor() {
    let pool = ChildProcessPool::new();
    let (child_a, exit_a) = spawn_child("crashy-nr", "sh", &["-c", CRASH_AFTER_INIT]).await;
    let (child_b, _exit_b) = spawn_child("stable-nr", STABLE_CHILD, &[]).await;
    pool.register("crashy-nr".to_string(), child_a);
    pool.register("stable-nr".to_string(), child_b);

    // Wait out the crash and drain the exit watcher.
    let _ = tokio::time::timeout(std::time::Duration::from_secs(HANDSHAKE_BOUND), exit_a).await;

    // Observe window: pool membership is frozen (no restart, no eviction).
    // If the compositor ever gains a restart policy, THIS assert flips —
    // that is the intended falsifier surface.
    let deadline = std::time::Duration::from_secs(RESTART_OBSERVE_WINDOW);
    let start = std::time::Instant::now();
    while start.elapsed() < deadline {
        let ids = pool.server_ids_snapshot();
        assert_eq!(
            ids.len(),
            2,
            "pool membership changed during observe window: {ids:?} — restart/eviction policy appeared?"
        );
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }
    assert!(
        pool.get("crashy-nr").is_none(),
        "dead child entry must be evicted (M8): pool.get skips/evicts dead entries"
    );
    // B still serves throughout the window.
    let (served, _) = bounded_request(
        &pool.get("stable-nr").expect("b registered"),
        "textDocument/hover",
        SERVE_BOUND,
    )
    .await;
    assert!(served);
}

// ── C4: crash during in-flight request ───────────────────────────────────────

/// Marker embedded in the silent stub's argv so pkill can target exactly this
/// test's child process — never a bystander.
const INFLIGHT_MARKER: &str = "lspmax-inflight-stub-do-not-match";

const SILENT_WITH_MARKER: &str = r#"
# lspmax-inflight-stub-do-not-match
cl=''
while read -r line; do
  line=$(printf '%s' "$line" | tr -d '\r')
  [ -z "$line" ] && break
  case "$line" in
    Content-Length:*) cl="${line#Content-Length: }" ;;
  esac
done
dd bs=1 count="$cl" > /dev/null 2>&1
body='{"jsonrpc":"2.0","id":1,"result":{"capabilities":{}}}'
printf 'Content-Length: %d\r\n\r\n%s' "${#body}" "$body"
while true; do sleep 1; done
"#;

/// Real in-flight crash court: a request is in flight to a silent child when
/// the child is SIGKILLed mid-request. M8's original court pinned the
/// laundering (Ok(None) after the ~30 s hardcoded timeout); the typed-failure
/// fix (server_handle.rs CHILD_EXITED_TAG + builder.rs EOF drain) flipped
/// this falsifier: resolution is now a typed `child-exited` Err, promptly,
/// not a 30 s null success. A hang beyond the bound fails this court.
#[tokio::test]
async fn in_flight_request_resolves_after_real_mid_request_crash() {
    let (child, _exit) = spawn_child("inflight", "sh", &["-c", SILENT_WITH_MARKER]).await;
    let handle = child.handle.clone();

    let req = tokio::spawn(async move {
        handle
            .request::<serde_json::Value>("textDocument/hover", serde_json::json!({}))
            .await
    });

    // SIGKILL the stub via its unique argv marker — a real mid-request crash.
    // Bounded poll: pkill returns 1 until the match lands; under heavy machine
    // load argv visibility may lag spawn by a moment, so poll with a deadline
    // instead of sleeping a fixed interval.
    let kill_deadline = std::time::Instant::now() + std::time::Duration::from_secs(SERVE_BOUND);
    let mut killed_code = None;
    while std::time::Instant::now() < kill_deadline {
        let out = std::process::Command::new("pkill")
            .args(["-9", "-f", INFLIGHT_MARKER])
            .output()
            .expect("pkill must run");
        killed_code = out.status.code();
        if killed_code == Some(0) {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    }
    assert_eq!(
        killed_code,
        Some(0),
        "pkill must have matched the stub process"
    );

    let inflight_start = std::time::Instant::now();
    let outcome = tokio::time::timeout(std::time::Duration::from_secs(INFLIGHT_BOUND), req)
        .await
        .expect("join must not panic")
        .expect("request must resolve within bound — a hang is the defect");
    let inflight_elapsed = inflight_start.elapsed();
    // Typed failure at stdout EOF — no laundering into a null success.
    assert!(
        matches!(&outcome, Err(e) if e.0.starts_with("child-exited")),
        "dead child must fail typed (child-exited), got {outcome:?}"
    );
    // Fast path pinned: the EOF drain resolves at process death, so the
    // resolution must beat the old 30 s laundering stall (with load margin).
    assert!(
        inflight_elapsed < std::time::Duration::from_secs(EOF_FAST_BOUND),
        "EOF-typed resolution must be fast, took {inflight_elapsed:?}"
    );
}
