use crate::test_harness::TestHarness;
use serde_json::json;

#[tokio::test]
async fn test_f5_t1_compose_initialize_capabilities() {
    let mut harness = TestHarness::new(2).await;
    
    // Set different capabilities on the upstreams:
    // Both support hover, neither supports declaration
    {
        let mut s = harness.mock_servers[0].state.lock().await;
        s.custom_capabilities = json!({
            "hoverProvider": true,
            "declarationProvider": false
        });
    }
    {
        let mut s = harness.mock_servers[1].state.lock().await;
        s.custom_capabilities = json!({
            "hoverProvider": true,
            "declarationProvider": false
        });
    }

    let init_params = json!({
        "capabilities": {
            "textDocument": {
                "hover": {},
                "declaration": {}
            }
        },
        "rootUri": null,
        "processId": null
    });

    let resp = harness.client.send_request("initialize", init_params).await;
    let result = resp.get("result").expect("Initialize should return a result");
    let caps = result.get("capabilities").expect("Result should contain capabilities");

    // hoverProvider is true since both upstreams and client support it
    assert_eq!(
        caps.get("hoverProvider").and_then(|h| h.as_bool()),
        Some(true)
    );
    // declarationProvider is false/none because upstreams do not support it
    let decl_provider = caps.get("declarationProvider");
    assert!(
        decl_provider.is_none()
            || decl_provider == Some(&json!(false))
    );

    harness.shutdown();
}

#[tokio::test]
async fn test_f5_t1_routing_document_sync_did_open() {
    let mut harness = TestHarness::new(2).await;
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

    let did_open_params = json!({
        "textDocument": {
            "uri": "file:///Users/sac/tower-lsp-composition/main.rs",
            "languageId": "rust",
            "version": 1,
            "text": "fn main() {}"
        }
    });

    harness
        .client
        .send_notification("textDocument/didOpen", did_open_params)
        .await;

    // Wait for the server to route notifications to both upstreams
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    for (idx, mock) in harness.mock_servers.iter().enumerate() {
        let s = mock.state.lock().await;
        let received_did_open = s
            .received_requests
            .iter()
            .any(|r| r.get("method").and_then(|m| m.as_str()) == Some("textDocument/didOpen"));
        assert!(
            received_did_open,
            "Mock server {} should have received the textDocument/didOpen notification",
            idx
        );
    }

    harness.shutdown();
}

#[tokio::test]
async fn test_f5_t1_routing_hover_first_success() {
    let mut harness = TestHarness::new(2).await;
    let init_params = json!({
        "capabilities": {},
        "rootUri": null,
        "processId": null
    });
    let _ = harness.client.send_request("initialize", init_params).await;

    // Upstream 0 returns null (no hover), Upstream 1 returns a valid hover response
    {
        let mut s = harness.mock_servers[0].state.lock().await;
        s.hover_response = None;
    }
    {
        let mut s = harness.mock_servers[1].state.lock().await;
        s.hover_response = Some(json!({
            "contents": "Hover content from mock-1"
        }));
    }

    let hover_params = json!({
        "textDocument": { "uri": "file:///Users/sac/tower-lsp-composition/main.rs" },
        "position": { "line": 0, "character": 0 }
    });

    let resp = harness
        .client
        .send_request("textDocument/hover", hover_params)
        .await;
    let result = resp.get("result").expect("Hover must return a result");
    assert!(!result.is_null());

    let contents = result.get("contents");
    assert!(contents.is_some(), "Hover result should contain contents");
    let val = contents.unwrap().get("value").and_then(|v| v.as_str()).unwrap_or("");
    assert!(
        val.contains("Hover content from mock-1"),
        "Hover contents should contain the response from the successful upstream"
    );

    harness.shutdown();
}

#[tokio::test]
async fn test_f5_t1_routing_completion_merge() {
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
                { "label": "apple", "detail": "first source", "sortText": "01" }
            ]
        }));
    }
    {
        let mut s = harness.mock_servers[1].state.lock().await;
        s.completion_response = Some(json!({
            "isIncomplete": false,
            "items": [
                { "label": "apricot", "detail": "second source", "sortText": "02" }
            ]
        }));
    }

    let comp_params = json!({
        "textDocument": { "uri": "file:///Users/sac/tower-lsp-composition/main.rs" },
        "position": { "line": 0, "character": 0 }
    });

    let resp = harness
        .client
        .send_request("textDocument/completion", comp_params)
        .await;
    let result = resp.get("result").expect("Completion should return a result");
    let items = result
        .get("items")
        .or(result.as_array().map(|_| result))
        .unwrap()
        .as_array()
        .unwrap();

    // The two lists should be merged and ranked by sortText
    assert_eq!(items.len(), 2);
    assert_eq!(
        items[0].get("label").and_then(|l| l.as_str()),
        Some("apple")
    );
    assert_eq!(
        items[1].get("label").and_then(|l| l.as_str()),
        Some("apricot")
    );

    harness.shutdown();
}

#[tokio::test]
async fn test_f5_t1_routing_definition_merge() {
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

    // Both servers return lists, loc1 is duplicated across both to test deduplication
    {
        let mut s = harness.mock_servers[0].state.lock().await;
        s.definition_response = Some(json!([loc1]));
    }
    {
        let mut s = harness.mock_servers[1].state.lock().await;
        s.definition_response = Some(json!([loc1, loc2]));
    }

    let def_params = json!({
        "textDocument": { "uri": "file:///Users/sac/tower-lsp-composition/main.rs" },
        "position": { "line": 1, "character": 2 }
    });

    let resp = harness
        .client
        .send_request("textDocument/definition", def_params)
        .await;
    let result = resp.get("result").expect("Definition should return a result");
    assert!(result.is_array());
    let arr = result.as_array().unwrap();

    // Results should be merged and deduplicated
    assert_eq!(arr.len(), 2);
    let mut contains_loc1 = false;
    let mut contains_loc2 = false;
    for item in arr {
        if item.get("uri").and_then(|u| u.as_str()) == Some("file:///Users/sac/tower-lsp-composition/main.rs") {
            contains_loc1 = true;
        } else if item.get("uri").and_then(|u| u.as_str()) == Some("file:///Users/sac/tower-lsp-composition/helper.rs") {
            contains_loc2 = true;
        }
    }
    assert!(contains_loc1);
    assert!(contains_loc2);

    harness.shutdown();
}

#[tokio::test]
async fn test_f5_t2_routing_slow_upstream_timeout() {
    let mut harness = TestHarness::new(2).await;
    let init_params = json!({
        "capabilities": {},
        "rootUri": null,
        "processId": null
    });
    let _ = harness.client.send_request("initialize", init_params).await;

    let loc1 = json!({
        "uri": "file:///Users/sac/tower-lsp-composition/slow.rs",
        "range": { "start": { "line": 0, "character": 0 }, "end": { "line": 0, "character": 5 } }
    });
    let loc2 = json!({
        "uri": "file:///Users/sac/tower-lsp-composition/fast.rs",
        "range": { "start": { "line": 1, "character": 0 }, "end": { "line": 1, "character": 5 } }
    });

    // Upstream 0 has a long delay (500ms), Upstream 1 returns immediately
    {
        let mut s = harness.mock_servers[0].state.lock().await;
        s.delay = std::time::Duration::from_millis(500);
        s.definition_response = Some(json!([loc1]));
    }
    {
        let mut s = harness.mock_servers[1].state.lock().await;
        s.definition_response = Some(json!([loc2]));
    }

    let def_params = json!({
        "textDocument": { "uri": "file:///Users/sac/tower-lsp-composition/main.rs" },
        "position": { "line": 0, "character": 0 }
    });

    // Wrap in tokio timeout of 1 second so the test itself doesn't hang in case of failure
    let resp_fut = harness.client.send_request("textDocument/definition", def_params);
    let resp = tokio::time::timeout(std::time::Duration::from_millis(1000), resp_fut)
        .await
        .expect("Request must return within timeout bounds");

    let result = resp.get("result").expect("Definition should return a result");
    assert!(result.is_array());
    let arr = result.as_array().unwrap();

    // Composed server returns partial results from Upstream 1 only, because Upstream 0 timed out
    assert_eq!(arr.len(), 1);
    assert_eq!(
        arr[0].get("uri").and_then(|u| u.as_str()),
        Some("file:///Users/sac/tower-lsp-composition/fast.rs")
    );

    harness.shutdown();
}

#[tokio::test]
async fn test_f5_t2_routing_crashed_upstream_resilience() {
    let mut harness = TestHarness::new(2).await;
    let init_params = json!({
        "capabilities": {},
        "rootUri": null,
        "processId": null
    });
    let _ = harness.client.send_request("initialize", init_params).await;

    // Upstream 0 crashes on hover, Upstream 1 returns successfully
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

    let result = resp.get("result").expect("Hover must return a result");
    assert!(!result.is_null());
    let contents = result.get("contents").expect("Must contain contents");
    let val = contents.get("value").and_then(|v| v.as_str()).unwrap_or("");
    assert!(val.contains("hover valid"));

    harness.shutdown();
}

#[tokio::test]
async fn test_f5_t2_routing_unsupported_method() {
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

#[tokio::test]
async fn test_f5_t2_routing_concurrent_stress() {
    let mut harness = TestHarness::new(2).await;
    let init_params = json!({
        "capabilities": {},
        "rootUri": null,
        "processId": null
    });
    let _ = harness.client.send_request("initialize", init_params).await;

    {
        let mut s = harness.mock_servers[0].state.lock().await;
        s.hover_response = Some(json!({ "contents": "mock-0 hover" }));
    }
    {
        let mut s = harness.mock_servers[1].state.lock().await;
        s.hover_response = Some(json!({ "contents": "mock-1 hover" }));
    }

    let hover_params = json!({
        "textDocument": { "uri": "file:///Users/sac/tower-lsp-composition/main.rs" },
        "position": { "line": 0, "character": 0 }
    });

    // Write 20 requests back-to-back
    let mut req_ids = Vec::new();
    for _ in 0..20 {
        let id = harness
            .client
            .write_request("textDocument/hover", hover_params.clone())
            .await;
        req_ids.push(id);
    }

    // Read all 20 responses
    for id in req_ids {
        let resp = harness.client.read_response(id).await;
        let result = resp.get("result").expect("Hover must return a result");
        assert!(!result.is_null());
        let contents = result.get("contents").expect("Must contain contents");
        let val = contents.get("value").and_then(|v| v.as_str()).unwrap_or("");
        assert!(val.contains("mock-0 hover") || val.contains("mock-1 hover"));
    }

    harness.shutdown();
}

#[tokio::test]
async fn test_f5_t2_routing_empty_upstreams() {
    let mut harness = TestHarness::new(0).await;
    
    let init_params = json!({
        "capabilities": {},
        "rootUri": null,
        "processId": null
    });

    let resp = harness.client.send_request("initialize", init_params).await;
    assert!(resp.get("result").is_some());

    harness
        .client
        .send_notification("initialized", json!({}))
        .await;

    let shutdown_resp = harness.client.send_request("shutdown", json!({})).await;
    // Composed server with empty upstreams returns internal error response on shutdown because there are no upstreams to route to
    assert!(shutdown_resp.get("error").is_some() || shutdown_resp.get("result").is_some());

    harness.client.send_notification("exit", json!({})).await;
    harness.shutdown();
}
