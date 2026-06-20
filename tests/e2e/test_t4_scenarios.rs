use crate::test_harness::TestHarness;
use serde_json::json;

#[tokio::test]
#[ignore = "e2e: requires live server, run with --include-ignored"]
async fn test_t4_scenario_multi_language_polyglot_workspace() {
    let mut harness = TestHarness::new(2).await;
    let init_params = json!({
        "capabilities": {},
        "rootUri": null,
        "processId": null
    });
    let _ = harness.client.send_request("initialize", init_params).await;

    // Upstream 0 handles Rust (*.rs), Upstream 1 handles TypeScript (*.ts)
    // Send didOpen for a Rust file
    harness
        .client
        .send_notification(
            "textDocument/didOpen",
            json!({
                "textDocument": {
                    "uri": "file:///Users/sac/tower-lsp-composition/main.rs",
                    "languageId": "rust",
                    "version": 1,
                    "text": "fn main() {}"
                }
            }),
        )
        .await;

    // Verify the server is still healthy and handles requests
    let hover_params = json!({
        "textDocument": { "uri": "file:///Users/sac/tower-lsp-composition/main.rs" },
        "position": { "line": 0, "character": 0 }
    });
    let resp = harness
        .client
        .send_request("textDocument/hover", hover_params)
        .await;
    assert!(resp.get("error").is_none() || resp.get("result").is_some());
    harness.shutdown();
}

#[tokio::test]
#[ignore = "e2e: requires live server, run with --include-ignored"]
async fn test_t4_scenario_slow_remote_lsp_with_local_fallback() {
    let mut harness = TestHarness::new(2).await;

    // Upstream 0 is fast local static fallback
    {
        let mut s0 = harness.mock_servers[0].state.lock().await;
        s0.custom_capabilities = json!({ "definitionProvider": true });
        s0.definition_response = Some(json!([{
            "uri": "file:///Users/sac/tower-lsp-composition/local.rs",
            "range": { "start": { "line": 0, "character": 0 }, "end": { "line": 0, "character": 5 } }
        }]));
    }
    // Upstream 1 is slow remote LSP
    {
        let mut s1 = harness.mock_servers[1].state.lock().await;
        s1.delay = std::time::Duration::from_millis(300);
        s1.custom_capabilities = json!({ "definitionProvider": true });
        s1.diagnostics_to_send = vec![json!({
            "jsonrpc": "2.0",
            "method": "textDocument/publishDiagnostics",
            "params": {
                "uri": "file:///Users/sac/tower-lsp-composition/main.rs",
                "diagnostics": [{
                    "range": { "start": { "line": 0, "character": 0 }, "end": { "line": 0, "character": 5 } },
                    "message": "Remote Diagnostic"
                }]
            }
        })];
    }

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

    // Send definition query
    let def_params = json!({
        "textDocument": { "uri": "file:///Users/sac/tower-lsp-composition/main.rs" },
        "position": { "line": 0, "character": 0 }
    });

    // Should get quick response from fast local fallback
    let start = std::time::Instant::now();
    let resp = harness
        .client
        .send_request("textDocument/definition", def_params)
        .await;
    let elapsed = start.elapsed();

    assert!(
        elapsed < std::time::Duration::from_millis(200),
        "Should resolve from local fallback quickly"
    );
    assert!(resp.get("error").is_none());

    // Wait for the slow remote to send diagnostics
    let msg = tokio::time::timeout(
        std::time::Duration::from_millis(500),
        harness.client.read_message(),
    )
    .await
    .expect("Timeout waiting for publishDiagnostics from slow remote")
    .expect("Connection closed while waiting for publishDiagnostics from slow remote");
    assert_eq!(
        msg.get("method").and_then(|m| m.as_str()),
        Some("textDocument/publishDiagnostics")
    );

    harness.shutdown();
}

#[tokio::test]
#[ignore = "e2e: requires live server, run with --include-ignored"]
async fn test_t4_scenario_collaborative_editing_with_conflict_resolution() {
    let mut harness = TestHarness::new(1).await;
    let init_params = json!({
        "capabilities": {},
        "rootUri": "file:///Users/sac/tower-lsp-composition",
        "processId": null
    });
    let _ = harness.client.send_request("initialize", init_params).await;

    // Upstream proposes edit 1
    {
        let mut s = harness.mock_servers[0].state.lock().await;
        s.rename_response = Some(json!({
            "changes": {
                "file:///Users/sac/tower-lsp-composition/main.rs": [
                    { "range": { "start": { "line": 0, "character": 0 }, "end": { "line": 0, "character": 5 } }, "newText": "alice" }
                ]
            }
        }));
    }
    let rename1 = json!({
        "textDocument": { "uri": "file:///Users/sac/tower-lsp-composition/main.rs" },
        "position": { "line": 0, "character": 0 },
        "newName": "alice"
    });
    let _resp1 = harness
        .client
        .send_request("textDocument/rename", rename1)
        .await;

    // Simulate concurrent/overlapping edit proposal
    {
        let mut s = harness.mock_servers[0].state.lock().await;
        s.rename_response = Some(json!({
            "changes": {
                "file:///Users/sac/tower-lsp-composition/main.rs": [
                    { "range": { "start": { "line": 0, "character": 3 }, "end": { "line": 0, "character": 8 } }, "newText": "bob" }
                ]
            }
        }));
    }
    let rename2 = json!({
        "textDocument": { "uri": "file:///Users/sac/tower-lsp-composition/main.rs" },
        "position": { "line": 0, "character": 3 },
        "newName": "bob"
    });
    let resp2 = harness
        .client
        .send_request("textDocument/rename", rename2)
        .await;
    // Overlapping edit must be refused
    assert!(
        resp2.get("error").is_some()
            || resp2.get("result").is_none()
            || resp2.get("result").unwrap().is_null()
    );

    harness.shutdown();
}

#[tokio::test]
#[ignore = "e2e: requires live server, run with --include-ignored"]
async fn test_t4_scenario_large_monorepo_lsif_navigation() {
    let mut harness = TestHarness::new(1).await;
    let init_params = json!({
        "capabilities": {},
        "rootUri": "file:///Users/sac/tower-lsp-composition",
        "processId": null
    });
    let _ = harness.client.send_request("initialize", init_params).await;

    let def_params = json!({
        "textDocument": { "uri": "file:///Users/sac/tower-lsp-composition/monorepo/pkg/lib.rs" },
        "position": { "line": 100, "character": 45 }
    });

    let start = std::time::Instant::now();
    let resp = harness
        .client
        .send_request("textDocument/definition", def_params)
        .await;
    let elapsed = start.elapsed();

    assert!(
        elapsed < std::time::Duration::from_millis(50),
        "LSIF navigation must be extremely fast"
    );
    assert!(resp.get("error").is_none());
    harness.shutdown();
}

#[tokio::test]
#[ignore = "e2e: requires live server, run with --include-ignored"]
async fn test_t4_scenario_server_crash_recovery_during_refactoring() {
    let mut harness = TestHarness::new(2).await;
    let init_params = json!({
        "capabilities": {},
        "rootUri": null,
        "processId": null
    });
    let _ = harness.client.send_request("initialize", init_params).await;

    // Upstream 0 crashes mid-rename transaction
    {
        let mut s = harness.mock_servers[0].state.lock().await;
        s.should_crash_on_request = Some("textDocument/rename".to_string());
    }

    let rename_params = json!({
        "textDocument": { "uri": "file:///Users/sac/tower-lsp-composition/main.rs" },
        "position": { "line": 0, "character": 0 },
        "newName": "crash_recovery"
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
