use lsp_max::{LspService, Server};
use lsp_max_playground::Backend;
/// RPC integration tests: rpc_attribute_diagnostics, rpc_completion.
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::io::AsyncWriteExt;

use super::common::{
    encode_message, read_message, wait_for_notification, wait_for_response, write_msg,
};

#[tokio::test(flavor = "current_thread")]
async fn test_rpc_attribute_diagnostics() {
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
                    let resp = serde_json::json!({"jsonrpc":"2.0","id":id,"result":null});
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

    let doc_uri = "file:///Users/sac/lsp-max/test_rpc.rs";
    let code_text = r#"
#[rpc]
pub trait TestServer {
    #[rpc(name = "textDocument/didchange")]
    async fn did_change(&self, params: DidChangeTextDocumentParams);
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
    .await;

    let params = diag_ntf.get("params").unwrap();
    let diags = params.get("diagnostics").unwrap().as_array().unwrap();

    let mut invalid_rpc_diag = None;
    for diag in diags {
        let code = diag.get("code").and_then(|c| c.as_str());
        if code == Some("TLM006") {
            invalid_rpc_diag = Some(diag.clone());
        }
    }
    let invalid_rpc_diag = invalid_rpc_diag.expect("Expected TLM006 diagnostic code");

    let message = invalid_rpc_diag.get("message").unwrap().as_str().unwrap();
    assert!(
        message.contains("Did you mean 'textDocument/didChange'?"),
        "Message was: {}",
        message
    );

    write_msg(
        &client_tx_shared,
        serde_json::json!({
            "jsonrpc":"2.0","id":2,"method":"textDocument/codeAction",
            "params":{
                "textDocument":{"uri":doc_uri},
                "range":invalid_rpc_diag.get("range").unwrap(),
                "context":{"diagnostics":[invalid_rpc_diag]}
            }
        }),
    )
    .await;

    let code_action_resp =
        wait_for_response(client_received_responses.clone(), 2, Duration::from_secs(2)).await;
    let action_result = code_action_resp.get("result").unwrap().as_array().unwrap();

    assert!(!action_result.is_empty(), "Expected code actions, got none");
    let has_fix_action = action_result.iter().any(|action| {
        let title = action.get("title").and_then(|t| t.as_str());
        title.is_some_and(|t| t.contains("Correct RPC name to `textDocument/didChange`"))
    });
    assert!(
        has_fix_action,
        "Expected quickfix action to correct rpc name"
    );

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
async fn test_rpc_completion() {
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

    write_msg(
        &client_tx_shared,
        serde_json::json!({"jsonrpc":"2.0","method":"initialized","params":{}}),
    )
    .await;

    let doc_uri = "file:///Users/sac/lsp-max/test_completion.rs";
    let code_text = r#"
#[rpc]
pub trait TestServer {
    #[rpc(name = "textDoc
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

    write_msg(
        &client_tx_shared,
        serde_json::json!({
            "jsonrpc":"2.0","id":2,"method":"textDocument/completion",
            "params":{
                "textDocument":{"uri":doc_uri},
                "position":{"line":3,"character":25},
                "context":{"triggerKind":1}
            }
        }),
    )
    .await;

    let completion_resp =
        wait_for_response(client_received_responses.clone(), 2, Duration::from_secs(2)).await;
    let comp_result = completion_resp.get("result").unwrap();
    let comp_items = match comp_result {
        serde_json::Value::Array(arr) => arr.clone(),
        serde_json::Value::Object(obj) => obj.get("items").unwrap().as_array().unwrap().clone(),
        _ => panic!("Unexpected completion result shape"),
    };

    assert!(
        !comp_items.is_empty(),
        "Expected completion items in rpc name block"
    );
    let has_completion = comp_items
        .iter()
        .any(|item| item.get("label").and_then(|l| l.as_str()) == Some("textDocument/completion"));
    assert!(has_completion, "Should suggest 'textDocument/completion'");

    let code_text_impl = r#"
impl LanguageServer for Backend {
    textDoc
}
"#;
    write_msg(
        &client_tx_shared,
        serde_json::json!({
            "jsonrpc":"2.0","method":"textDocument/didChange",
            "params":{
                "textDocument":{"uri":doc_uri,"version":2},
                "contentChanges":[{"text":code_text_impl}]
            }
        }),
    )
    .await;

    write_msg(
        &client_tx_shared,
        serde_json::json!({
            "jsonrpc":"2.0","id":3,"method":"textDocument/completion",
            "params":{
                "textDocument":{"uri":doc_uri},
                "position":{"line":2,"character":11},
                "context":{"triggerKind":1}
            }
        }),
    )
    .await;

    let completion_resp_impl =
        wait_for_response(client_received_responses.clone(), 3, Duration::from_secs(2)).await;
    let comp_result_impl = completion_resp_impl.get("result").unwrap();
    let comp_items_impl = match comp_result_impl {
        serde_json::Value::Array(arr) => arr.clone(),
        serde_json::Value::Object(obj) => obj.get("items").unwrap().as_array().unwrap().clone(),
        _ => panic!("Unexpected completion result shape"),
    };

    assert!(
        !comp_items_impl.is_empty(),
        "Expected completion items inside impl block"
    );
    let has_rpc_method = comp_items_impl
        .iter()
        .any(|item| item.get("label").and_then(|l| l.as_str()) == Some("textDocument/completion"));
    assert!(
        has_rpc_method,
        "Should suggest 'textDocument/completion' as a method name in impl block"
    );

    write_msg(
        &client_tx_shared,
        serde_json::json!({"jsonrpc":"2.0","id":4,"method":"shutdown","params":null}),
    )
    .await;
    let _ = wait_for_response(client_received_responses.clone(), 4, Duration::from_secs(2)).await;

    write_msg(
        &client_tx_shared,
        serde_json::json!({"jsonrpc":"2.0","method":"exit","params":null}),
    )
    .await;

    client_tx_shared.lock().await.take();
    let _ = client_reader_task.await;
    let _ = server_handle.await;
}
