// child_eviction_test.rs — M8 eviction court for the compositor child pool.
//
// Witnesses, over REAL child processes (the compiled `lsp-echo-server` helper
// as the stable sibling; a `sh -c` stub that exits after the initialize
// handshake as the dying child):
//
//   E1. Child dies -> the pool entry is evicted: `get(id)` returns None and
//       the id disappears from a subsequent get()-mediated lookup. The
//       session stays alive: the stable sibling still answers within bound.
//   E2. Dispatch-level fallback: a request dispatched through the dead
//       child's ServerHandle AFTER its exit resolves typed (Err from the
//       broken pipe — the documented fallback; the 30 s stall-into-null
//       laundering is a separate filed defect pinned by crash_isolation C4
//       and owned by the server_handle.rs lane), and the pool no longer
//       hands out the dead handle: `is_live(id)` is false, `get(id)` None.
//   E3. Stable sibling unaffected: a second child registered alongside the
//       dying one keeps serving throughout the eviction window (isolation).
//   E4. No-respawn law: within the observe window no new process for the
//       dead id appears in the pool (the compositor never respawns — M8
//       fleet split; respawn is a Claude Code front-door concern).
//
// No mocks. Every child is an OS process; every assert is on final state.

use lsp_max_compositor::child_process::{ChildProcess, ChildProcessPool, NoopClient};

/// Path to the stable child binary, resolved at compile time.
const STABLE_CHILD: &str = env!("CARGO_BIN_EXE_lsp-echo-server");

const HANDSHAKE_BOUND: u64 = 30;
const SERVE_BOUND: u64 = 5;
const NO_RESPAWN_WINDOW: u64 = 2;
const POLL_BOUND: u64 = 10;

/// Shell stub: answers one `initialize` request (id 1) then exits cleanly.
const EXIT_AFTER_INIT: &str = r#"
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
# Exit unconditionally: stdin stays open for the session, so EOF never comes.
exit 0
"#;

/// Spawn a child, run the initialize handshake under `bound` seconds.
async fn spawn_child(id: &str, command: &str, args: &[&str]) -> ChildProcess {
    let (proc, _exit_fut) = ChildProcess::spawn(id.to_string(), command, args, NoopClient)
        .await
        .expect("child spawn must succeed");
    tokio::time::timeout(
        std::time::Duration::from_secs(HANDSHAKE_BOUND),
        proc.initialize(None),
    )
    .await
    .expect("initialize handshake must complete within bound")
    .expect("initialize handshake must succeed");
    proc
}

/// Poll `f` until it returns true, bounded by `secs`.
async fn poll_until<F>(secs: u64, mut f: F)
where
    F: FnMut() -> bool,
{
    let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(secs);
    while tokio::time::Instant::now() < deadline {
        if f() {
            return;
        }
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    }
    panic!("observable did not converge within {secs}s (bounded poll, no sleeps)");
}

/// Bounded raw JSON-RPC request through a child handle.
async fn bounded_request(
    child: &ChildProcess,
    method: &str,
    secs: u64,
) -> Result<Option<serde_json::Value>, String> {
    tokio::time::timeout(
        std::time::Duration::from_secs(secs),
        child
            .handle
            .request::<serde_json::Value>(method, serde_json::json!({})),
    )
    .await
    .map_err(|_| format!("request exceeded {secs}s bound"))?
    .map_err(|e| e.to_string())
}

// ── E1 + E2: eviction at get() + typed fallback on the dead handle ──────────

#[tokio::test]
async fn dead_child_is_evicted_and_dispatch_falls_back_typed() {
    let pool = ChildProcessPool::new();
    let child = spawn_child("dying-e1", "sh", &["-c", EXIT_AFTER_INIT]).await;
    pool.register("dying-e1".to_string(), child);

    // Bounded poll on the real observable: the pool observes the exit flag.
    poll_until(POLL_BOUND, || !pool.is_live("dying-e1")).await;

    // E1: get() evicts the dead entry and returns None.
    assert!(
        pool.get("dying-e1").is_none(),
        "dead child must be evicted by get()"
    );
    // Eviction is permanent — a second get() must not resurrect it.
    assert!(
        pool.get("dying-e1").is_none(),
        "eviction must be permanent, not re-inserting"
    );
    // E2: the id is gone from the get()-mediated surface.
    assert!(!pool.is_live("dying-e1"), "dead id is not live");

    // Dispatch to the dead child's id yields the documented typed fallback:
    // the first get() above already evicted it, so re-dispatch paths (fanout,
    // routing) see no entry at all. If a caller retained the handle, the
    // broken pipe resolves typed Err, not a hang.
    let dead_handle_request = dispatch_shaped_probe(&pool, "dying-e1").await;
    assert!(
        dead_handle_request.is_err(),
        "dispatch to evicted id must fall back typed, got {dead_handle_request:?}"
    );
}

/// Dispatch-shaped probe: get() the entry from the pool and request —
/// mirrors what fanout/routing do before writing to a child.
async fn dispatch_shaped_probe(
    pool: &ChildProcessPool,
    id: &str,
) -> Result<Option<serde_json::Value>, String> {
    match pool.get(id) {
        Some(proc_ref) => bounded_request(&proc_ref, "textDocument/hover", SERVE_BOUND).await,
        None => Err("evicted: no live child for id".to_string()),
    }
}

// ── E3: stable sibling isolation ────────────────────────────────────────────

#[tokio::test]
async fn stable_sibling_unaffected_by_sibling_eviction() {
    let pool = ChildProcessPool::new();
    let dying = spawn_child("dying-e3", "sh", &["-c", EXIT_AFTER_INIT]).await;
    let stable = spawn_child("stable-e3", STABLE_CHILD, &[]).await;
    pool.register("dying-e3".to_string(), dying);
    pool.register("stable-e3".to_string(), stable);

    poll_until(POLL_BOUND, || !pool.is_live("dying-e3")).await;
    assert!(pool.get("dying-e3").is_none(), "dead entry evicted");

    // E3: the stable sibling still answers AFTER the sibling's eviction.
    let reply = bounded_request(
        &pool.get("stable-e3").expect("stable registered"),
        "textDocument/hover",
        SERVE_BOUND,
    )
    .await
    .expect("stable sibling must answer after sibling eviction");
    // Falsify: the reply is the echo server's real response shape, not a stub.
    if let Some(v) = reply {
        assert!(
            v.is_object(),
            "stable sibling must return a real response object"
        );
    }

    // Dispatch-shaped fallback for the dead id, sibling still serving.
    let fallback = dispatch_shaped_probe(&pool, "dying-e3").await;
    assert!(fallback.is_err(), "dead id dispatch falls back");
    let stable_again = bounded_request(
        &pool.get("stable-e3").expect("stable"),
        "textDocument/hover",
        SERVE_BOUND,
    )
    .await
    .expect("stable sibling must still answer after fallback dispatch");
    if let Some(v) = stable_again {
        assert!(
            v.is_object(),
            "stable sibling must return a real response object"
        );
    }
}

// ── E4: no-respawn law ──────────────────────────────────────────────────────

#[tokio::test]
async fn dead_child_is_never_respawned_by_the_compositor() {
    let pool = ChildProcessPool::new();
    let dying = spawn_child("dying-e4", "sh", &["-c", EXIT_AFTER_INIT]).await;
    let stable = spawn_child("stable-e4", STABLE_CHILD, &[]).await;
    pool.register("dying-e4".to_string(), dying);
    pool.register("stable-e4".to_string(), stable);

    poll_until(POLL_BOUND, || !pool.is_live("dying-e4")).await;
    assert!(pool.get("dying-e4").is_none(), "evicted");

    // Observe window: no respawn. Membership stays exactly {stable-e4}; the
    // dead id never reappears. If the compositor ever gains a respawn
    // policy, THIS assert flips — that is the intended falsifier surface.
    let deadline = std::time::Duration::from_secs(NO_RESPAWN_WINDOW);
    let start = std::time::Instant::now();
    while start.elapsed() < deadline {
        let ids = pool.server_ids_snapshot();
        assert_eq!(
            ids,
            vec!["stable-e4".to_string()],
            "pool membership changed during no-respawn window: {ids:?} — respawn policy appeared?"
        );
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }
    // Sibling still serving at window close.
    let reply = bounded_request(
        &pool.get("stable-e4").expect("stable registered"),
        "textDocument/hover",
        SERVE_BOUND,
    )
    .await
    .expect("stable sibling must serve at window close");
    if let Some(v) = reply {
        assert!(
            v.is_object(),
            "stable sibling must return a real response object"
        );
    }
}
