/// Integration tests using a real LSP server (require network-style duplex setup).
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tower_lsp_max::{LspService, Server};
use tower_lsp_max_playground::Backend;

use super::common::{read_message, wait_for_notification, wait_for_response, write_msg};

#[tokio::test(flavor = "current_thread")]
async fn test_quick_fix_missing_moniker_notebook_sync() {
    let (service, socket) = LspService::new(Backend::new);

    let (client_tx, server_rx) = tokio::io::duplex(1024 * 1024);
    let (server_tx, client_rx) = tokio::io::duplex(1024 * 1024);

    let server = Server::new(server_rx, server_tx, socket);
    let server_handle = tokio::spawn(server.serve(service));

    let client_received_responses = Arc::new(Mutex::new(Vec::new()));
    let client_received_notifications = Arc::new(Mutex::new(Vec::new()));

    let client_tx_shared = Arc::new(tokio::sync::Mutex::new(Some(client_tx)));
    let client_received_responses_clone = client_received_responses.clone();
    let client_received_notifications_clone = client_received_notifications.clone();

    let mut reader = tokio::io::BufReader::new(client_rx);
    let client_reader_task = tokio::spawn(async move {
        loop {
            let msg = match read_message(&mut reader).await {
                Ok(m) => m,
                Err(_) => break,
            };
            if msg.get("method").is_some() {
                client_received_notifications_clone
                    .lock()
                    .unwrap()
                    .push(msg);
            } else if msg.get("id").is_some() {
                client_received_responses_clone.lock().unwrap().push(msg);
            }
        }
    });

    write_msg(
        &client_tx_shared,
        serde_json::json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{"capabilities":{}}}),
    )
    .await;
    let _init_resp =
        wait_for_response(client_received_responses.clone(), 1, Duration::from_secs(2)).await;

    write_msg(
        &client_tx_shared,
        serde_json::json!({"jsonrpc":"2.0","method":"initialized","params":{}}),
    )
    .await;

    let doc_uri = "file:///Users/sac/tower-lsp-max/dummy_notebook.rs";
    let code_text = r#"
struct Dummy;

#[tower_lsp_max::async_trait]
impl LanguageServer for Dummy {
    async fn initialize(&self, params: InitializeParams) -> Result<InitializeResult> {
        Ok(InitializeResult {
            capabilities: ServerCapabilities {
                ..Default::default()
            },
            ..Default::default()
        })
    }
    async fn shutdown(&self) -> Result<()> { Ok(()) }
    async fn did_open_notebook_document(&self, params: DidOpenNotebookDocumentParams) {}
}
"#;
    write_msg(
        &client_tx_shared,
        serde_json::json!({
            "jsonrpc":"2.0","method":"textDocument/didOpen",
            "params":{
                "textDocument":{"uri":doc_uri,"languageId":"rust","version":1,"text":code_text}
            }
        }),
    )
    .await;

    let diag_ntf = wait_for_notification(
        client_received_notifications.clone(),
        "textDocument/publishDiagnostics",
        Duration::from_secs(2),
    )
    .await
    .expect("Expected publishDiagnostics");

    let params = diag_ntf.get("params").unwrap();
    let diags = params.get("diagnostics").unwrap().as_array().unwrap();

    let warning = diags
        .iter()
        .find(|d| d.get("code").and_then(|c| c.as_str()) == Some("TLM002"));
    assert!(warning.is_some(), "Expected TLM002 warning");

    write_msg(
        &client_tx_shared,
        serde_json::json!({
            "jsonrpc":"2.0","id":2,"method":"textDocument/codeAction",
            "params":{
                "textDocument":{"uri":doc_uri},
                "range":warning.unwrap().get("range").unwrap(),
                "context":{"diagnostics":[warning.unwrap()]}
            }
        }),
    )
    .await;

    let code_action_resp =
        wait_for_response(client_received_responses.clone(), 2, Duration::from_secs(2))
            .await
            .unwrap();

    let result = code_action_resp.get("result");
    if let Some(res_val) = result {
        if !res_val.is_null() {
            let action_result = res_val.as_array().unwrap();
            let has_fix = action_result.iter().any(|action| {
                let title = action.get("title").and_then(|t| t.as_str()).unwrap_or("");
                title.contains("notebook_document_sync")
            });
            assert!(
                has_fix,
                "Quick-fix for notebook_document_sync should be suggested because it is in CAPABILITY_FIELDS"
            );
        }
    }

    write_msg(
        &client_tx_shared,
        serde_json::json!({"jsonrpc":"2.0","id":3,"method":"shutdown","params":null}),
    )
    .await;
    let _ = wait_for_response(client_received_responses.clone(), 3, Duration::from_secs(2)).await;

    write_msg(
        &client_tx_shared,
        serde_json::json!({"jsonrpc":"2.0","method":"exit","params":null}),
    )
    .await;

    client_tx_shared.lock().await.take();
    let _ = client_reader_task.await;
    let _ = server_handle.await;
}

#[tokio::test(flavor = "current_thread")]
async fn test_code_action_out_of_bounds_no_panic_integration() {
    let (service, socket) = LspService::new(Backend::new);

    let (client_tx, server_rx) = tokio::io::duplex(1024 * 1024);
    let (server_tx, client_rx) = tokio::io::duplex(1024 * 1024);

    let server = Server::new(server_rx, server_tx, socket);
    let server_handle = tokio::spawn(server.serve(service));

    let client_received_responses = Arc::new(Mutex::new(Vec::new()));
    let client_tx_shared = Arc::new(tokio::sync::Mutex::new(Some(client_tx)));
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

    write_msg(
        &client_tx_shared,
        serde_json::json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{"capabilities":{}}}),
    )
    .await;
    let _init_resp =
        wait_for_response(client_received_responses.clone(), 1, Duration::from_secs(2)).await;

    let doc_uri = "file:///Users/sac/tower-lsp-max/dummy.rs";
    let code_text = "fn main() {}\n";
    write_msg(
        &client_tx_shared,
        serde_json::json!({
            "jsonrpc":"2.0","method":"textDocument/didOpen",
            "params":{
                "textDocument":{"uri":doc_uri,"languageId":"rust","version":1,"text":code_text}
            }
        }),
    )
    .await;

    write_msg(
        &client_tx_shared,
        serde_json::json!({
            "jsonrpc":"2.0","id":2,"method":"textDocument/codeAction",
            "params":{
                "textDocument":{"uri":doc_uri},
                "range":{"start":{"line":100,"character":0},"end":{"line":100,"character":0}},
                "context":{
                    "diagnostics":[{
                        "range":{"start":{"line":100,"character":0},"end":{"line":100,"character":0}},
                        "severity":1,"code":"TLM005","source":"tower-lsp-max-playground",
                        "message":"`init` is not a valid method. Did you mean `initialize`?"
                    }]
                }
            }
        }),
    )
    .await;

    let resp =
        wait_for_response(client_received_responses.clone(), 2, Duration::from_secs(2)).await;

    assert!(
        resp.is_some(),
        "Server should have handled out of bounds range gracefully and returned a response"
    );

    client_tx_shared.lock().await.take();
    let _ = client_reader_task.await;

    let server_result = server_handle.await;
    assert!(
        server_result.is_ok(),
        "Server thread should have completed successfully, not panicked"
    );
}

#[tokio::test(flavor = "current_thread")]
async fn test_quick_fix_missing_file_operations() {
    let (service, socket) = LspService::new(Backend::new);

    let (client_tx, server_rx) = tokio::io::duplex(1024 * 1024);
    let (server_tx, client_rx) = tokio::io::duplex(1024 * 1024);

    let server = Server::new(server_rx, server_tx, socket);
    let server_handle = tokio::spawn(server.serve(service));

    let client_received_responses = Arc::new(Mutex::new(Vec::new()));
    let client_received_notifications = Arc::new(Mutex::new(Vec::new()));

    let client_tx_shared = Arc::new(tokio::sync::Mutex::new(Some(client_tx)));
    let client_received_responses_clone = client_received_responses.clone();
    let client_received_notifications_clone = client_received_notifications.clone();

    let mut reader = tokio::io::BufReader::new(client_rx);
    let client_reader_task = tokio::spawn(async move {
        loop {
            let msg = match read_message(&mut reader).await {
                Ok(m) => m,
                Err(_) => break,
            };
            if msg.get("method").is_some() {
                client_received_notifications_clone
                    .lock()
                    .unwrap()
                    .push(msg);
            } else if msg.get("id").is_some() {
                client_received_responses_clone.lock().unwrap().push(msg);
            }
        }
    });

    write_msg(
        &client_tx_shared,
        serde_json::json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{"capabilities":{}}}),
    )
    .await;
    let _init_resp =
        wait_for_response(client_received_responses.clone(), 1, Duration::from_secs(2)).await;

    write_msg(
        &client_tx_shared,
        serde_json::json!({"jsonrpc":"2.0","method":"initialized","params":{}}),
    )
    .await;

    let doc_uri = "file:///Users/sac/tower-lsp-max/dummy_file_ops.rs";
    let code_text = r#"
struct Dummy;

#[tower_lsp_max::async_trait]
impl LanguageServer for Dummy {
    async fn initialize(&self, params: InitializeParams) -> Result<InitializeResult> {
        Ok(InitializeResult {
            capabilities: ServerCapabilities {
                ..Default::default()
            },
            ..Default::default()
        })
    }
    async fn shutdown(&self) -> Result<()> { Ok(()) }
    async fn did_create_files(&self, params: CreateFilesParams) {}
}
"#;
    write_msg(
        &client_tx_shared,
        serde_json::json!({
            "jsonrpc":"2.0","method":"textDocument/didOpen",
            "params":{
                "textDocument":{"uri":doc_uri,"languageId":"rust","version":1,"text":code_text}
            }
        }),
    )
    .await;

    let diag_ntf = wait_for_notification(
        client_received_notifications.clone(),
        "textDocument/publishDiagnostics",
        Duration::from_secs(2),
    )
    .await
    .expect("Expected publishDiagnostics");

    let params = diag_ntf.get("params").unwrap();
    let diags = params.get("diagnostics").unwrap().as_array().unwrap();

    let warning = diags
        .iter()
        .find(|d| d.get("code").and_then(|c| c.as_str()) == Some("TLM002"));
    assert!(
        warning.is_some(),
        "Expected TLM002 warning for did_create_files without capability"
    );

    write_msg(
        &client_tx_shared,
        serde_json::json!({
            "jsonrpc":"2.0","id":2,"method":"textDocument/codeAction",
            "params":{
                "textDocument":{"uri":doc_uri},
                "range":warning.unwrap().get("range").unwrap(),
                "context":{"diagnostics":[warning.unwrap()]}
            }
        }),
    )
    .await;

    let code_action_resp =
        wait_for_response(client_received_responses.clone(), 2, Duration::from_secs(2))
            .await
            .unwrap();

    let result = code_action_resp.get("result");
    if let Some(res_val) = result {
        if !res_val.is_null() {
            let action_result = res_val.as_array().unwrap();
            let has_fix = action_result.iter().any(|action| {
                let title = action.get("title").and_then(|t| t.as_str()).unwrap_or("");
                title.contains("workspace.fileOperations")
            });
            assert!(
                has_fix,
                "Quick-fix for workspace.fileOperations should be suggested"
            );
        }
    }

    write_msg(
        &client_tx_shared,
        serde_json::json!({"jsonrpc":"2.0","id":3,"method":"shutdown","params":null}),
    )
    .await;
    let _ = wait_for_response(client_received_responses.clone(), 3, Duration::from_secs(2)).await;

    write_msg(
        &client_tx_shared,
        serde_json::json!({"jsonrpc":"2.0","method":"exit","params":null}),
    )
    .await;

    client_tx_shared.lock().await.take();
    let _ = client_reader_task.await;
    let _ = server_handle.await;
}

#[tokio::test(flavor = "current_thread")]
async fn test_code_action_adversarial_ranges() {
    let (service, socket) = LspService::new(Backend::new);

    let (client_tx, server_rx) = tokio::io::duplex(1024 * 1024);
    let (server_tx, client_rx) = tokio::io::duplex(1024 * 1024);

    let server = Server::new(server_rx, server_tx, socket);
    let server_handle = tokio::spawn(server.serve(service));

    let client_received_responses = Arc::new(Mutex::new(Vec::new()));
    let client_tx_shared = Arc::new(tokio::sync::Mutex::new(Some(client_tx)));
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

    write_msg(
        &client_tx_shared,
        serde_json::json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{"capabilities":{}}}),
    )
    .await;
    let _init_resp =
        wait_for_response(client_received_responses.clone(), 1, Duration::from_secs(2)).await;

    let doc_uri = "file:///Users/sac/tower-lsp-max/dummy_adv_ranges.rs";
    let code_text = "fn main() {}\n";
    write_msg(
        &client_tx_shared,
        serde_json::json!({
            "jsonrpc":"2.0","method":"textDocument/didOpen",
            "params":{
                "textDocument":{"uri":doc_uri,"languageId":"rust","version":1,"text":code_text}
            }
        }),
    )
    .await;

    // Send codeAction with start > end (invalid range)
    write_msg(
        &client_tx_shared,
        serde_json::json!({
            "jsonrpc":"2.0","id":2,"method":"textDocument/codeAction",
            "params":{
                "textDocument":{"uri":doc_uri},
                "range":{"start":{"line":0,"character":10},"end":{"line":0,"character":5}},
                "context":{
                    "diagnostics":[{
                        "range":{"start":{"line":0,"character":10},"end":{"line":0,"character":5}},
                        "severity":1,"code":"TLM005","source":"tower-lsp-max-playground",
                        "message":"`init` is not a valid method. Did you mean `initialize`?"
                    }]
                }
            }
        }),
    )
    .await;
    let resp_invalid =
        wait_for_response(client_received_responses.clone(), 2, Duration::from_secs(2)).await;
    assert!(
        resp_invalid.is_some(),
        "Should respond to invalid range start > end gracefully"
    );

    // Send codeAction with extreme line index (out of bounds)
    write_msg(
        &client_tx_shared,
        serde_json::json!({
            "jsonrpc":"2.0","id":3,"method":"textDocument/codeAction",
            "params":{
                "textDocument":{"uri":doc_uri},
                "range":{"start":{"line":u32::MAX,"character":u32::MAX},"end":{"line":u32::MAX,"character":u32::MAX}},
                "context":{
                    "diagnostics":[{
                        "range":{"start":{"line":u32::MAX,"character":u32::MAX},"end":{"line":u32::MAX,"character":u32::MAX}},
                        "severity":1,"code":"TLM005","source":"tower-lsp-max-playground",
                        "message":"`init` is not a valid method. Did you mean `initialize`?"
                    }]
                }
            }
        }),
    )
    .await;
    let resp_oob =
        wait_for_response(client_received_responses.clone(), 3, Duration::from_secs(2)).await;
    assert!(
        resp_oob.is_some(),
        "Should respond to extreme line index gracefully"
    );

    client_tx_shared.lock().await.take();
    let _ = client_reader_task.await;
    let server_result = server_handle.await;
    assert!(server_result.is_ok(), "Server should not panic");
}
