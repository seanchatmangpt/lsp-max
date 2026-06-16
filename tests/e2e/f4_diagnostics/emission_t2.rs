use crate::test_harness::TestHarness;
use serde_json::json;

#[tokio::test]
#[ignore = "e2e: requires live server, run with --include-ignored"]
async fn test_f4_t2_diagnostics_clear_on_save() {
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

    // First publish diagnostics
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
                    "message": "Error from upstream"
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
    .expect("Timeout waiting for diagnostics")
    .expect("Connection closed");
    let params = msg.get("params").unwrap();
    assert_eq!(
        params.get("diagnostics").unwrap().as_array().unwrap().len(),
        1
    );

    // Client sends didSave
    harness
        .client
        .send_notification(
            "textDocument/didSave",
            json!({
                "textDocument": {
                    "uri": "file:///Users/sac/tower-lsp-composition/main.rs"
                }
            }),
        )
        .await;

    // Simulate mock server clearing/updating diagnostics in response to save
    let clear_diag = json!({
        "jsonrpc": "2.0",
        "method": "textDocument/publishDiagnostics",
        "params": {
            "uri": "file:///Users/sac/tower-lsp-composition/main.rs",
            "diagnostics": []
        }
    });

    {
        let mut s = harness.mock_servers[0].state.lock().await;
        s.diagnostics_to_send.push(clear_diag);
    }

    let msg2 = tokio::time::timeout(
        std::time::Duration::from_millis(500),
        harness.client.read_message(),
    )
    .await
    .expect("Timeout waiting for cleared diagnostics")
    .expect("Connection closed");
    let params2 = msg2.get("params").unwrap();
    assert_eq!(
        params2
            .get("diagnostics")
            .unwrap()
            .as_array()
            .unwrap()
            .len(),
        0
    );

    harness.shutdown();
}

#[tokio::test]
#[ignore = "e2e: requires live server, run with --include-ignored"]
async fn test_f4_t2_diagnostics_invalid_syntax() {
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

    // Upstream publishes invalid diagnostic structure (diagnostics is not an array, but a string)
    let invalid_diag = json!({
        "jsonrpc": "2.0",
        "method": "textDocument/publishDiagnostics",
        "params": {
            "uri": "file:///Users/sac/tower-lsp-composition/main.rs",
            "diagnostics": "this should be an array, but is a string"
        }
    });

    {
        let mut s = harness.mock_servers[0].state.lock().await;
        s.diagnostics_to_send.push(invalid_diag);
    }

    // Composed server receives and handles this by forwarding an empty diagnostics notification.
    // Let's read and assert it contains 0 diagnostics.
    let msg1 = tokio::time::timeout(
        std::time::Duration::from_millis(500),
        harness.client.read_message(),
    )
    .await
    .expect("Timeout waiting for invalid diagnostic response")
    .expect("Connection closed");
    let params1 = msg1.get("params").unwrap();
    let diags1 = params1.get("diagnostics").unwrap().as_array().unwrap();
    assert_eq!(diags1.len(), 0);

    // Now publish a valid diagnostic to verify server is still responsive and healthy
    let valid_diag = json!({
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
                    "message": "Valid diagnostic"
                }
            ]
        }
    });

    {
        let mut s = harness.mock_servers[0].state.lock().await;
        s.diagnostics_to_send.push(valid_diag);
    }

    let msg = tokio::time::timeout(
        std::time::Duration::from_millis(500),
        harness.client.read_message(),
    )
    .await
    .expect("Timeout waiting for valid diagnostic after invalid diagnostic")
    .expect("Connection closed");

    let params = msg.get("params").unwrap();
    let diags = params.get("diagnostics").unwrap().as_array().unwrap();
    assert_eq!(diags.len(), 1);
    assert_eq!(
        diags[0].get("message").and_then(|m| m.as_str()),
        Some("Valid diagnostic")
    );

    harness.shutdown();
}

#[tokio::test]
#[ignore = "e2e: requires live server, run with --include-ignored"]
async fn test_f4_t2_diagnostics_empty_array() {
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

    // Publish non-empty diagnostic first
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
                    "message": "Initial error"
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
    .expect("Timeout waiting for initial diagnostic")
    .expect("Connection closed");
    let params = msg.get("params").unwrap();
    assert_eq!(
        params.get("diagnostics").unwrap().as_array().unwrap().len(),
        1
    );

    // Publish empty diagnostics array
    let empty_diag = json!({
        "jsonrpc": "2.0",
        "method": "textDocument/publishDiagnostics",
        "params": {
            "uri": "file:///Users/sac/tower-lsp-composition/main.rs",
            "diagnostics": []
        }
    });

    {
        let mut s = harness.mock_servers[0].state.lock().await;
        s.diagnostics_to_send.push(empty_diag);
    }

    let msg2 = tokio::time::timeout(
        std::time::Duration::from_millis(500),
        harness.client.read_message(),
    )
    .await
    .expect("Timeout waiting for empty diagnostics")
    .expect("Connection closed");
    let params2 = msg2.get("params").unwrap();
    let diags2 = params2.get("diagnostics").unwrap().as_array().unwrap();
    assert!(diags2.is_empty(), "Diagnostics should be empty");

    harness.shutdown();
}

#[tokio::test]
#[ignore = "e2e: requires live server, run with --include-ignored"]
async fn test_f4_t2_diagnostics_non_standard_severity() {
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

    // Upstream publishes a diagnostic with non-standard severity (e.g. 5)
    let non_standard_diag = json!({
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
                    "severity": 5,
                    "message": "Invalid severity diagnostic"
                }
            ]
        }
    });

    {
        let mut s = harness.mock_servers[0].state.lock().await;
        s.diagnostics_to_send.push(non_standard_diag);
    }

    // Read the non-standard severity diagnostic (which is forwarded/mapped successfully)
    let msg1 = tokio::time::timeout(
        std::time::Duration::from_millis(500),
        harness.client.read_message(),
    )
    .await
    .expect("Timeout waiting for non-standard diagnostic")
    .expect("Connection closed");

    let params1 = msg1.get("params").unwrap();
    let diags1 = params1.get("diagnostics").unwrap().as_array().unwrap();
    assert_eq!(diags1.len(), 1);
    assert_eq!(
        diags1[0].get("message").and_then(|m| m.as_str()),
        Some("Invalid severity diagnostic")
    );
    assert_eq!(diags1[0].get("severity").and_then(|s| s.as_i64()), Some(5));

    // Send a standard severity diagnostic (severity 1 = Error)
    let standard_diag = json!({
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
                    "severity": 1,
                    "message": "Standard severity diagnostic"
                }
            ]
        }
    });

    {
        let mut s = harness.mock_servers[0].state.lock().await;
        s.diagnostics_to_send.push(standard_diag);
    }

    let msg2 = tokio::time::timeout(
        std::time::Duration::from_millis(500),
        harness.client.read_message(),
    )
    .await
    .expect("Timeout waiting for standard diagnostic")
    .expect("Connection closed");

    let params2 = msg2.get("params").unwrap();
    let diags2 = params2.get("diagnostics").unwrap().as_array().unwrap();
    assert_eq!(diags2.len(), 1);
    assert_eq!(
        diags2[0].get("message").and_then(|m| m.as_str()),
        Some("Standard severity diagnostic")
    );
    assert_eq!(diags2[0].get("severity").and_then(|s| s.as_i64()), Some(1));

    harness.shutdown();
}

#[tokio::test]
#[ignore = "e2e: requires live server, run with --include-ignored"]
async fn test_f4_t2_diagnostics_maximum_limit() {
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

    // Publish large payload with 500 diagnostics
    let mut diagnostics = Vec::new();
    for i in 0..500 {
        diagnostics.push(json!({
            "range": {
                "start": { "line": i, "character": 0 },
                "end": { "line": i, "character": 5 }
            },
            "message": format!("Error number {}", i)
        }));
    }

    let large_diag = json!({
        "jsonrpc": "2.0",
        "method": "textDocument/publishDiagnostics",
        "params": {
            "uri": "file:///Users/sac/tower-lsp-composition/main.rs",
            "diagnostics": diagnostics
        }
    });

    {
        let mut s = harness.mock_servers[0].state.lock().await;
        s.diagnostics_to_send.push(large_diag);
    }

    let msg = tokio::time::timeout(
        std::time::Duration::from_millis(1000),
        harness.client.read_message(),
    )
    .await
    .expect("Timeout waiting for large diagnostics payload")
    .expect("Connection closed");

    let params = msg.get("params").unwrap();
    let diags = params.get("diagnostics").unwrap().as_array().unwrap();
    assert_eq!(diags.len(), 500);

    harness.shutdown();
}
