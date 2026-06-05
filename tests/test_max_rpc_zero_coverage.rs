//! Smoke-level integration tests for the 15 max/ RPC methods that previously had
//! zero test coverage.
//!
//! Each test: boot server → send request → assert response has 'result' key (not 'error').
//! This prevents silent regressions when dispatch branches are refactored.

use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tower_lsp_max::{LanguageServer, LspService, Server};

static TEST_MUTEX: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

use tower_lsp_max::jsonrpc::Result as RpcResult;
use tower_lsp_max::lsp_types as lsp;

struct TestBackend;

#[tower_lsp_max::async_trait]
impl LanguageServer for TestBackend {
    async fn initialize(&self, _: lsp::InitializeParams) -> RpcResult<lsp::InitializeResult> {
        Ok(lsp::InitializeResult::default())
    }
    async fn shutdown(&self) -> RpcResult<()> {
        Ok(())
    }
}

type TxShared = Arc<tokio::sync::Mutex<Option<tokio::io::DuplexStream>>>;
type RxLog = Arc<std::sync::Mutex<Vec<serde_json::Value>>>;
type SerialGuard = tokio::sync::MutexGuard<'static, ()>;

async fn read_message<R: tokio::io::AsyncRead + Unpin>(
    reader: &mut R,
) -> std::io::Result<serde_json::Value> {
    let mut header_buf = Vec::new();
    loop {
        let mut byte = [0u8; 1];
        reader.read_exact(&mut byte).await?;
        header_buf.push(byte[0]);
        if header_buf.ends_with(b"\r\n\r\n") {
            break;
        }
    }
    let header_str = String::from_utf8(header_buf)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    let len_line = header_str
        .lines()
        .next()
        .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::InvalidData, "Empty header"))?;
    let content_len: usize = len_line["Content-Length: ".len()..]
        .trim()
        .parse()
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    let mut body = vec![0u8; content_len];
    reader.read_exact(&mut body).await?;
    Ok(serde_json::from_slice(&body)?)
}

fn encode_message(msg: &serde_json::Value) -> Vec<u8> {
    let payload = serde_json::to_string(msg).unwrap();
    format!("Content-Length: {}\r\n\r\n{}", payload.len(), payload).into_bytes()
}

async fn write_msg(tx_shared: &TxShared, msg: serde_json::Value) {
    let mut guard = tx_shared.lock().await;
    if let Some(ref mut tx) = *guard {
        tx.write_all(&encode_message(&msg)).await.unwrap();
    }
}

async fn wait_for_response(received: RxLog, id: i64, timeout: Duration) -> serde_json::Value {
    let start = std::time::Instant::now();
    loop {
        if start.elapsed() > timeout {
            panic!("Timeout waiting for response id={}", id);
        }
        {
            let mut guard = received.lock().unwrap();
            if let Some(pos) = guard
                .iter()
                .position(|msg| msg.get("id").and_then(|i| i.as_i64()) == Some(id))
            {
                return guard.remove(pos);
            }
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
}

async fn boot_server() -> (TxShared, RxLog, tokio::task::JoinHandle<()>, SerialGuard) {
    let _guard = TEST_MUTEX.lock().await;
    tower_lsp_max::reset_registry_for_tests();
    let _ = std::fs::remove_file("admission.receipt");
    let _ = std::fs::remove_file("security.receipt");
    let _ = std::fs::remove_file("auth.receipt");

    let (service, socket) = LspService::new(|_| TestBackend);
    let (client_tx, server_rx) = tokio::io::duplex(1024 * 1024);
    let (server_tx, client_rx) = tokio::io::duplex(1024 * 1024);

    let server_handle = tokio::spawn(async move {
        let _ = Server::new(server_rx, server_tx, socket).serve(service).await;
    });

    let client_tx_shared: TxShared = Arc::new(tokio::sync::Mutex::new(Some(client_tx)));
    let received: RxLog = Arc::new(std::sync::Mutex::new(Vec::new()));
    let received_clone = received.clone();

    let mut client_rx_owned = client_rx;
    tokio::spawn(async move {
        while let Ok(msg) = read_message(&mut client_rx_owned).await {
            received_clone.lock().unwrap().push(msg);
        }
    });

    write_msg(
        &client_tx_shared,
        serde_json::json!({"jsonrpc":"2.0","id":0,"method":"initialize","params":{"capabilities":{}}}),
    )
    .await;
    wait_for_response(received.clone(), 0, Duration::from_secs(3)).await;

    (client_tx_shared, received, server_handle, _guard)
}

fn assert_has_result(resp: &serde_json::Value, method: &str) {
    assert!(
        resp.get("result").is_some(),
        "method {} must return 'result', got: {}",
        method,
        resp
    );
}

fn cleanup_receipts() {
    let _ = std::fs::remove_file("admission.receipt");
    let _ = std::fs::remove_file("security.receipt");
    let _ = std::fs::remove_file("auth.receipt");
}

// ---------------------------------------------------------------------------
// max/hook
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "current_thread")]
async fn test_max_hook_returns_result() {
    let (tx, rx, _h, _guard) = boot_server().await;
    write_msg(&tx, serde_json::json!({"jsonrpc":"2.0","id":1,"method":"max/hook"})).await;
    let resp = wait_for_response(rx, 1, Duration::from_secs(3)).await;
    assert_has_result(&resp, "max/hook");
    cleanup_receipts();
}

// ---------------------------------------------------------------------------
// max/hookGraph
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "current_thread")]
async fn test_max_hook_graph_returns_result() {
    let (tx, rx, _h, _guard) = boot_server().await;
    write_msg(&tx, serde_json::json!({"jsonrpc":"2.0","id":1,"method":"max/hookGraph"})).await;
    let resp = wait_for_response(rx, 1, Duration::from_secs(3)).await;
    assert_has_result(&resp, "max/hookGraph");
    cleanup_receipts();
}

// ---------------------------------------------------------------------------
// max/chain
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "current_thread")]
async fn test_max_chain_returns_result() {
    let (tx, rx, _h, _guard) = boot_server().await;
    write_msg(&tx, serde_json::json!({"jsonrpc":"2.0","id":1,"method":"max/chain"})).await;
    let resp = wait_for_response(rx, 1, Duration::from_secs(3)).await;
    assert_has_result(&resp, "max/chain");
    cleanup_receipts();
}

// ---------------------------------------------------------------------------
// max/propagate
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "current_thread")]
async fn test_max_propagate_returns_result() {
    let (tx, rx, _h, _guard) = boot_server().await;
    write_msg(
        &tx,
        serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "max/propagate",
            "params": {
                "receipt_id": "rcpt-propagate-test",
                "hash": "abc123",
                "prev_receipt_hash": null
            }
        }),
    )
    .await;
    let resp = wait_for_response(rx, 1, Duration::from_secs(3)).await;
    assert_has_result(&resp, "max/propagate");
    cleanup_receipts();
}

// ---------------------------------------------------------------------------
// max/autonomicLoop
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "current_thread")]
async fn test_max_autonomic_loop_returns_result() {
    let (tx, rx, _h, _guard) = boot_server().await;
    write_msg(&tx, serde_json::json!({"jsonrpc":"2.0","id":1,"method":"max/autonomicLoop"})).await;
    let resp = wait_for_response(rx, 1, Duration::from_secs(3)).await;
    assert_has_result(&resp, "max/autonomicLoop");
    cleanup_receipts();
}

// ---------------------------------------------------------------------------
// max/manifoldSnapshot
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "current_thread")]
async fn test_max_manifold_snapshot_returns_result() {
    let (tx, rx, _h, _guard) = boot_server().await;
    write_msg(
        &tx,
        serde_json::json!({"jsonrpc":"2.0","id":1,"method":"max/manifoldSnapshot"}),
    )
    .await;
    let resp = wait_for_response(rx, 1, Duration::from_secs(3)).await;
    assert_has_result(&resp, "max/manifoldSnapshot");
    cleanup_receipts();
}

// ---------------------------------------------------------------------------
// max/lawfulTransition
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "current_thread")]
async fn test_max_lawful_transition_returns_result() {
    let (tx, rx, _h, _guard) = boot_server().await;
    write_msg(
        &tx,
        serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "max/lawfulTransition",
            "params": "Initializing"
        }),
    )
    .await;
    let resp = wait_for_response(rx, 1, Duration::from_secs(3)).await;
    assert_has_result(&resp, "max/lawfulTransition");
    cleanup_receipts();
}

// ---------------------------------------------------------------------------
// max/admission
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "current_thread")]
async fn test_max_admission_returns_result() {
    let (tx, rx, _h, _guard) = boot_server().await;
    write_msg(&tx, serde_json::json!({"jsonrpc":"2.0","id":1,"method":"max/admission"})).await;
    let resp = wait_for_response(rx, 1, Duration::from_secs(3)).await;
    assert_has_result(&resp, "max/admission");
    let result = resp.get("result").unwrap();
    let verdict = result.get("verdict").and_then(|v| v.as_str()).unwrap_or("");
    assert!(
        ["Admitted", "Refused", "Unknown"].contains(&verdict),
        "verdict must be Admitted/Refused/Unknown, got: {}",
        verdict
    );
    cleanup_receipts();
}

// ---------------------------------------------------------------------------
// max/refusal
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "current_thread")]
async fn test_max_refusal_returns_result() {
    let (tx, rx, _h, _guard) = boot_server().await;
    write_msg(
        &tx,
        serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "max/refusal",
            "params": "diag-test-refusal"
        }),
    )
    .await;
    let resp = wait_for_response(rx, 1, Duration::from_secs(3)).await;
    assert_has_result(&resp, "max/refusal");
    cleanup_receipts();
}

// ---------------------------------------------------------------------------
// max/replay
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "current_thread")]
async fn test_max_replay_returns_result() {
    let (tx, rx, _h, _guard) = boot_server().await;
    write_msg(&tx, serde_json::json!({"jsonrpc":"2.0","id":1,"method":"max/replay"})).await;
    let resp = wait_for_response(rx, 1, Duration::from_secs(3)).await;
    assert_has_result(&resp, "max/replay");
    cleanup_receipts();
}

// ---------------------------------------------------------------------------
// max/releaseActuation — may succeed or return an error if diagnostics block;
// either way the transport must return a well-formed JSON-RPC response.
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "current_thread")]
async fn test_max_release_actuation_returns_rpc_response() {
    let (tx, rx, _h, _guard) = boot_server().await;
    write_msg(
        &tx,
        serde_json::json!({"jsonrpc":"2.0","id":1,"method":"max/releaseActuation"}),
    )
    .await;
    let resp = wait_for_response(rx, 1, Duration::from_secs(3)).await;
    // Either result or error is acceptable — what matters is a well-formed response.
    assert!(
        resp.get("result").is_some() || resp.get("error").is_some(),
        "max/releaseActuation must return a JSON-RPC response, got: {}",
        resp
    );
    cleanup_receipts();
}

// ---------------------------------------------------------------------------
// max/dumpState
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "current_thread")]
async fn test_max_dump_state_returns_result() {
    let (tx, rx, _h, _guard) = boot_server().await;
    write_msg(&tx, serde_json::json!({"jsonrpc":"2.0","id":1,"method":"max/dumpState"})).await;
    let resp = wait_for_response(rx, 1, Duration::from_secs(3)).await;
    assert_has_result(&resp, "max/dumpState");
    cleanup_receipts();
}

// ---------------------------------------------------------------------------
// max/restoreState — dump first, then restore the same state
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "current_thread")]
async fn test_max_restore_state_returns_result() {
    let (tx, rx, _h, _guard) = boot_server().await;

    // First dump to get a valid state object
    write_msg(&tx, serde_json::json!({"jsonrpc":"2.0","id":1,"method":"max/dumpState"})).await;
    let dump_resp = wait_for_response(rx.clone(), 1, Duration::from_secs(3)).await;
    let state = dump_resp.get("result").expect("dumpState must return result").clone();

    // Now restore it
    write_msg(
        &tx,
        serde_json::json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "max/restoreState",
            "params": state
        }),
    )
    .await;
    let resp = wait_for_response(rx, 2, Duration::from_secs(3)).await;
    assert_has_result(&resp, "max/restoreState");
    cleanup_receipts();
}

// ---------------------------------------------------------------------------
// max/reset
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "current_thread")]
async fn test_max_reset_returns_result() {
    let (tx, rx, _h, _guard) = boot_server().await;
    write_msg(&tx, serde_json::json!({"jsonrpc":"2.0","id":1,"method":"max/reset"})).await;
    let resp = wait_for_response(rx, 1, Duration::from_secs(3)).await;
    assert_has_result(&resp, "max/reset");
    cleanup_receipts();
}

// ---------------------------------------------------------------------------
// max/conformanceDelta
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "current_thread")]
async fn test_max_conformance_delta_returns_result() {
    let (tx, rx, _h, _guard) = boot_server().await;
    write_msg(
        &tx,
        serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "max/conformanceDelta",
            "params": { "since_seq": 0 }
        }),
    )
    .await;
    let resp = wait_for_response(rx, 1, Duration::from_secs(3)).await;
    assert_has_result(&resp, "max/conformanceDelta");
    let result = resp.get("result").unwrap();
    assert!(
        result.get("deltas").is_some(),
        "conformanceDelta result must have 'deltas' key"
    );
    assert!(
        result.get("current_seq").is_some(),
        "conformanceDelta result must have 'current_seq' key"
    );
    cleanup_receipts();
}
