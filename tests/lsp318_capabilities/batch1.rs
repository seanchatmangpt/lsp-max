use lsp_max::jsonrpc::Result;
use lsp_max::lsp_types as lsp;
use lsp_max::{Client, LspService, Server};
/// Batch 1 — initialized, didOpen, didChange, didClose, didSave, willSave,
/// willSaveWaitUntil, completion notification/stub-handler tests.
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::io::AsyncWriteExt;

use super::common::{encode_message, read_message, wait_for_response_b1, write_msg_b1};

#[derive(Clone, Default)]
pub struct CallLog {
    pub initialized: Arc<Mutex<bool>>,
    pub did_open: Arc<Mutex<bool>>,
    pub did_change: Arc<Mutex<bool>>,
    pub did_close: Arc<Mutex<bool>>,
    pub did_save: Arc<Mutex<bool>>,
    pub will_save: Arc<Mutex<bool>>,
    pub will_save_wait_until: Arc<Mutex<bool>>,
    pub completion: Arc<Mutex<bool>>,
}

pub struct Lsp318Backend {
    pub _client: Client,
    pub log: CallLog,
}

#[lsp_max::async_trait]
impl lsp_max::LanguageServer for Lsp318Backend {
    async fn initialize(&self, _: lsp::InitializeParams) -> Result<lsp::InitializeResult> {
        Ok(lsp::InitializeResult::default())
    }
    async fn initialized(&self, _: lsp::InitializedParams) {
        *self.log.initialized.lock().unwrap() = true;
    }
    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }
    async fn did_open(&self, _: lsp::DidOpenTextDocumentParams) {
        *self.log.did_open.lock().unwrap() = true;
    }
    async fn did_change(&self, _: lsp::DidChangeTextDocumentParams) {
        *self.log.did_change.lock().unwrap() = true;
    }
    async fn did_close(&self, _: lsp::DidCloseTextDocumentParams) {
        *self.log.did_close.lock().unwrap() = true;
    }
    async fn did_save(&self, _: lsp::DidSaveTextDocumentParams) {
        *self.log.did_save.lock().unwrap() = true;
    }
    async fn will_save(&self, _: lsp::WillSaveTextDocumentParams) {
        *self.log.will_save.lock().unwrap() = true;
    }
    async fn will_save_wait_until(
        &self,
        _: lsp::WillSaveTextDocumentParams,
    ) -> Result<Option<Vec<lsp::TextEdit>>> {
        *self.log.will_save_wait_until.lock().unwrap() = true;
        Ok(None)
    }
    async fn completion(
        &self,
        _: lsp::CompletionParams,
    ) -> Result<Option<lsp::CompletionResponse>> {
        *self.log.completion.lock().unwrap() = true;
        Ok(None)
    }
}

pub async fn boot_server_b1() -> (
    Arc<tokio::sync::Mutex<Option<tokio::io::DuplexStream>>>,
    Arc<Mutex<Vec<serde_json::Value>>>,
    CallLog,
    tokio::task::JoinHandle<()>,
) {
    let log = CallLog::default();
    let log_clone = log.clone();

    let (service, socket) = LspService::new(|client| Lsp318Backend {
        _client: client,
        log: log_clone,
    });

    let (client_tx, server_rx) = tokio::io::duplex(1024 * 1024);
    let (server_tx, client_rx) = tokio::io::duplex(1024 * 1024);
    let server = Server::new(server_rx, server_tx, socket);
    let server_handle = tokio::spawn(async move {
        server.serve(service).await.ok();
    });

    let responses: Arc<Mutex<Vec<serde_json::Value>>> = Arc::new(Mutex::new(Vec::new()));
    let responses_clone = responses.clone();
    let client_tx_shared = Arc::new(tokio::sync::Mutex::new(Some(client_tx)));
    let client_tx_for_reader = client_tx_shared.clone();

    let mut reader = tokio::io::BufReader::new(client_rx);
    tokio::spawn(async move {
        loop {
            let msg = match read_message(&mut reader).await {
                Ok(m) => m,
                Err(_) => break,
            };
            if let Some(id) = msg.get("id") {
                if msg.get("method").is_some() {
                    let resp = serde_json::json!({ "jsonrpc":"2.0","id":id,"result":null });
                    let mut g = client_tx_for_reader.lock().await;
                    if let Some(ref mut tx) = *g {
                        let _ = tx.write_all(&encode_message(&resp)).await;
                    }
                } else {
                    responses_clone.lock().unwrap().push(msg);
                }
            }
        }
    });

    write_msg_b1(
        &client_tx_shared,
        serde_json::json!({ "jsonrpc":"2.0","id":0,"method":"initialize","params":{"capabilities":{}} }),
    )
    .await;
    wait_for_response_b1(responses.clone(), 0, Duration::from_millis(200)).await;

    (client_tx_shared, responses, log, server_handle)
}

#[tokio::test(flavor = "current_thread")]
async fn test_b1_initialized() {
    let (tx, _resp, log, _h) = boot_server_b1().await;
    write_msg_b1(
        &tx,
        serde_json::json!({ "jsonrpc":"2.0","method":"initialized","params":{} }),
    )
    .await;
    tokio::time::sleep(Duration::from_millis(5)).await;
    assert!(
        *log.initialized.lock().unwrap(),
        "initialized handler was not called"
    );
}

#[tokio::test(flavor = "current_thread")]
async fn test_b1_did_open() {
    let (tx, _resp, log, _h) = boot_server_b1().await;
    write_msg_b1(
        &tx,
        serde_json::json!({
            "jsonrpc":"2.0","method":"textDocument/didOpen",
            "params":{"textDocument":{"uri":"file:///test.rs","languageId":"rust","version":1,"text":""}}
        }),
    )
    .await;
    tokio::time::sleep(Duration::from_millis(5)).await;
    assert!(
        *log.did_open.lock().unwrap(),
        "did_open handler was not called"
    );
}

#[tokio::test(flavor = "current_thread")]
async fn test_b1_did_change() {
    let (tx, _resp, log, _h) = boot_server_b1().await;
    write_msg_b1(
        &tx,
        serde_json::json!({
            "jsonrpc":"2.0","method":"textDocument/didChange",
            "params":{
                "textDocument":{"uri":"file:///test.rs","version":2},
                "contentChanges":[{"text":"fn main(){}"}]
            }
        }),
    )
    .await;
    tokio::time::sleep(Duration::from_millis(5)).await;
    assert!(
        *log.did_change.lock().unwrap(),
        "did_change handler was not called"
    );
}

#[tokio::test(flavor = "current_thread")]
async fn test_b1_did_close() {
    let (tx, _resp, log, _h) = boot_server_b1().await;
    write_msg_b1(
        &tx,
        serde_json::json!({
            "jsonrpc":"2.0","method":"textDocument/didClose",
            "params":{"textDocument":{"uri":"file:///test.rs"}}
        }),
    )
    .await;
    tokio::time::sleep(Duration::from_millis(5)).await;
    assert!(
        *log.did_close.lock().unwrap(),
        "did_close handler was not called"
    );
}

#[tokio::test(flavor = "current_thread")]
async fn test_b1_did_save() {
    let (tx, _resp, log, _h) = boot_server_b1().await;
    write_msg_b1(
        &tx,
        serde_json::json!({
            "jsonrpc":"2.0","method":"textDocument/didSave",
            "params":{"textDocument":{"uri":"file:///test.rs"}}
        }),
    )
    .await;
    tokio::time::sleep(Duration::from_millis(5)).await;
    assert!(
        *log.did_save.lock().unwrap(),
        "did_save handler was not called"
    );
}

#[tokio::test(flavor = "current_thread")]
async fn test_b1_will_save() {
    let (tx, _resp, log, _h) = boot_server_b1().await;
    write_msg_b1(
        &tx,
        serde_json::json!({
            "jsonrpc":"2.0","method":"textDocument/willSave",
            "params":{"textDocument":{"uri":"file:///test.rs"},"reason":1}
        }),
    )
    .await;
    tokio::time::sleep(Duration::from_millis(5)).await;
    assert!(
        *log.will_save.lock().unwrap(),
        "will_save handler was not called"
    );
}

#[tokio::test(flavor = "current_thread")]
async fn test_b1_will_save_wait_until() {
    let (tx, resp, log, _h) = boot_server_b1().await;
    write_msg_b1(
        &tx,
        serde_json::json!({
            "jsonrpc":"2.0","id":10,"method":"textDocument/willSaveWaitUntil",
            "params":{"textDocument":{"uri":"file:///test.rs"},"reason":1}
        }),
    )
    .await;
    let r = wait_for_response_b1(resp, 10, Duration::from_millis(200)).await;
    assert!(
        r.get("error").is_none(),
        "willSaveWaitUntil returned error: {:?}",
        r
    );
    assert!(
        *log.will_save_wait_until.lock().unwrap(),
        "will_save_wait_until handler was not called"
    );
}

#[tokio::test(flavor = "current_thread")]
async fn test_b1_completion() {
    let (tx, resp, log, _h) = boot_server_b1().await;
    write_msg_b1(
        &tx,
        serde_json::json!({
            "jsonrpc":"2.0","id":11,"method":"textDocument/completion",
            "params":{"textDocument":{"uri":"file:///test.rs"},"position":{"line":0,"character":5}}
        }),
    )
    .await;
    let r = wait_for_response_b1(resp, 11, Duration::from_millis(200)).await;
    assert!(
        r.get("error").is_none(),
        "completion returned error: {:?}",
        r
    );
    assert!(
        *log.completion.lock().unwrap(),
        "completion handler was not called"
    );
}
