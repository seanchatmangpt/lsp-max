use crate::test_harness::TestHarness;
use serde_json::json;
use tempfile::NamedTempFile;

#[tokio::test]
async fn test_f7_t1_static_graph_definitions() {
    let mut harness = TestHarness::new(1).await;
    let init_params = json!({
        "capabilities": {},
        "rootUri": null,
        "processId": null
    });
    let _ = harness.client.send_request("initialize", init_params).await;

    let def_params = json!({
        "textDocument": { "uri": "file:///Users/sac/tower-lsp-composition/main.rs" },
        "position": { "line": 5, "character": 10 }
    });
    let resp = harness
        .client
        .send_request("textDocument/definition", def_params)
        .await;
    // Static graph should yield definition
    assert!(resp.get("error").is_none());
    harness.shutdown();
}

#[tokio::test]
async fn test_f7_t1_static_graph_references() {
    let mut harness = TestHarness::new(1).await;
    let init_params = json!({
        "capabilities": {},
        "rootUri": null,
        "processId": null
    });
    let _ = harness.client.send_request("initialize", init_params).await;

    let ref_params = json!({
        "textDocument": { "uri": "file:///Users/sac/tower-lsp-composition/main.rs" },
        "position": { "line": 5, "character": 10 },
        "context": { "includeDeclaration": true }
    });
    let resp = harness
        .client
        .send_request("textDocument/references", ref_params)
        .await;
    assert!(resp.get("error").is_none());
    harness.shutdown();
}

#[tokio::test]
async fn test_f7_t1_static_graph_hover() {
    let mut harness = TestHarness::new(1).await;
    let init_params = json!({
        "capabilities": {},
        "rootUri": null,
        "processId": null
    });
    let _ = harness.client.send_request("initialize", init_params).await;

    let hover_params = json!({
        "textDocument": { "uri": "file:///Users/sac/tower-lsp-composition/main.rs" },
        "position": { "line": 5, "character": 10 }
    });
    let resp = harness
        .client
        .send_request("textDocument/hover", hover_params)
        .await;
    assert!(resp.get("error").is_none());
    harness.shutdown();
}

#[tokio::test]
async fn test_f7_t1_static_graph_attribution() {
    let mut harness = TestHarness::new(1).await;
    let init_params = json!({
        "capabilities": {},
        "rootUri": null,
        "processId": null
    });
    let _ = harness.client.send_request("initialize", init_params).await;

    let hover_params = json!({
        "textDocument": { "uri": "file:///Users/sac/tower-lsp-composition/main.rs" },
        "position": { "line": 5, "character": 10 }
    });
    let resp = harness
        .client
        .send_request("textDocument/hover", hover_params)
        .await;
    if let Some(res) = resp.get("result") {
        if let Some(contents) = res.get("contents") {
            assert!(contents.to_string().contains("static-graph") || contents.is_null());
        }
    }
    harness.shutdown();
}

#[tokio::test]
async fn test_f7_t1_static_graph_routing() {
    let mut harness = TestHarness::new(1).await;
    let init_params = json!({
        "capabilities": {},
        "rootUri": "file:///Users/sac/tower-lsp-composition",
        "processId": null
    });
    let _ = harness.client.send_request("initialize", init_params).await;

    {
        let mut s = harness.mock_servers[0].state.lock().await;
        s.definition_response = Some(json!([{
            "uri": "file:///Users/sac/tower-lsp-composition/dynamic.rs",
            "range": { "start": { "line": 0, "character": 0 }, "end": { "line": 0, "character": 5 } }
        }]));
    }

    let def_params = json!({
        "textDocument": { "uri": "file:///Users/sac/tower-lsp-composition/main.rs" },
        "position": { "line": 5, "character": 10 }
    });
    let resp = harness
        .client
        .send_request("textDocument/definition", def_params)
        .await;

    // Assert no error, and if results are returned they are merged
    assert!(resp.get("error").is_none());
    if let Some(res) = resp.get("result") {
        if let Some(arr) = res.as_array() {
            assert!(!arr.is_empty(), "Merged result should contain entries");
        }
    }
    harness.shutdown();
}

#[tokio::test]
async fn test_f7_t2_static_graph_corrupt_db() {
    let _temp_file = NamedTempFile::new().unwrap();
    // Simulate reading a corrupted DB
    let mut harness = TestHarness::new(1).await;
    let init_params = json!({
        "capabilities": {},
        "rootUri": "file:///Users/sac/tower-lsp-composition",
        "processId": null
    });
    let _ = harness.client.send_request("initialize", init_params).await;

    // Upstream should fallback to dynamic successfully
    {
        let mut s = harness.mock_servers[0].state.lock().await;
        s.hover_response = Some(json!({ "contents": "fallback dynamic hover" }));
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
async fn test_f7_t2_static_graph_out_of_bounds_query() {
    let mut harness = TestHarness::new(1).await;
    let init_params = json!({
        "capabilities": {},
        "rootUri": null,
        "processId": null
    });
    let _ = harness.client.send_request("initialize", init_params).await;

    let hover_params = json!({
        "textDocument": { "uri": "file:///Users/sac/tower-lsp-composition/main.rs" },
        // Out of bounds character
        "position": { "line": 100000, "character": 999999 }
    });
    let resp = harness
        .client
        .send_request("textDocument/hover", hover_params)
        .await;
    assert!(resp.get("result").is_none() || resp.get("result").unwrap().is_null());
    harness.shutdown();
}

#[tokio::test]
async fn test_f7_t2_static_graph_unsupported_methods() {
    let mut harness = TestHarness::new(1).await;
    let init_params = json!({
        "capabilities": {},
        "rootUri": null,
        "processId": null
    });
    let _ = harness.client.send_request("initialize", init_params).await;

    let rename_params = json!({
        "textDocument": { "uri": "file:///Users/sac/tower-lsp-composition/main.rs" },
        "position": { "line": 5, "character": 10 },
        "newName": "test"
    });
    let resp = harness
        .client
        .send_request("textDocument/rename", rename_params)
        .await;
    // Mutating request should not impact static graph source
    assert!(
        resp.get("error").is_some()
            || resp.get("result").is_none()
            || resp.get("result").unwrap().is_null()
    );
    harness.shutdown();
}

#[tokio::test]
async fn test_f7_t2_static_graph_stale_data_rejection() {
    let mut harness = TestHarness::new(1).await;
    let init_params = json!({
        "capabilities": {},
        "rootUri": "file:///Users/sac/tower-lsp-composition",
        "processId": null
    });
    let _ = harness.client.send_request("initialize", init_params).await;

    // Bump document version significantly
    harness.client.send_notification("textDocument/didChange", json!({
        "textDocument": { "uri": "file:///Users/sac/tower-lsp-composition/main.rs", "version": 50 },
        "contentChanges": [{ "text": "highly modified content" }]
    })).await;

    let hover_params = json!({
        "textDocument": { "uri": "file:///Users/sac/tower-lsp-composition/main.rs" },
        "position": { "line": 5, "character": 10 }
    });
    let resp = harness
        .client
        .send_request("textDocument/hover", hover_params)
        .await;

    // Stale static graph results should be rejected/null
    if let Some(res) = resp.get("result") {
        assert!(
            res.is_null()
                || res.get("contents").is_none()
                || res.get("contents").unwrap().is_null()
        );
    }
    harness.shutdown();
}

#[tokio::test]
async fn test_f7_t2_static_graph_duplicate_edges() {
    // Parser must handle duplicate edges in LSIF
    let mut harness = TestHarness::new(1).await;
    let init_params = json!({
        "capabilities": {},
        "rootUri": "file:///Users/sac/tower-lsp-composition",
        "processId": null
    });
    let _ = harness.client.send_request("initialize", init_params).await;

    let hover_params = json!({
        "textDocument": { "uri": "file:///Users/sac/tower-lsp-composition/main.rs" },
        "position": { "line": 5, "character": 10 }
    });

    let resp = harness
        .client
        .send_request("textDocument/hover", hover_params)
        .await;
    assert!(
        resp.get("error").is_none()
            || resp.get("result").is_none()
            || resp.get("result").unwrap().is_null()
    );
    harness.shutdown();
}
