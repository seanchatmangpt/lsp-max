use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tower_lsp_max::{LspService, Server};
use tower_lsp_max_playground::Backend;

async fn read_message<R: tokio::io::AsyncRead + Unpin>(
    reader: &mut R,
) -> std::io::Result<serde_json::Value> {
    let mut header_buf = Vec::new();
    loop {
        let mut byte = [0u8; 1];
        reader.read_exact(&mut byte).await?;
        header_buf.push(byte[0]);
        if header_buf.ends_with(b"\r\n\r\n") {
            break;
        }
    }
    let header_str = String::from_utf8(header_buf)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    let len_line = header_str
        .lines()
        .next()
        .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::InvalidData, "Empty header"))?;
    if !len_line.starts_with("Content-Length: ") {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "Invalid Content-Length header",
        ));
    }
    let content_len: usize = len_line["Content-Length: ".len()..]
        .trim()
        .parse()
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;

    let mut body = vec![0u8; content_len];
    reader.read_exact(&mut body).await?;
    let val = serde_json::from_slice(&body)?;
    Ok(val)
}

fn encode_message(msg: &serde_json::Value) -> Vec<u8> {
    let payload = serde_json::to_string(msg).unwrap();
    format!("Content-Length: {}\r\n\r\n{}", payload.len(), payload).into_bytes()
}

async fn wait_for_response(
    received: Arc<Mutex<Vec<serde_json::Value>>>,
    id: i64,
    timeout: Duration,
) -> serde_json::Value {
    let start = std::time::Instant::now();
    loop {
        if start.elapsed() > timeout {
            panic!("Timeout waiting for response ID {}", id);
        }
        {
            let mut guard = received.lock().unwrap();
            if let Some(pos) = guard
                .iter()
                .position(|msg| msg.get("id").and_then(|i| i.as_i64()) == Some(id))
            {
                return guard.remove(pos);
            }
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
}

async fn wait_for_notification(
    received: Arc<Mutex<Vec<serde_json::Value>>>,
    method: &str,
    timeout: Duration,
) -> serde_json::Value {
    let start = std::time::Instant::now();
    loop {
        if start.elapsed() > timeout {
            panic!("Timeout waiting for notification {}", method);
        }
        {
            let mut guard = received.lock().unwrap();
            if let Some(pos) = guard
                .iter()
                .position(|msg| msg.get("method").and_then(|m| m.as_str()) == Some(method))
            {
                return guard.remove(pos);
            }
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
}

async fn write_msg(
    tx_shared: &Arc<tokio::sync::Mutex<Option<tokio::io::DuplexStream>>>,
    msg: serde_json::Value,
) {
    let mut guard = tx_shared.lock().await;
    if let Some(ref mut tx) = *guard {
        tx.write_all(&encode_message(&msg)).await.unwrap();
    }
}

#[tokio::test(flavor = "current_thread")]
async fn test_playground_integration() {
    // 1. Set up the LspService
    let (service, socket) = LspService::new(Backend::new);

    // Set up duplex streams for bidirectional communication
    let (client_tx, server_rx) = tokio::io::duplex(1024 * 1024);
    let (server_tx, client_rx) = tokio::io::duplex(1024 * 1024);

    // Create Server
    let server = Server::new(server_rx, server_tx, socket);

    // Spawn server to process requests asynchronously
    let server_handle = tokio::spawn(server.serve(service));

    // Client receive buffers
    let client_received_responses = Arc::new(Mutex::new(Vec::new()));
    let client_received_notifications = Arc::new(Mutex::new(Vec::new()));

    let client_tx_shared = Arc::new(tokio::sync::Mutex::new(Some(client_tx)));
    let client_tx_shared_clone = client_tx_shared.clone();

    let client_received_responses_clone = client_received_responses.clone();
    let client_received_notifications_clone = client_received_notifications.clone();

    let mut reader = tokio::io::BufReader::new(client_rx);
    let _client_reader_task = tokio::spawn(async move {
        loop {
            let msg = match read_message(&mut reader).await {
                Ok(m) => m,
                Err(_) => break, // EOF/closed
            };

            if let Some(_method) = msg.get("method").and_then(|m| m.as_str()) {
                if let Some(id) = msg.get("id") {
                    // Respond to server requests
                    let resp = serde_json::json!({
                        "jsonrpc": "2.0",
                        "id": id,
                        "result": null
                    });
                    let encoded = encode_message(&resp);
                    let mut writer_guard = client_tx_shared_clone.lock().await;
                    if let Some(ref mut tx) = *writer_guard {
                        let _ = tx.write_all(&encoded).await;
                    }
                } else {
                    client_received_notifications_clone
                        .lock()
                        .unwrap()
                        .push(msg);
                }
            } else if msg.get("id").is_some() {
                client_received_responses_clone.lock().unwrap().push(msg);
            }
        }
    });

    // Send initialize request
    let init_req = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "capabilities": {}
        }
    });
    write_msg(&client_tx_shared, init_req).await;

    let init_resp =
        wait_for_response(client_received_responses.clone(), 1, Duration::from_secs(2)).await;
    assert!(init_resp.get("result").is_some());

    // Send initialized notification
    let init_ntf = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "initialized",
        "params": {}
    });
    write_msg(&client_tx_shared, init_ntf).await;

    // 2. Open a document containing incomplete LanguageServer impl
    let doc_uri = "file:///Users/sac/tower-lsp-max/dummy_server.rs";
    let code_text = r#"
struct DummyServer;

#[tower_lsp_max::async_trait]
impl LanguageServer for DummyServer {
}
"#;

    let open_ntf = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "textDocument/didOpen",
        "params": {
            "textDocument": {
                "uri": doc_uri,
                "languageId": "rust",
                "version": 1,
                "text": code_text
            }
        }
    });
    write_msg(&client_tx_shared, open_ntf).await;

    // Wait for the publishDiagnostics notification
    let diag_ntf = wait_for_notification(
        client_received_notifications.clone(),
        "textDocument/publishDiagnostics",
        Duration::from_secs(2),
    )
    .await;

    let params = diag_ntf
        .get("params")
        .expect("Missing params in publishDiagnostics");
    assert_eq!(params.get("uri").and_then(|u| u.as_str()), Some(doc_uri));
    let diags = params
        .get("diagnostics")
        .expect("Missing diagnostics array")
        .as_array()
        .expect("diagnostics is not an array");

    // We expect TLM003 warnings/errors because initialize and shutdown are missing.
    assert!(
        !diags.is_empty(),
        "Expected diagnostics, but none were returned."
    );
    let mut missing_mandatory_diag = None;
    for diag in diags {
        let code = diag.get("code").and_then(|c| c.as_str());
        if code == Some("TLM003") {
            missing_mandatory_diag = Some(diag.clone());
        }
    }
    let missing_mandatory_diag = missing_mandatory_diag.expect("Expected TLM003 diagnostic code");

    // 3. Request Completion inside the impl block
    let code_text_with_i = r#"
struct DummyServer;

#[tower_lsp_max::async_trait]
impl LanguageServer for DummyServer {
    i
}
"#;
    let change_ntf = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "textDocument/didChange",
        "params": {
            "textDocument": {
                "uri": doc_uri,
                "version": 2
            },
            "contentChanges": [
                {
                    "text": code_text_with_i
                }
            ]
        }
    });
    write_msg(&client_tx_shared, change_ntf).await;

    // Query completion at Line 5, character 5 (after "    i")
    let completion_req = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "textDocument/completion",
        "params": {
            "textDocument": {
                "uri": doc_uri
            },
            "position": {
                "line": 5,
                "character": 5
            },
            "context": {
                "triggerKind": 1 // Invoked
            }
        }
    });
    write_msg(&client_tx_shared, completion_req).await;

    let completion_resp =
        wait_for_response(client_received_responses.clone(), 2, Duration::from_secs(2)).await;
    let comp_result = completion_resp
        .get("result")
        .expect("Missing result in completion response");

    let comp_items = match comp_result {
        serde_json::Value::Array(arr) => arr.clone(),
        serde_json::Value::Object(obj) => {
            if let Some(serde_json::Value::Array(arr)) = obj.get("items") {
                arr.clone()
            } else {
                panic!(
                    "Completions result is not an array and has no items field: {:?}",
                    comp_result
                );
            }
        }
        _ => panic!("Unexpected completion result shape: {:?}", comp_result),
    };

    assert!(
        !comp_items.is_empty(),
        "Expected completion items, got none"
    );
    let has_initialize = comp_items
        .iter()
        .any(|item| item.get("label").and_then(|l| l.as_str()) == Some("initialize"));
    assert!(
        has_initialize,
        "Completion list should contain `initialize` method suggestion"
    );

    // 4. Request Code Actions
    let code_action_req = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 3,
        "method": "textDocument/codeAction",
        "params": {
            "textDocument": {
                "uri": doc_uri
            },
            "range": {
                "start": { "line": 0, "character": 0 },
                "end": { "line": 0, "character": 0 }
            },
            "context": {
                "diagnostics": [
                    missing_mandatory_diag
                ]
            }
        }
    });
    write_msg(&client_tx_shared, code_action_req).await;

    let code_action_resp =
        wait_for_response(client_received_responses.clone(), 3, Duration::from_secs(2)).await;
    println!("DEBUG CODE ACTION RESP: {:#?}", code_action_resp);
    let action_result = code_action_resp
        .get("result")
        .expect("Missing result in codeAction response")
        .as_array()
        .expect("codeAction result is not an array");

    assert!(!action_result.is_empty(), "Expected code actions, got none");
    let has_scaffold_action = action_result.iter().any(|action| {
        let title = action.get("title").and_then(|t| t.as_str());
        title.is_some_and(|t| t.contains("Generate minimal server scaffold"))
    });
    assert!(
        has_scaffold_action,
        "Expected a quickfix/source code action to generate a minimal server scaffold"
    );

    // Shutdown client/server
    let shutdown_req = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 4,
        "method": "shutdown",
        "params": null
    });
    write_msg(&client_tx_shared, shutdown_req).await;
    let _ = wait_for_response(client_received_responses.clone(), 4, Duration::from_secs(2)).await;

    let exit_ntf = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "exit",
        "params": null
    });
    write_msg(&client_tx_shared, exit_ntf).await;

    // Drop client_tx to close the reader loop
    client_tx_shared.lock().await.take();
    let _ = server_handle.await;
}

#[tokio::test(flavor = "current_thread")]
async fn test_rpc_attribute_diagnostics() {
    // 1. Set up the LspService
    let (service, socket) = LspService::new(Backend::new);

    let (client_tx, server_rx) = tokio::io::duplex(1024 * 1024);
    let (server_tx, client_rx) = tokio::io::duplex(1024 * 1024);

    let server = Server::new(server_rx, server_tx, socket);
    let server_handle = tokio::spawn(server.serve(service));

    let client_received_responses = Arc::new(Mutex::new(Vec::new()));
    let client_received_notifications = Arc::new(Mutex::new(Vec::new()));

    let client_tx_shared = Arc::new(tokio::sync::Mutex::new(Some(client_tx)));
    let client_tx_shared_clone = client_tx_shared.clone();

    let client_received_responses_clone = client_received_responses.clone();
    let client_received_notifications_clone = client_received_notifications.clone();

    let mut reader = tokio::io::BufReader::new(client_rx);
    let client_reader_task = tokio::spawn(async move {
        loop {
            let msg = match read_message(&mut reader).await {
                Ok(m) => m,
                Err(_) => break,
            };

            if let Some(_method) = msg.get("method").and_then(|m| m.as_str()) {
                if let Some(id) = msg.get("id") {
                    let resp = serde_json::json!({
                        "jsonrpc": "2.0",
                        "id": id,
                        "result": null
                    });
                    let encoded = encode_message(&resp);
                    let mut writer_guard = client_tx_shared_clone.lock().await;
                    if let Some(ref mut tx) = *writer_guard {
                        let _ = tx.write_all(&encoded).await;
                    }
                } else {
                    client_received_notifications_clone
                        .lock()
                        .unwrap()
                        .push(msg);
                }
            } else if msg.get("id").is_some() {
                client_received_responses_clone.lock().unwrap().push(msg);
            }
        }
    });

    // Send initialize request
    let init_req = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "capabilities": {}
        }
    });
    write_msg(&client_tx_shared, init_req).await;
    let _init_resp =
        wait_for_response(client_received_responses.clone(), 1, Duration::from_secs(2)).await;

    // Send initialized notification
    let init_ntf = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "initialized",
        "params": {}
    });
    write_msg(&client_tx_shared, init_ntf).await;

    // 2. Open a document containing an invalid rpc attribute name
    let doc_uri = "file:///Users/sac/tower-lsp-max/test_rpc.rs";
    let code_text = r#"
#[rpc]
pub trait TestServer {
    #[rpc(name = "textDocument/didchange")]
    async fn did_change(&self, params: DidChangeTextDocumentParams);
}
"#;

    let open_ntf = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "textDocument/didOpen",
        "params": {
            "textDocument": {
                "uri": doc_uri,
                "languageId": "rust",
                "version": 1,
                "text": code_text
            }
        }
    });
    write_msg(&client_tx_shared, open_ntf).await;

    // Wait for the publishDiagnostics notification
    let diag_ntf = wait_for_notification(
        client_received_notifications.clone(),
        "textDocument/publishDiagnostics",
        Duration::from_secs(2),
    )
    .await;

    let params = diag_ntf.get("params").unwrap();
    let diags = params.get("diagnostics").unwrap().as_array().unwrap();

    // Verify we got TLM006 diagnostic code
    let mut invalid_rpc_diag = None;
    for diag in diags {
        let code = diag.get("code").and_then(|c| c.as_str());
        if code == Some("TLM006") {
            invalid_rpc_diag = Some(diag.clone());
        }
    }
    let invalid_rpc_diag = invalid_rpc_diag.expect("Expected TLM006 diagnostic code");
    
    // Check message has suggestion
    let message = invalid_rpc_diag.get("message").unwrap().as_str().unwrap();
    assert!(message.contains("Did you mean 'textDocument/didChange'?"), "Message was: {}", message);

    // 3. Request Code Actions for the typo
    let code_action_req = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "textDocument/codeAction",
        "params": {
            "textDocument": {
                "uri": doc_uri
            },
            "range": invalid_rpc_diag.get("range").unwrap(),
            "context": {
                "diagnostics": [
                    invalid_rpc_diag
                ]
            }
        }
    });
    write_msg(&client_tx_shared, code_action_req).await;

    let code_action_resp =
        wait_for_response(client_received_responses.clone(), 2, Duration::from_secs(2)).await;
    let action_result = code_action_resp
        .get("result")
        .unwrap()
        .as_array()
        .unwrap();

    assert!(!action_result.is_empty(), "Expected code actions, got none");
    let has_fix_action = action_result.iter().any(|action| {
        let title = action.get("title").and_then(|t| t.as_str());
        title.is_some_and(|t| t.contains("Correct RPC name to `textDocument/didChange`"))
    });
    assert!(
        has_fix_action,
        "Expected quickfix action to correct rpc name"
    );

    // Shutdown client/server
    let shutdown_req = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 3,
        "method": "shutdown",
        "params": null
    });
    write_msg(&client_tx_shared, shutdown_req).await;
    let _ = wait_for_response(client_received_responses.clone(), 3, Duration::from_secs(2)).await;

    let exit_ntf = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "exit",
        "params": null
    });
    write_msg(&client_tx_shared, exit_ntf).await;

    // Drop client_tx to close the reader loop
    client_tx_shared.lock().await.take();
    let _ = client_reader_task.await;
    let _ = server_handle.await;
}

#[tokio::test(flavor = "current_thread")]
async fn test_rpc_completion() {
    // 1. Set up the LspService
    let (service, socket) = LspService::new(Backend::new);

    let (client_tx, server_rx) = tokio::io::duplex(1024 * 1024);
    let (server_tx, client_rx) = tokio::io::duplex(1024 * 1024);

    let server = Server::new(server_rx, server_tx, socket);
    let server_handle = tokio::spawn(server.serve(service));

    let client_received_responses = Arc::new(Mutex::new(Vec::new()));
    let client_tx_shared = Arc::new(tokio::sync::Mutex::new(Some(client_tx)));
    let _client_tx_shared_clone = client_tx_shared.clone();
    let client_received_responses_clone = client_received_responses.clone();

    let mut reader = tokio::io::BufReader::new(client_rx);
    let client_reader_task = tokio::spawn(async move {
        loop {
            let msg = match read_message(&mut reader).await {
                Ok(m) => m,
                Err(_) => break,
            };
            if msg.get("id").is_some() {
                client_received_responses_clone.lock().unwrap().push(msg);
            }
        }
    });

    // Send initialize request
    let init_req = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "capabilities": {}
        }
    });
    write_msg(&client_tx_shared, init_req).await;
    let _init_resp =
        wait_for_response(client_received_responses.clone(), 1, Duration::from_secs(2)).await;

    // Send initialized notification
    let init_ntf = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "initialized",
        "params": {}
    });
    write_msg(&client_tx_shared, init_ntf).await;

    // 2. Open a document containing #[rpc(name = "textDoc
    let doc_uri = "file:///Users/sac/tower-lsp-max/test_completion.rs";
    let code_text = r#"
#[rpc]
pub trait TestServer {
    #[rpc(name = "textDoc
}
"#;

    let open_ntf = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "textDocument/didOpen",
        "params": {
            "textDocument": {
                "uri": doc_uri,
                "languageId": "rust",
                "version": 1,
                "text": code_text
            }
        }
    });
    write_msg(&client_tx_shared, open_ntf).await;

    // Query completion at Line 3, character 25 (after "#[rpc(name = \"textDoc")
    let completion_req = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "textDocument/completion",
        "params": {
            "textDocument": {
                "uri": doc_uri
            },
            "position": {
                "line": 3,
                "character": 25
            },
            "context": {
                "triggerKind": 1
            }
        }
    });
    write_msg(&client_tx_shared, completion_req).await;

    let completion_resp =
        wait_for_response(client_received_responses.clone(), 2, Duration::from_secs(2)).await;
    let comp_result = completion_resp.get("result").unwrap();
    let comp_items = match comp_result {
        serde_json::Value::Array(arr) => arr.clone(),
        serde_json::Value::Object(obj) => obj.get("items").unwrap().as_array().unwrap().clone(),
        _ => panic!("Unexpected completion result shape"),
    };

    assert!(!comp_items.is_empty(), "Expected completion items in rpc name block");
    let has_completion = comp_items
        .iter()
        .any(|item| item.get("label").and_then(|l| l.as_str()) == Some("textDocument/completion"));
    assert!(has_completion, "Should suggest 'textDocument/completion'");

    // 3. Let's verify standard LSP methods completions inside impl LanguageServer block
    let code_text_impl = r#"
impl LanguageServer for Backend {
    textDoc
}
"#;
    let change_ntf = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "textDocument/didChange",
        "params": {
            "textDocument": {
                "uri": doc_uri,
                "version": 2
            },
            "contentChanges": [
                {
                    "text": code_text_impl
                }
            ]
        }
    });
    write_msg(&client_tx_shared, change_ntf).await;

    let completion_req_impl = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 3,
        "method": "textDocument/completion",
        "params": {
            "textDocument": {
                "uri": doc_uri
            },
            "position": {
                "line": 2,
                "character": 11
            },
            "context": {
                "triggerKind": 1
            }
        }
    });
    write_msg(&client_tx_shared, completion_req_impl).await;

    let completion_resp_impl =
        wait_for_response(client_received_responses.clone(), 3, Duration::from_secs(2)).await;
    let comp_result_impl = completion_resp_impl.get("result").unwrap();
    let comp_items_impl = match comp_result_impl {
        serde_json::Value::Array(arr) => arr.clone(),
        serde_json::Value::Object(obj) => obj.get("items").unwrap().as_array().unwrap().clone(),
        _ => panic!("Unexpected completion result shape"),
    };

    assert!(!comp_items_impl.is_empty(), "Expected completion items inside impl block");
    let has_rpc_method = comp_items_impl
        .iter()
        .any(|item| item.get("label").and_then(|l| l.as_str()) == Some("textDocument/completion"));
    assert!(has_rpc_method, "Should suggest 'textDocument/completion' as a method name in impl block");

    // Shutdown client/server
    let shutdown_req = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 4,
        "method": "shutdown",
        "params": null
    });
    write_msg(&client_tx_shared, shutdown_req).await;
    let _ = wait_for_response(client_received_responses.clone(), 4, Duration::from_secs(2)).await;

    let exit_ntf = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "exit",
        "params": null
    });
    write_msg(&client_tx_shared, exit_ntf).await;

    client_tx_shared.lock().await.take();
    let _ = client_reader_task.await;
    let _ = server_handle.await;
}

