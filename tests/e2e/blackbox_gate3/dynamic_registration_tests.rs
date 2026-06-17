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

/// 2. Dynamic unregistration: dynamic register, dynamic unregister, and verify
///    downstream capability changes and query refusals.
#[tokio::test]
#[ignore = "e2e: requires live server, run with --include-ignored"]
async fn test_gate3_dynamic_unregistration_and_refusal() {
    println!("--- Dynamic unregistration: Starting TestHarness::new");
    let mut harness = TestHarness::new(1).await;

    println!("--- Setting custom capabilities to hoverProvider = false");
    // Initially disable hover capability on the mock server
    {
        let mut s0 = harness.mock_servers[0].state.lock().await;
        s0.custom_capabilities = json!({
            "hoverProvider": false
        });
    }

    println!("--- Sending initialize");
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
    println!("--- Sending initialized notification");
    harness
        .client
        .send_notification("initialized", json!({}))
        .await;

    // A hover request now should fail because hover is not routable yet
    let hover_params = json!({
        "textDocument": { "uri": "file:///Users/sac/tower-lsp-composition/main.rs" },
        "position": { "line": 0, "character": 0 }
    });
    println!("--- Sending initial hover request (expected to fail)");
    let initial_hover = harness
        .client
        .send_request("textDocument/hover", hover_params.clone())
        .await;
    println!("--- Got initial hover result: {:?}", initial_hover);
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
    println!("--- Mock server pushing dynamic registration");
    {
        let mut s0 = harness.mock_servers[0].state.lock().await;
        s0.dynamic_registrations.push(registration);
    }

    println!("--- Reading message from client (waiting for registerCapability)");
    // Composed server should forward the dynamic registration request to the client
    let reg_msg = tokio::time::timeout(
        std::time::Duration::from_millis(2000),
        harness.client.read_message(),
    )
    .await
    .expect("Timeout waiting for client/registerCapability")
    .expect("Connection closed");
    println!("--- Got client/registerCapability message: {:?}", reg_msg);
    assert_eq!(
        reg_msg.get("method").and_then(|m| m.as_str()),
        Some("client/registerCapability")
    );

    // Respond back from the client confirming the registration
    let reg_id = reg_msg.get("id").unwrap().clone();
    println!(
        "--- Responding to registerCapability request with id {:?}",
        reg_id
    );
    respond_to_request(&mut harness.client.stream, reg_id, json!(null)).await;

    println!("--- Configuring mock server hover response");
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

    println!("--- Sending active hover request");
    // Now, sending a hover request should succeed
    let active_hover = harness
        .client
        .send_request("textDocument/hover", hover_params.clone())
        .await;
    println!("--- Got active hover response: {:?}", active_hover);
    let hover_res = active_hover
        .get("result")
        .expect("Hover request must return a result");
    let hover_val = hover_res
        .get("contents")
        .and_then(|c| c.get("value"))
        .and_then(|v| v.as_str())
        .unwrap_or("");
    assert!(
        hover_val.contains("Hover Active"),
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
    println!("--- Mock server pushing dynamic unregistration");
    {
        let mut s0 = harness.mock_servers[0].state.lock().await;
        s0.dynamic_unregistrations.push(unregistration);
    }

    println!("--- Reading message from client (waiting for unregisterCapability)");
    // Composed server should forward the dynamic unregistration request to the client
    let unreg_msg = tokio::time::timeout(
        std::time::Duration::from_millis(2000),
        harness.client.read_message(),
    )
    .await
    .expect("Timeout waiting for client/unregisterCapability")
    .expect("Connection closed");
    println!(
        "--- Got client/unregisterCapability message: {:?}",
        unreg_msg
    );
    assert_eq!(
        unreg_msg.get("method").and_then(|m| m.as_str()),
        Some("client/unregisterCapability")
    );

    // Respond back from the client confirming the unregistration
    let unreg_id = unreg_msg.get("id").unwrap().clone();
    println!(
        "--- Responding to unregisterCapability request with id {:?}",
        unreg_id
    );
    respond_to_request(&mut harness.client.stream, unreg_id, json!(null)).await;

    println!("--- Sending final hover request (expected to fail after unregistration)");
    // Verify hover requests are refused once again after unregistration
    let final_hover = harness
        .client
        .send_request("textDocument/hover", hover_params)
        .await;
    println!("--- Got final hover response: {:?}", final_hover);
    assert!(
        final_hover.get("error").is_some()
            || final_hover.get("result").is_none()
            || final_hover.get("result").unwrap().is_null(),
        "Hover query must be refused after dynamic unregistration"
    );

    println!("--- Shutting down harness");
    harness.shutdown();
}
