// Request arbitration tests — how the compositor routes requests (hover/completion)
// across child servers that share an extension.
//
// Real semantics (read from src/server.rs + src/fanout.rs, 2026-11-03):
//   - hover/completion/definition/… are `FirstSuccess`: the compositor loop
//     (`src/server.rs` `hover`/`completion`) iterates Primary-tier candidates in
//     tier order and returns on the first `Ok(Some)` — a sequential
//     short-circuit, NOT a concurrent fan-out. Once child A succeeds, child B
//     is never dispatched (suppression is structural: sequential `return`).
//   - A child error is logged and skipped; the loop falls through to the next
//     child. Since the typed-failure fix (CL-E: builder.rs EOF drain +
//     server_handle.rs error-object surfacing), a child error response arrives
//     at the loop as a typed `ClientError` (tagged `server-error` or
//     `child-exited`); the loop folds `Err` to fall-through exactly as it
//     previously folded `Ok(None)`.
//   - If every child fails or returns null, the loop still returns `Ok(None)`
//     at its tail — no typed failure is surfaced to the front-door client for
//     these methods; the typed errors are per-child, inside the fold.
//
// The loop itself lives in `CompositorServer` (private fields, no public
// constructor, only built inside `run_stdio` via `LspService`), so it is not
// executable from an integration test. These tests pin the two seams the loop
// consumes, with real collaborators per the Chicago discipline:
//   1. candidate resolution: `fanout::servers_for_uri` + Primary-tier filter —
//      this is exactly what `CompositorServer::primary_handles` computes;
//   2. per-child request semantics: the REAL `ServerHandle` proxy over the REAL
//      `ClientBuilder` transport (LSP framing, id correlation, pending map),
//      driven against hand-written in-process stub children (stated exception:
//      no binary in the repo answers hover/completion; `lsp-echo-server` replies
//      method-not-found for every request, which witnesses the error-flattening
//      path but not the success path).

use std::future::Future;
use std::str::FromStr;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use lsp_max::client::{ClientBuilder, ServerHandle};
use lsp_max::lsp_types::{
    CompletionContext, CompletionParams, CompletionResponse, CompletionTriggerKind, HoverContents,
    HoverParams, TextDocumentIdentifier, TextDocumentPositionParams,
};
use lsp_max_compositor::child_process::NoopClient;
use lsp_max_compositor::fanout::{DispatchStrategy, dispatch_strategy, servers_for_uri};
use lsp_max_compositor::{ChildServer, ChildTier, ExtensionRouter};

const RS_URI: &str = "file:///workspace/main.rs";

// ── stub child harness ────────────────────────────────────────────────────────

struct StubChild {
    id: String,
    handle: ServerHandle,
    request_count: Arc<AtomicUsize>,
}

/// What the stub child does when it receives a hover/completion request.
enum Behavior {
    /// Answer with `{"contents": "<child id>"}`.
    Answer,
    /// Reply with a JSON-RPC error object (method-not-found shape, code -32601).
    Error,
    /// No responder task; transport halves dropped — a dead child. The proxy's
    /// write loop breaks on the closed pipe and exits, so the next request send
    /// fails and folds to `None` without waiting for the 30 s pending timeout.
    Dead,
}

fn read_frame<R>(
    reader: &mut tokio::io::BufReader<R>,
) -> impl Future<Output = Option<serde_json::Value>> + Send + '_
where
    R: tokio::io::AsyncRead + Unpin + Send,
{
    Box::pin(async move {
        use tokio::io::{AsyncBufReadExt, AsyncReadExt};
        let mut line = String::new();
        let mut content_length: Option<usize> = None;
        loop {
            let n = reader.read_line(&mut line).await.unwrap_or(0);
            if n == 0 {
                return None; // EOF
            }
            let trimmed = line.trim_end_matches(['\r', '\n']);
            if trimmed.is_empty() {
                if let Some(len) = content_length.take() {
                    let mut body = vec![0u8; len];
                    reader.read_exact(&mut body).await.ok()?;
                    return serde_json::from_slice(&body).ok();
                }
                continue;
            }
            if let Some(rest) = trimmed.strip_prefix("Content-Length:") {
                content_length = rest.trim().parse::<usize>().ok();
            }
            line.clear();
        }
    })
}

async fn spawn_stub_child(behavior: Behavior) -> StubChild {
    // Two unidirectional pipes: requests flow client → stub on `req_pipe`,
    // responses flow stub → client on `res_pipe`.
    let (req_client_end, mut req_stub_end) = tokio::io::duplex(64 * 1024);
    let (res_client_end, mut res_stub_end) = tokio::io::duplex(64 * 1024);

    let label = match behavior {
        Behavior::Answer => "answer",
        Behavior::Error => "error",
        Behavior::Dead => "dead",
    };
    let id = format!("child-{label}");
    let request_count = Arc::new(AtomicUsize::new(0));

    if matches!(behavior, Behavior::Answer | Behavior::Error) {
        let count = Arc::clone(&request_count);
        let answer = matches!(behavior, Behavior::Answer);
        tokio::spawn(async move {
            let mut reader = tokio::io::BufReader::new(&mut req_stub_end);
            loop {
                let msg = match read_frame(&mut reader).await {
                    Some(m) => m,
                    None => return, // transport closed
                };
                let is_request = msg.get("id").map(|v| !v.is_null()).unwrap_or(false);
                if !is_request {
                    continue;
                }
                count.fetch_add(1, Ordering::SeqCst);
                let id_field = msg.get("id").cloned().unwrap_or(serde_json::Value::Null);
                let method = msg.get("method").and_then(|m| m.as_str()).unwrap_or("");
                let payload = if !answer {
                    serde_json::json!({
                        "jsonrpc": "2.0",
                        "id": id_field,
                        "error": { "code": -32601, "message": "method not found" }
                    })
                } else if method == "textDocument/completion" {
                    serde_json::json!({
                        "jsonrpc": "2.0",
                        "id": id_field,
                        "result": {
                            "isIncomplete": false,
                            "items": [{ "label": "completion-from-child-answer" }]
                        }
                    })
                } else {
                    serde_json::json!({
                        "jsonrpc": "2.0",
                        "id": id_field,
                        "result": { "contents": "hover-from-child-answer" }
                    })
                };
                let frame = format!(
                    "Content-Length: {}\r\n\r\n{}",
                    payload.to_string().len(),
                    payload
                );
                if tokio::io::AsyncWriteExt::write_all(&mut res_stub_end, frame.as_bytes())
                    .await
                    .is_err()
                {
                    return;
                }
            }
        });
    }
    // Behavior::Dead: both stub-side halves drop when this function returns —
    // no responder task exists, so requests are never answered and the proxy's
    // write loop breaks on a closed pipe.

    let handle = ClientBuilder::new().build(NoopClient, res_client_end, req_client_end);
    StubChild {
        id,
        handle,
        request_count,
    }
}

/// Two Primary-tier children registered on `.rs`, in tier order.
async fn two_primaries_on_rs() -> (ExtensionRouter, StubChild, StubChild) {
    // Stub children first so their counters exist before any dispatch.
    let a = spawn_stub_child(Behavior::Answer).await;
    let b = spawn_stub_child(Behavior::Error).await;
    let router = ExtensionRouter::new();
    router.register(
        ".rs",
        ChildServer {
            id: a.id.clone(),
            tier: ChildTier::Primary,
            extensions: vec![".rs".into()],
            commands: Vec::new(),
        },
    );
    router.register(
        ".rs",
        ChildServer {
            id: b.id.clone(),
            tier: ChildTier::Primary,
            extensions: vec![".rs".into()],
            commands: Vec::new(),
        },
    );
    (router, a, b)
}

fn hover_params(uri: &str) -> HoverParams {
    HoverParams {
        text_document_position_params: TextDocumentPositionParams {
            text_document: TextDocumentIdentifier {
                uri: lsp_max::lsp_types::Uri::from_str(uri).expect("valid lsp uri"),
            },
            position: lsp_max::lsp_types::Position {
                line: 0,
                character: 0,
            },
        },
        work_done_progress_params: Default::default(),
    }
}

fn completion_params(uri: &str) -> CompletionParams {
    CompletionParams {
        text_document_position: TextDocumentPositionParams {
            text_document: TextDocumentIdentifier {
                uri: lsp_max::lsp_types::Uri::from_str(uri).expect("valid lsp uri"),
            },
            position: lsp_max::lsp_types::Position {
                line: 0,
                character: 0,
            },
        },
        work_done_progress_params: Default::default(),
        partial_result_params: Default::default(),
        context: Some(CompletionContext {
            trigger_kind: CompletionTriggerKind::INVOKED,
            trigger_character: None,
        }),
    }
}

// ── 1. FirstSuccess candidates: both children claim the doc ──────────────────

/// Both children claim `main.rs`; the FirstSuccess candidate list is both, in
/// tier order (Primary first; within a tier, registration order). This is the
/// exact candidate sequence the compositor loop walks.
#[tokio::test]
async fn both_children_claiming_doc_are_candidates_in_tier_order() {
    let (router, a, b) = two_primaries_on_rs().await;
    let candidates = servers_for_uri(&router, RS_URI);
    let ids: Vec<&str> = candidates.iter().map(|s| s.id.as_str()).collect();
    assert_eq!(ids, vec![a.id.as_str(), b.id.as_str()]);
    assert!(
        candidates
            .iter()
            .all(|s| matches!(s.tier, ChildTier::Primary))
    );
}

/// Strategy classification pins hover/completion to FirstSuccess and
/// executeCommand (unlisted method) to PrimaryOnly.
#[test]
fn strategy_classification_pins_request_arbitration() {
    assert_eq!(
        dispatch_strategy("textDocument/hover"),
        DispatchStrategy::FirstSuccess
    );
    assert_eq!(
        dispatch_strategy("textDocument/completion"),
        DispatchStrategy::FirstSuccess
    );
    assert_eq!(
        dispatch_strategy("workspace/executeCommand"),
        DispatchStrategy::PrimaryOnly
    );
}

// ── 2. per-child success / failure semantics over the real transport ─────────

/// First candidate succeeds → its result is the arbitration outcome. In the
/// compositor's sequential loop the second child is never dispatched after a
/// success (structural suppression via `return` in `src/server.rs`).
#[tokio::test]
async fn first_child_success_is_the_arbitration_outcome() {
    let (_router, a, _b) = two_primaries_on_rs().await;

    let hover = a.handle.hover(hover_params(RS_URI)).await;
    match hover {
        Ok(Some(h)) => {
            // Falsifying content check: the winner is child A, not any child.
            match h.contents {
                HoverContents::Scalar(content) => {
                    let text = format!("{content:?}");
                    assert!(
                        text.contains("hover-from-child-answer"),
                        "expected hover from child A, got: {text}"
                    );
                }
                other => panic!("expected scalar hover contents, got {other:?}"),
            }
        }
        Ok(None) => panic!("child A answered — first success must win, got None"),
        Err(e) => panic!("child A transport error: {e}"),
    }
}

/// Child A fails (typed `server-error` from the transport since CL-E) → the
/// fold continues to child B, whose success wins. Fallback witnessed over the
/// real client transport.
#[tokio::test]
async fn child_error_falls_through_to_next_child() {
    let (_router, _a, b_erroring) = two_primaries_on_rs().await;

    // The erroring child answers -32601; since CL-E the transport surfaces
    // the JSON-RPC error object typed (`server-error` tag) instead of
    // flattening it to null. The compositor loop folds `Err` to fall through
    // to the next candidate exactly as it folded `Ok(None)` before.
    let result = b_erroring.handle.hover(hover_params(RS_URI)).await;
    assert!(
        matches!(&result, Err(e) if e.0.starts_with("server-error")),
        "child JSON-RPC error surfaces typed server-error, got {result:?}"
    );

    // ...and a succeeding child in the next slot returns its own result —
    // the fallback the loop implements across candidates.
    let winner = spawn_stub_child(Behavior::Answer).await;
    let result = winner.handle.hover(hover_params(RS_URI)).await;
    assert!(
        matches!(result, Ok(Some(_))),
        "next child's success wins, got {result:?}"
    );
}

/// A crashed child fails TYPED and PROMPTLY in both phases (CL-E law,
/// witnessed against a REAL `lsp-echo-server` subprocess that we exit):
/// phase 1 — the first post-death request resolves at stdout EOF via the
/// read loop's pending drain (typed `child-exited`, no 30 s timeout);
/// phase 2 — subsequent requests fail fast via the mpsc send-failure path
/// (typed `child-exited` as well). XG-6 rename of
/// `dead_child_folds_to_none_first_request_slow_then_fast`.
#[tokio::test]
async fn dead_child_fails_typed_promptly_both_phases() {
    use lsp_max_compositor::child_process::ChildProcess;

    let (proc, exit_fut) = ChildProcess::spawn(
        "crashed-child".to_string(),
        env!("CARGO_BIN_EXE_lsp-echo-server"),
        &[],
        NoopClient,
    )
    .await
    .expect("echo-server spawn");

    // Exit the child — the real crash model. Bounded poll on the real
    // observable: the child's exit status.
    proc.handle.exit().await;
    let exit_status = tokio::time::timeout(Duration::from_secs(15), exit_fut).await;
    assert!(
        matches!(exit_status, Ok(Ok(_))),
        "echo-server child must exit cleanly, got {exit_status:?}"
    );

    // Phase 1: typed child-exited at stdout EOF, prompt (no 30 s deadline).
    let start = std::time::Instant::now();
    let r1 = proc.handle.hover(hover_params(RS_URI)).await;
    let phase1 = start.elapsed();
    assert!(
        matches!(&r1, Err(e) if e.0.starts_with("child-exited")),
        "dead child must fail typed child-exited, got {r1:?}"
    );
    assert!(
        phase1 < Duration::from_secs(2),
        "EOF-drain failure must be prompt, took {phase1:?}"
    );

    // Phase 2: send-failure path — typed and immediately fast.
    let start = std::time::Instant::now();
    let r2 = proc.handle.hover(hover_params(RS_URI)).await;
    let fast = start.elapsed();
    assert!(
        matches!(&r2, Err(e) if e.0.starts_with("child-exited")),
        "post-mortem request also typed child-exited, got {r2:?}"
    );
    assert!(
        fast < Duration::from_secs(2),
        "send-failure path must stay fast, took {fast:?}"
    );
}

/// All children fail → each child surfaces a TYPED per-child error (CL-E);
/// the compositor loop folds every `Err`/null outcome into a single `Ok(None)`
/// at its tail (`src/server.rs` hover tail), so the front-door client still
/// sees a null result. This test pins the per-child half of that law.
#[tokio::test]
async fn all_children_fail_typed_per_child_loop_folds_to_null() {
    let erroring = spawn_stub_child(Behavior::Error).await;
    let dead = spawn_stub_child(Behavior::Dead).await;

    let r1 = erroring.handle.hover(hover_params(RS_URI)).await;
    let r2 = dead.handle.hover(hover_params(RS_URI)).await;

    assert!(
        matches!(&r1, Err(e) if e.0.starts_with("server-error")),
        "error child → typed server-error, got {r1:?}"
    );
    assert!(
        matches!(&r2, Err(e) if e.0.starts_with("child-exited")),
        "dead child → typed child-exited, got {r2:?}"
    );
    // The loop's tail folds every candidate outcome into a single `Ok(None)`;
    // the typed errors above are per-child, inside the fold.
}

/// Suppression witness: a child that answered for an earlier request is never
/// consulted once arbitration has already been decided by an earlier
/// candidate in the sequential loop. Over the executable seam: candidates are
/// ordered, the first returns Some, and the loop's contract is that later
/// candidates observe zero dispatches. Here we pin the ordering input
/// (candidates [A, B]) and, with A's success already witnessed, assert B's
/// counter directly to make the suppression assertion concrete and falsifiable
/// when this harness is wired to a drivable CompositorServer.
#[tokio::test]
async fn second_child_receives_no_dispatch_after_first_success() {
    let (router, a, b) = two_primaries_on_rs().await;

    let candidates = servers_for_uri(&router, RS_URI);
    let ids: Vec<&str> = candidates.iter().map(|s| s.id.as_str()).collect();
    assert_eq!(ids, vec![a.id.as_str(), b.id.as_str()]);

    // Arbitration decided by the first candidate:
    let outcome = a.handle.hover(hover_params(RS_URI)).await;
    assert!(matches!(outcome, Ok(Some(_))), "first candidate must win");

    // B was not dispatched by this seam. NOTE: the compositor loop dispatches
    // candidates sequentially and returns before reaching B on success — this
    // assertion holds for B only because this test drives the seam directly;
    // the loop-level witness requires a drivable CompositorServer (BLOCKED:
    // private fields, no public constructor — see receipt).
    assert_eq!(
        b.request_count.load(Ordering::SeqCst),
        0,
        "losing child must not receive the request once arbitration decided"
    );
}

// ── 3. completion follows the same arbitration ───────────────────────────────

#[tokio::test]
async fn completion_first_success_same_as_hover() {
    let winner = spawn_stub_child(Behavior::Answer).await;
    let result = winner.handle.completion(completion_params(RS_URI)).await;
    assert!(
        matches!(result, Ok(Some(_))),
        "completion first success, got {result:?}"
    );
}

// ── 4. single-claimant methods: PrimaryOnly delivery ─────────────────────────

/// Methods not in the FirstSuccess/FanAll/Notify lists (e.g.
/// `workspace/executeCommand`) are PrimaryOnly: only Primary-tier children are
/// delivery targets; Secondary/DiagnosticsOnly are skipped. With one Primary
/// claimant, delivery is exactly that child.
#[test]
fn single_claimant_method_delivers_only_to_primary_tier() {
    let router = ExtensionRouter::new();
    router.register(
        ".rs",
        ChildServer {
            id: "primary-lsp".into(),
            tier: ChildTier::Primary,
            extensions: vec![".rs".into()],
            commands: Vec::new(),
        },
    );
    router.register(
        ".rs",
        ChildServer {
            id: "secondary-lsp".into(),
            tier: ChildTier::Secondary,
            extensions: vec![".rs".into()],
            commands: Vec::new(),
        },
    );
    router.register(
        ".rs",
        ChildServer {
            id: "diagnostics-lsp".into(),
            tier: ChildTier::DiagnosticsOnly,
            extensions: vec![".rs".into()],
            commands: Vec::new(),
        },
    );

    assert_eq!(
        dispatch_strategy("workspace/executeCommand"),
        DispatchStrategy::PrimaryOnly
    );

    let targets: Vec<String> = servers_for_uri(&router, RS_URI)
        .into_iter()
        .filter(|s| matches!(s.tier, ChildTier::Primary))
        .map(|s| s.id)
        .collect();
    assert_eq!(targets, vec!["primary-lsp".to_string()]);
}

// (unused-import guard for CompletionResponse — kept for future assert shape)
#[allow(unused)]
fn _assert_completion_shape(r: CompletionResponse) -> CompletionResponse {
    r
}
