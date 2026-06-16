use crate::test_harness::TestHarness;
use serde_json::json;

/// 3. Mutating paths: verify formatting, rename, and code action edit behavior under version constraints.
#[tokio::test]
#[ignore = "e2e: requires live server, run with --include-ignored"]
async fn test_gate3_mutating_paths_version_constraints() {
    println!("--- Mutating paths: Starting TestHarness::new");
    let mut harness = TestHarness::new(1).await;

    // Enable codeActionProvider on mock server
    {
        let mut s0 = harness.mock_servers[0].state.lock().await;
        if let Some(obj) = s0.custom_capabilities.as_object_mut() {
            obj.insert("codeActionProvider".to_string(), json!(true));
        }
    }

    let init_params = json!({
        "capabilities": {
            "textDocument": {
                "rename": { "prepareSupport": true },
                "formatting": {},
                "codeAction": { "codeActionLiteralSupport": { "codeActionKind": { "valueSet": [] } } }
            }
        },
        "rootUri": "file:///Users/sac/tower-lsp-composition",
        "processId": 1234
    });
    println!("--- Sending initialize");
    let _ = harness.client.send_request("initialize", init_params).await;
    println!("--- Sending initialized notification");
    harness
        .client
        .send_notification("initialized", json!({}))
        .await;

    let file_uri = "file:///Users/sac/tower-lsp-composition/main.rs";

    println!("--- Sending didOpen (version 1)");
    // 1. Open the file with version 1
    harness
        .client
        .send_notification(
            "textDocument/didOpen",
            json!({
                "textDocument": {
                    "uri": file_uri,
                    "languageId": "rust",
                    "version": 1,
                    "text": "fn main() {}"
                }
            }),
        )
        .await;

    println!("--- Sending didChange (version 5)");
    // 2. Change the file to version 5
    harness
        .client
        .send_notification(
            "textDocument/didChange",
            json!({
                "textDocument": {
                    "uri": file_uri,
                    "version": 5
                },
                "contentChanges": [{ "text": "fn main() {\n    // Some code\n}" }]
            }),
        )
        .await;

    // --- RENAME VERSION CONSTRAINT TESTS ---
    // A. Positive: current version (5)
    let rename_edit_v5 = json!({
        "documentChanges": [
            {
                "textDocument": {
                    "uri": file_uri,
                    "version": 5
                },
                "edits": [
                    { "range": { "start": { "line": 0, "character": 3 }, "end": { "line": 0, "character": 7 } }, "newText": "my_main" }
                ]
            }
        ]
    });
    println!("--- Configuring mock server rename response (v5)");
    {
        let mut s0 = harness.mock_servers[0].state.lock().await;
        s0.rename_response = Some(rename_edit_v5);
    }
    println!("--- Sending rename request (v5)");
    let rename_resp_v5 = harness
        .client
        .send_request(
            "textDocument/rename",
            json!({
                "textDocument": { "uri": file_uri },
                "position": { "line": 0, "character": 3 },
                "newName": "my_main"
            }),
        )
        .await;
    println!("--- Got rename response (v5): {:?}", rename_resp_v5);
    assert!(
        rename_resp_v5.get("result").is_some() && !rename_resp_v5.get("result").unwrap().is_null(),
        "Rename request with matching version 5 must be accepted"
    );

    // B. Negative: stale version (1)
    let rename_edit_v1 = json!({
        "documentChanges": [
            {
                "textDocument": {
                    "uri": file_uri,
                    "version": 1
                },
                "edits": [
                    { "range": { "start": { "line": 0, "character": 3 }, "end": { "line": 0, "character": 7 } }, "newText": "stale_main" }
                ]
            }
        ]
    });
    println!("--- Configuring mock server rename response (v1)");
    {
        let mut s0 = harness.mock_servers[0].state.lock().await;
        s0.rename_response = Some(rename_edit_v1);
    }
    println!("--- Sending rename request (v1)");
    let rename_resp_v1 = harness
        .client
        .send_request(
            "textDocument/rename",
            json!({
                "textDocument": { "uri": file_uri },
                "position": { "line": 0, "character": 3 },
                "newName": "stale_main"
            }),
        )
        .await;
    println!("--- Got rename response (v1): {:?}", rename_resp_v1);
    assert!(
        rename_resp_v1.get("error").is_some()
            || rename_resp_v1.get("result").is_none()
            || rename_resp_v1.get("result").unwrap().is_null(),
        "Rename request with stale version 1 must be rejected"
    );

    // --- FORMATTING VERSION CONSTRAINT TESTS ---
    // Clear pending rename edit by sending a didChange notification with version 6
    harness
        .client
        .send_notification(
            "textDocument/didChange",
            json!({
                "textDocument": {
                    "uri": file_uri,
                    "version": 6
                },
                "contentChanges": [
                    {
                        "text": "fn main() {\n    // Some code\n}"
                    }
                ]
            }),
        )
        .await;

    // A. Positive: current request context (6)
    let format_response = json!([
        {
            "range": { "start": { "line": 0, "character": 0 }, "end": { "line": 2, "character": 1 } },
            "newText": "fn main() {\n    // Formatted code\n}"
        }
    ]);
    println!("--- Configuring mock server formatting response");
    {
        let mut s0 = harness.mock_servers[0].state.lock().await;
        s0.formatting_response = Some(format_response);
    }
    println!("--- Sending formatting request (v6)");
    let format_resp_v6 = harness
        .client
        .send_request(
            "textDocument/formatting",
            json!({
                "textDocument": { "uri": file_uri },
                "options": { "tabSize": 4, "insertSpaces": true },
                "context": { "version": 6 }
            }),
        )
        .await;
    println!("--- Got formatting response (v6): {:?}", format_resp_v6);
    assert!(
        format_resp_v6.get("result").is_some() && !format_resp_v6.get("result").unwrap().is_null(),
        "Formatting request with matching version 6 must be accepted"
    );

    // B. Negative: stale request context (1)
    println!("--- Sending formatting request (v1)");
    let format_resp_v1 = harness
        .client
        .send_request(
            "textDocument/formatting",
            json!({
                "textDocument": { "uri": file_uri },
                "options": { "tabSize": 4, "insertSpaces": true },
                "context": { "version": 1 }
            }),
        )
        .await;
    println!("--- Got formatting response (v1): {:?}", format_resp_v1);
    assert!(
        format_resp_v1.get("error").is_some()
            || format_resp_v1.get("result").is_none()
            || format_resp_v1.get("result").unwrap().is_null(),
        "Formatting request with stale version 1 must be rejected or return null"
    );

    // --- CODE ACTION VERSION CONSTRAINT TESTS ---
    // Clear pending formatting edit by sending a didChange notification with version 7
    harness
        .client
        .send_notification(
            "textDocument/didChange",
            json!({
                "textDocument": {
                    "uri": file_uri,
                    "version": 7
                },
                "contentChanges": [
                    {
                        "text": "fn main() {\n    // Some code\n}"
                    }
                ]
            }),
        )
        .await;

    // A. Positive: current request context (7)
    let code_action_response = json!([
        {
            "title": "Clean Code",
            "kind": "quickfix",
            "edit": {
                "changes": {
                    file_uri: [
                        { "range": { "start": { "line": 1, "character": 4 }, "end": { "line": 1, "character": 17 } }, "newText": "// Cleaned" }
                    ]
                }
            }
        }
    ]);
    println!("--- Configuring mock server code action response");
    {
        let mut s0 = harness.mock_servers[0].state.lock().await;
        s0.code_action_response = Some(code_action_response);
    }
    println!("--- Sending code action request (v7)");
    let code_action_resp_v7 = harness.client.send_request("textDocument/codeAction", json!({
        "textDocument": { "uri": file_uri },
        "range": { "start": { "line": 1, "character": 0 }, "end": { "line": 1, "character": 20 } },
        "context": { "diagnostics": [], "version": 7 }
    })).await;
    println!(
        "--- Got code action response (v7): {:?}",
        code_action_resp_v7
    );
    assert!(
        code_action_resp_v7.get("result").is_some()
            && !code_action_resp_v7.get("result").unwrap().is_null(),
        "Code action request with matching version 7 must be accepted"
    );

    // B. Negative: stale request context (1)
    println!("--- Sending code action request (v1)");
    let code_action_resp_v1 = harness.client.send_request("textDocument/codeAction", json!({
        "textDocument": { "uri": file_uri },
        "range": { "start": { "line": 1, "character": 0 }, "end": { "line": 1, "character": 20 } },
        "context": { "diagnostics": [], "version": 1 }
    })).await;
    println!(
        "--- Got code action response (v1): {:?}",
        code_action_resp_v1
    );
    assert!(
        code_action_resp_v1.get("error").is_some()
            || code_action_resp_v1.get("result").is_none()
            || code_action_resp_v1.get("result").unwrap().is_null(),
        "Code action request with stale version 1 must be rejected or return null"
    );

    println!("--- Shutting down harness");
    harness.shutdown();
}
