use crate::test_harness::TestHarness;
use serde_json::{json, Value};

async fn respond_to_request(stream: &mut tokio::io::DuplexStream, id: Value, result: Value) {
    let resp = json!({
        "jsonrpc": "2.0",
        "id": id,
        "result": result
    });
    let payload = serde_json::to_string(&resp).unwrap();
    let msg = format!("Content-Length: {}\r\n\r\n{}", payload.len(), payload).into_bytes();
    use tokio::io::AsyncWriteExt;
    stream.write_all(&msg).await.unwrap();
    stream.flush().await.unwrap();
}

/// 1. Composed Initialize: client sends initialize, upstream A advertises hover, upstream B does not;
///    verify downstream capabilities.
#[tokio::test]
async fn test_gate3_composed_initialize() {
    let mut harness = TestHarness::new(2).await;

    // Upstream A advertises hoverProvider: true
    {
        let mut s0 = harness.mock_servers[0].state.lock().await;
        s0.custom_capabilities = json!({
            "hoverProvider": true
        });
    }

    // Upstream B does not advertise hoverProvider
    {
        let mut s1 = harness.mock_servers[1].state.lock().await;
        s1.custom_capabilities = json!({
            "hoverProvider": false
        });
    }

    let init_params = json!({
        "capabilities": {
            "textDocument": {
                "hover": {
                    "contentFormat": ["markdown", "plaintext"]
                }
            }
        },
        "rootUri": "file:///Users/sac/tower-lsp-composition",
        "processId": 1234
    });

    let resp = harness.client.send_request("initialize", init_params).await;
    let result = resp.get("result").expect("Initialize must return a result");
    let caps = result.get("capabilities").expect("Result must contain capabilities");

    // Since Upstream A advertises hover, the composed server must advertise hover downstream
    assert_eq!(
        caps.get("hoverProvider").and_then(|h| h.as_bool()),
        Some(true),
        "Composed server must inherit hoverProvider capability from Upstream A"
    );

    harness.shutdown();

    // Now test when NEITHER upstream supports hover
    let mut harness2 = TestHarness::new(2).await;
    {
        let mut s0 = harness2.mock_servers[0].state.lock().await;
        s0.custom_capabilities = json!({ "hoverProvider": false });
    }
    {
        let mut s1 = harness2.mock_servers[1].state.lock().await;
        s1.custom_capabilities = json!({ "hoverProvider": false });
    }

    let init_params2 = json!({
        "capabilities": {
            "textDocument": {
                "hover": {
                    "contentFormat": ["markdown", "plaintext"]
                }
            }
        },
        "rootUri": "file:///Users/sac/tower-lsp-composition",
        "processId": 1234
    });

    let resp2 = harness2.client.send_request("initialize", init_params2).await;
    let result2 = resp2.get("result").expect("Initialize must return a result");
    let caps2 = result2.get("capabilities").expect("Result must contain capabilities");

    let hover_provider = caps2.get("hoverProvider");
    assert!(
        hover_provider.is_none() || hover_provider == Some(&json!(false)),
        "Composed server must not advertise hoverProvider if no upstreams support it"
    );

    harness2.shutdown();
}

/// 2. Dynamic unregistration: dynamic register, dynamic unregister, and verify
///    downstream capability changes and query refusals.
#[tokio::test]
async fn test_gate3_dynamic_unregistration_and_refusal() {
    let mut harness = TestHarness::new(1).await;

    // Initially disable hover capability on the mock server
    {
        let mut s0 = harness.mock_servers[0].state.lock().await;
        s0.custom_capabilities = json!({
            "hoverProvider": false
        });
    }

    let init_params = json!({
        "capabilities": {
            "textDocument": {
                "hover": { "dynamicRegistration": true }
            }
        },
        "rootUri": "file:///Users/sac/tower-lsp-composition",
        "processId": 1234
    });
    let _ = harness.client.send_request("initialize", init_params).await;
    harness.client.send_notification("initialized", json!({})).await;

    // A hover request now should fail because hover is not routable yet
    let hover_params = json!({
        "textDocument": { "uri": "file:///Users/sac/tower-lsp-composition/main.rs" },
        "position": { "line": 0, "character": 0 }
    });
    let initial_hover = harness.client.send_request("textDocument/hover", hover_params.clone()).await;
    assert!(
        initial_hover.get("error").is_some()
            || initial_hover.get("result").is_none()
            || initial_hover.get("result").unwrap().is_null(),
        "Hover query must be refused when no provider is registered"
    );

    // Mock server pushes a dynamic registration to register hover capability
    let registration = json!({
        "jsonrpc": "2.0",
        "method": "client/registerCapability",
        "params": {
            "registrations": [
                {
                    "id": "dynamic-hover-gate3",
                    "method": "textDocument/hover",
                    "registerOptions": {}
                }
            ]
        }
    });
    {
        let mut s0 = harness.mock_servers[0].state.lock().await;
        s0.dynamic_registrations.push(registration);
    }

    // Composed server should forward the dynamic registration request to the client
    let reg_msg = tokio::time::timeout(
        std::time::Duration::from_millis(2000),
        harness.client.read_message(),
    )
    .await
    .expect("Timeout waiting for client/registerCapability")
    .expect("Connection closed");
    assert_eq!(
        reg_msg.get("method").and_then(|m| m.as_str()),
        Some("client/registerCapability")
    );

    // Respond back from the client confirming the registration
    let reg_id = reg_msg.get("id").unwrap().clone();
    respond_to_request(&mut harness.client.stream, reg_id, json!(null)).await;

    // Configure the mock server to return a specific hover response
    {
        let mut s0 = harness.mock_servers[0].state.lock().await;
        s0.hover_response = Some(json!({
            "contents": {
                "kind": "markdown",
                "value": "Hover Active"
            }
        }));
    }

    // Now, sending a hover request should succeed
    let active_hover = harness.client.send_request("textDocument/hover", hover_params.clone()).await;
    let hover_res = active_hover.get("result").expect("Hover request must return a result");
    assert!(
        hover_res.get("contents").and_then(|c| c.get("value")).and_then(|v| v.as_str()) == Some("Hover Active"),
        "Hover request should return mock content after dynamic registration"
    );

    // Now, dynamically unregister hover capability
    let unregistration = json!({
        "jsonrpc": "2.0",
        "method": "client/unregisterCapability",
        "params": {
            "unregisterations": [
                {
                    "id": "dynamic-hover-gate3",
                    "method": "textDocument/hover"
                }
            ]
        }
    });
    {
        let mut s0 = harness.mock_servers[0].state.lock().await;
        s0.dynamic_unregistrations.push(unregistration);
    }

    // Composed server should forward the dynamic unregistration request to the client
    let unreg_msg = tokio::time::timeout(
        std::time::Duration::from_millis(2000),
        harness.client.read_message(),
    )
    .await
    .expect("Timeout waiting for client/unregisterCapability")
    .expect("Connection closed");
    assert_eq!(
        unreg_msg.get("method").and_then(|m| m.as_str()),
        Some("client/unregisterCapability")
    );

    // Respond back from the client confirming the unregistration
    let unreg_id = unreg_msg.get("id").unwrap().clone();
    respond_to_request(&mut harness.client.stream, unreg_id, json!(null)).await;

    // Verify hover requests are refused once again after unregistration
    let final_hover = harness.client.send_request("textDocument/hover", hover_params).await;
    assert!(
        final_hover.get("error").is_some()
            || final_hover.get("result").is_none()
            || final_hover.get("result").unwrap().is_null(),
        "Hover query must be refused after dynamic unregistration"
    );

    harness.shutdown();
}

/// 3. Mutating paths: verify formatting, rename, and code action edit behavior under version constraints.
#[tokio::test]
async fn test_gate3_mutating_paths_version_constraints() {
    let mut harness = TestHarness::new(1).await;

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
    let _ = harness.client.send_request("initialize", init_params).await;
    harness.client.send_notification("initialized", json!({})).await;

    let file_uri = "file:///Users/sac/tower-lsp-composition/main.rs";

    // 1. Open the file with version 1
    harness.client.send_notification("textDocument/didOpen", json!({
        "textDocument": {
            "uri": file_uri,
            "languageId": "rust",
            "version": 1,
            "text": "fn main() {}"
        }
    })).await;

    // 2. Change the file to version 5
    harness.client.send_notification("textDocument/didChange", json!({
        "textDocument": {
            "uri": file_uri,
            "version": 5
        },
        "contentChanges": [{ "text": "fn main() {\n    // Some code\n}" }]
    })).await;

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
    {
        let mut s0 = harness.mock_servers[0].state.lock().await;
        s0.rename_response = Some(rename_edit_v5);
    }
    let rename_resp_v5 = harness.client.send_request("textDocument/rename", json!({
        "textDocument": { "uri": file_uri },
        "position": { "line": 0, "character": 3 },
        "newName": "my_main"
    })).await;
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
    {
        let mut s0 = harness.mock_servers[0].state.lock().await;
        s0.rename_response = Some(rename_edit_v1);
    }
    let rename_resp_v1 = harness.client.send_request("textDocument/rename", json!({
        "textDocument": { "uri": file_uri },
        "position": { "line": 0, "character": 3 },
        "newName": "stale_main"
    })).await;
    assert!(
        rename_resp_v1.get("error").is_some()
            || rename_resp_v1.get("result").is_none()
            || rename_resp_v1.get("result").unwrap().is_null(),
        "Rename request with stale version 1 must be rejected"
    );

    // --- FORMATTING VERSION CONSTRAINT TESTS ---
    // A. Positive: current request context (5)
    let format_response = json!([
        {
            "range": { "start": { "line": 0, "character": 0 }, "end": { "line": 2, "character": 1 } },
            "newText": "fn main() {\n    // Formatted code\n}"
        }
    ]);
    {
        let mut s0 = harness.mock_servers[0].state.lock().await;
        s0.formatting_response = Some(format_response);
    }
    let format_resp_v5 = harness.client.send_request("textDocument/formatting", json!({
        "textDocument": { "uri": file_uri },
        "options": { "tabSize": 4, "insertSpaces": true },
        "context": { "version": 5 }
    })).await;
    assert!(
        format_resp_v5.get("result").is_some() && !format_resp_v5.get("result").unwrap().is_null(),
        "Formatting request with matching version 5 must be accepted"
    );

    // B. Negative: stale request context (1)
    let format_resp_v1 = harness.client.send_request("textDocument/formatting", json!({
        "textDocument": { "uri": file_uri },
        "options": { "tabSize": 4, "insertSpaces": true },
        "context": { "version": 1 }
    })).await;
    assert!(
        format_resp_v1.get("error").is_some()
            || format_resp_v1.get("result").is_none()
            || format_resp_v1.get("result").unwrap().is_null(),
        "Formatting request with stale version 1 must be rejected or return null"
    );

    // --- CODE ACTION VERSION CONSTRAINT TESTS ---
    // A. Positive: current request context (5)
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
    {
        let mut s0 = harness.mock_servers[0].state.lock().await;
        s0.code_action_response = Some(code_action_response);
    }
    let code_action_resp_v5 = harness.client.send_request("textDocument/codeAction", json!({
        "textDocument": { "uri": file_uri },
        "range": { "start": { "line": 1, "character": 0 }, "end": { "line": 1, "character": 20 } },
        "context": { "diagnostics": [], "version": 5 }
    })).await;
    assert!(
        code_action_resp_v5.get("result").is_some() && !code_action_resp_v5.get("result").unwrap().is_null(),
        "Code action request with matching version 5 must be accepted"
    );

    // B. Negative: stale request context (1)
    let code_action_resp_v1 = harness.client.send_request("textDocument/codeAction", json!({
        "textDocument": { "uri": file_uri },
        "range": { "start": { "line": 1, "character": 0 }, "end": { "line": 1, "character": 20 } },
        "context": { "diagnostics": [], "version": 1 }
    })).await;
    assert!(
        code_action_resp_v1.get("error").is_some()
            || code_action_resp_v1.get("result").is_none()
            || code_action_resp_v1.get("result").unwrap().is_null(),
        "Code action request with stale version 1 must be rejected or return null"
    );

    harness.shutdown();
}
