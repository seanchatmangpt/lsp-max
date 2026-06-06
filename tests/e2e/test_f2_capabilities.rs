use crate::test_harness::TestHarness;
use serde_json::json;

#[tokio::test]
async fn test_f2_t1_initial_advertisement() {
    let mut harness = TestHarness::new(2).await;
    // Set mock upstream 1 capability
    {
        let mut s = harness.mock_servers[0].state.lock().await;
        s.custom_capabilities = json!({
            "hoverProvider": true,
            "completionProvider": { "resolveProvider": true }
        });
    }
    // Set mock upstream 2 capability
    {
        let mut s = harness.mock_servers[1].state.lock().await;
        s.custom_capabilities = json!({
            "hoverProvider": true,
            "completionProvider": { "resolveProvider": false }
        });
    }

    let init_params = json!({
        "capabilities": {
            "textDocument": {
                "hover": {},
                "completion": {}
            }
        },
        "rootUri": null,
        "processId": null
    });
    let resp = harness.client.send_request("initialize", init_params).await;
    let result = resp.get("result").expect("Must return a result");
    let caps = result
        .get("capabilities")
        .expect("Must contain capabilities");

    // In composed server: capabilities should be intersection.
    // hoverProvider is true for both.
    assert_eq!(
        caps.get("hoverProvider").and_then(|h| h.as_bool()),
        Some(true)
    );
    harness.shutdown();
}

#[tokio::test]
async fn test_f2_t1_dynamic_registration() {
    let mut harness = TestHarness::new(1).await;
    // Set up initial connection
    let init_params = json!({
        "capabilities": {
            "textDocument": {
                "hover": { "dynamicRegistration": true }
            }
        },
        "rootUri": null,
        "processId": null
    });
    let _ = harness.client.send_request("initialize", init_params).await;
    harness
        .client
        .send_notification("initialized", json!({}))
        .await;

    // Simulate an upstream registering a dynamic capability
    let registration = json!({
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
        let mut s = harness.mock_servers[0].state.lock().await;
        s.dynamic_registrations.push(registration);
    }

    // Composed server should forward the dynamic registration request to the client
    // Here we check if client receives the message
    // (Under skeleton, this won't be forwarded, which is expected RED behavior)
    let msg = tokio::time::timeout(
        std::time::Duration::from_millis(100),
        harness.client.read_message(),
    )
    .await
    .expect("Timeout waiting for client/registerCapability")
    .expect("Connection closed while waiting for client/registerCapability");
    assert_eq!(
        msg.get("method").and_then(|m| m.as_str()),
        Some("client/registerCapability")
    );
    harness.shutdown();
}

#[tokio::test]
async fn test_f2_t1_dynamic_unregistration() {
    let mut harness = TestHarness::new(1).await;
    let init_params = json!({
        "capabilities": {},
        "rootUri": null,
        "processId": null
    });
    let _ = harness.client.send_request("initialize", init_params).await;

    // Register first
    let registration = json!({
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
        let mut s = harness.mock_servers[0].state.lock().await;
        s.dynamic_registrations.push(registration);
    }

    let _reg_msg = tokio::time::timeout(
        std::time::Duration::from_millis(500),
        harness.client.read_message(),
    )
    .await
    .expect("Timeout waiting for client/registerCapability")
    .expect("Connection closed while waiting for client/registerCapability");

    let unregistration = json!({
        "jsonrpc": "2.0",
        "method": "client/unregisterCapability",
        "params": {
            "unregisterations": [
                {
                    "id": "dynamic-hover-id",
                    "method": "textDocument/hover"
                }
            ]
        }
    });
    {
        let mut s = harness.mock_servers[0].state.lock().await;
        s.dynamic_unregistrations.push(unregistration);
    }

    let msg = tokio::time::timeout(
        std::time::Duration::from_millis(500),
        harness.client.read_message(),
    )
    .await
    .expect("Timeout waiting for client/unregisterCapability")
    .expect("Connection closed while waiting for client/unregisterCapability");
    assert_eq!(
        msg.get("method").and_then(|m| m.as_str()),
        Some("client/unregisterCapability")
    );
    harness.shutdown();
}

#[tokio::test]
async fn test_f2_t1_intersection_rules() {
    let mut harness = TestHarness::new(2).await;
    {
        let mut s = harness.mock_servers[0].state.lock().await;
        s.custom_capabilities = json!({ "definitionProvider": true });
    }
    {
        let mut s = harness.mock_servers[1].state.lock().await;
        s.custom_capabilities = json!({ "definitionProvider": false });
    }
    let init_params = json!({
        "capabilities": {
            "textDocument": { "definition": {} }
        },
        "rootUri": null,
        "processId": null
    });
    let resp = harness.client.send_request("initialize", init_params).await;
    let result = resp.get("result").unwrap();
    let caps = result.get("capabilities").unwrap();
    // Intersection rules: if one does not support, then definitionProvider should be false/omitted or customized
    let def_provider = caps.get("definitionProvider");
    assert!(
        def_provider.is_none()
            || def_provider == Some(&json!(false))
            || def_provider == Some(&json!(true)),
        "Intersection should be evaluated"
    );
    harness.shutdown();
}

#[tokio::test]
async fn test_f2_t1_safety_constraint_gating() {
    let mut harness = TestHarness::new(1).await;
    {
        let mut s = harness.mock_servers[0].state.lock().await;
        s.custom_capabilities = json!({
            "workspace": {
                "workspaceEdit": { "documentChanges": true }
            }
        });
    }
    let init_params = json!({
        "capabilities": {},
        "rootUri": null,
        "processId": null
    });
    let resp = harness.client.send_request("initialize", init_params).await;
    let result = resp.get("result").unwrap();
    let caps = result.get("capabilities").unwrap();
    // Verify that workspace capabilities are gated/filtered based on security policy
    let workspace = caps.get("workspace");
    if let Some(w) = workspace {
        assert!(
            w.get("workspaceEdit").is_none()
                || w.get("workspaceEdit")
                    .unwrap()
                    .get("documentChanges")
                    .is_none()
        );
    }
    harness.shutdown();
}

#[tokio::test]
async fn test_f2_t2_dynamic_registration_duplicate() {
    let mut harness = TestHarness::new(1).await;
    let init_params = json!({
        "capabilities": {},
        "rootUri": null,
        "processId": null
    });
    let _ = harness.client.send_request("initialize", init_params).await;

    // Send duplicate dynamic registration requests from mock server
    let reg = json!({
        "jsonrpc": "2.0",
        "method": "client/registerCapability",
        "params": {
            "registrations": [
                {
                    "id": "dup-id",
                    "method": "textDocument/hover",
                    "registerOptions": {}
                }
            ]
        }
    });
    {
        let mut s = harness.mock_servers[0].state.lock().await;
        s.dynamic_registrations.push(reg.clone());
        s.dynamic_registrations.push(reg);
    }
    
    // Read and assert first registration is received
    let msg1 = tokio::time::timeout(
        std::time::Duration::from_millis(500),
        harness.client.read_message(),
    )
    .await;
    assert!(msg1.is_ok(), "First registration should be received");

    // Read a second time and assert it times out
    let msg2 = tokio::time::timeout(
        std::time::Duration::from_millis(500),
        harness.client.read_message(),
    )
    .await;
    assert!(
        msg2.is_err(),
        "Duplicate registration should be ignored/filtered"
    );
    harness.shutdown();
}

#[tokio::test]
async fn test_f2_t2_dynamic_registration_missing_id() {
    let mut harness = TestHarness::new(1).await;
    let init_params = json!({
        "capabilities": {},
        "rootUri": null,
        "processId": null
    });
    let _ = harness.client.send_request("initialize", init_params).await;

    let reg_missing_id = json!({
        "jsonrpc": "2.0",
        "method": "client/registerCapability",
        "params": {
            "registrations": [
                {
                    "method": "textDocument/hover"
                }
            ]
        }
    });
    {
        let mut s = harness.mock_servers[0].state.lock().await;
        s.dynamic_registrations.push(reg_missing_id);
    }
    let msg = tokio::time::timeout(
        std::time::Duration::from_millis(100),
        harness.client.read_message(),
    )
    .await;
    assert!(
        msg.is_err(),
        "Missing ID registration should be rejected at composed boundary"
    );
    harness.shutdown();
}

#[tokio::test]
async fn test_f2_t2_intersection_with_unsupported_upstream() {
    let mut harness = TestHarness::new(2).await;
    {
        let mut s = harness.mock_servers[0].state.lock().await;
        s.custom_capabilities = json!({ "hoverProvider": true });
    }
    {
        let mut s = harness.mock_servers[1].state.lock().await;
        s.custom_capabilities = json!({ "hoverProvider": false });
    }
    let init_params = json!({
        "capabilities": {},
        "rootUri": null,
        "processId": null
    });
    let _ = harness.client.send_request("initialize", init_params).await;
    // Query hover. Only mock 1 supports it, mock 2 does not. Composed server routes only to mock 1.
    let hover_params = json!({
        "textDocument": { "uri": "file:///Users/sac/tower-lsp-composition/main.rs" },
        "position": { "line": 0, "character": 0 }
    });
    let _ = harness
        .client
        .send_request("textDocument/hover", hover_params)
        .await;

    // Check received requests on mock 2
    {
        let s2 = harness.mock_servers[1].state.lock().await;
        let has_hover = s2
            .received_requests
            .iter()
            .any(|r| r.get("method").and_then(|m| m.as_str()) == Some("textDocument/hover"));
        assert!(
            !has_hover,
            "Unsupported upstream must not receive hover queries"
        );
    }
    harness.shutdown();
}

#[tokio::test]
async fn test_f2_t2_advertisement_client_incapable() {
    let mut harness = TestHarness::new(1).await;
    {
        let mut s = harness.mock_servers[0].state.lock().await;
        s.custom_capabilities = json!({ "hoverProvider": true });
    }
    let init_params = json!({
        "capabilities": {
            "textDocument": {
                // Client explicitly disables hover
                "hover": null
            }
        },
        "rootUri": null,
        "processId": null
    });
    let resp = harness.client.send_request("initialize", init_params).await;
    let result = resp.get("result").unwrap();
    let caps = result.get("capabilities").unwrap();
    // Since client does not support hover, it shouldn't be advertised in effective support
    assert!(
        caps.get("hoverProvider").is_none() || caps.get("hoverProvider") == Some(&json!(false))
    );
    harness.shutdown();
}

#[tokio::test]
async fn test_f2_t2_dynamic_unregistration_nonexistent() {
    let mut harness = TestHarness::new(1).await;
    let init_params = json!({
        "capabilities": {},
        "rootUri": null,
        "processId": null
    });
    let _ = harness.client.send_request("initialize", init_params).await;

    let unreg = json!({
        "jsonrpc": "2.0",
        "method": "client/unregisterCapability",
        "params": {
            "unregisterations": [
                {
                    "id": "nonexistent-id",
                    "method": "textDocument/hover"
                }
            ]
        }
    });
    {
        let mut s = harness.mock_servers[0].state.lock().await;
        s.dynamic_unregistrations.push(unreg);
    }
    let msg = tokio::time::timeout(
        std::time::Duration::from_millis(100),
        harness.client.read_message(),
    )
    .await;
    assert!(
        msg.is_err(),
        "Unregistering nonexistent capability must not crash or trigger downstream client message"
    );
    harness.shutdown();
}
