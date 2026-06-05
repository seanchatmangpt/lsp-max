//! Smoke-level integration tests for the 15 max/ RPC methods that previously had
//! zero test coverage.
//!
//! Each test: boot server → send request → assert response has 'result' key (not 'error').
//! This prevents silent regressions when dispatch branches are refactored.

use std::sync::Arc;
use std::time::Duration;
use tower_lsp_max::{LanguageServer, LspService, Server};

static TEST_MUTEX: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

use tower_lsp_max::jsonrpc::Result as RpcResult;
use tower_lsp_max::lsp_types as lsp;

mod common;
use common::{cleanup_receipts, read_message, wait_for_response, write_msg, RxLog, TxShared};

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

type SerialGuard = tokio::sync::MutexGuard<'static, ()>;

async fn boot_server() -> (TxShared, RxLog, tokio::task::JoinHandle<()>, SerialGuard) {
    let _guard = TEST_MUTEX.lock().await;
    tower_lsp_max::reset_registry_for_tests();
    let temp_dir = tempfile::tempdir().unwrap();
    let temp_path = temp_dir.path().to_path_buf();
    std::boxed::Box::leak(std::boxed::Box::new(temp_dir));
    if let Ok(mut reg) = tower_lsp_max::get_registry().lock() {
        reg.root_path = temp_path.clone();
    }
    let _ = std::fs::remove_file(temp_path.join("admission.receipt"));
    let _ = std::fs::remove_file(temp_path.join("security.receipt"));
    let _ = std::fs::remove_file(temp_path.join("auth.receipt"));

    let (service, socket) = LspService::new(|_| TestBackend);
    let (client_tx, server_rx) = tokio::io::duplex(1024 * 1024);
    let (server_tx, client_rx) = tokio::io::duplex(1024 * 1024);

    let server_handle = tokio::spawn(async move {
        let _ = Server::new(server_rx, server_tx, socket)
            .serve(service)
            .await;
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

async fn test_rpc_method(method: &str, params: serde_json::Value) -> serde_json::Value {
    let (tx, rx, _h, _guard) = boot_server().await;
    let payload = if params.is_null() {
        serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": method
        })
    } else {
        serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": method,
            "params": params
        })
    };
    write_msg(&tx, payload).await;
    let resp = wait_for_response(rx, 1, Duration::from_secs(3)).await;
    cleanup_receipts();
    resp
}

// ---------------------------------------------------------------------------
// max/hook
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "current_thread")]
async fn test_max_hook_returns_result() {
    let resp = test_rpc_method("max/hook", serde_json::Value::Null).await;
    assert_has_result(&resp, "max/hook");
}

// ---------------------------------------------------------------------------
// max/hookGraph
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "current_thread")]
async fn test_max_hook_graph_returns_result() {
    let resp = test_rpc_method("max/hookGraph", serde_json::Value::Null).await;
    assert_has_result(&resp, "max/hookGraph");
}

// ---------------------------------------------------------------------------
// max/chain
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "current_thread")]
async fn test_max_chain_returns_result() {
    let resp = test_rpc_method("max/chain", serde_json::Value::Null).await;
    assert_has_result(&resp, "max/chain");
}

// ---------------------------------------------------------------------------
// max/propagate
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "current_thread")]
async fn test_max_propagate_returns_result() {
    let resp = test_rpc_method(
        "max/propagate",
        serde_json::json!({
            "receipt_id": "rcpt-propagate-test",
            "hash": "abc123",
            "prev_receipt_hash": null
        }),
    )
    .await;
    assert_has_result(&resp, "max/propagate");
}

// ---------------------------------------------------------------------------
// max/autonomicLoop
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "current_thread")]
async fn test_max_autonomic_loop_returns_result() {
    let resp = test_rpc_method("max/autonomicLoop", serde_json::Value::Null).await;
    assert_has_result(&resp, "max/autonomicLoop");
}

// ---------------------------------------------------------------------------
// max/manifoldSnapshot
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "current_thread")]
async fn test_max_manifold_snapshot_returns_result() {
    let resp = test_rpc_method("max/manifoldSnapshot", serde_json::Value::Null).await;
    assert_has_result(&resp, "max/manifoldSnapshot");
}

// ---------------------------------------------------------------------------
// max/lawfulTransition
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "current_thread")]
async fn test_max_lawful_transition_returns_result() {
    let resp = test_rpc_method("max/lawfulTransition", serde_json::json!("Initializing")).await;
    assert_has_result(&resp, "max/lawfulTransition");
}

// ---------------------------------------------------------------------------
// max/admission
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "current_thread")]
async fn test_max_admission_returns_result() {
    let resp = test_rpc_method("max/admission", serde_json::Value::Null).await;
    assert_has_result(&resp, "max/admission");
    let result = resp.get("result").unwrap();
    let verdict = result.get("verdict").and_then(|v| v.as_str()).unwrap_or("");
    assert!(
        ["Admitted", "Refused", "Unknown"].contains(&verdict),
        "verdict must be Admitted/Refused/Unknown, got: {}",
        verdict
    );
}

// ---------------------------------------------------------------------------
// max/refusal
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "current_thread")]
async fn test_max_refusal_returns_result() {
    let resp = test_rpc_method("max/refusal", serde_json::json!("diag-test-refusal")).await;
    assert_has_result(&resp, "max/refusal");
}

// ---------------------------------------------------------------------------
// max/replay
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "current_thread")]
async fn test_max_replay_returns_result() {
    let resp = test_rpc_method("max/replay", serde_json::Value::Null).await;
    assert_has_result(&resp, "max/replay");
    let result = resp.get("result").unwrap();
    assert!(
        result.get("receipt_count").is_some(),
        "max/replay result must have 'receipt_count' key, got: {}",
        result
    );
    assert!(
        result.get("receipts").is_some(),
        "max/replay result must have 'receipts' key, got: {}",
        result
    );
}

// ---------------------------------------------------------------------------
// max/releaseActuation — may succeed or return an error if diagnostics block;
// either way the transport must return a well-formed JSON-RPC response.
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "current_thread")]
async fn test_max_release_actuation_returns_rpc_response() {
    let resp = test_rpc_method("max/releaseActuation", serde_json::Value::Null).await;
    assert!(
        resp.get("result").is_some() || resp.get("error").is_some(),
        "max/releaseActuation must return a JSON-RPC response, got: {}",
        resp
    );
}

// ---------------------------------------------------------------------------
// max/dumpState — DfLSS CTQ: result must contain 'diagnostics' field
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "current_thread")]
async fn test_max_dump_state_returns_result() {
    let resp = test_rpc_method("max/dumpState", serde_json::Value::Null).await;
    assert_has_result(&resp, "max/dumpState");
    let result = resp.get("result").unwrap();
    assert!(
        result.get("diagnostics").is_some(),
        "max/dumpState result must contain 'diagnostics' field (ServerRegistry), got: {}",
        result
    );
}

// ---------------------------------------------------------------------------
// max/restoreState — dump first, then restore the same state
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "current_thread")]
async fn test_max_restore_state_returns_result() {
    let (tx, rx, _h, _guard) = boot_server().await;

    // First dump to get a valid state object
    write_msg(
        &tx,
        serde_json::json!({"jsonrpc":"2.0","id":1,"method":"max/dumpState"}),
    )
    .await;
    let dump_resp = wait_for_response(rx.clone(), 1, Duration::from_secs(3)).await;
    let state = dump_resp
        .get("result")
        .expect("dumpState must return result")
        .clone();

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
    let resp = test_rpc_method("max/reset", serde_json::Value::Null).await;
    assert_has_result(&resp, "max/reset");
}

// ---------------------------------------------------------------------------
// DfLSS CTQ: max/lawfulTransition — verify "admitted" boolean in result
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "current_thread")]
async fn test_max_lawful_transition_has_admitted_field() {
    let resp = test_rpc_method("max/lawfulTransition", serde_json::json!("Initializing")).await;
    assert_has_result(&resp, "max/lawfulTransition");
    let result = resp.get("result").unwrap();
    assert!(
        result.get("admitted").is_some(),
        "max/lawfulTransition result must have 'admitted' field, got: {}",
        result
    );
    let admitted = result.get("admitted").unwrap();
    assert!(
        admitted.is_boolean(),
        "max/lawfulTransition 'admitted' must be a boolean, got: {}",
        admitted
    );
    assert!(
        result.get("current_phase").is_some(),
        "max/lawfulTransition result must have 'current_phase' field"
    );
    assert!(
        result.get("requested_phase").is_some(),
        "max/lawfulTransition result must have 'requested_phase' field"
    );
}

// ---------------------------------------------------------------------------
// DfLSS CTQ: max/admission — verify ConformanceVector fields
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "current_thread")]
async fn test_max_admission_conformance_vector_fields() {
    let resp = test_rpc_method("max/admission", serde_json::Value::Null).await;
    assert_has_result(&resp, "max/admission");
    let result = resp.get("result").unwrap();
    // The verdict must be one of the three ConformanceVector states
    let verdict = result.get("verdict").and_then(|v| v.as_str()).unwrap_or("");
    assert!(
        ["Admitted", "Refused", "Unknown"].contains(&verdict),
        "max/admission verdict must be Admitted/Refused/Unknown (ConformanceVector states), got: {}",
        verdict
    );
    // diagnostic_count is a required field
    assert!(
        result.get("diagnostic_count").is_some(),
        "max/admission result must have 'diagnostic_count' field"
    );
}

// ---------------------------------------------------------------------------
// DfLSS CTQ: max/refusal — verify response shape with refused/receipt fields
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "current_thread")]
async fn test_max_refusal_response_shape() {
    let resp = test_rpc_method("max/refusal", serde_json::json!("diag-ctq-refusal")).await;
    assert_has_result(&resp, "max/refusal");
    let result = resp.get("result").unwrap();
    assert!(
        result.get("refused").is_some(),
        "max/refusal result must have 'refused' field, got: {}",
        result
    );
    let refused = result.get("refused").unwrap().as_bool().unwrap_or(false);
    assert!(refused, "max/refusal 'refused' must be true");
    assert!(
        result.get("diagnostic_id").is_some(),
        "max/refusal result must have 'diagnostic_id' field"
    );
    assert!(
        result.get("receipt").is_some(),
        "max/refusal result must have 'receipt' field"
    );
    let receipt = result.get("receipt").unwrap();
    assert!(
        receipt.get("receipt_id").is_some(),
        "max/refusal receipt must have 'receipt_id' field"
    );
    assert!(
        receipt.get("hash").is_some(),
        "max/refusal receipt must have 'hash' field"
    );
}

// ---------------------------------------------------------------------------
// max/conformanceDelta
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "current_thread")]
async fn test_max_conformance_delta_returns_result() {
    let resp = test_rpc_method(
        "max/conformanceDelta",
        serde_json::json!({ "since_seq": 0 }),
    )
    .await;
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
}
