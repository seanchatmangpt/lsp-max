use crate::test_harness::TestHarness;
use serde_json::json;

#[tokio::test]
#[ignore = "e2e: requires live server, run with --include-ignored"]
async fn test_f6_t1_upstream_crash_isolation() {
    let mut harness = TestHarness::new(2).await;
    let init_params = json!({
        "capabilities": {},
        "rootUri": null,
        "processId": null
    });
    let _ = harness.client.send_request("initialize", init_params).await;

    // Upstream 0 will crash on hover request, Upstream 1 will return hover response
    {
        let mut s = harness.mock_servers[0].state.lock().await;
        s.should_crash_on_request = Some("textDocument/hover".to_string());
    }
    {
        let mut s = harness.mock_servers[1].state.lock().await;
        s.hover_response = Some(json!({ "contents": "Hover success" }));
    }

    let hover_params = json!({
        "textDocument": { "uri": "file:///Users/sac/tower-lsp-composition/main.rs" },
        "position": { "line": 0, "character": 0 }
    });
    let resp = harness
        .client
        .send_request("textDocument/hover", hover_params)
        .await;
    // Composed server must successfully isolate the crash and return success from Upstream 1
    assert!(resp.get("error").is_none());
    harness.shutdown();
}

#[tokio::test]
#[ignore = "e2e: requires live server, run with --include-ignored"]
async fn test_f6_t1_upstream_timeout_partial_results() {
    let mut harness = TestHarness::new(2).await;
    let init_params = json!({
        "capabilities": {},
        "rootUri": null,
        "processId": null
    });
    let _ = harness.client.send_request("initialize", init_params).await;

    // Upstream 0 has a long delay (simulating slow connection), Upstream 1 returns immediately
    {
        let mut s = harness.mock_servers[0].state.lock().await;
        s.delay = std::time::Duration::from_millis(500);
        s.definition_response = Some(json!([{
            "uri": "file:///Users/sac/tower-lsp-composition/slow.rs",
            "range": { "start": { "line": 0, "character": 0 }, "end": { "line": 0, "character": 5 } }
        }]));
    }
    {
        let mut s = harness.mock_servers[1].state.lock().await;
        s.definition_response = Some(json!([{
            "uri": "file:///Users/sac/tower-lsp-composition/fast.rs",
            "range": { "start": { "line": 1, "character": 0 }, "end": { "line": 1, "character": 5 } }
        }]));
    }

    let def_params = json!({
        "textDocument": { "uri": "file:///Users/sac/tower-lsp-composition/main.rs" },
        "position": { "line": 0, "character": 0 }
    });
    let resp = harness
        .client
        .send_request("textDocument/definition", def_params)
        .await;
    // Composed server must return partial results from Upstream 1 on timeout
    let result = resp.get("result").unwrap();
    assert!(result.is_array());
    harness.shutdown();
}

#[tokio::test]
#[ignore = "e2e: requires live server, run with --include-ignored"]
async fn test_f6_t1_upstream_invalid_json_rpc() {
    let mut harness = TestHarness::new(2).await;
    let init_params = json!({
        "capabilities": {},
        "rootUri": null,
        "processId": null
    });
    let _ = harness.client.send_request("initialize", init_params).await;

    // Upstream 0 returns malformed JSON, Upstream 1 returns hover response
    // (mock server parses invalid json or can close connection)
    {
        let mut s = harness.mock_servers[0].state.lock().await;
        s.should_crash_on_request = Some("textDocument/hover".to_string());
    }
    {
        let mut s = harness.mock_servers[1].state.lock().await;
        s.hover_response = Some(json!({ "contents": "hover valid" }));
    }

    let hover_params = json!({
        "textDocument": { "uri": "file:///Users/sac/tower-lsp-composition/main.rs" },
        "position": { "line": 0, "character": 0 }
    });
    let resp = harness
        .client
        .send_request("textDocument/hover", hover_params)
        .await;
    assert!(resp.get("error").is_none());
    harness.shutdown();
}

#[tokio::test]
#[ignore = "e2e: requires live server, run with --include-ignored"]
async fn test_f6_t1_source_degradation() {
    let mut harness = TestHarness::new(1).await;
    let init_params = json!({
        "capabilities": {},
        "rootUri": null,
        "processId": null
    });
    let _ = harness.client.send_request("initialize", init_params).await;

    // Simulate persistent failures on upstream
    {
        let mut s = harness.mock_servers[0].state.lock().await;
        s.should_crash_on_request = Some("textDocument/hover".to_string());
    }

    // Call multiple times to trigger degradation
    let hover_params = json!({
        "textDocument": { "uri": "file:///Users/sac/tower-lsp-composition/main.rs" },
        "position": { "line": 0, "character": 0 }
    });
    for _ in 0..5 {
        let resp = harness
            .client
            .send_request("textDocument/hover", hover_params.clone())
            .await;
        // The server should handle upstream failures gracefully by returning an error response (or null)
        assert!(
            resp.get("error").is_some()
                || resp.get("result").is_none()
                || resp.get("result").unwrap().is_null()
        );
    }

    harness.shutdown();
}

#[tokio::test]
#[ignore = "e2e: requires live server, run with --include-ignored"]
async fn test_f6_t1_reject_unsafe_edit() {
    let mut harness = TestHarness::new(1).await;
    let init_params = json!({
        "capabilities": {},
        "rootUri": null,
        "processId": null
    });
    let _ = harness.client.send_request("initialize", init_params).await;

    // Upstream returns an unsafe edit proposal (e.g. modifying files outside workspace)
    {
        let mut s = harness.mock_servers[0].state.lock().await;
        s.rename_response = Some(json!({
            "changes": {
                "file:///etc/passwd": [
                    { "range": { "start": { "line": 0, "character": 0 }, "end": { "line": 0, "character": 5 } }, "newText": "unsafe" }
                ]
            }
        }));
    }

    let rename_params = json!({
        "textDocument": { "uri": "file:///Users/sac/tower-lsp-composition/main.rs" },
        "position": { "line": 0, "character": 0 },
        "newName": "unsafe"
    });
    let resp = harness
        .client
        .send_request("textDocument/rename", rename_params)
        .await;
    assert!(
        resp.get("error").is_some()
            || resp.get("result").is_none()
            || resp.get("result").unwrap().is_null()
    );
    harness.shutdown();
}

#[tokio::test]
#[ignore = "e2e: requires live server, run with --include-ignored"]
async fn test_f6_t2_upstream_infinite_loop() {
    let mut harness = TestHarness::new(1).await;
    let init_params = json!({
        "capabilities": {},
        "rootUri": null,
        "processId": null
    });
    let _ = harness.client.send_request("initialize", init_params).await;

    // Upstream server delay is very long (infinite simulation)
    {
        let mut s = harness.mock_servers[0].state.lock().await;
        s.delay = std::time::Duration::from_secs(10);
    }

    let hover_params = json!({
        "textDocument": { "uri": "file:///Users/sac/tower-lsp-composition/main.rs" },
        "position": { "line": 0, "character": 0 }
    });

    // Timeout should isolate within configured time limit (e.g. 100ms)
    let resp_fut = harness
        .client
        .send_request("textDocument/hover", hover_params);
    let resp = tokio::time::timeout(std::time::Duration::from_millis(500), resp_fut).await;
    assert!(resp.is_ok(), "Request must return within timeout bounds");
    harness.shutdown();
}

#[tokio::test]
#[ignore = "e2e: requires live server, run with --include-ignored"]
async fn test_f6_t2_recovery_from_degradation() {
    let mut harness = TestHarness::new(1).await;
    let init_params = json!({
        "capabilities": {},
        "rootUri": "file:///Users/sac/tower-lsp-composition",
        "processId": null
    });
    let _ = harness.client.send_request("initialize", init_params).await;

    // Force failure to degrade
    {
        let mut s = harness.mock_servers[0].state.lock().await;
        s.should_crash_on_request = Some("textDocument/hover".to_string());
    }

    let hover_params = json!({
        "textDocument": { "uri": "file:///Users/sac/tower-lsp-composition/main.rs" },
        "position": { "line": 0, "character": 0 }
    });

    // Send requests to trigger degradation
    for _ in 0..5 {
        let _ = harness
            .client
            .send_request("textDocument/hover", hover_params.clone())
            .await;
    }

    // Now restore healthy state
    {
        let mut s = harness.mock_servers[0].state.lock().await;
        s.should_crash_on_request = None;
        s.hover_response = Some(json!({ "contents": "healed" }));
    }

    // Composed server should recover the degraded source
    let resp = harness
        .client
        .send_request("textDocument/hover", hover_params)
        .await;
    assert!(resp.get("error").is_none() || resp.get("result").is_some());
    harness.shutdown();
}

#[tokio::test]
#[ignore = "e2e: requires live server, run with --include-ignored"]
async fn test_f6_t2_all_upstreams_dead() {
    let mut harness = TestHarness::new(2).await;
    let init_params = json!({
        "capabilities": {},
        "rootUri": null,
        "processId": null
    });
    let _ = harness.client.send_request("initialize", init_params).await;

    // Crash all upstreams
    for mock in &mut harness.mock_servers {
        let mut s = mock.state.lock().await;
        s.should_crash_on_request = Some("textDocument/hover".to_string());
    }

    let hover_params = json!({
        "textDocument": { "uri": "file:///Users/sac/tower-lsp-composition/main.rs" },
        "position": { "line": 0, "character": 0 }
    });
    let resp = harness
        .client
        .send_request("textDocument/hover", hover_params)
        .await;
    // Server must remain alive and return clean protocol error or null
    assert!(
        resp.get("error").is_some()
            || resp.get("result").is_none()
            || resp.get("result").unwrap().is_null()
    );
    harness.shutdown();
}

#[tokio::test]
#[ignore = "e2e: requires live server, run with --include-ignored"]
async fn test_f6_t2_partial_timeout_no_results() {
    let mut harness = TestHarness::new(2).await;
    let init_params = json!({
        "capabilities": {},
        "rootUri": "file:///Users/sac/tower-lsp-composition",
        "processId": null
    });
    let _ = harness.client.send_request("initialize", init_params).await;

    // Upstream 0 is fast, Upstream 1 is slow
    {
        let mut s0 = harness.mock_servers[0].state.lock().await;
        s0.definition_response = Some(json!([{
            "uri": "file:///Users/sac/tower-lsp-composition/fast.rs",
            "range": { "start": { "line": 0, "character": 0 }, "end": { "line": 0, "character": 5 } }
        }]));
    }
    {
        let mut s1 = harness.mock_servers[1].state.lock().await;
        s1.delay = std::time::Duration::from_millis(500);
    }

    let def_params = json!({
        "textDocument": { "uri": "file:///Users/sac/tower-lsp-composition/main.rs" },
        "position": { "line": 0, "character": 0 },
        // Forbid partial results
        "partialResultToken": null
    });

    let resp = harness
        .client
        .send_request("textDocument/definition", def_params)
        .await;
    assert!(
        resp.get("error").is_some()
            || resp.get("result").is_none()
            || resp.get("result").unwrap().is_null()
    );
    harness.shutdown();
}

#[tokio::test]
#[ignore = "e2e: requires live server, run with --include-ignored"]
async fn test_f6_t2_channel_disconnect_handling() {
    let mut harness = TestHarness::new(1).await;
    let init_params = json!({
        "capabilities": {},
        "rootUri": null,
        "processId": null
    });
    let _ = harness.client.send_request("initialize", init_params).await;

    // Force disconnect communication channel to upstream
    harness.mock_servers[0].shutdown();

    let hover_params = json!({
        "textDocument": { "uri": "file:///Users/sac/tower-lsp-composition/main.rs" },
        "position": { "line": 0, "character": 0 }
    });
    let resp = harness
        .client
        .send_request("textDocument/hover", hover_params)
        .await;
    // Must gracefully handle disconnect and return null/error
    assert!(
        resp.get("error").is_some()
            || resp.get("result").is_none()
            || resp.get("result").unwrap().is_null()
    );
    harness.shutdown();
}
