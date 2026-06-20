use crate::test_harness::TestHarness;
use serde_json::json;

#[tokio::test]
#[ignore = "e2e: requires live server, run with --include-ignored"]
async fn test_f3_t1_did_change_fanout() {
    let mut harness = TestHarness::new(3).await;
    let init_params = json!({
        "capabilities": {},
        "rootUri": null,
        "processId": null
    });
    let _ = harness.client.send_request("initialize", init_params).await;

    let did_change_params = json!({
        "textDocument": {
            "uri": "file:///Users/sac/tower-lsp-composition/main.rs",
            "version": 1
        },
        "contentChanges": [
            {
                "text": "fn main() {}"
            }
        ]
    });
    harness
        .client
        .send_notification("textDocument/didChange", did_change_params)
        .await;

    // Give time for fanout
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;

    for (idx, mock) in harness.mock_servers.iter().enumerate() {
        let s = mock.state.lock().await;
        let has_change = s
            .received_requests
            .iter()
            .any(|r| r.get("method").and_then(|m| m.as_str()) == Some("textDocument/didChange"));
        assert!(
            has_change,
            "Mock server {} should have received the didChange notification",
            idx
        );
    }
    harness.shutdown();
}

#[tokio::test]
#[ignore = "e2e: requires live server, run with --include-ignored"]
async fn test_f3_t1_definition_merge() {
    let mut harness = TestHarness::new(2).await;
    let init_params = json!({
        "capabilities": {},
        "rootUri": null,
        "processId": null
    });
    let _ = harness.client.send_request("initialize", init_params).await;

    let loc1 = json!({
        "uri": "file:///Users/sac/tower-lsp-composition/main.rs",
        "range": { "start": { "line": 1, "character": 0 }, "end": { "line": 1, "character": 5 } }
    });
    let loc2 = json!({
        "uri": "file:///Users/sac/tower-lsp-composition/helper.rs",
        "range": { "start": { "line": 2, "character": 0 }, "end": { "line": 2, "character": 5 } }
    });

    {
        let mut s = harness.mock_servers[0].state.lock().await;
        s.definition_response = Some(json!([loc1]));
    }
    {
        let mut s = harness.mock_servers[1].state.lock().await;
        s.definition_response = Some(json!([loc2]));
    }

    let def_params = json!({
        "textDocument": { "uri": "file:///Users/sac/tower-lsp-composition/main.rs" },
        "position": { "line": 1, "character": 2 }
    });
    let resp = harness
        .client
        .send_request("textDocument/definition", def_params)
        .await;
    let result = resp
        .get("result")
        .expect("Definition should return a result");
    assert!(result.is_array());
    let arr = result.as_array().unwrap();
    // Expected behavior is to merge definitions
    assert_eq!(arr.len(), 2);
    harness.shutdown();
}

#[tokio::test]
#[ignore = "e2e: requires live server, run with --include-ignored"]
async fn test_f3_t1_completion_ranking() {
    let mut harness = TestHarness::new(2).await;
    let init_params = json!({
        "capabilities": {},
        "rootUri": null,
        "processId": null
    });
    let _ = harness.client.send_request("initialize", init_params).await;

    {
        let mut s = harness.mock_servers[0].state.lock().await;
        s.completion_response = Some(json!({
            "isIncomplete": false,
            "items": [
                { "label": "apple", "detail": "high priority", "sortText": "01" }
            ]
        }));
    }
    {
        let mut s = harness.mock_servers[1].state.lock().await;
        s.completion_response = Some(json!({
            "isIncomplete": false,
            "items": [
                { "label": "apricot", "detail": "low priority", "sortText": "02" }
            ]
        }));
    }

    let comp_params = json!({
        "textDocument": { "uri": "file:///Users/sac/tower-lsp-composition/main.rs" },
        "position": { "line": 1, "character": 2 }
    });
    let resp = harness
        .client
        .send_request("textDocument/completion", comp_params)
        .await;
    let result = resp
        .get("result")
        .expect("Completion should return a result");
    let items = result
        .get("items")
        .or(result.as_array().map(|_| result))
        .unwrap()
        .as_array()
        .unwrap();
    assert_eq!(items.len(), 2);
    assert_eq!(
        items[0].get("label").and_then(|l| l.as_str()),
        Some("apple")
    );
    harness.shutdown();
}

#[tokio::test]
#[ignore = "e2e: requires live server, run with --include-ignored"]
async fn test_f3_t1_hover_first_success() {
    let mut harness = TestHarness::new(2).await;
    let init_params = json!({
        "capabilities": {},
        "rootUri": null,
        "processId": null
    });
    let _ = harness.client.send_request("initialize", init_params).await;

    {
        let mut s = harness.mock_servers[0].state.lock().await;
        s.hover_response = Some(json!({
            "contents": "Hover from first server"
        }));
    }
    {
        let mut s = harness.mock_servers[1].state.lock().await;
        s.hover_response = Some(json!({
            "contents": "Hover from second server"
        }));
    }

    let hover_params = json!({
        "textDocument": { "uri": "file:///Users/sac/tower-lsp-composition/main.rs" },
        "position": { "line": 1, "character": 2 }
    });
    let resp = harness
        .client
        .send_request("textDocument/hover", hover_params)
        .await;
    let result = resp.get("result").expect("Hover should return a result");
    assert!(result.get("contents").is_some());
    harness.shutdown();
}

#[tokio::test]
#[ignore = "e2e: requires live server, run with --include-ignored"]
async fn test_f3_t1_rename_single_owner() {
    let mut harness = TestHarness::new(2).await;
    let init_params = json!({
        "capabilities": {},
        "rootUri": null,
        "processId": null
    });
    let _ = harness.client.send_request("initialize", init_params).await;

    // Only second server claims/accepts rename for this specific URI
    {
        let mut s = harness.mock_servers[0].state.lock().await;
        s.rename_response = None;
    }
    {
        let mut s = harness.mock_servers[1].state.lock().await;
        s.rename_response = Some(json!({
            "changes": {
                "file:///Users/sac/tower-lsp-composition/main.rs": [
                    { "range": { "start": { "line": 0, "character": 0 }, "end": { "line": 0, "character": 5 } }, "newText": "foo" }
                ]
            }
        }));
    }

    let rename_params = json!({
        "textDocument": { "uri": "file:///Users/sac/tower-lsp-composition/main.rs" },
        "position": { "line": 0, "character": 0 },
        "newName": "foo"
    });
    let resp = harness
        .client
        .send_request("textDocument/rename", rename_params)
        .await;
    let result = resp.get("result").expect("Rename should return a result");
    assert!(result.get("changes").is_some());
    harness.shutdown();
}

#[tokio::test]
#[ignore = "e2e: requires live server, run with --include-ignored"]
async fn test_f3_t2_no_owners_available() {
    let mut harness = TestHarness::new(2).await;
    let init_params = json!({
        "capabilities": {},
        "rootUri": null,
        "processId": null
    });
    let _ = harness.client.send_request("initialize", init_params).await;

    let format_params = json!({
        "textDocument": { "uri": "file:///Users/sac/tower-lsp-composition/main.rs" },
        "options": { "tabSize": 4, "insertSpaces": true }
    });
    let resp = harness
        .client
        .send_request("textDocument/formatting", format_params)
        .await;
    // When no owner available, return empty result or error, not panic
    let result = resp.get("result");
    assert!(
        result.is_none()
            || result.unwrap().is_null()
            || result
                .unwrap()
                .as_array()
                .map(|a| a.is_empty())
                .unwrap_or(false)
    );
    harness.shutdown();
}

#[tokio::test]
#[ignore = "e2e: requires live server, run with --include-ignored"]
async fn test_f3_t2_partial_failures_in_merge() {
    let mut harness = TestHarness::new(2).await;
    let init_params = json!({
        "capabilities": {},
        "rootUri": null,
        "processId": null
    });
    let _ = harness.client.send_request("initialize", init_params).await;

    // Upstream 0 crashes/fails on definition request, Upstream 1 returns a definition
    {
        let mut s = harness.mock_servers[0].state.lock().await;
        s.should_crash_on_request = Some("textDocument/definition".to_string());
    }
    {
        let mut s = harness.mock_servers[1].state.lock().await;
        s.definition_response = Some(json!([{
            "uri": "file:///Users/sac/tower-lsp-composition/helper.rs",
            "range": { "start": { "line": 2, "character": 0 }, "end": { "line": 2, "character": 5 } }
        }]));
    }

    let def_params = json!({
        "textDocument": { "uri": "file:///Users/sac/tower-lsp-composition/main.rs" },
        "position": { "line": 1, "character": 2 }
    });
    let resp = harness
        .client
        .send_request("textDocument/definition", def_params)
        .await;
    let result = resp
        .get("result")
        .expect("Definition should return a result despite partial upstream failure");
    assert!(result.is_array());
    assert_eq!(result.as_array().unwrap().len(), 1);
    harness.shutdown();
}

#[tokio::test]
#[ignore = "e2e: requires live server, run with --include-ignored"]
async fn test_f3_t2_conflicting_completion_items() {
    let mut harness = TestHarness::new(2).await;
    let init_params = json!({
        "capabilities": {},
        "rootUri": null,
        "processId": null
    });
    let _ = harness.client.send_request("initialize", init_params).await;

    {
        let mut s = harness.mock_servers[0].state.lock().await;
        s.completion_response = Some(json!({
            "isIncomplete": false,
            "items": [
                { "label": "common", "detail": "first-source-detail" }
            ]
        }));
    }
    {
        let mut s = harness.mock_servers[1].state.lock().await;
        s.completion_response = Some(json!({
            "isIncomplete": false,
            "items": [
                { "label": "common", "detail": "second-source-detail" }
            ]
        }));
    }

    let comp_params = json!({
        "textDocument": { "uri": "file:///Users/sac/tower-lsp-composition/main.rs" },
        "position": { "line": 1, "character": 2 }
    });
    let resp = harness
        .client
        .send_request("textDocument/completion", comp_params)
        .await;
    let result = resp.get("result").unwrap();
    let items = result
        .get("items")
        .or(result.as_array().map(|_| result))
        .unwrap()
        .as_array()
        .unwrap();
    assert_eq!(
        items.len(),
        1,
        "Should merge and deduplicate conflicting completion items"
    );
    harness.shutdown();
}

#[tokio::test]
#[ignore = "e2e: requires live server, run with --include-ignored"]
async fn test_f3_t2_hover_merge_all() {
    let mut harness = TestHarness::new(2).await;
    let init_params = json!({
        "capabilities": {},
        "rootUri": null,
        "processId": null
    });
    let _ = harness.client.send_request("initialize", init_params).await;

    {
        let mut s = harness.mock_servers[0].state.lock().await;
        s.hover_response = Some(json!({ "contents": "hover part 1" }));
    }
    {
        let mut s = harness.mock_servers[1].state.lock().await;
        s.hover_response = Some(json!({ "contents": "hover part 2" }));
    }

    let hover_params = json!({
        "textDocument": { "uri": "file:///Users/sac/tower-lsp-composition/main.rs" },
        "position": { "line": 1, "character": 2 }
    });
    let resp = harness
        .client
        .send_request("textDocument/hover", hover_params)
        .await;
    let result = resp.get("result").expect("Hover should return result");
    let contents = result.get("contents").expect("Should contain contents");
    assert!(contents.to_string().contains("hover part 1"));
    assert!(contents.to_string().contains("hover part 2"));
    harness.shutdown();
}

#[tokio::test]
#[ignore = "e2e: requires live server, run with --include-ignored"]
async fn test_f3_t2_invalid_method() {
    let mut harness = TestHarness::new(1).await;
    let init_params = json!({
        "capabilities": {},
        "rootUri": null,
        "processId": null
    });
    let _ = harness.client.send_request("initialize", init_params).await;

    let resp = harness
        .client
        .send_request("invalid/methodName", json!({}))
        .await;
    let err = resp.get("error").expect("Invalid method must return error");
    let code = err.get("code").and_then(|c| c.as_i64()).unwrap_or(0);
    assert_eq!(code, -32601, "Error code must be MethodNotFound (-32601)");
    harness.shutdown();
}
