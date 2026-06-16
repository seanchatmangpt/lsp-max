use lsp_max::lsp_types as lsp;
use lsp_max::{ExitedError, LspService, Server};
/// test_dogfood_loopback_integration — full loopback integration test.
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::io::AsyncWriteExt;

use super::backend::{DogfoodBackend, TestEvents};
use super::helpers::{
    encode_message, read_message, wait_for_notification, wait_for_response, write_msg,
};

#[tokio::test(flavor = "current_thread")]
async fn test_dogfood_loopback_integration() {
    let events = TestEvents::default();
    let events_clone = events.clone();

    let (service, socket) = LspService::new(|client| DogfoodBackend {
        client,
        events: events_clone,
    });

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
                Ok(m) => {
                    println!("Client reader read: {:?}", m);
                    m
                }
                Err(e) => {
                    println!("Client reader EOF/error: {:?}", e);
                    break;
                }
            };

            if let Some(method) = msg.get("method").and_then(|m| m.as_str()) {
                if let Some(id) = msg.get("id") {
                    println!(
                        "Client reader: handling request '{}' with ID {:?}",
                        method, id
                    );
                    let resp = serde_json::json!({"jsonrpc":"2.0","id":id,"result":null});
                    let encoded = encode_message(&resp);
                    let mut writer_guard = client_tx_shared_clone.lock().await;
                    if let Some(ref mut tx) = *writer_guard {
                        tx.write_all(&encoded).await.unwrap();
                    }
                } else {
                    println!("Client reader: saving notification '{}'", method);
                    client_received_notifications_clone
                        .lock()
                        .unwrap()
                        .push(msg);
                }
            } else if msg.get("id").is_some() {
                println!("Client reader: saving response with ID {:?}", msg.get("id"));
                client_received_responses_clone.lock().unwrap().push(msg);
            }
        }
    });

    println!("Client: sending initialize");
    write_msg(
        &client_tx_shared,
        serde_json::json!({
            "jsonrpc":"2.0","id":1,"method":"initialize","params":{"capabilities":{}}
        }),
    )
    .await;

    let init_resp =
        wait_for_response(client_received_responses.clone(), 1, Duration::from_millis(200)).await;
    println!("Client: received initialize response: {:?}", init_resp);
    assert!(
        init_resp.get("result").is_some(),
        "Initialization failed: {:?}",
        init_resp
    );

    println!("Client: sending initialized");
    write_msg(
        &client_tx_shared,
        serde_json::json!({"jsonrpc":"2.0","method":"initialized","params":{}}),
    )
    .await;

    // textDocument/inlineCompletion
    println!("Client: sending inlineCompletion");
    write_msg(
        &client_tx_shared,
        serde_json::json!({
            "jsonrpc":"2.0","id":2,"method":"textDocument/inlineCompletion",
            "params":{
                "textDocument":{"uri":"file:///dogfood.rs"},
                "position":{"line":0,"character":0},
                "context":{"triggerKind":1}
            }
        }),
    )
    .await;

    let inline_resp =
        wait_for_response(client_received_responses.clone(), 2, Duration::from_millis(200)).await;
    println!(
        "Client: received inlineCompletion response: {:?}",
        inline_resp
    );
    let inline_result = inline_resp.get("result").unwrap();
    assert_eq!(
        inline_result
            .get("items")
            .unwrap()
            .get(0)
            .unwrap()
            .get("insertText")
            .unwrap()
            .as_str(),
        Some("dogfood_inline_completion_text")
    );

    // workspace/textDocumentContent
    println!("Client: sending textDocumentContent");
    write_msg(
        &client_tx_shared,
        serde_json::json!({
            "jsonrpc":"2.0","id":3,"method":"workspace/textDocumentContent",
            "params":{"textDocument":{"uri":"file:///dogfood.rs"}}
        }),
    )
    .await;

    let content_resp =
        wait_for_response(client_received_responses.clone(), 3, Duration::from_millis(200)).await;
    let content_result = content_resp.get("result").unwrap();
    assert_eq!(
        content_result.get("text").unwrap().as_str(),
        Some("dogfood document content text")
    );

    // textDocument/rangesFormatting
    println!("Client: sending rangesFormatting");
    write_msg(
        &client_tx_shared,
        serde_json::json!({
            "jsonrpc":"2.0","id":4,"method":"textDocument/rangesFormatting",
            "params":{
                "textDocument":{"uri":"file:///dogfood.rs"},
                "ranges":[{"start":{"line":0,"character":0},"end":{"line":0,"character":10}}],
                "options":{"tabSize":4,"insertSpaces":true}
            }
        }),
    )
    .await;

    let format_resp =
        wait_for_response(client_received_responses.clone(), 4, Duration::from_millis(200)).await;
    let format_result = format_resp.get("result").unwrap();
    assert_eq!(
        format_result
            .get(0)
            .unwrap()
            .get("newText")
            .unwrap()
            .as_str(),
        Some("formatted_dogfood_ranges")
    );

    // Notifications
    println!("Client: sending didOpen notebook");
    write_msg(
        &client_tx_shared,
        serde_json::json!({
            "jsonrpc":"2.0","method":"notebookDocument/didOpen",
            "params":{
                "notebookDocument":{
                    "uri":"file:///notebook.ipynb","notebookType":"jupyter",
                    "version":1,"metadata":{},"cells":[]
                },
                "cellTextDocuments":[]
            }
        }),
    )
    .await;

    println!("Client: sending $/setTrace");
    write_msg(
        &client_tx_shared,
        serde_json::json!({"jsonrpc":"2.0","method":"$/setTrace","params":{"value":"verbose"}}),
    )
    .await;

    println!("Client: sending $/progress");
    write_msg(
        &client_tx_shared,
        serde_json::json!({
            "jsonrpc":"2.0","method":"$/progress",
            "params":{"token":"test-progress-token","value":{"kind":"begin","title":"indexing"}}
        }),
    )
    .await;

    println!("Client: sending workDoneProgress/cancel");
    write_msg(
        &client_tx_shared,
        serde_json::json!({
            "jsonrpc":"2.0","method":"window/workDoneProgress/cancel",
            "params":{"token":"test-progress-token"}
        }),
    )
    .await;

    println!("Client: verifying notifications received by server");
    let start_verify = std::time::Instant::now();
    loop {
        if start_verify.elapsed() > Duration::from_millis(300) {
            panic!("Timeout waiting for server to receive all 4 notifications");
        }
        let did_open = events.did_open_notebook.lock().unwrap().is_some();
        let set_trace = events.set_trace.lock().unwrap().is_some();
        let progress = events.progress.lock().unwrap().is_some();
        let cancel = events.work_done_cancel.lock().unwrap().is_some();
        if did_open && set_trace && progress && cancel {
            break;
        }
        tokio::time::sleep(Duration::from_millis(1)).await;
    }

    {
        let did_open = events.did_open_notebook.lock().unwrap().take().unwrap();
        assert_eq!(
            did_open.notebook_document.uri.as_str(),
            "file:///notebook.ipynb"
        );
        assert_eq!(did_open.notebook_document.notebook_type, "jupyter");

        let set_trace = events.set_trace.lock().unwrap().take().unwrap();
        assert_eq!(set_trace.value, lsp::TraceValue::Verbose);

        let progress = events.progress.lock().unwrap().take().unwrap();
        assert_eq!(
            progress.token,
            lsp::NumberOrString::String("test-progress-token".to_string())
        );

        let cancel = events.work_done_cancel.lock().unwrap().take().unwrap();
        assert_eq!(
            cancel.token,
            lsp::NumberOrString::String("test-progress-token".to_string())
        );
    }

    println!("Client: waiting for $/logTrace notification");
    let log_trace = wait_for_notification(
        client_received_notifications.clone(),
        "$/logTrace",
        Duration::from_millis(200),
    )
    .await;
    assert_eq!(
        log_trace
            .get("params")
            .unwrap()
            .get("message")
            .unwrap()
            .as_str(),
        Some("dogfood log trace message")
    );
    assert_eq!(
        log_trace
            .get("params")
            .unwrap()
            .get("verbose")
            .unwrap()
            .as_str(),
        Some("dogfood log trace verbose content")
    );

    // max/verifyLedger
    println!("Client: sending max/verifyLedger");
    write_msg(
        &client_tx_shared,
        serde_json::json!({"jsonrpc":"2.0","id":10,"method":"max/verifyLedger","params":null}),
    )
    .await;
    let verify_resp = wait_for_response(
        client_received_responses.clone(),
        10,
        Duration::from_millis(200),
    )
    .await;
    assert!(
        verify_resp.get("error").is_none(),
        "Ledger verification failed: {:?}",
        verify_resp
    );

    println!("Client: sending max/ledgerReport");
    write_msg(
        &client_tx_shared,
        serde_json::json!({"jsonrpc":"2.0","id":11,"method":"max/ledgerReport","params":null}),
    )
    .await;
    let report_resp = wait_for_response(
        client_received_responses.clone(),
        11,
        Duration::from_millis(200),
    )
    .await;
    let report_str = report_resp.get("result").unwrap().as_str().unwrap();
    assert!(
        report_str.contains("Status: VERIFIED"),
        "Report did not contain VERIFIED status"
    );
    assert!(
        report_str.contains("Active Phase: Initialized"),
        "Report active phase mismatch"
    );
    assert!(
        report_str.contains("Receipts count: 3"),
        "Report receipts count mismatch"
    );
    assert!(
        report_str.contains("rcpt-uninitialized"),
        "Report did not contain rcpt-uninitialized"
    );

    println!("Client: sending shutdown");
    write_msg(
        &client_tx_shared,
        serde_json::json!({"jsonrpc":"2.0","id":5,"method":"shutdown"}),
    )
    .await;

    let shutdown_resp =
        wait_for_response(client_received_responses.clone(), 5, Duration::from_millis(200)).await;
    assert!(shutdown_resp.get("result").is_some());

    println!("Client: sending exit");
    write_msg(
        &client_tx_shared,
        serde_json::json!({"jsonrpc":"2.0","method":"exit"}),
    )
    .await;

    {
        let mut guard = client_tx_shared.lock().await;
        if let Some(ref mut tx) = *guard {
            tx.shutdown().await.unwrap();
        }
    }

    println!("Client: waiting for server to exit");
    let serve_res = server_handle.await.unwrap();
    assert!(
        matches!(serve_res, Err(ExitedError(0))),
        "Server did not exit with ExitedError(0): {:?}",
        serve_res
    );

    let _ = client_reader_task.await;
    println!("Client: test finished successfully");
}
