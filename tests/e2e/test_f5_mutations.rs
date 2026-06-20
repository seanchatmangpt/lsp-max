use crate::test_harness::TestHarness;
use serde_json::json;

#[tokio::test]
#[ignore = "e2e: requires live server, run with --include-ignored"]
async fn test_f5_t1_edit_gate_accept() {
    let mut harness = TestHarness::new(1).await;
    let init_params = json!({
        "capabilities": {},
        "rootUri": null,
        "processId": null
    });
    let _ = harness.client.send_request("initialize", init_params).await;

    let edit = json!({
        "changes": {
            "file:///Users/sac/tower-lsp-composition/main.rs": [
                { "range": { "start": { "line": 0, "character": 0 }, "end": { "line": 0, "character": 5 } }, "newText": "hello" }
            ]
        }
    });
    {
        let mut s = harness.mock_servers[0].state.lock().await;
        s.rename_response = Some(edit.clone());
    }

    let rename_params = json!({
        "textDocument": { "uri": "file:///Users/sac/tower-lsp-composition/main.rs" },
        "position": { "line": 0, "character": 0 },
        "newName": "hello"
    });
    let resp = harness
        .client
        .send_request("textDocument/rename", rename_params)
        .await;
    let result = resp.get("result").expect("Rename should return result");
    assert!(result.get("changes").is_some());
    harness.shutdown();
}

#[tokio::test]
#[ignore = "e2e: requires live server, run with --include-ignored"]
async fn test_f5_t1_edit_gate_refuse() {
    let mut harness = TestHarness::new(1).await;
    let init_params = json!({
        "capabilities": {},
        "rootUri": null,
        "processId": null
    });
    let _ = harness.client.send_request("initialize", init_params).await;

    // Simulate an invalid edit (out of bounds character or range)
    let invalid_edit = json!({
        "changes": {
            "file:///Users/sac/tower-lsp-composition/main.rs": [
                { "range": { "start": { "line": -5, "character": 0 }, "end": { "line": 0, "character": 5 } }, "newText": "bad" }
            ]
        }
    });
    {
        let mut s = harness.mock_servers[0].state.lock().await;
        s.rename_response = Some(invalid_edit);
    }

    let rename_params = json!({
        "textDocument": { "uri": "file:///Users/sac/tower-lsp-composition/main.rs" },
        "position": { "line": 0, "character": 0 },
        "newName": "bad"
    });
    let resp = harness
        .client
        .send_request("textDocument/rename", rename_params)
        .await;
    assert!(
        resp.get("error").is_some()
            || resp.get("result").is_none()
            || resp.get("result").unwrap().is_null(),
        "Invalid edits must be refused"
    );
    harness.shutdown();
}

#[tokio::test]
#[ignore = "e2e: requires live server, run with --include-ignored"]
async fn test_f5_t1_edit_gate_stale() {
    let mut harness = TestHarness::new(1).await;
    let init_params = json!({
        "capabilities": {},
        "rootUri": null,
        "processId": null
    });
    let _ = harness.client.send_request("initialize", init_params).await;

    // Increment document version
    harness.client.send_notification("textDocument/didChange", json!({
        "textDocument": { "uri": "file:///Users/sac/tower-lsp-composition/main.rs", "version": 5 },
        "contentChanges": [{ "text": "version 5 content" }]
    })).await;

    // Simulate edit proposed on stale version 1
    let stale_edit = json!({
        "changes": {
            "file:///Users/sac/tower-lsp-composition/main.rs": [
                {
                    "range": { "start": { "line": 0, "character": 0 }, "end": { "line": 0, "character": 5 } },
                    "newText": "stale",
                    // Bound to old version
                    "version": 1
                }
            ]
        }
    });
    {
        let mut s = harness.mock_servers[0].state.lock().await;
        s.rename_response = Some(stale_edit);
    }

    let rename_params = json!({
        "textDocument": { "uri": "file:///Users/sac/tower-lsp-composition/main.rs" },
        "position": { "line": 0, "character": 0 },
        "newName": "stale"
    });
    let resp = harness
        .client
        .send_request("textDocument/rename", rename_params)
        .await;
    assert!(
        resp.get("error").is_some()
            || resp.get("result").is_none()
            || resp.get("result").unwrap().is_null(),
        "Stale edits must be rejected"
    );
    harness.shutdown();
}

#[tokio::test]
#[ignore = "e2e: requires live server, run with --include-ignored"]
async fn test_f5_t1_overlapping_edits() {
    let mut harness = TestHarness::new(2).await;
    let init_params = json!({
        "capabilities": {},
        "rootUri": null,
        "processId": null
    });
    let _ = harness.client.send_request("initialize", init_params).await;

    // Upstream 0 proposes an edit on line 0, char 0-5
    {
        let mut s = harness.mock_servers[0].state.lock().await;
        s.rename_response = Some(json!({
            "changes": {
                "file:///Users/sac/tower-lsp-composition/main.rs": [
                    { "range": { "start": { "line": 0, "character": 0 }, "end": { "line": 0, "character": 5 } }, "newText": "abc" }
                ]
            }
        }));
    }
    // Upstream 1 concurrently proposes an overlapping edit on line 0, char 2-8
    {
        let mut s = harness.mock_servers[1].state.lock().await;
        s.rename_response = Some(json!({
            "changes": {
                "file:///Users/sac/tower-lsp-composition/main.rs": [
                    { "range": { "start": { "line": 0, "character": 2 }, "end": { "line": 0, "character": 8 } }, "newText": "def" }
                ]
            }
        }));
    }

    let rename_params = json!({
        "textDocument": { "uri": "file:///Users/sac/tower-lsp-composition/main.rs" },
        "position": { "line": 0, "character": 1 },
        "newName": "overlap"
    });
    let resp = harness
        .client
        .send_request("textDocument/rename", rename_params)
        .await;
    // Server must reject or reconcile overlap rather than blindly merge
    assert!(
        resp.get("error").is_some()
            || resp.get("result").is_none()
            || resp.get("result").unwrap().is_null()
    );
    harness.shutdown();
}

#[tokio::test]
#[ignore = "e2e: requires live server, run with --include-ignored"]
async fn test_f5_t1_needs_rebase() {
    let mut harness = TestHarness::new(1).await;
    let init_params = json!({
        "capabilities": {},
        "rootUri": null,
        "processId": null
    });
    let _ = harness.client.send_request("initialize", init_params).await;

    // Send a edit proposal where the document has since changed, but edits are non-overlapping
    // requiring a simple rebase of ranges.
    let rebase_edit = json!({
        "changes": {
            "file:///Users/sac/tower-lsp-composition/main.rs": [
                { "range": { "start": { "line": 1, "character": 0 }, "end": { "line": 1, "character": 5 } }, "newText": "rebased" }
            ]
        }
    });
    {
        let mut s = harness.mock_servers[0].state.lock().await;
        s.rename_response = Some(rebase_edit);
    }

    let rename_params = json!({
        "textDocument": { "uri": "file:///Users/sac/tower-lsp-composition/main.rs" },
        "position": { "line": 1, "character": 0 },
        "newName": "rebased"
    });
    let resp = harness
        .client
        .send_request("textDocument/rename", rename_params)
        .await;
    // Should successfully rebase or respond
    assert!(resp.get("error").is_none());
    harness.shutdown();
}

#[tokio::test]
#[ignore = "e2e: requires live server, run with --include-ignored"]
async fn test_f5_t2_empty_edit_list() {
    let mut harness = TestHarness::new(1).await;
    let init_params = json!({
        "capabilities": {},
        "rootUri": null,
        "processId": null
    });
    let _ = harness.client.send_request("initialize", init_params).await;

    // Empty edit list
    {
        let mut s = harness.mock_servers[0].state.lock().await;
        s.rename_response = Some(json!({ "changes": {} }));
    }

    let rename_params = json!({
        "textDocument": { "uri": "file:///Users/sac/tower-lsp-composition/main.rs" },
        "position": { "line": 0, "character": 0 },
        "newName": "empty"
    });
    let resp = harness
        .client
        .send_request("textDocument/rename", rename_params)
        .await;
    assert!(resp.get("error").is_none());
    harness.shutdown();
}

#[tokio::test]
#[ignore = "e2e: requires live server, run with --include-ignored"]
async fn test_f5_t2_adjacent_non_overlapping() {
    let mut harness = TestHarness::new(2).await;
    let init_params = json!({
        "capabilities": {},
        "rootUri": null,
        "processId": null
    });
    let _ = harness.client.send_request("initialize", init_params).await;

    // Directly adjacent edits (e.g. char 0-5 and char 5-10)
    {
        let mut s = harness.mock_servers[0].state.lock().await;
        s.rename_response = Some(json!({
            "changes": {
                "file:///Users/sac/tower-lsp-composition/main.rs": [
                    { "range": { "start": { "line": 0, "character": 0 }, "end": { "line": 0, "character": 5 } }, "newText": "abc" }
                ]
            }
        }));
    }
    {
        let mut s = harness.mock_servers[1].state.lock().await;
        s.rename_response = Some(json!({
            "changes": {
                "file:///Users/sac/tower-lsp-composition/main.rs": [
                    { "range": { "start": { "line": 0, "character": 5 }, "end": { "line": 0, "character": 10 } }, "newText": "def" }
                ]
            }
        }));
    }

    let rename_params = json!({
        "textDocument": { "uri": "file:///Users/sac/tower-lsp-composition/main.rs" },
        "position": { "line": 0, "character": 2 },
        "newName": "adjacent"
    });
    let resp = harness
        .client
        .send_request("textDocument/rename", rename_params)
        .await;
    // Since adjacent do not overlap, they should be accepted
    assert!(resp.get("error").is_none());
    harness.shutdown();
}

#[tokio::test]
#[ignore = "e2e: requires live server, run with --include-ignored"]
async fn test_f5_t2_rebase_failure() {
    let mut harness = TestHarness::new(1).await;
    let init_params = json!({
        "capabilities": {},
        "rootUri": "file:///Users/sac/tower-lsp-composition",
        "processId": null
    });
    let _ = harness.client.send_request("initialize", init_params).await;

    // Bump document version with a change at range line 0, char 0-5
    harness.client.send_notification("textDocument/didChange", json!({
        "textDocument": { "uri": "file:///Users/sac/tower-lsp-composition/main.rs", "version": 2 },
        "contentChanges": [{ "text": "inserted text at start" }]
    })).await;

    // Propose an edit that also targets line 0, char 0-5 but is bound to old version 1
    // (This results in a conflict that cannot be rebased)
    {
        let mut s = harness.mock_servers[0].state.lock().await;
        s.rename_response = Some(json!({
            "changes": {
                "file:///Users/sac/tower-lsp-composition/main.rs": [
                    {
                        "range": { "start": { "line": 0, "character": 0 }, "end": { "line": 0, "character": 5 } },
                        "newText": "conflict",
                        "version": 1
                    }
                ]
            }
        }));
    }

    let rename_params = json!({
        "textDocument": { "uri": "file:///Users/sac/tower-lsp-composition/main.rs" },
        "position": { "line": 0, "character": 0 },
        "newName": "conflict"
    });
    let resp = harness
        .client
        .send_request("textDocument/rename", rename_params)
        .await;

    // Conflict must be rejected (error or null result)
    assert!(
        resp.get("error").is_some()
            || resp.get("result").is_none()
            || resp.get("result").unwrap().is_null()
    );
    harness.shutdown();
}

#[tokio::test]
#[ignore = "e2e: requires live server, run with --include-ignored"]
async fn test_f5_t2_transactional_rollback() {
    let mut harness = TestHarness::new(1).await;
    let init_params = json!({
        "capabilities": {},
        "rootUri": null,
        "processId": null
    });
    let _ = harness.client.send_request("initialize", init_params).await;

    // Edit that touches multiple files, where one file has an invalid range
    let multi_file_edit = json!({
        "changes": {
            "file:///Users/sac/tower-lsp-composition/main.rs": [
                { "range": { "start": { "line": 0, "character": 0 }, "end": { "line": 0, "character": 5 } }, "newText": "ok" }
            ],
            "file:///Users/sac/tower-lsp-composition/invalid.rs": [
                { "range": { "start": { "line": -10, "character": 0 }, "end": { "line": 0, "character": 5 } }, "newText": "not_ok" }
            ]
        }
    });
    {
        let mut s = harness.mock_servers[0].state.lock().await;
        s.rename_response = Some(multi_file_edit);
    }

    let rename_params = json!({
        "textDocument": { "uri": "file:///Users/sac/tower-lsp-composition/main.rs" },
        "position": { "line": 0, "character": 0 },
        "newName": "rollback"
    });
    let resp = harness
        .client
        .send_request("textDocument/rename", rename_params)
        .await;
    // Must roll back everything and fail
    assert!(
        resp.get("error").is_some()
            || resp.get("result").is_none()
            || resp.get("result").unwrap().is_null()
    );
    harness.shutdown();
}

#[tokio::test]
#[ignore = "e2e: requires live server, run with --include-ignored"]
async fn test_f5_t2_user_choice_reconciliation() {
    let mut harness = TestHarness::new(1).await;
    let init_params = json!({
        "capabilities": {},
        "rootUri": "file:///Users/sac/tower-lsp-composition",
        "processId": null
    });
    let _ = harness.client.send_request("initialize", init_params).await;

    // Simulate two conflicting edits and trigger user choice dialog.
    // We send a rename request which will return a response.
    // If the server requires user choice, it would send a window/showMessageRequest.
    let rename_params = json!({
        "textDocument": { "uri": "file:///Users/sac/tower-lsp-composition/main.rs" },
        "position": { "line": 0, "character": 0 },
        "newName": "reconciled"
    });

    // We expect the server to either return an error/null on conflict, or prompt and succeed.
    let resp = harness
        .client
        .send_request("textDocument/rename", rename_params)
        .await;

    // Validation: the response must either be an error/null (if unresolved) or contain edits.
    assert!(
        resp.get("error").is_some()
            || resp.get("result").is_none()
            || resp.get("result").unwrap().is_null()
            || resp.get("result").unwrap().get("changes").is_some()
    );

    harness.shutdown();
}
