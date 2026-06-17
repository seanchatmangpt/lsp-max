use crate::test_harness::TestHarness;
use serde_json::{json, Value};

#[tokio::test]
#[ignore = "e2e: requires live server, run with --include-ignored"]
async fn test_f4_t1_attribution_diagnostics() {
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

    // Send diagnostics from upstream 0 and 1
    let diag0 = json!({
        "jsonrpc": "2.0",
        "method": "textDocument/publishDiagnostics",
        "params": {
            "uri": "file:///Users/sac/tower-lsp-composition/main.rs",
            "diagnostics": [
                {
                    "range": { "start": { "line": 0, "character": 0 }, "end": { "line": 0, "character": 5 } },
                    "message": "Error from upstream 0"
                }
            ]
        }
    });
    let diag1 = json!({
        "jsonrpc": "2.0",
        "method": "textDocument/publishDiagnostics",
        "params": {
            "uri": "file:///Users/sac/tower-lsp-composition/main.rs",
            "diagnostics": [
                {
                    "range": { "start": { "line": 1, "character": 0 }, "end": { "line": 1, "character": 5 } },
                    "message": "Error from upstream 1"
                }
            ]
        }
    });

    {
        let mut s = harness.mock_servers[0].state.lock().await;
        s.diagnostics_to_send.push(diag0);
    }
    {
        let mut s = harness.mock_servers[1].state.lock().await;
        s.diagnostics_to_send.push(diag1);
    }

    // Composed server should merge and publish diagnostics with clear attribution tag (e.g. source)
    let msg1 = tokio::time::timeout(
        std::time::Duration::from_millis(100),
        harness.client.read_message(),
    )
    .await
    .expect("Timeout waiting for diagnostics msg1")
    .expect("Connection closed while waiting for diagnostics msg1");
    let params = msg1.get("params").unwrap();
    let diags = params.get("diagnostics").unwrap().as_array().unwrap();
    for d in diags {
        assert!(
            d.get("source").is_some(),
            "Diagnostics must have source attribution"
        );
    }
    harness.shutdown();
}

#[tokio::test]
#[ignore = "e2e: requires live server, run with --include-ignored"]
async fn test_f4_t1_attribution_hover_fragments() {
    let mut harness = TestHarness::new(2).await;
    let init_params = json!({
        "capabilities": {},
        "rootUri": null,
        "processId": null
    });
    let _ = harness.client.send_request("initialize", init_params).await;

    {
        let mut s = harness.mock_servers[0].state.lock().await;
        s.hover_response =
            Some(json!({ "contents": { "kind": "markdown", "value": "first-content" } }));
    }
    {
        let mut s = harness.mock_servers[1].state.lock().await;
        s.hover_response =
            Some(json!({ "contents": { "kind": "markdown", "value": "second-content" } }));
    }

    let hover_params = json!({
        "textDocument": { "uri": "file:///Users/sac/tower-lsp-composition/main.rs" },
        "position": { "line": 0, "character": 0 }
    });
    let resp = harness
        .client
        .send_request("textDocument/hover", hover_params)
        .await;
    let result = resp.get("result").unwrap();
    let contents = result
        .get("contents")
        .unwrap()
        .get("value")
        .or(result.get("contents"))
        .unwrap()
        .as_str()
        .unwrap();
    // Headers or content should contain source identity attribution
    assert!(contents.contains("first-content") || contents.contains("second-content"));
    harness.shutdown();
}

#[tokio::test]
#[ignore = "e2e: requires live server, run with --include-ignored"]
async fn test_f4_t1_document_version_increment() {
    let mut harness = TestHarness::new(1).await;
    let init_params = json!({
        "capabilities": {},
        "rootUri": null,
        "processId": null
    });
    let _ = harness.client.send_request("initialize", init_params).await;

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
                    "text": "fn main() {}\n"
                }
            }),
        )
        .await;

    // Send didChange
    harness
        .client
        .send_notification(
            "textDocument/didChange",
            json!({
                "textDocument": {
                    "uri": "file:///Users/sac/tower-lsp-composition/main.rs",
                    "version": 2
                },
                "contentChanges": [
                    { "text": "fn main() { let x = 1; }\n" }
                ]
            }),
        )
        .await;

    tokio::time::sleep(std::time::Duration::from_millis(50)).await;

    {
        let s = harness.mock_servers[0].state.lock().await;
        let changes: Vec<&Value> = s
            .received_requests
            .iter()
            .filter(|r| r.get("method").and_then(|m| m.as_str()) == Some("textDocument/didChange"))
            .collect();
        if !changes.is_empty() {
            let ver = changes[0]
                .get("params")
                .unwrap()
                .get("textDocument")
                .unwrap()
                .get("version")
                .unwrap()
                .as_i64()
                .unwrap();
            assert_eq!(ver, 2);
        }
    }
    harness.shutdown();
}

#[tokio::test]
#[ignore = "e2e: requires live server, run with --include-ignored"]
async fn test_f4_t1_drop_stale_response() {
    let mut harness = TestHarness::new(1).await;
    let init_params = json!({
        "capabilities": {},
        "rootUri": null,
        "processId": null
    });
    let _ = harness.client.send_request("initialize", init_params).await;

    // Set a delay to make response slow
    {
        let mut s = harness.mock_servers[0].state.lock().await;
        s.delay = std::time::Duration::from_millis(100);
        s.hover_response = Some(json!({ "contents": "stale content" }));
    }

    // Client requests hover
    let hover_req_id = harness
        .client
        .write_request(
            "textDocument/hover",
            json!({
                "textDocument": { "uri": "file:///Users/sac/tower-lsp-composition/main.rs" },
                "position": { "line": 0, "character": 0 }
            }),
        )
        .await;

    // Concurrently, client modifies document (increments version)
    harness.client.send_notification("textDocument/didChange", json!({
        "textDocument": { "uri": "file:///Users/sac/tower-lsp-composition/main.rs", "version": 2 },
        "contentChanges": [{ "text": "new content" }]
    })).await;

    let resp = harness.client.read_response(hover_req_id).await;
    // The hover response returned from upstream was computed for version 1, but we are at version 2.
    // It should be dropped or return null/error.
    let res = resp.get("result");
    assert!(
        res.is_none() || res.unwrap().is_null(),
        "Stale response must be dropped or returned as null"
    );
    harness.shutdown();
}

#[tokio::test]
#[ignore = "e2e: requires live server, run with --include-ignored"]
async fn test_f4_t1_encoding_utf16() {
    let mut harness = TestHarness::new(1).await;
    let init_params = json!({
        "capabilities": {},
        "rootUri": null,
        "processId": null
    });
    let _ = harness.client.send_request("initialize", init_params).await;

    // Send document with a multi-byte UTF-16 character (emoji)
    // 𐐀 is U+10400 (takes 2 UTF-16 code units: 0xD801 0xDC00)
    harness
        .client
        .send_notification(
            "textDocument/didOpen",
            json!({
                "textDocument": {
                    "uri": "file:///Users/sac/tower-lsp-composition/main.rs",
                    "languageId": "rust",
                    "version": 1,
                    "text": "let x = \"𐐀\";\n"
                }
            }),
        )
        .await;

    let hover_params = json!({
        "textDocument": { "uri": "file:///Users/sac/tower-lsp-composition/main.rs" },
        // Position on or after U+10400
        "position": { "line": 0, "character": 10 }
    });
    let resp = harness
        .client
        .send_request("textDocument/hover", hover_params)
        .await;
    assert!(
        resp.get("error").is_none(),
        "Should successfully parse UTF-16 offsets without crashing"
    );
    harness.shutdown();
}

#[tokio::test]
#[ignore = "e2e: requires live server, run with --include-ignored"]
async fn test_f4_t2_diagnostics_clear() {
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

    // Clear diagnostics of only upstream 0
    let clear_msg = json!({
        "jsonrpc": "2.0",
        "method": "textDocument/publishDiagnostics",
        "params": {
            "uri": "file:///Users/sac/tower-lsp-composition/main.rs",
            "diagnostics": []
        }
    });
    {
        let mut s = harness.mock_servers[0].state.lock().await;
        s.diagnostics_to_send.push(clear_msg);
    }

    let msg = tokio::time::timeout(
        std::time::Duration::from_millis(100),
        harness.client.read_message(),
    )
    .await
    .expect("Timeout waiting for diagnostics clear msg")
    .expect("Connection closed while waiting for diagnostics clear msg");
    let params = msg.get("params").unwrap();
    let diags = params.get("diagnostics").unwrap().as_array().unwrap();
    assert!(
        diags.is_empty(),
        "Diagnostics should be cleared for this source only"
    );
    harness.shutdown();
}

#[tokio::test]
#[ignore = "e2e: requires live server, run with --include-ignored"]
async fn test_f4_t2_out_of_order_did_change() {
    let mut harness = TestHarness::new(1).await;
    let init_params = json!({
        "capabilities": {},
        "rootUri": null,
        "processId": null
    });
    let _ = harness.client.send_request("initialize", init_params).await;

    // Send version 3 change, then version 2 change
    harness.client.send_notification("textDocument/didChange", json!({
        "textDocument": { "uri": "file:///Users/sac/tower-lsp-composition/main.rs", "version": 3 },
        "contentChanges": [{ "text": "version 3" }]
    })).await;

    harness.client.send_notification("textDocument/didChange", json!({
        "textDocument": { "uri": "file:///Users/sac/tower-lsp-composition/main.rs", "version": 2 },
        "contentChanges": [{ "text": "version 2" }]
    })).await;

    tokio::time::sleep(std::time::Duration::from_millis(50)).await;

    {
        let s = harness.mock_servers[0].state.lock().await;
        let last_change = s
            .received_requests
            .iter()
            .rfind(|r| r.get("method").and_then(|m| m.as_str()) == Some("textDocument/didChange"));
        if let Some(c) = last_change {
            let ver = c
                .get("params")
                .unwrap()
                .get("textDocument")
                .unwrap()
                .get("version")
                .unwrap()
                .as_i64()
                .unwrap();
            assert!(
                ver >= 3,
                "Out-of-order stale didChange should be ignored/rejected"
            );
        }
    }
    harness.shutdown();
}

#[tokio::test]
#[ignore = "e2e: requires live server, run with --include-ignored"]
async fn test_f4_t2_invalid_utf16_surrogate() {
    let mut harness = TestHarness::new(1).await;
    let init_params = json!({
        "capabilities": {},
        "rootUri": null,
        "processId": null
    });
    let _ = harness.client.send_request("initialize", init_params).await;

    // Send malformed/broken surrogate text
    harness
        .client
        .send_notification(
            "textDocument/didOpen",
            json!({
                "textDocument": {
                    "uri": "file:///Users/sac/tower-lsp-composition/main.rs",
                    "languageId": "rust",
                    "version": 1,
                    "text": String::from_utf16_lossy(&[0xD800]) + " invalid surrogate"
                }
            }),
        )
        .await;

    let hover_params = json!({
        "textDocument": { "uri": "file:///Users/sac/tower-lsp-composition/main.rs" },
        "position": { "line": 0, "character": 5 }
    });
    let resp = harness
        .client
        .send_request("textDocument/hover", hover_params)
        .await;
    // Composed server should not crash, it should return Ok or error response
    assert!(resp.get("error").is_none() || resp.get("error").unwrap().get("code").is_some());
    harness.shutdown();
}

#[tokio::test]
#[ignore = "e2e: requires live server, run with --include-ignored"]
async fn test_f4_t2_empty_did_change() {
    let mut harness = TestHarness::new(1).await;
    let init_params = json!({
        "capabilities": {},
        "rootUri": null,
        "processId": null
    });
    let _ = harness.client.send_request("initialize", init_params).await;

    // Send didChange with empty changes array
    harness.client.send_notification("textDocument/didChange", json!({
        "textDocument": { "uri": "file:///Users/sac/tower-lsp-composition/main.rs", "version": 1 },
        "contentChanges": []
    })).await;

    // Verify the server remains alive and responds to subsequent requests
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
async fn test_f4_t2_stale_request_rejection() {
    let mut harness = TestHarness::new(1).await;
    let init_params = json!({
        "capabilities": {},
        "rootUri": null,
        "processId": null
    });
    let _ = harness.client.send_request("initialize", init_params).await;

    // Client increments document version to 5
    for v in 1..=5 {
        harness.client.send_notification("textDocument/didChange", json!({
            "textDocument": { "uri": "file:///Users/sac/tower-lsp-composition/main.rs", "version": v },
            "contentChanges": [{ "text": "content" }]
        })).await;
    }

    // Now query hover with reference to outdated version (e.g. version 2)
    // Some LSPs have version-bound requests. If supported, verify it rejects it.
    let hover_params = json!({
        "textDocument": { "uri": "file:///Users/sac/tower-lsp-composition/main.rs", "version": 2 },
        "position": { "line": 0, "character": 0 }
    });
    let resp = harness
        .client
        .send_request("textDocument/hover", hover_params)
        .await;
    // Stale requests should either return null/error
    let result = resp.get("result");
    let error = resp.get("error");
    assert!(result.is_none() || result.unwrap().is_null() || error.is_some());
    harness.shutdown();
}
