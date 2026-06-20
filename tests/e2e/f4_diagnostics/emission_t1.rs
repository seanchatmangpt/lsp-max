use crate::test_harness::TestHarness;
use serde_json::json;

#[tokio::test]
#[ignore = "e2e: requires live server, run with --include-ignored"]
async fn test_f4_t1_publish_diagnostics_forwarding() {
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

    let diag = json!({
        "jsonrpc": "2.0",
        "method": "textDocument/publishDiagnostics",
        "params": {
            "uri": "file:///Users/sac/tower-lsp-composition/main.rs",
            "diagnostics": [
                {
                    "range": {
                        "start": { "line": 0, "character": 0 },
                        "end": { "line": 0, "character": 5 }
                    },
                    "message": "Generic error from upstream mock-0"
                }
            ]
        }
    });

    {
        let mut s = harness.mock_servers[0].state.lock().await;
        s.diagnostics_to_send.push(diag);
    }

    let msg = tokio::time::timeout(
        std::time::Duration::from_millis(500),
        harness.client.read_message(),
    )
    .await
    .expect("Timeout waiting for diagnostics forwarding")
    .expect("Connection closed while waiting for diagnostics forwarding");

    let method = msg.get("method").and_then(|m| m.as_str());
    assert_eq!(method, Some("textDocument/publishDiagnostics"));

    let params = msg.get("params").unwrap();
    let diags = params.get("diagnostics").unwrap().as_array().unwrap();
    assert_eq!(diags.len(), 1);
    assert_eq!(
        diags[0].get("message").and_then(|m| m.as_str()),
        Some("Generic error from upstream mock-0")
    );
    assert_eq!(
        diags[0].get("source").and_then(|s| s.as_str()),
        Some("mock-0")
    );

    harness.shutdown();
}

#[tokio::test]
#[ignore = "e2e: requires live server, run with --include-ignored"]
async fn test_f4_t1_diagnose_drift_code() {
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

    let diag = json!({
        "jsonrpc": "2.0",
        "method": "textDocument/publishDiagnostics",
        "params": {
            "uri": "file:///Users/sac/tower-lsp-composition/main.rs",
            "diagnostics": [
                {
                    "range": {
                        "start": { "line": 1, "character": 0 },
                        "end": { "line": 1, "character": 10 }
                    },
                    "code": "GGEN-DRIFT-001",
                    "message": "Drifted content detected"
                }
            ]
        }
    });

    {
        let mut s = harness.mock_servers[0].state.lock().await;
        s.diagnostics_to_send.push(diag);
    }

    let msg = tokio::time::timeout(
        std::time::Duration::from_millis(500),
        harness.client.read_message(),
    )
    .await
    .expect("Timeout waiting for GGEN-DRIFT-001")
    .expect("Connection closed while waiting for GGEN-DRIFT-001");

    let params = msg.get("params").unwrap();
    let diags = params.get("diagnostics").unwrap().as_array().unwrap();
    assert_eq!(diags.len(), 1);
    assert_eq!(
        diags[0].get("code").and_then(|c| c.as_str()),
        Some("GGEN-DRIFT-001")
    );

    harness.shutdown();
}

#[tokio::test]
#[ignore = "e2e: requires live server, run with --include-ignored"]
async fn test_f4_t1_diagnose_missing_receipt() {
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

    let diag = json!({
        "jsonrpc": "2.0",
        "method": "textDocument/publishDiagnostics",
        "params": {
            "uri": "file:///Users/sac/tower-lsp-composition/main.rs",
            "diagnostics": [
                {
                    "range": {
                        "start": { "line": 2, "character": 0 },
                        "end": { "line": 2, "character": 8 }
                    },
                    "code": "GGEN-EVIDENCE-001",
                    "message": "Missing execution receipt evidence"
                }
            ]
        }
    });

    {
        let mut s = harness.mock_servers[0].state.lock().await;
        s.diagnostics_to_send.push(diag);
    }

    let msg = tokio::time::timeout(
        std::time::Duration::from_millis(500),
        harness.client.read_message(),
    )
    .await
    .expect("Timeout waiting for GGEN-EVIDENCE-001")
    .expect("Connection closed while waiting for GGEN-EVIDENCE-001");

    let params = msg.get("params").unwrap();
    let diags = params.get("diagnostics").unwrap().as_array().unwrap();
    assert_eq!(diags.len(), 1);
    assert_eq!(
        diags[0].get("code").and_then(|c| c.as_str()),
        Some("GGEN-EVIDENCE-001")
    );

    harness.shutdown();
}

#[tokio::test]
#[ignore = "e2e: requires live server, run with --include-ignored"]
async fn test_f4_t1_diagnose_incomplete_customization() {
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

    let diag = json!({
        "jsonrpc": "2.0",
        "method": "textDocument/publishDiagnostics",
        "params": {
            "uri": "file:///Users/sac/tower-lsp-composition/main.rs",
            "diagnostics": [
                {
                    "range": {
                        "start": { "line": 3, "character": 0 },
                        "end": { "line": 3, "character": 12 }
                    },
                    "code": "GGEN-CUSTOMIZE-001",
                    "message": "Incomplete customization points"
                }
            ]
        }
    });

    {
        let mut s = harness.mock_servers[0].state.lock().await;
        s.diagnostics_to_send.push(diag);
    }

    let msg = tokio::time::timeout(
        std::time::Duration::from_millis(500),
        harness.client.read_message(),
    )
    .await
    .expect("Timeout waiting for GGEN-CUSTOMIZE-001")
    .expect("Connection closed while waiting for GGEN-CUSTOMIZE-001");

    let params = msg.get("params").unwrap();
    let diags = params.get("diagnostics").unwrap().as_array().unwrap();
    assert_eq!(diags.len(), 1);
    assert_eq!(
        diags[0].get("code").and_then(|c| c.as_str()),
        Some("GGEN-CUSTOMIZE-001")
    );

    harness.shutdown();
}

#[tokio::test]
#[ignore = "e2e: requires live server, run with --include-ignored"]
async fn test_f4_t1_detect_projection_opportunity() {
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

    let diag = json!({
        "jsonrpc": "2.0",
        "method": "textDocument/publishDiagnostics",
        "params": {
            "uri": "file:///Users/sac/tower-lsp-composition/main.rs",
            "diagnostics": [
                {
                    "range": {
                        "start": { "line": 4, "character": 0 },
                        "end": { "line": 4, "character": 15 }
                    },
                    "code": "GGEN-PROJECT-OPPORTUNITY-001",
                    "message": "Projection opportunity detected"
                }
            ]
        }
    });

    {
        let mut s = harness.mock_servers[0].state.lock().await;
        s.diagnostics_to_send.push(diag);
    }

    let msg = tokio::time::timeout(
        std::time::Duration::from_millis(500),
        harness.client.read_message(),
    )
    .await
    .expect("Timeout waiting for GGEN-PROJECT-OPPORTUNITY-001")
    .expect("Connection closed while waiting for GGEN-PROJECT-OPPORTUNITY-001");

    let params = msg.get("params").unwrap();
    let diags = params.get("diagnostics").unwrap().as_array().unwrap();
    assert_eq!(diags.len(), 1);
    assert_eq!(
        diags[0].get("code").and_then(|c| c.as_str()),
        Some("GGEN-PROJECT-OPPORTUNITY-001")
    );

    harness.shutdown();
}
