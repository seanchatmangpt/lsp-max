/// test_playground_integration — full playground loopback integration test.
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::io::AsyncWriteExt;
use tower_lsp_max::{LspService, Server};
use tower_lsp_max_playground::Backend;

use super::common::{
    encode_message, read_message, wait_for_notification, wait_for_response, write_msg,
};

#[tokio::test(flavor = "current_thread")]
async fn test_playground_integration() {
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
    let _client_reader_task = tokio::spawn(async move {
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

    let init_resp =
        wait_for_response(client_received_responses.clone(), 1, Duration::from_secs(2)).await;
    assert!(init_resp.get("result").is_some());

    write_msg(
        &client_tx_shared,
        serde_json::json!({"jsonrpc":"2.0","method":"initialized","params":{}}),
    )
    .await;

    let doc_uri = "file:///Users/sac/tower-lsp-max/dummy_server.rs";
    let code_text = r#"
struct DummyServer;

#[tower_lsp_max::async_trait]
impl LanguageServer for DummyServer {
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

    let params = diag_ntf
        .get("params")
        .expect("Missing params in publishDiagnostics");
    assert_eq!(params.get("uri").and_then(|u| u.as_str()), Some(doc_uri));
    let diags = params
        .get("diagnostics")
        .expect("Missing diagnostics array")
        .as_array()
        .expect("diagnostics is not an array");

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

    let code_text_with_i = r#"
struct DummyServer;

#[tower_lsp_max::async_trait]
impl LanguageServer for DummyServer {
    i
}
"#;
    write_msg(
        &client_tx_shared,
        serde_json::json!({
            "jsonrpc":"2.0","method":"textDocument/didChange",
            "params":{
                "textDocument":{"uri":doc_uri,"version":2},
                "contentChanges":[{"text":code_text_with_i}]
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
                "position":{"line":5,"character":5},
                "context":{"triggerKind":1}
            }
        }),
    )
    .await;

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

    write_msg(
        &client_tx_shared,
        serde_json::json!({
            "jsonrpc":"2.0","id":3,"method":"textDocument/codeAction",
            "params":{
                "textDocument":{"uri":doc_uri},
                "range":{"start":{"line":0,"character":0},"end":{"line":0,"character":0}},
                "context":{"diagnostics":[missing_mandatory_diag]}
            }
        }),
    )
    .await;

    let code_action_resp =
        wait_for_response(client_received_responses.clone(), 3, Duration::from_secs(2)).await;
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
    let _ = server_handle.await;
}
