use lsp_max::jsonrpc::Result;
use lsp_max::lsp_types as lsp;
use lsp_max::{Client, ExitedError, LspService, Server};
/// Batch 9 — notebookDocument/didSave, notebookDocument/didClose,
/// window/workDoneProgress/cancel, $/progress, $/setTrace.
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::io::AsyncWriteExt;

use super::common::{encode_message, read_message};

#[derive(Clone, Default)]
struct Batch9Events {
    did_save: Arc<Mutex<Option<lsp::DidSaveNotebookDocumentParams>>>,
    did_close: Arc<Mutex<Option<lsp::DidCloseNotebookDocumentParams>>>,
    work_done_cancel: Arc<Mutex<Option<lsp::WorkDoneProgressCancelParams>>>,
    progress: Arc<Mutex<Option<lsp::ProgressParams>>>,
    set_trace: Arc<Mutex<Option<lsp::SetTraceParams>>>,
}

struct Batch9Backend {
    _client: Client,
    events: Batch9Events,
}

#[lsp_max::async_trait]
impl lsp_max::LanguageServer for Batch9Backend {
    async fn initialize(&self, _: lsp::InitializeParams) -> Result<lsp::InitializeResult> {
        Ok(lsp::InitializeResult::default())
    }
    async fn initialized(&self, _: lsp::InitializedParams) {}
    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }
    async fn did_save_notebook_document(&self, params: lsp::DidSaveNotebookDocumentParams) {
        *self.events.did_save.lock().unwrap() = Some(params);
    }
    async fn did_close_notebook_document(&self, params: lsp::DidCloseNotebookDocumentParams) {
        *self.events.did_close.lock().unwrap() = Some(params);
    }
    async fn work_done_progress_cancel(&self, params: lsp::WorkDoneProgressCancelParams) {
        *self.events.work_done_cancel.lock().unwrap() = Some(params);
    }
    async fn progress(&self, params: lsp::ProgressParams) {
        *self.events.progress.lock().unwrap() = Some(params);
    }
    async fn set_trace(&self, params: lsp::SetTraceParams) {
        *self.events.set_trace.lock().unwrap() = Some(params);
    }
}

async fn write_msg_b9(
    tx: &Arc<tokio::sync::Mutex<Option<tokio::io::DuplexStream>>>,
    msg: serde_json::Value,
) {
    let mut guard = tx.lock().await;
    if let Some(ref mut stream) = *guard {
        stream.write_all(&encode_message(&msg)).await.unwrap();
    }
}

async fn wait_for_response_b9(
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
        tokio::time::sleep(Duration::from_millis(1)).await;
    }
}

async fn wait_for_event_b9<T: Clone>(
    cell: &Arc<Mutex<Option<T>>>,
    timeout: Duration,
    label: &str,
) -> T {
    let start = std::time::Instant::now();
    loop {
        if start.elapsed() > timeout {
            panic!("Timeout waiting for event: {}", label);
        }
        if let Some(v) = cell.lock().unwrap().clone() {
            return v;
        }
        tokio::time::sleep(Duration::from_millis(1)).await;
    }
}

async fn boot_batch9() -> (
    Batch9Events,
    Arc<tokio::sync::Mutex<Option<tokio::io::DuplexStream>>>,
    Arc<Mutex<Vec<serde_json::Value>>>,
    tokio::task::JoinHandle<ExitedError>,
    tokio::task::JoinHandle<()>,
) {
    let events = Batch9Events::default();
    let events_clone = events.clone();

    let (service, socket) = LspService::new(|client| Batch9Backend {
        _client: client,
        events: events_clone,
    });

    let (client_tx, server_rx) = tokio::io::duplex(1024 * 1024);
    let (server_tx, client_rx) = tokio::io::duplex(1024 * 1024);

    let server = Server::new(server_rx, server_tx, socket);
    let server_handle = tokio::spawn(async move { server.serve(service).await.unwrap_err() });

    let responses = Arc::new(Mutex::new(Vec::<serde_json::Value>::new()));
    let responses_clone = responses.clone();
    let client_tx_shared = Arc::new(tokio::sync::Mutex::new(Some(client_tx)));
    let tx_for_reader = client_tx_shared.clone();

    let mut reader = tokio::io::BufReader::new(client_rx);
    let reader_handle = tokio::spawn(async move {
        loop {
            let msg = match read_message(&mut reader).await {
                Ok(m) => m,
                Err(_) => break,
            };
            if let Some(id) = msg.get("id") {
                if msg.get("method").is_some() {
                    let resp = serde_json::json!({"jsonrpc":"2.0","id":id,"result":null});
                    let mut g = tx_for_reader.lock().await;
                    if let Some(ref mut s) = *g {
                        let _ = s.write_all(&encode_message(&resp)).await;
                    }
                } else {
                    responses_clone.lock().unwrap().push(msg);
                }
            }
        }
    });

    write_msg_b9(
        &client_tx_shared,
        serde_json::json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{"capabilities":{}}}),
    ).await;
    wait_for_response_b9(responses.clone(), 1, Duration::from_millis(300)).await;
    write_msg_b9(
        &client_tx_shared,
        serde_json::json!({"jsonrpc":"2.0","method":"initialized","params":{}}),
    )
    .await;
    tokio::time::sleep(Duration::from_millis(20)).await;

    (
        events,
        client_tx_shared,
        responses,
        server_handle,
        reader_handle,
    )
}

async fn shutdown_b9(
    tx: Arc<tokio::sync::Mutex<Option<tokio::io::DuplexStream>>>,
    responses: Arc<Mutex<Vec<serde_json::Value>>>,
    server_handle: tokio::task::JoinHandle<ExitedError>,
    reader_handle: tokio::task::JoinHandle<()>,
) {
    write_msg_b9(
        &tx,
        serde_json::json!({"jsonrpc":"2.0","id":99,"method":"shutdown"}),
    )
    .await;
    wait_for_response_b9(responses, 99, Duration::from_millis(200)).await;
    write_msg_b9(&tx, serde_json::json!({"jsonrpc":"2.0","method":"exit"})).await;
    {
        let mut g = tx.lock().await;
        if let Some(ref mut s) = *g {
            let _ = s.shutdown().await;
        }
    }
    let _ = server_handle.await;
    let _ = reader_handle.await;
}

#[tokio::test(flavor = "current_thread")]
async fn test_b9_notebook_document_did_save() {
    let (events, tx, responses, server_handle, reader_handle) = boot_batch9().await;

    write_msg_b9(
        &tx,
        serde_json::json!({
            "jsonrpc": "2.0",
            "method": "notebookDocument/didSave",
            "params": {
                "notebookDocument": { "uri": "file:///save_test.ipynb" }
            }
        }),
    )
    .await;

    let saved = wait_for_event_b9(
        &events.did_save,
        Duration::from_millis(200),
        "notebookDocument/didSave",
    )
    .await;
    assert_eq!(
        saved.notebook_document.uri.as_str(),
        "file:///save_test.ipynb"
    );

    shutdown_b9(tx, responses, server_handle, reader_handle).await;
}

#[tokio::test(flavor = "current_thread")]
async fn test_b9_notebook_document_did_close() {
    let (events, tx, responses, server_handle, reader_handle) = boot_batch9().await;

    write_msg_b9(
        &tx,
        serde_json::json!({
            "jsonrpc": "2.0",
            "method": "notebookDocument/didClose",
            "params": {
                "notebookDocument": { "uri": "file:///close_test.ipynb" },
                "cellTextDocuments": []
            }
        }),
    )
    .await;

    let closed = wait_for_event_b9(
        &events.did_close,
        Duration::from_millis(200),
        "notebookDocument/didClose",
    )
    .await;
    assert_eq!(
        closed.notebook_document.uri.as_str(),
        "file:///close_test.ipynb"
    );

    shutdown_b9(tx, responses, server_handle, reader_handle).await;
}

#[tokio::test(flavor = "current_thread")]
async fn test_b9_work_done_progress_cancel() {
    let (events, tx, responses, server_handle, reader_handle) = boot_batch9().await;

    write_msg_b9(
        &tx,
        serde_json::json!({
            "jsonrpc": "2.0",
            "method": "window/workDoneProgress/cancel",
            "params": { "token": "cancel-token-b9" }
        }),
    )
    .await;

    let cancel = wait_for_event_b9(
        &events.work_done_cancel,
        Duration::from_millis(200),
        "window/workDoneProgress/cancel",
    )
    .await;
    assert_eq!(
        cancel.token,
        lsp::NumberOrString::String("cancel-token-b9".to_string())
    );

    shutdown_b9(tx, responses, server_handle, reader_handle).await;
}

#[tokio::test(flavor = "current_thread")]
async fn test_b9_progress_notification() {
    let (events, tx, responses, server_handle, reader_handle) = boot_batch9().await;

    write_msg_b9(
        &tx,
        serde_json::json!({
            "jsonrpc": "2.0",
            "method": "$/progress",
            "params": {
                "token": "progress-b9",
                "value": { "kind": "begin", "title": "building" }
            }
        }),
    )
    .await;

    let prog = wait_for_event_b9(&events.progress, Duration::from_millis(200), "$/progress").await;
    assert_eq!(
        prog.token,
        lsp::NumberOrString::String("progress-b9".to_string())
    );

    shutdown_b9(tx, responses, server_handle, reader_handle).await;
}

#[tokio::test(flavor = "current_thread")]
async fn test_b9_set_trace_notification() {
    let (events, tx, responses, server_handle, reader_handle) = boot_batch9().await;

    write_msg_b9(
        &tx,
        serde_json::json!({
            "jsonrpc": "2.0",
            "method": "$/setTrace",
            "params": { "value": "messages" }
        }),
    )
    .await;

    let trace = wait_for_event_b9(&events.set_trace, Duration::from_millis(200), "$/setTrace").await;
    assert_eq!(trace.value, lsp::TraceValue::Messages);

    shutdown_b9(tx, responses, server_handle, reader_handle).await;
}
