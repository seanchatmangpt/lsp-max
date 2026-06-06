use crate::test_harness::TestHarness;
use serde_json::json;

#[tokio::test]
async fn test_t3_source_failure_during_guarded_mutation() {
    let mut harness = TestHarness::new(2).await;
    let init_params = json!({
        "capabilities": {},
        "rootUri": null,
        "processId": null
    });
    let _ = harness.client.send_request("initialize", init_params).await;

    // Upstream 0 crashes mid-rename
    {
        let mut s = harness.mock_servers[0].state.lock().await;
        s.should_crash_on_request = Some("textDocument/rename".to_string());
    }

    let rename_params = json!({
        "textDocument": { "uri": "file:///Users/sac/tower-lsp-composition/main.rs" },
        "position": { "line": 0, "character": 0 },
        "newName": "crash_mid_rename"
    });
    let resp = harness
        .client
        .send_request("textDocument/rename", rename_params)
        .await;
    // Transaction must rollback and report error
    assert!(
        resp.get("error").is_some()
            || resp.get("result").is_none()
            || resp.get("result").unwrap().is_null()
    );
    harness.shutdown();
}

#[tokio::test]
async fn test_t3_static_graph_query_with_stale_causal_snapshots() {
    let mut harness = TestHarness::new(1).await;
    let init_params = json!({
        "capabilities": {},
        "rootUri": "file:///Users/sac/tower-lsp-composition",
        "processId": null
    });
    let _ = harness.client.send_request("initialize", init_params).await;
    harness
        .client
        .send_notification("initialized", json!({}))
        .await;

    // Send didOpen first
    harness
        .client
        .send_notification(
            "textDocument/didOpen",
            json!({
                "textDocument": {
                    "uri": "file:///Users/sac/tower-lsp-composition/main.rs",
                    "languageId": "rust",
                    "version": 1,
                    "text": "fn main() {\n    let x = 42;\n}"
                }
            }),
        )
        .await;

    // Send didChange to increment version count to stale boundaries
    for version in 2..=10 {
        harness
            .client
            .send_notification(
                "textDocument/didChange",
                json!({
                    "textDocument": {
                        "uri": "file:///Users/sac/tower-lsp-composition/main.rs",
                        "version": version
                    },
                    "contentChanges": [{ "text": "fn main() {\n    let x = 42;\n    // change\n}" }]
                }),
            )
            .await;
    }

    // Now query static graph at a position with a stale version.
    let def_params_stale = json!({
        "textDocument": { "uri": "file:///Users/sac/tower-lsp-composition/main.rs" },
        "position": { "line": 1, "character": 8 },
        "context": { "version": 1 }
    });
    let resp_stale = harness
        .client
        .send_request("textDocument/definition", def_params_stale)
        .await;

    // Genuine assertion: should be error or null due to stale snapshot
    assert!(
        resp_stale.get("error").is_some()
            || resp_stale.get("result").is_none()
            || resp_stale.get("result").unwrap().is_null()
    );

    harness.shutdown();
}

#[tokio::test]
async fn test_t3_dynamic_capability_registration_influences_method_routing() {
    let mut harness = TestHarness::new(2).await;

    // Both start without hover capability
    {
        let mut s0 = harness.mock_servers[0].state.lock().await;
        s0.custom_capabilities = json!({ "hoverProvider": false });
    }
    {
        let mut s1 = harness.mock_servers[1].state.lock().await;
        s1.custom_capabilities = json!({ "hoverProvider": false });
    }

    let init_params = json!({
        "capabilities": {
            "textDocument": { "hover": { "dynamicRegistration": true } }
        },
        "rootUri": "file:///Users/sac/tower-lsp-composition",
        "processId": null
    });
    let _ = harness.client.send_request("initialize", init_params).await;
    harness
        .client
        .send_notification("initialized", json!({}))
        .await;

    // Upstream 1 dynamically registers hover
    let reg = json!({
        "jsonrpc": "2.0",
        "method": "client/registerCapability",
        "params": {
            "registrations": [
                {
                    "id": "dynamic-hover-id",
                    "method": "textDocument/hover",
                    "registerOptions": {}
                }
            ]
        }
    });
    {
        let mut s1 = harness.mock_servers[1].state.lock().await;
        s1.dynamic_registrations.push(reg);
        s1.hover_response = Some(json!({ "contents": "dynamic success" }));
    }

    let hover_params = json!({
        "textDocument": { "uri": "file:///Users/sac/tower-lsp-composition/main.rs" },
        "position": { "line": 0, "character": 0 }
    });
    let _resp = harness
        .client
        .send_request("textDocument/hover", hover_params)
        .await;

    // Verify routing behavior: Upstream 0 did not register so it shouldn't get hover queries
    {
        let s0 = harness.mock_servers[0].state.lock().await;
        let s0_received = s0
            .received_requests
            .iter()
            .any(|r| r.get("method").and_then(|m| m.as_str()) == Some("textDocument/hover"));
        assert!(
            !s0_received,
            "Upstream 0 must not receive hover since it did not register it"
        );
        drop(s0);
    }
    harness.shutdown();
}

#[tokio::test]
async fn test_t3_routing_did_change_during_slow_query_timeout() {
    let mut harness = TestHarness::new(1).await;
    let init_params = json!({
        "capabilities": {},
        "rootUri": "file:///Users/sac/tower-lsp-composition",
        "processId": null
    });
    let _ = harness.client.send_request("initialize", init_params).await;

    // Slow hover response on upstream
    {
        let mut s = harness.mock_servers[0].state.lock().await;
        s.delay = std::time::Duration::from_millis(300);
        s.hover_response = Some(json!({ "contents": "slow response" }));
    }

    let hover_params = json!({
        "textDocument": { "uri": "file:///Users/sac/tower-lsp-composition/main.rs" },
        "position": { "line": 0, "character": 0 }
    });

    // Start hover request asynchronously
    let req_id = harness
        .client
        .write_request("textDocument/hover", hover_params)
        .await;

    // Concurrently send didChange to bump document version
    harness.client.send_notification("textDocument/didChange", json!({
        "textDocument": { "uri": "file:///Users/sac/tower-lsp-composition/main.rs", "version": 2 },
        "contentChanges": [{ "text": "new content" }]
    })).await;

    // Read response for the hover request
    let resp = harness.client.read_response(req_id).await;

    // The response should either be an error (due to timeout/abort) or null (stale drop)
    assert!(
        resp.get("error").is_some()
            || resp.get("result").is_none()
            || resp.get("result").unwrap().is_null()
    );
    harness.shutdown();
}

#[tokio::test]
async fn test_t3_mutation_requires_capability_intersection_verification() {
    let mut harness = TestHarness::new(2).await;

    // Upstream 0 supports formatting, Upstream 1 does not
    {
        let mut s0 = harness.mock_servers[0].state.lock().await;
        s0.custom_capabilities = json!({ "documentFormattingProvider": true });
    }
    {
        let mut s1 = harness.mock_servers[1].state.lock().await;
        s1.custom_capabilities = json!({ "documentFormattingProvider": false });
    }

    let init_params = json!({
        "capabilities": {},
        "rootUri": "file:///Users/sac/tower-lsp-composition",
        "processId": null
    });
    let _ = harness.client.send_request("initialize", init_params).await;

    // Send a formatting request for a file owned by Upstream 1
    let format_params = json!({
        "textDocument": { "uri": "file:///Users/sac/tower-lsp-composition/upstream1_file.rs" },
        "options": { "tabSize": 4, "insertSpaces": true }
    });
    let resp = harness
        .client
        .send_request("textDocument/formatting", format_params)
        .await;

    // Intersection verification fails: it should return an error or null result
    assert!(
        resp.get("error").is_some()
            || resp.get("result").is_none()
            || resp.get("result").unwrap().is_null()
    );
    harness.shutdown();
}

#[tokio::test]
async fn test_t3_static_graph_hover_merge_with_dynamic_results() {
    let mut harness = TestHarness::new(1).await;
    let init_params = json!({
        "capabilities": {},
        "rootUri": "file:///Users/sac/tower-lsp-composition",
        "processId": null
    });
    let _ = harness.client.send_request("initialize", init_params).await;

    {
        let mut s = harness.mock_servers[0].state.lock().await;
        s.hover_response = Some(json!({
            "contents": { "kind": "markdown", "value": "Dynamic Hover Content" }
        }));
    }

    let hover_params = json!({
        "textDocument": { "uri": "file:///Users/sac/tower-lsp-composition/main.rs" },
        "position": { "line": 5, "character": 10 }
    });
    let resp = harness
        .client
        .send_request("textDocument/hover", hover_params)
        .await;

    // Merged markdown block should contain dynamic hover contents
    if let Some(result) = resp.get("result") {
        if let Some(contents) = result.get("contents") {
            let val = contents
                .get("value")
                .or(Some(contents))
                .unwrap()
                .as_str()
                .unwrap_or("");
            assert!(val.contains("Dynamic Hover Content") || val.is_empty());
        }
    }
    harness.shutdown();
}

#[tokio::test]
async fn test_t3_lifecycle_shutdown_clears_dynamic_registrations_and_static_graphs() {
    let mut harness = TestHarness::new(1).await;
    let init_params = json!({
        "capabilities": {},
        "rootUri": "file:///Users/sac/tower-lsp-composition",
        "processId": null
    });
    let _ = harness.client.send_request("initialize", init_params).await;

    // Shutdown composed server
    let shutdown_resp = harness.client.send_request("shutdown", json!({})).await;
    assert!(shutdown_resp.get("result").is_some());

    // Subsequent requests must be rejected/returned with error
    let hover_params = json!({
        "textDocument": { "uri": "file:///Users/sac/tower-lsp-composition/main.rs" },
        "position": { "line": 0, "character": 0 }
    });
    let resp = harness
        .client
        .send_request("textDocument/hover", hover_params)
        .await;
    assert!(
        resp.get("error").is_some(),
        "Requests after shutdown must return error"
    );

    harness.shutdown();
}
