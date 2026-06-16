use crate::test_harness::TestHarness;
use serde_json::json;

#[tokio::test]
#[ignore = "e2e: requires live server, run with --include-ignored"]
async fn test_f1_t1_initialize_handshake() {
    let mut harness = TestHarness::new(1).await;
    let params = json!({
        "capabilities": {
            "textDocument": {
                "hover": {
                    "contentFormat": ["markdown", "plaintext"]
                }
            }
        },
        "rootUri": "file:///Users/sac/tower-lsp-composition",
        "processId": 1234
    });
    let resp = harness.client.send_request("initialize", params).await;
    assert!(
        resp.get("result").is_some(),
        "Initialization should return a result"
    );
    let result = resp.get("result").unwrap();
    assert!(
        result.get("capabilities").is_some(),
        "Result should contain capabilities"
    );
    harness.shutdown();
}

#[tokio::test]
#[ignore = "e2e: requires live server, run with --include-ignored"]
async fn test_f1_t1_initialized_notification() {
    let mut harness = TestHarness::new(1).await;
    let init_params = json!({
        "capabilities": {},
        "rootUri": null,
        "processId": null
    });
    let _ = harness.client.send_request("initialize", init_params).await;
    harness
        .client
        .send_notification("initialized", json!({}))
        .await;

    // Verify server accepts requests after initialization and does not return ServerNotInitialized (-32002)
    let hover_params = json!({
        "textDocument": { "uri": "file:///Users/sac/tower-lsp-composition/main.rs" },
        "position": { "line": 0, "character": 0 }
    });
    let resp = harness
        .client
        .send_request("textDocument/hover", hover_params)
        .await;
    if let Some(err) = resp.get("error") {
        let code = err.get("code").and_then(|c| c.as_i64()).unwrap_or(0);
        assert_ne!(
            code, -32002,
            "Server must not return ServerNotInitialized after initialized notification"
        );
    }
    harness.shutdown();
}

#[tokio::test]
#[ignore = "e2e: requires live server, run with --include-ignored"]
async fn test_f1_t1_shutdown_request() {
    let mut harness = TestHarness::new(1).await;
    let init_params = json!({
        "capabilities": {},
        "rootUri": null,
        "processId": null
    });
    let _ = harness.client.send_request("initialize", init_params).await;
    let resp = harness.client.send_request("shutdown", json!({})).await;
    assert!(
        resp.get("result").is_some(),
        "Shutdown should return a result"
    );
    assert!(
        resp.get("result").unwrap().is_null(),
        "Shutdown result should be null"
    );
    harness.shutdown();
}

#[tokio::test]
#[ignore = "e2e: requires live server, run with --include-ignored"]
async fn test_f1_t1_exit_notification() {
    let mut harness = TestHarness::new(1).await;
    let init_params = json!({
        "capabilities": {},
        "rootUri": null,
        "processId": null
    });
    let init_resp = harness.client.send_request("initialize", init_params).await;
    assert!(init_resp.get("result").is_some());

    let shutdown_resp = harness.client.send_request("shutdown", json!({})).await;
    assert!(
        shutdown_resp.get("result").is_some(),
        "Shutdown should return a result"
    );
    assert!(
        shutdown_resp.get("result").unwrap().is_null(),
        "Shutdown result should be null"
    );

    harness.client.send_notification("exit", json!({})).await;
    harness.shutdown();
}

#[tokio::test]
#[ignore = "e2e: requires live server, run with --include-ignored"]
async fn test_f1_t1_reject_requests_before_initialize() {
    let mut harness = TestHarness::new(1).await;
    // Sending hover before initialize
    let hover_params = json!({
        "textDocument": {
            "uri": "file:///Users/sac/tower-lsp-composition/main.rs"
        },
        "position": {
            "line": 0,
            "character": 0
        }
    });
    let resp = harness
        .client
        .send_request("textDocument/hover", hover_params)
        .await;
    assert!(
        resp.get("error").is_some(),
        "Requests before initialize must be rejected"
    );
    let err = resp.get("error").unwrap();
    let code = err.get("code").and_then(|c| c.as_i64()).unwrap_or(0);
    // Standard LSP error code: ServerNotInitialized = -32002
    assert!(
        code == -32002 || code == -32600,
        "Error code should be ServerNotInitialized or InvalidRequest"
    );
    harness.shutdown();
}

#[tokio::test]
#[ignore = "e2e: requires live server, run with --include-ignored"]
async fn test_f1_t2_duplicate_initialize() {
    let mut harness = TestHarness::new(1).await;
    let init_params = json!({
        "capabilities": {},
        "rootUri": null,
        "processId": null
    });
    let resp1 = harness
        .client
        .send_request("initialize", init_params.clone())
        .await;
    assert!(resp1.get("result").is_some());

    let resp2 = harness.client.send_request("initialize", init_params).await;
    assert!(
        resp2.get("error").is_some(),
        "Duplicate initialize must return error"
    );
    let err = resp2.get("error").unwrap();
    let code = err.get("code").and_then(|c| c.as_i64()).unwrap_or(0);
    assert_eq!(code, -32600, "Should return InvalidRequest (-32600)");
    harness.shutdown();
}

#[tokio::test]
#[ignore = "e2e: requires live server, run with --include-ignored"]
async fn test_f1_t2_exit_without_shutdown() {
    let mut harness = TestHarness::new(1).await;
    let init_params = json!({
        "capabilities": {},
        "rootUri": null,
        "processId": null
    });
    let init_resp = harness.client.send_request("initialize", init_params).await;
    assert!(init_resp.get("result").is_some());

    // Sending exit without shutdown should terminate the server process
    harness.client.send_notification("exit", json!({})).await;
    harness.shutdown();
}

#[tokio::test]
#[ignore = "e2e: requires live server, run with --include-ignored"]
async fn test_f1_t2_malformed_initialize_payload() {
    let mut harness = TestHarness::new(1).await;
    // Sending non-standard JSON payload
    let malformed = json!({
        "capabilities": "this should be an object, not a string",
        "rootUri": 12345
    });
    let resp = harness.client.send_request("initialize", malformed).await;
    assert!(
        resp.get("error").is_some(),
        "Malformed payload must return ParseError or InvalidRequest"
    );
    harness.shutdown();
}

#[tokio::test]
#[ignore = "e2e: requires live server, run with --include-ignored"]
async fn test_f1_t2_shutdown_followed_by_request() {
    let mut harness = TestHarness::new(1).await;
    let init_params = json!({
        "capabilities": {},
        "rootUri": null,
        "processId": null
    });
    let _ = harness.client.send_request("initialize", init_params).await;
    let _ = harness.client.send_request("shutdown", json!({})).await;

    let hover_params = json!({
        "textDocument": {
            "uri": "file:///Users/sac/tower-lsp-composition/main.rs"
        },
        "position": {
            "line": 0,
            "character": 0
        }
    });
    let resp = harness
        .client
        .send_request("textDocument/hover", hover_params)
        .await;
    assert!(
        resp.get("error").is_some(),
        "Requests after shutdown must return error"
    );
    let err = resp.get("error").unwrap();
    let code = err.get("code").and_then(|c| c.as_i64()).unwrap_or(0);
    assert_eq!(code, -32600, "Should return InvalidRequest (-32600)");
    harness.shutdown();
}

#[tokio::test]
#[ignore = "e2e: requires live server, run with --include-ignored"]
async fn test_f1_t2_initialize_with_unregistered_client_capabilities() {
    let mut harness = TestHarness::new(1).await;
    let init_params = json!({
        "capabilities": {
            "textDocument": {
                "dynamicRegistration": true,
                "unknownFutureCapability": {
                    "foo": "bar"
                }
            }
        },
        "rootUri": null,
        "processId": null
    });
    let resp = harness.client.send_request("initialize", init_params).await;
    assert!(
        resp.get("result").is_some(),
        "Server should handle future capabilities gracefully"
    );
    harness.shutdown();
}
