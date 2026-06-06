use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tower_lsp_max::jsonrpc::Result;
use tower_lsp_max::lsp_types as lsp;
use tower_lsp_max::max_protocol::lsp_3_18 as lsp318;
use tower_lsp_max::{Client, ExitedError, LanguageServer, LspService, Server};

#[derive(Clone, Default)]
struct TestEvents {
    did_open_notebook: Arc<Mutex<Option<lsp::DidOpenNotebookDocumentParams>>>,
    set_trace: Arc<Mutex<Option<lsp::SetTraceParams>>>,
    progress: Arc<Mutex<Option<lsp::ProgressParams>>>,
    work_done_cancel: Arc<Mutex<Option<lsp::WorkDoneProgressCancelParams>>>,
}

struct DogfoodBackend {
    client: Client,
    events: TestEvents,
}

#[tower_lsp_max::async_trait]
impl LanguageServer for DogfoodBackend {
    async fn initialize(&self, _: lsp::InitializeParams) -> Result<lsp::InitializeResult> {
        println!("Server: initialize called");
        Ok(lsp::InitializeResult::default())
    }

    async fn initialized(&self, _: lsp::InitializedParams) {
        println!("Server: initialized called");
        let client = self.client.clone();
        tokio::spawn(async move {
            println!("Server task: starting sleep");
            tokio::time::sleep(Duration::from_millis(50)).await;

            println!("Server task: sending work_done_progress_create");
            let create_res = client
                .work_done_progress_create(lsp::WorkDoneProgressCreateParams {
                    token: lsp::NumberOrString::String("dogfood-progress-token".to_string()),
                })
                .await;
            println!(
                "Server task: work_done_progress_create res = {:?}",
                create_res
            );
            assert!(
                create_res.is_ok(),
                "Server failed to send window/workDoneProgress/create request: {:?}",
                create_res
            );

            println!("Server task: sending folding_range_refresh");
            let refresh_res = client.folding_range_refresh().await;
            println!("Server task: folding_range_refresh res = {:?}", refresh_res);
            assert!(
                refresh_res.is_ok(),
                "Server failed to send workspace/foldingRange/refresh request: {:?}",
                refresh_res
            );

            println!("Server task: sending text_document_content_refresh");
            let refresh_content_res = client
                .text_document_content_refresh(lsp318::TextDocumentContentRefreshParams {
                    uri: "file:///dogfood.rs".to_string(),
                })
                .await;
            println!(
                "Server task: text_document_content_refresh res = {:?}",
                refresh_content_res
            );
            assert!(
                refresh_content_res.is_ok(),
                "Server failed to send workspace/textDocumentContent/refresh request: {:?}",
                refresh_content_res
            );

            println!("Server task: sending log_trace");
            client
                .log_trace(lsp::LogTraceParams {
                    message: "dogfood log trace message".to_string(),
                    verbose: Some("dogfood log trace verbose content".to_string()),
                })
                .await;
            println!("Server task: log_trace completed");
        });
    }

    async fn shutdown(&self) -> Result<()> {
        println!("Server: shutdown called");
        Ok(())
    }

    async fn inline_completion(
        &self,
        params: lsp::InlineCompletionParams,
    ) -> Result<Option<lsp::InlineCompletionResponse>> {
        println!("Server: inline_completion called");
        assert_eq!(
            params.text_document_position.text_document.uri.as_str(),
            "file:///dogfood.rs"
        );
        Ok(Some(lsp::InlineCompletionResponse::List(
            lsp::InlineCompletionList {
                items: vec![lsp::InlineCompletionItem {
                    insert_text: lsp::StringOrStringValue::String("dogfood_inline_completion_text".to_string()),
                    filter_text: None,
                    range: None,
                    command: None,
                    insert_text_format: None,
                }],
            },
        )))
    }

    async fn text_document_content(
        &self,
        params: lsp318::TextDocumentContentParams,
    ) -> Result<lsp318::TextDocumentContentResult> {
        println!("Server: text_document_content called");
        assert_eq!(params.uri.as_str(), "file:///dogfood.rs");
        Ok(lsp318::TextDocumentContentResult {
            text: "dogfood document content text".to_string(),
        })
    }

    async fn ranges_formatting(
        &self,
        params: lsp318::DocumentRangesFormattingParams,
    ) -> Result<Option<Vec<lsp318::TextEdit>>> {
        println!("Server: ranges_formatting called");
        assert_eq!(params.text_document.uri.as_str(), "file:///dogfood.rs");
        Ok(Some(vec![lsp318::TextEdit {
            range: lsp318::Range {
                start: lsp318::Position {
                    line: 0,
                    character: 0,
                },
                end: lsp318::Position {
                    line: 0,
                    character: 10,
                },
            },
            new_text: "formatted_dogfood_ranges".to_string(),
        }]))
    }

    async fn did_open_notebook_document(&self, params: lsp::DidOpenNotebookDocumentParams) {
        println!("Server: did_open_notebook_document called");
        *self.events.did_open_notebook.lock().unwrap() = Some(params);
    }

    async fn set_trace(&self, params: lsp::SetTraceParams) {
        println!("Server: set_trace called");
        *self.events.set_trace.lock().unwrap() = Some(params);
    }

    async fn progress(&self, params: lsp::ProgressParams) {
        println!("Server: progress called");
        *self.events.progress.lock().unwrap() = Some(params);
    }

    async fn work_done_progress_cancel(&self, params: lsp::WorkDoneProgressCancelParams) {
        println!("Server: work_done_progress_cancel called");
        *self.events.work_done_cancel.lock().unwrap() = Some(params);
    }
}

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
async fn test_dogfood_loopback_integration() {
    let events = TestEvents::default();
    let events_clone = events.clone();

    // 1. Set up the LspService
    let (service, socket) = LspService::new(|client| DogfoodBackend {
        client,
        events: events_clone,
    });

    // Set up duplex streams for bidirectional communication
    let (client_tx, server_rx) = tokio::io::duplex(1024 * 1024);
    let (server_tx, client_rx) = tokio::io::duplex(1024 * 1024);

    // Create Server using Loopback trait socket
    let server = Server::new(server_rx, server_tx, socket);

    // Spawn server to process requests asynchronously
    let server_handle = tokio::spawn(server.serve(service));

    // Spawn client reader task to handle incoming server-to-client requests/notifications
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
                    break; // EOF or stream closed
                }
            };

            if let Some(method) = msg.get("method").and_then(|m| m.as_str()) {
                if let Some(id) = msg.get("id") {
                    println!(
                        "Client reader: handling request '{}' with ID {:?}",
                        method, id
                    );
                    // Respond to server-to-client requests
                    let resp = serde_json::json!({
                        "jsonrpc": "2.0",
                        "id": id,
                        "result": null
                    });
                    let encoded = encode_message(&resp);
                    let mut writer_guard = client_tx_shared_clone.lock().await;
                    if let Some(ref mut tx) = *writer_guard {
                        tx.write_all(&encoded).await.unwrap();
                    }
                    println!(
                        "Client reader: sent response for request '{}' with ID {:?}",
                        method, id
                    );
                } else {
                    // Save notification
                    println!("Client reader: saving notification '{}'", method);
                    client_received_notifications_clone
                        .lock()
                        .unwrap()
                        .push(msg);
                }
            } else if msg.get("id").is_some() {
                // Save response
                println!("Client reader: saving response with ID {:?}", msg.get("id"));
                client_received_responses_clone.lock().unwrap().push(msg);
            }
        }
    });

    // 1. Client initializes the server.
    println!("Client: sending initialize");
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
    println!("Client: received initialize response: {:?}", init_resp);
    assert!(
        init_resp.get("result").is_some(),
        "Initialization failed: {:?}",
        init_resp
    );

    println!("Client: sending initialized");
    let init_ntf = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "initialized",
        "params": {}
    });
    write_msg(&client_tx_shared, init_ntf).await;

    // 2. Client sends LSP 3.18 requests:
    // - textDocument/inlineCompletion
    println!("Client: sending inlineCompletion");
    let inline_req = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "textDocument/inlineCompletion",
        "params": {
            "textDocument": { "uri": "file:///dogfood.rs" },
            "position": { "line": 0, "character": 0 },
            "context": { "triggerKind": 1 }
        }
    });
    write_msg(&client_tx_shared, inline_req).await;

    let inline_resp =
        wait_for_response(client_received_responses.clone(), 2, Duration::from_secs(2)).await;
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

    // - workspace/textDocumentContent
    println!("Client: sending textDocumentContent");
    let content_req = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 3,
        "method": "workspace/textDocumentContent",
        "params": {
            "uri": "file:///dogfood.rs"
        }
    });
    write_msg(&client_tx_shared, content_req).await;

    let content_resp =
        wait_for_response(client_received_responses.clone(), 3, Duration::from_secs(2)).await;
    println!(
        "Client: received textDocumentContent response: {:?}",
        content_resp
    );
    let content_result = content_resp.get("result").unwrap();
    assert_eq!(
        content_result.get("text").unwrap().as_str(),
        Some("dogfood document content text")
    );

    // - textDocument/rangesFormatting
    println!("Client: sending rangesFormatting");
    let format_req = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 4,
        "method": "textDocument/rangesFormatting",
        "params": {
            "textDocument": { "uri": "file:///dogfood.rs" },
            "ranges": [
                {
                    "start": { "line": 0, "character": 0 },
                    "end": { "line": 0, "character": 10 }
                }
            ],
            "options": {
                "tabSize": 4,
                "insertSpaces": true
            }
        }
    });
    write_msg(&client_tx_shared, format_req).await;

    let format_resp =
        wait_for_response(client_received_responses.clone(), 4, Duration::from_secs(2)).await;
    println!(
        "Client: received rangesFormatting response: {:?}",
        format_resp
    );
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

    // 3. Client sends LSP 3.18 notifications:
    // - notebookDocument/didOpen
    println!("Client: sending didOpen notebook");
    let did_open_ntf = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "notebookDocument/didOpen",
        "params": {
            "notebookDocument": {
                "uri": "file:///notebook.ipynb",
                "notebookType": "jupyter",
                "version": 1,
                "metadata": {},
                "cells": []
            },
            "cellTextDocuments": []
        }
    });
    write_msg(&client_tx_shared, did_open_ntf).await;

    // - $/setTrace
    println!("Client: sending $/setTrace");
    let set_trace_ntf = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "$/setTrace",
        "params": {
            "value": "verbose"
        }
    });
    write_msg(&client_tx_shared, set_trace_ntf).await;

    // - $/progress
    println!("Client: sending $/progress");
    let progress_ntf = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "$/progress",
        "params": {
            "token": "test-progress-token",
            "value": {
                "kind": "begin",
                "title": "indexing"
            }
        }
    });
    write_msg(&client_tx_shared, progress_ntf).await;

    // - window/workDoneProgress/cancel
    println!("Client: sending workDoneProgress/cancel");
    let cancel_ntf = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "window/workDoneProgress/cancel",
        "params": {
            "token": "test-progress-token"
        }
    });
    write_msg(&client_tx_shared, cancel_ntf).await;

    // Verify notifications parameters received by server
    println!("Client: verifying notifications received by server");
    let start_verify = std::time::Instant::now();
    loop {
        if start_verify.elapsed() > Duration::from_secs(3) {
            panic!("Timeout waiting for server to receive all 4 notifications");
        }
        let did_open = events.did_open_notebook.lock().unwrap().is_some();
        let set_trace = events.set_trace.lock().unwrap().is_some();
        let progress = events.progress.lock().unwrap().is_some();
        let cancel = events.work_done_cancel.lock().unwrap().is_some();
        if did_open && set_trace && progress && cancel {
            break;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
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
    println!("Client: server notifications verified successfully");

    // 4. Verify client received $/logTrace notification from the server
    println!("Client: waiting for $/logTrace notification");
    let log_trace = wait_for_notification(
        client_received_notifications.clone(),
        "$/logTrace",
        Duration::from_secs(2),
    )
    .await;
    println!("Client: $/logTrace notification received: {:?}", log_trace);
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

    // Verify ledger integrity via loopback RPC before shutdown
    println!("Client: sending max/verifyLedger");
    let verify_req = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 10,
        "method": "max/verifyLedger",
        "params": null
    });
    write_msg(&client_tx_shared, verify_req).await;
    let verify_resp = wait_for_response(
        client_received_responses.clone(),
        10,
        Duration::from_secs(2),
    )
    .await;
    println!(
        "Client: received max/verifyLedger response: {:?}",
        verify_resp
    );
    assert!(
        verify_resp.get("error").is_none(),
        "Ledger verification failed: {:?}",
        verify_resp
    );

    println!("Client: sending max/ledgerReport");
    let report_req = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 11,
        "method": "max/ledgerReport",
        "params": null
    });
    write_msg(&client_tx_shared, report_req).await;
    let report_resp = wait_for_response(
        client_received_responses.clone(),
        11,
        Duration::from_secs(2),
    )
    .await;
    println!(
        "Client: received max/ledgerReport response: {:?}",
        report_resp
    );
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

    // Send shutdown request (id: 5)
    println!("Client: sending shutdown");
    let shutdown_req = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 5,
        "method": "shutdown"
    });
    write_msg(&client_tx_shared, shutdown_req).await;

    let shutdown_resp =
        wait_for_response(client_received_responses.clone(), 5, Duration::from_secs(2)).await;
    println!("Client: shutdown response: {:?}", shutdown_resp);
    assert!(shutdown_resp.get("result").is_some());

    // Send exit notification
    println!("Client: sending exit");
    let exit_ntf = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "exit"
    });
    write_msg(&client_tx_shared, exit_ntf).await;

    // Shutdown client write stream to send EOF to the server
    {
        let mut guard = client_tx_shared.lock().await;
        if let Some(ref mut tx) = *guard {
            tx.shutdown().await.unwrap();
        }
    }

    // Verify server exited cleanly
    println!("Client: waiting for server to exit");
    let serve_res = server_handle.await.unwrap();
    println!("Client: server exit status = {:?}", serve_res);
    assert!(
        matches!(serve_res, Err(ExitedError(0))),
        "Server did not exit with ExitedError(0): {:?}",
        serve_res
    );

    // Cleanup and wait for client reader task to finish
    let _ = client_reader_task.await;
    println!("Client: test finished successfully");
}
