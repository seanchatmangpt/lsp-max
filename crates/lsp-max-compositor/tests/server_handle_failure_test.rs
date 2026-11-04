// server_handle_failure_test.rs — dead-child vs live-silent typed-failure
// court for `lsp_max::client::ServerHandle`, driven through the compositor's
// real child processes. Closes M8's OPEN defect 1 (30 s laundering of in-flight
// child death into `Ok(None)`).
//
// Courts:
//   D1. SIGKILL a real child mid-request → each pending request resolves
//       promptly (bounded < 2 s) with a typed `child-exited` Err — never a
//       30 s Ok(None).
//   D3. Non-vacuity: a live-but-silent child still honors the bounded 30 s
//       timeout and resolves Ok(None) — the typed path must not fire for a
//       live child.
//
// No mocks. Every child is an OS process; every assert is on the resolved
// outcome and wall-clock bounds.

use lsp_max_compositor::child_process::{ChildProcess, NoopClient};

/// Stable child binary, resolved at compile time.
const STABLE_CHILD: &str = env!("CARGO_BIN_EXE_lsp-echo-server");

/// Bounded waits (seconds).
const HANDSHAKE_BOUND: u64 = 30;
const DEAD_CHILD_BOUND: u64 = 2;
const INFLIGHT_BOUND: u64 = 40;

/// Markers embedded in stubs' argv so pkill targets exactly one test's child —
/// never a bystander. DISTINCT per test: the courts run in parallel threads,
/// so a shared marker would let D1's pkill kill D3's live stub (witnessed
/// flake, fixed 2026-11-03).
const KILL_MARKER: &str = "lspmax-shf-kill-do-not-match";
// The live-silent court's stub carries its own marker in argv
// (`lspmax-shf-silent-do-not-match`) so no court's pkill can match it.

/// Shell stub: answers one `initialize` (id 1), then goes silent forever
/// while advertising its kill marker in argv.
const SILENT_STUB: &str = r#"
# lspmax-shf-kill-do-not-match
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

/// Same silent stub, different argv marker — never pkill'ed by any court.
const LIVE_SILENT_STUB: &str = r#"
# lspmax-shf-silent-do-not-match
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

/// Spawn a child, run the initialize handshake under `HANDSHAKE_BOUND`.
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
    tokio::time::timeout(
        std::time::Duration::from_secs(HANDSHAKE_BOUND),
        proc.initialize(None),
    )
    .await
    .expect("initialize handshake must complete within bound")
    .expect("initialize handshake must succeed");
    (proc, exit_fut)
}

/// pkill -9 -f the marker, bounded poll (no fixed sleep; argv visibility can
/// lag spawn under load).
async fn kill_by_marker(marker: &str) {
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    while std::time::Instant::now() < deadline {
        let out = std::process::Command::new("pkill")
            .args(["-9", "-f", marker])
            .output()
            .expect("pkill must run");
        if out.status.code() == Some(0) {
            return;
        }
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    }
    panic!("pkill never matched marker {marker}");
}

/// D1: SIGKILL a real child with TWO requests in flight; both pending
/// requests must resolve promptly (bounded < 2 s) with a typed `child-exited`
/// Err — never a 30 s Ok(None).
#[tokio::test]
async fn sigkilled_child_fails_pending_requests_typed() {
    let (child, _exit) = spawn_child("shf-kill", "sh", &["-c", SILENT_STUB]).await;
    let handle_a = child.handle.clone();
    let handle_b = child.handle.clone();

    let req_a = tokio::spawn(async move {
        handle_a
            .request::<serde_json::Value>("textDocument/hover", serde_json::json!({}))
            .await
    });
    let req_b = tokio::spawn(async move {
        handle_b
            .request::<serde_json::Value>("textDocument/completion", serde_json::json!({}))
            .await
    });

    kill_by_marker(KILL_MARKER).await;

    let start = std::time::Instant::now();
    let outcome_a = tokio::time::timeout(std::time::Duration::from_secs(INFLIGHT_BOUND), req_a)
        .await
        .expect("join must not panic")
        .expect("pending request must resolve within bound — a hang is the defect");
    let outcome_b = tokio::time::timeout(std::time::Duration::from_secs(INFLIGHT_BOUND), req_b)
        .await
        .expect("join must not panic")
        .expect("pending request must resolve within bound — a hang is the defect");
    let elapsed = start.elapsed();

    // D1: typed child-exited, promptly (bound 2 s on the resolution; the
    // timeout() ceiling above is just the anti-hang fence).
    for outcome in [outcome_a, outcome_b] {
        assert!(
            matches!(&outcome, Err(e) if e.0.starts_with("child-exited")),
            "dead child must fail typed child-exited, got {outcome:?}"
        );
    }
    assert!(
        elapsed < std::time::Duration::from_secs(DEAD_CHILD_BOUND),
        "typed failure must be prompt at stdout EOF, took {elapsed:?}"
    );
}

/// D3 — non-vacuity: a LIVE-but-silent child must still hit the bounded 30 s
/// timeout and resolve Ok(None); the typed child-exited path must not fire
/// for a live child.
#[tokio::test]
#[ignore = "30 s wall-clock by design; run with --include-ignored"]
async fn live_silent_child_still_honors_bounded_timeout() {
    let (child, _exit) = spawn_child("shf-silent", "sh", &["-c", LIVE_SILENT_STUB]).await;

    let start = std::time::Instant::now();
    let outcome = child
        .handle
        .request::<serde_json::Value>("textDocument/hover", serde_json::json!({}))
        .await;
    let elapsed = start.elapsed();

    assert!(
        matches!(&outcome, Ok(None)),
        "live-silent child must resolve Ok(None) via timeout, got {outcome:?}"
    );
    assert!(
        elapsed >= std::time::Duration::from_secs(28),
        "timeout path must actually burn the 30 s budget, took {elapsed:?}"
    );
    assert!(
        elapsed < std::time::Duration::from_secs(INFLIGHT_BOUND),
        "timeout path must stay bounded, took {elapsed:?}"
    );
}

/// D4 — healthy-transport witness: a STABLE live child answers promptly
/// (echo-server replies method-not-found for hover, which now surfaces as a
/// typed `server-error`) — the point is prompt resolution with the child
/// still alive: never `child-exited`, never the 30 s timeout.
#[tokio::test]
async fn stable_child_transport_healthy_prompt_response() {
    let (child, _exit) = spawn_child("shf-stable", STABLE_CHILD, &[]).await;
    let start = std::time::Instant::now();
    let outcome = tokio::time::timeout(
        std::time::Duration::from_secs(DEAD_CHILD_BOUND),
        child
            .handle
            .request::<serde_json::Value>("textDocument/hover", serde_json::json!({})),
    )
    .await
    .expect("stable round-trip must complete within bound");
    let elapsed = start.elapsed();
    match &outcome {
        Ok(_) => {} // a real result/null answer from a live child
        Err(e) => assert!(
            e.0.starts_with("server-error"),
            "live child must answer promptly (ok or server-error), got {outcome:?}"
        ),
    }
    assert!(
        elapsed < std::time::Duration::from_secs(DEAD_CHILD_BOUND),
        "healthy round-trip must be prompt, took {elapsed:?}"
    );
}

/// D5 — server-sent JSON-RPC error responses surface typed, not laundered
/// into Ok(None). Stub answers initialize (id 1) then answers the next
/// request with a JSON-RPC error object.
#[tokio::test]
async fn server_error_response_surfaces_typed() {
    let stub = r#"
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
cl2=''
while read -r line; do
  line=$(printf '%s' "$line" | tr -d '\r')
  [ -z "$line" ] && break
  case "$line" in
    Content-Length:*) cl2="${line#Content-Length: }" ;;
  esac
done
dd bs=1 count="$cl2" > /dev/null 2>&1
err='{"jsonrpc":"2.0","id":2,"error":{"code":-32601,"message":"MethodNotFound"}}'
printf 'Content-Length: %d\r\n\r\n%s' "${#err}" "$err"
while true; do sleep 1; done
"#;
    let (child, _exit) = spawn_child("shf-err", "sh", &["-c", stub]).await;
    let outcome = tokio::time::timeout(
        std::time::Duration::from_secs(DEAD_CHILD_BOUND),
        child
            .handle
            .request::<serde_json::Value>("textDocument/hover", serde_json::json!({})),
    )
    .await
    .expect("error response must arrive within bound");
    assert!(
        matches!(&outcome, Err(e) if e.0.starts_with("server-error") && e.0.contains("MethodNotFound")),
        "server JSON-RPC error must surface typed server-error, got {outcome:?}"
    );
}
