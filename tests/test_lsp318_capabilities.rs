use std::str::FromStr;
use std::sync::Arc;
/// Integration tests for LSP 3.18 capabilities (batch 1 + batch 2).
///
/// Batch 1 covers: initialized, textDocument/didOpen, textDocument/didChange,
/// textDocument/didClose, textDocument/didSave, textDocument/willSave,
/// textDocument/willSaveWaitUntil, textDocument/completion
///
/// Batch 2 covers: completionItem/resolve, textDocument/hover, textDocument/signatureHelp,
/// textDocument/declaration, textDocument/definition, textDocument/typeDefinition,
/// textDocument/implementation, textDocument/references
///
/// The tests verify that:
///   1. The dispatch path is wired (requests reach the server and produce a JSON-RPC response).
///   2. The process does not panic.
///   3. The response is well-formed JSON-RPC (either a result or an error object).
use std::sync::Mutex;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tower_lsp_max::jsonrpc::Result;
use tower_lsp_max::lsp_types as lsp;
use tower_lsp_max::{Client, LanguageServer, LspService, Server};

// ══════════════════════════════════════════════════════════════════════════════
// Batch 1 — notification / stub-handler tests
// ══════════════════════════════════════════════════════════════════════════════

#[derive(Clone, Default)]
struct CallLog {
    initialized: Arc<Mutex<bool>>,
    did_open: Arc<Mutex<bool>>,
    did_change: Arc<Mutex<bool>>,
    did_close: Arc<Mutex<bool>>,
    did_save: Arc<Mutex<bool>>,
    will_save: Arc<Mutex<bool>>,
    will_save_wait_until: Arc<Mutex<bool>>,
    completion: Arc<Mutex<bool>>,
}

struct Lsp318Backend {
    _client: Client,
    log: CallLog,
}

#[tower_lsp_max::async_trait]
impl LanguageServer for Lsp318Backend {
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

async fn wait_for_response_b1(
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

async fn write_msg_b1(
    tx_shared: &Arc<tokio::sync::Mutex<Option<tokio::io::DuplexStream>>>,
    msg: serde_json::Value,
) {
    let mut guard = tx_shared.lock().await;
    if let Some(ref mut tx) = *guard {
        tx.write_all(&encode_message(&msg)).await.unwrap();
    }
}

async fn boot_server_b1() -> (
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
    wait_for_response_b1(responses.clone(), 0, Duration::from_secs(2)).await;

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
    tokio::time::sleep(Duration::from_millis(50)).await;
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
    tokio::time::sleep(Duration::from_millis(50)).await;
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
    tokio::time::sleep(Duration::from_millis(50)).await;
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
    tokio::time::sleep(Duration::from_millis(50)).await;
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
    tokio::time::sleep(Duration::from_millis(50)).await;
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
    tokio::time::sleep(Duration::from_millis(50)).await;
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
    let r = wait_for_response_b1(resp, 10, Duration::from_secs(2)).await;
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
    let r = wait_for_response_b1(resp, 11, Duration::from_secs(2)).await;
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

// ══════════════════════════════════════════════════════════════════════════════
// Batch 2 — default-impl / method-not-found tests
// ══════════════════════════════════════════════════════════════════════════════

struct StubBackend;

#[tower_lsp_max::async_trait]
impl LanguageServer for StubBackend {
    async fn initialize(&self, _: lsp::InitializeParams) -> Result<lsp::InitializeResult> {
        Ok(lsp::InitializeResult::default())
    }
    async fn initialized(&self, _: lsp::InitializedParams) {}
    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }
}

// ── wire helpers ───────────────────────────────────────────────────────────────

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
    Ok(serde_json::from_slice(&body)?)
}

fn encode_message(msg: &serde_json::Value) -> Vec<u8> {
    let payload = serde_json::to_string(msg).unwrap();
    format!("Content-Length: {}\r\n\r\n{}", payload.len(), payload).into_bytes()
}

/// Spin up a server, send initialize + the given request, return the response for that request id.
async fn roundtrip(method: &str, params: serde_json::Value) -> serde_json::Value {
    let (service, socket) = LspService::new(|_client| StubBackend);
    let (mut client_tx, server_rx) = tokio::io::duplex(1024 * 1024);
    let (server_tx, mut client_rx) = tokio::io::duplex(1024 * 1024);
    let server = Server::new(server_rx, server_tx, socket);
    tokio::spawn(server.serve(service));

    // initialize
    let init = serde_json::json!({
        "jsonrpc":"2.0","id":1,"method":"initialize","params":{"capabilities":{}}
    });
    client_tx.write_all(&encode_message(&init)).await.unwrap();

    // drain the initialize response
    let mut reader = tokio::io::BufReader::new(&mut client_rx);
    let _init_resp = tokio::time::timeout(Duration::from_secs(5), read_message(&mut reader))
        .await
        .expect("timeout waiting for initialize response")
        .unwrap();

    // send the actual request
    let req = serde_json::json!({
        "jsonrpc":"2.0","id":2,"method":method,"params":params
    });
    client_tx.write_all(&encode_message(&req)).await.unwrap();

    // read until we get id=2
    loop {
        let msg = tokio::time::timeout(Duration::from_secs(5), read_message(&mut reader))
            .await
            .expect("timeout waiting for response")
            .unwrap();
        if msg.get("id").and_then(|i| i.as_i64()) == Some(2) {
            return msg;
        }
    }
}

/// Assert that the response is well-formed JSON-RPC: has "id":2 and either
/// a "result" key OR an "error" key.  The test proves dispatch reached the
/// server without panicking.
fn assert_well_formed(resp: &serde_json::Value) {
    assert_eq!(resp.get("id").and_then(|i| i.as_i64()), Some(2));
    assert!(
        resp.get("result").is_some() || resp.get("error").is_some(),
        "Response had neither result nor error: {:?}",
        resp
    );
}

// ── text-document position helper ─────────────────────────────────────────────

fn td_pos(uri: &str) -> serde_json::Value {
    serde_json::json!({
        "textDocument": { "uri": uri },
        "position":     { "line": 0, "character": 0 }
    })
}

// ══════════════════════════════════════════════════════════════════════════════
// Tests
// ══════════════════════════════════════════════════════════════════════════════

/// completionItem/resolve — dispatched, default impl returns method_not_found.
#[tokio::test(flavor = "current_thread")]
async fn test_completion_item_resolve_dispatch() {
    // Params: a CompletionItem (minimal)
    let params = serde_json::json!({
        "label": "myFunction"
    });
    let resp = roundtrip("completionItem/resolve", params).await;
    assert_well_formed(&resp);
}

/// textDocument/hover — dispatched, default impl returns method_not_found.
#[tokio::test(flavor = "current_thread")]
async fn test_hover_dispatch() {
    let resp = roundtrip("textDocument/hover", td_pos("file:///test.rs")).await;
    assert_well_formed(&resp);
}

/// textDocument/signatureHelp — dispatched, default impl returns method_not_found.
#[tokio::test(flavor = "current_thread")]
async fn test_signature_help_dispatch() {
    let params = serde_json::json!({
        "textDocument": { "uri": "file:///test.rs" },
        "position": { "line": 0, "character": 0 },
        "context": {
            "triggerKind": 1,
            "isRetrigger": false
        }
    });
    let resp = roundtrip("textDocument/signatureHelp", params).await;
    assert_well_formed(&resp);
}

/// textDocument/declaration — dispatched, default impl returns method_not_found.
#[tokio::test(flavor = "current_thread")]
async fn test_declaration_dispatch() {
    let resp = roundtrip("textDocument/declaration", td_pos("file:///test.rs")).await;
    assert_well_formed(&resp);
}

/// textDocument/definition — dispatched, default impl returns method_not_found.
#[tokio::test(flavor = "current_thread")]
async fn test_definition_dispatch() {
    let resp = roundtrip("textDocument/definition", td_pos("file:///test.rs")).await;
    assert_well_formed(&resp);
}

/// textDocument/typeDefinition — dispatched, default impl returns method_not_found.
#[tokio::test(flavor = "current_thread")]
async fn test_type_definition_dispatch() {
    let resp = roundtrip("textDocument/typeDefinition", td_pos("file:///test.rs")).await;
    assert_well_formed(&resp);
}

/// textDocument/implementation — dispatched, default impl returns method_not_found.
#[tokio::test(flavor = "current_thread")]
async fn test_implementation_dispatch() {
    let resp = roundtrip("textDocument/implementation", td_pos("file:///test.rs")).await;
    assert_well_formed(&resp);
}

/// textDocument/references — dispatched, default impl returns method_not_found.
#[tokio::test(flavor = "current_thread")]
async fn test_references_dispatch() {
    let params = serde_json::json!({
        "textDocument": { "uri": "file:///test.rs" },
        "position": { "line": 0, "character": 0 },
        "context": { "includeDeclaration": true }
    });
    let resp = roundtrip("textDocument/references", params).await;
    assert_well_formed(&resp);
}

// ── batch 6 ────────────────────────────────────────────────────────────────────

fn call_hierarchy_item_json(uri: &str) -> serde_json::Value {
    serde_json::json!({
        "name": "myFunc",
        "kind": 12,
        "uri": uri,
        "range": {
            "start": { "line": 0, "character": 0 },
            "end":   { "line": 0, "character": 10 }
        },
        "selectionRange": {
            "start": { "line": 0, "character": 0 },
            "end":   { "line": 0, "character": 6 }
        }
    })
}

fn type_hierarchy_item_json(uri: &str) -> serde_json::Value {
    serde_json::json!({
        "name": "MyType",
        "kind": 5,
        "uri": uri,
        "range": {
            "start": { "line": 0, "character": 0 },
            "end":   { "line": 10, "character": 1 }
        },
        "selectionRange": {
            "start": { "line": 0, "character": 0 },
            "end":   { "line": 0, "character": 6 }
        }
    })
}

/// textDocument/inlineCompletion — dispatched, default impl returns method_not_found.
#[tokio::test(flavor = "current_thread")]
async fn test_inline_completion_dispatch() {
    let params = serde_json::json!({
        "textDocument": { "uri": "file:///test.rs" },
        "position": { "line": 0, "character": 5 },
        "context": { "triggerKind": 1 }
    });
    let resp = roundtrip("textDocument/inlineCompletion", params).await;
    assert_well_formed(&resp);
}

/// textDocument/prepareCallHierarchy — dispatched, default impl returns method_not_found.
#[tokio::test(flavor = "current_thread")]
async fn test_prepare_call_hierarchy_dispatch() {
    let resp = roundtrip(
        "textDocument/prepareCallHierarchy",
        td_pos("file:///test.rs"),
    )
    .await;
    assert_well_formed(&resp);
}

/// callHierarchy/incomingCalls — dispatched, default impl returns method_not_found.
#[tokio::test(flavor = "current_thread")]
async fn test_incoming_calls_dispatch() {
    let params = serde_json::json!({
        "item": call_hierarchy_item_json("file:///test.rs")
    });
    let resp = roundtrip("callHierarchy/incomingCalls", params).await;
    assert_well_formed(&resp);
}

/// callHierarchy/outgoingCalls — dispatched, default impl returns method_not_found.
#[tokio::test(flavor = "current_thread")]
async fn test_outgoing_calls_dispatch() {
    let params = serde_json::json!({
        "item": call_hierarchy_item_json("file:///test.rs")
    });
    let resp = roundtrip("callHierarchy/outgoingCalls", params).await;
    assert_well_formed(&resp);
}

/// textDocument/prepareTypeHierarchy — dispatched, default impl returns method_not_found.
#[tokio::test(flavor = "current_thread")]
async fn test_prepare_type_hierarchy_dispatch() {
    let resp = roundtrip(
        "textDocument/prepareTypeHierarchy",
        td_pos("file:///test.rs"),
    )
    .await;
    assert_well_formed(&resp);
}

/// typeHierarchy/supertypes — dispatched, default impl returns method_not_found.
#[tokio::test(flavor = "current_thread")]
async fn test_type_hierarchy_supertypes_dispatch() {
    let params = serde_json::json!({
        "item": type_hierarchy_item_json("file:///test.rs")
    });
    let resp = roundtrip("typeHierarchy/supertypes", params).await;
    assert_well_formed(&resp);
}

/// typeHierarchy/subtypes — dispatched, default impl returns method_not_found.
#[tokio::test(flavor = "current_thread")]
async fn test_type_hierarchy_subtypes_dispatch() {
    let params = serde_json::json!({
        "item": type_hierarchy_item_json("file:///test.rs")
    });
    let resp = roundtrip("typeHierarchy/subtypes", params).await;
    assert_well_formed(&resp);
}

/// textDocument/moniker — dispatched, default impl returns method_not_found.
#[tokio::test(flavor = "current_thread")]
async fn test_moniker_dispatch() {
    let resp = roundtrip("textDocument/moniker", td_pos("file:///test.rs")).await;
    assert_well_formed(&resp);
}

// ══════════════════════════════════════════════════════════════════════════════
// Batch 3 — document highlight, symbol, code action, code lens, document link
// ══════════════════════════════════════════════════════════════════════════════

/// textDocument/documentHighlight — dispatched, default impl returns method_not_found.
#[tokio::test(flavor = "current_thread")]
async fn test_document_highlight_dispatch() {
    let resp = roundtrip("textDocument/documentHighlight", td_pos("file:///test.rs")).await;
    assert_well_formed(&resp);
}

/// textDocument/documentSymbol — dispatched, default impl returns method_not_found.
#[tokio::test(flavor = "current_thread")]
async fn test_document_symbol_dispatch() {
    let params = serde_json::json!({
        "textDocument": { "uri": "file:///test.rs" }
    });
    let resp = roundtrip("textDocument/documentSymbol", params).await;
    assert_well_formed(&resp);
}

/// textDocument/codeAction — dispatched, default impl returns method_not_found.
#[tokio::test(flavor = "current_thread")]
async fn test_code_action_dispatch() {
    let params = serde_json::json!({
        "textDocument": { "uri": "file:///test.rs" },
        "range": {
            "start": { "line": 0, "character": 0 },
            "end":   { "line": 0, "character": 5 }
        },
        "context": { "diagnostics": [], "triggerKind": 1 }
    });
    let resp = roundtrip("textDocument/codeAction", params).await;
    assert_well_formed(&resp);
}

/// codeAction/resolve — dispatched, default impl returns method_not_found.
#[tokio::test(flavor = "current_thread")]
async fn test_code_action_resolve_dispatch() {
    let params = serde_json::json!({
        "title": "resolve_me"
    });
    let resp = roundtrip("codeAction/resolve", params).await;
    assert_well_formed(&resp);
}

/// textDocument/codeLens — dispatched, default impl returns method_not_found.
#[tokio::test(flavor = "current_thread")]
async fn test_code_lens_dispatch() {
    let params = serde_json::json!({
        "textDocument": { "uri": "file:///test.rs" }
    });
    let resp = roundtrip("textDocument/codeLens", params).await;
    assert_well_formed(&resp);
}

/// codeLens/resolve — dispatched, default impl returns method_not_found.
#[tokio::test(flavor = "current_thread")]
async fn test_code_lens_resolve_dispatch() {
    let params = serde_json::json!({
        "range": {
            "start": { "line": 0, "character": 0 },
            "end":   { "line": 0, "character": 5 }
        }
    });
    let resp = roundtrip("codeLens/resolve", params).await;
    assert_well_formed(&resp);
}

/// textDocument/documentLink — dispatched, default impl returns method_not_found.
#[tokio::test(flavor = "current_thread")]
async fn test_document_link_dispatch() {
    let params = serde_json::json!({
        "textDocument": { "uri": "file:///test.rs" }
    });
    let resp = roundtrip("textDocument/documentLink", params).await;
    assert_well_formed(&resp);
}

/// documentLink/resolve — dispatched, default impl returns method_not_found.
#[tokio::test(flavor = "current_thread")]
async fn test_document_link_resolve_dispatch() {
    let params = serde_json::json!({
        "range": {
            "start": { "line": 0, "character": 0 },
            "end":   { "line": 0, "character": 10 }
        },
        "target": "https://example.com/"
    });
    let resp = roundtrip("documentLink/resolve", params).await;
    assert_well_formed(&resp);
}

// ══════════════════════════════════════════════════════════════════════════════
// Batch 4/9 — documentColor, colorPresentation, formatting, rangeFormatting,
//             onTypeFormatting, rename, prepareRename, foldingRange
// ══════════════════════════════════════════════════════════════════════════════

/// textDocument/documentColor — dispatch path wired; default impl returns method_not_found.
#[tokio::test(flavor = "current_thread")]
async fn test_document_color_dispatch() {
    let params = serde_json::json!({ "textDocument": { "uri": "file:///test.rs" } });
    let resp = roundtrip("textDocument/documentColor", params).await;
    assert_well_formed(&resp);
}

/// textDocument/colorPresentation — dispatch path wired; default impl returns method_not_found.
#[tokio::test(flavor = "current_thread")]
async fn test_color_presentation_dispatch() {
    let params = serde_json::json!({
        "textDocument": { "uri": "file:///test.rs" },
        "color": { "red": 1.0, "green": 0.0, "blue": 0.0, "alpha": 1.0 },
        "range": {
            "start": { "line": 0, "character": 0 },
            "end":   { "line": 0, "character": 7 }
        }
    });
    let resp = roundtrip("textDocument/colorPresentation", params).await;
    assert_well_formed(&resp);
}

/// textDocument/formatting — dispatch path wired; default impl returns method_not_found.
#[tokio::test(flavor = "current_thread")]
async fn test_formatting_dispatch() {
    let params = serde_json::json!({
        "textDocument": { "uri": "file:///test.rs" },
        "options": { "tabSize": 4, "insertSpaces": true }
    });
    let resp = roundtrip("textDocument/formatting", params).await;
    assert_well_formed(&resp);
}

/// textDocument/rangeFormatting — dispatch path wired; default impl returns method_not_found.
#[tokio::test(flavor = "current_thread")]
async fn test_range_formatting_dispatch() {
    let params = serde_json::json!({
        "textDocument": { "uri": "file:///test.rs" },
        "range": {
            "start": { "line": 0, "character": 0 },
            "end":   { "line": 5, "character": 0 }
        },
        "options": { "tabSize": 4, "insertSpaces": true }
    });
    let resp = roundtrip("textDocument/rangeFormatting", params).await;
    assert_well_formed(&resp);
}

/// textDocument/onTypeFormatting — dispatch path wired; default impl returns method_not_found.
#[tokio::test(flavor = "current_thread")]
async fn test_on_type_formatting_dispatch() {
    let params = serde_json::json!({
        "textDocument": { "uri": "file:///test.rs" },
        "position": { "line": 3, "character": 1 },
        "ch": ";",
        "options": { "tabSize": 4, "insertSpaces": true }
    });
    let resp = roundtrip("textDocument/onTypeFormatting", params).await;
    assert_well_formed(&resp);
}

/// textDocument/rename — dispatch path wired; default impl returns method_not_found.
#[tokio::test(flavor = "current_thread")]
async fn test_rename_dispatch() {
    let params = serde_json::json!({
        "textDocument": { "uri": "file:///test.rs" },
        "position": { "line": 1, "character": 5 },
        "newName": "newSymbol"
    });
    let resp = roundtrip("textDocument/rename", params).await;
    assert_well_formed(&resp);
}

/// textDocument/prepareRename — dispatch path wired; default impl returns method_not_found.
#[tokio::test(flavor = "current_thread")]
async fn test_prepare_rename_dispatch() {
    let resp = roundtrip("textDocument/prepareRename", td_pos("file:///test.rs")).await;
    assert_well_formed(&resp);
}

/// textDocument/foldingRange — dispatch path wired; default impl returns method_not_found.
#[tokio::test(flavor = "current_thread")]
async fn test_folding_range_dispatch() {
    let params = serde_json::json!({ "textDocument": { "uri": "file:///test.rs" } });
    let resp = roundtrip("textDocument/foldingRange", params).await;
    assert_well_formed(&resp);
}

// ══════════════════════════════════════════════════════════════════════════════
// Batch 7/9 — workspace notification and request methods
// ══════════════════════════════════════════════════════════════════════════════

/// workspace/didChangeWorkspaceFolders — notification dispatch.
/// Notifications produce no JSON-RPC response.  We verify the server survives
/// by issuing a follow-up shutdown request and checking its well-formed reply.
#[tokio::test(flavor = "current_thread")]
async fn test_did_change_workspace_folders_dispatch() {
    let params = serde_json::json!({
        "event": {
            "added": [{"uri": "file:///tmp/added", "name": "added"}],
            "removed": []
        }
    });
    roundtrip_notification_then_shutdown("workspace/didChangeWorkspaceFolders", params).await;
}

/// workspace/didChangeConfiguration — notification dispatch.
#[tokio::test(flavor = "current_thread")]
async fn test_did_change_configuration_dispatch() {
    let params = serde_json::json!({
        "settings": {"editor": {"tabSize": 4}}
    });
    roundtrip_notification_then_shutdown("workspace/didChangeConfiguration", params).await;
}

/// workspace/didChangeWatchedFiles — notification dispatch.
#[tokio::test(flavor = "current_thread")]
async fn test_did_change_watched_files_dispatch() {
    let params = serde_json::json!({
        "changes": [{"uri": "file:///tmp/foo.rs", "type": 2}]
    });
    roundtrip_notification_then_shutdown("workspace/didChangeWatchedFiles", params).await;
}

/// workspace/symbol — default stub returns method_not_found error.
#[tokio::test(flavor = "current_thread")]
async fn test_workspace_symbol_dispatch() {
    let params = serde_json::json!({ "query": "MyStruct" });
    let resp = roundtrip("workspace/symbol", params).await;
    assert_well_formed(&resp);
}

/// workspaceSymbol/resolve — default stub returns method_not_found error.
#[tokio::test(flavor = "current_thread")]
async fn test_workspace_symbol_resolve_dispatch() {
    let params = serde_json::json!({
        "name": "MyStruct",
        "kind": 23,
        "location": {
            "uri": "file:///tmp/foo.rs",
            "range": {
                "start": {"line": 0, "character": 0},
                "end":   {"line": 0, "character": 0}
            }
        }
    });
    let resp = roundtrip("workspaceSymbol/resolve", params).await;
    assert_well_formed(&resp);
}

/// workspace/executeCommand — default stub returns method_not_found error.
#[tokio::test(flavor = "current_thread")]
async fn test_execute_command_dispatch() {
    let params = serde_json::json!({
        "command": "editor.action.formatDocument",
        "arguments": []
    });
    let resp = roundtrip("workspace/executeCommand", params).await;
    assert_well_formed(&resp);
}

/// workspace/diagnostic — default stub returns method_not_found error.
#[tokio::test(flavor = "current_thread")]
async fn test_workspace_diagnostic_dispatch() {
    let params = serde_json::json!({
        "previousResultIds": []
    });
    let resp = roundtrip("workspace/diagnostic", params).await;
    assert_well_formed(&resp);
}

/// workspace/willCreateFiles — default stub returns method_not_found error.
#[tokio::test(flavor = "current_thread")]
async fn test_will_create_files_dispatch() {
    let params = serde_json::json!({
        "files": [{"uri": "file:///tmp/new_file.rs"}]
    });
    let resp = roundtrip("workspace/willCreateFiles", params).await;
    assert_well_formed(&resp);
}

// ── notification helper ────────────────────────────────────────────────────────
// Sends a notification (no id) then a shutdown request to prove no panic.

async fn roundtrip_notification_then_shutdown(method: &str, params: serde_json::Value) {
    let (service, socket) = LspService::new(|_client| StubBackend);
    let (mut client_tx, server_rx) = tokio::io::duplex(1024 * 1024);
    let (server_tx, mut client_rx) = tokio::io::duplex(1024 * 1024);
    let server = Server::new(server_rx, server_tx, socket);
    tokio::spawn(server.serve(service));

    let mut reader = tokio::io::BufReader::new(&mut client_rx);

    // initialize handshake
    let init = serde_json::json!({
        "jsonrpc":"2.0","id":1,"method":"initialize","params":{"capabilities":{}}
    });
    client_tx.write_all(&encode_message(&init)).await.unwrap();
    let _init_resp = tokio::time::timeout(Duration::from_secs(5), read_message(&mut reader))
        .await
        .expect("timeout waiting for initialize response")
        .unwrap();

    // fire the notification
    let notif = serde_json::json!({"jsonrpc":"2.0","method":method,"params":params});
    client_tx.write_all(&encode_message(&notif)).await.unwrap();

    // shutdown — proves server is alive after the notification
    let shutdown = serde_json::json!({"jsonrpc":"2.0","id":2,"method":"shutdown","params":null});
    client_tx
        .write_all(&encode_message(&shutdown))
        .await
        .unwrap();

    let resp = loop {
        let msg = tokio::time::timeout(Duration::from_secs(5), read_message(&mut reader))
            .await
            .expect("timeout waiting for shutdown response")
            .unwrap();
        if msg.get("id").and_then(|i| i.as_i64()) == Some(2) {
            break msg;
        }
    };
    assert_well_formed(&resp);
}

// ══════════════════════════════════════════════════════════════════════════════
// Batch 5/9: selectionRange, linkedEditingRange, semanticTokens/*, inlayHint,
//            inlayHint/resolve, textDocument/diagnostic
// ══════════════════════════════════════════════════════════════════════════════

/// textDocument/selectionRange — dispatched, default impl returns method_not_found.
/// Also verifies that `lsp::SelectionRangeParams` compiles and serializes.
#[tokio::test(flavor = "current_thread")]
async fn test_selection_range_dispatch() {
    let params_typed = lsp::SelectionRangeParams {
        work_done_progress_params: lsp::WorkDoneProgressParams::default(),
        partial_result_params: lsp::PartialResultParams::default(),
        text_document: lsp::TextDocumentIdentifier {
            uri: lsp::Uri::from_str("file:///test.rs").unwrap(),
        },
        positions: vec![lsp::Position {
            line: 0,
            character: 0,
        }],
    };
    let serialized = serde_json::to_value(&params_typed).unwrap();
    assert!(serialized.is_object());

    let params = serde_json::json!({
        "textDocument": { "uri": "file:///test.rs" },
        "positions": [{ "line": 0, "character": 0 }]
    });
    let resp = roundtrip("textDocument/selectionRange", params).await;
    assert_well_formed(&resp);
}

/// textDocument/linkedEditingRange — dispatched, default impl returns method_not_found.
#[tokio::test(flavor = "current_thread")]
async fn test_linked_editing_range_dispatch() {
    let params_typed = lsp::LinkedEditingRangeParams {
        text_document_position_params: lsp::TextDocumentPositionParams {
            text_document: lsp::TextDocumentIdentifier {
                uri: lsp::Uri::from_str("file:///test.rs").unwrap(),
            },
            position: lsp::Position {
                line: 1,
                character: 5,
            },
        },
        work_done_progress_params: lsp::WorkDoneProgressParams::default(),
    };
    let serialized = serde_json::to_value(&params_typed).unwrap();
    assert!(serialized.is_object());

    let resp = roundtrip("textDocument/linkedEditingRange", td_pos("file:///test.rs")).await;
    assert_well_formed(&resp);
}

/// textDocument/semanticTokens/full — dispatched, default impl returns method_not_found.
#[tokio::test(flavor = "current_thread")]
async fn test_semantic_tokens_full_dispatch() {
    let params_typed = lsp::SemanticTokensParams {
        work_done_progress_params: lsp::WorkDoneProgressParams::default(),
        partial_result_params: lsp::PartialResultParams::default(),
        text_document: lsp::TextDocumentIdentifier {
            uri: lsp::Uri::from_str("file:///test.rs").unwrap(),
        },
    };
    let serialized = serde_json::to_value(&params_typed).unwrap();
    assert!(serialized.is_object());

    let params = serde_json::json!({
        "textDocument": { "uri": "file:///test.rs" }
    });
    let resp = roundtrip("textDocument/semanticTokens/full", params).await;
    assert_well_formed(&resp);
}

/// textDocument/semanticTokens/full/delta — dispatched, default impl returns method_not_found.
#[tokio::test(flavor = "current_thread")]
async fn test_semantic_tokens_full_delta_dispatch() {
    let params_typed = lsp::SemanticTokensDeltaParams {
        work_done_progress_params: lsp::WorkDoneProgressParams::default(),
        partial_result_params: lsp::PartialResultParams::default(),
        text_document: lsp::TextDocumentIdentifier {
            uri: lsp::Uri::from_str("file:///test.rs").unwrap(),
        },
        previous_result_id: "prev-result-id".to_string(),
    };
    let serialized = serde_json::to_value(&params_typed).unwrap();
    assert!(serialized.is_object());

    let params = serde_json::json!({
        "textDocument": { "uri": "file:///test.rs" },
        "previousResultId": "prev-result-id"
    });
    let resp = roundtrip("textDocument/semanticTokens/full/delta", params).await;
    assert_well_formed(&resp);
}

/// textDocument/semanticTokens/range — dispatched, default impl returns method_not_found.
#[tokio::test(flavor = "current_thread")]
async fn test_semantic_tokens_range_dispatch() {
    let params_typed = lsp::SemanticTokensRangeParams {
        work_done_progress_params: lsp::WorkDoneProgressParams::default(),
        partial_result_params: lsp::PartialResultParams::default(),
        text_document: lsp::TextDocumentIdentifier {
            uri: lsp::Uri::from_str("file:///test.rs").unwrap(),
        },
        range: lsp::Range {
            start: lsp::Position {
                line: 0,
                character: 0,
            },
            end: lsp::Position {
                line: 10,
                character: 0,
            },
        },
    };
    let serialized = serde_json::to_value(&params_typed).unwrap();
    assert!(serialized.is_object());

    let params = serde_json::json!({
        "textDocument": { "uri": "file:///test.rs" },
        "range": {
            "start": { "line": 0, "character": 0 },
            "end":   { "line": 10, "character": 0 }
        }
    });
    let resp = roundtrip("textDocument/semanticTokens/range", params).await;
    assert_well_formed(&resp);
}

/// textDocument/inlayHint — dispatched, default impl returns method_not_found.
#[tokio::test(flavor = "current_thread")]
async fn test_inlay_hint_dispatch() {
    let params_typed = lsp::InlayHintParams {
        work_done_progress_params: lsp::WorkDoneProgressParams::default(),
        text_document: lsp::TextDocumentIdentifier {
            uri: lsp::Uri::from_str("file:///test.rs").unwrap(),
        },
        range: lsp::Range {
            start: lsp::Position {
                line: 0,
                character: 0,
            },
            end: lsp::Position {
                line: 20,
                character: 0,
            },
        },
    };
    let serialized = serde_json::to_value(&params_typed).unwrap();
    assert!(serialized.is_object());

    let params = serde_json::json!({
        "textDocument": { "uri": "file:///test.rs" },
        "range": {
            "start": { "line": 0, "character": 0 },
            "end":   { "line": 20, "character": 0 }
        }
    });
    let resp = roundtrip("textDocument/inlayHint", params).await;
    assert_well_formed(&resp);
}

/// inlayHint/resolve — dispatched, default impl returns method_not_found.
/// `InlayHint` is both the request param and the resolved result type.
#[tokio::test(flavor = "current_thread")]
async fn test_inlay_hint_resolve_dispatch() {
    let hint = lsp::InlayHint {
        position: lsp::Position {
            line: 3,
            character: 10,
        },
        label: lsp::InlayHintLabel::String("i32".to_string()),
        kind: None,
        text_edits: None,
        tooltip: None,
        padding_left: None,
        padding_right: None,
        data: None,
    };
    let serialized = serde_json::to_value(&hint).unwrap();
    assert!(serialized.is_object());

    let params = serde_json::json!({
        "position": { "line": 3, "character": 10 },
        "label": "i32"
    });
    let resp = roundtrip("inlayHint/resolve", params).await;
    assert_well_formed(&resp);
}

/// textDocument/diagnostic — dispatched, default impl returns method_not_found.
#[tokio::test(flavor = "current_thread")]
async fn test_document_diagnostic_dispatch() {
    let params_typed = lsp::DocumentDiagnosticParams {
        work_done_progress_params: lsp::WorkDoneProgressParams::default(),
        partial_result_params: lsp::PartialResultParams::default(),
        text_document: lsp::TextDocumentIdentifier {
            uri: lsp::Uri::from_str("file:///test.rs").unwrap(),
        },
        identifier: None,
        previous_result_id: None,
    };
    let serialized = serde_json::to_value(&params_typed).unwrap();
    assert!(serialized.is_object());

    let params = serde_json::json!({
        "textDocument": { "uri": "file:///test.rs" }
    });
    let resp = roundtrip("textDocument/diagnostic", params).await;
    assert_well_formed(&resp);
}

// ═══════════════════════════════════════════════════════════════════════════
// Batch 9/9 – notification dispatch tests
//   notebookDocument/didSave, notebookDocument/didClose,
//   window/workDoneProgress/cancel, $/progress, $/setTrace
// ═══════════════════════════════════════════════════════════════════════════

use tower_lsp_max::max_protocol::lsp_3_18 as lsp318;
use tower_lsp_max::ExitedError;

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

#[tower_lsp_max::async_trait]
impl LanguageServer for Batch9Backend {
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
        tokio::time::sleep(Duration::from_millis(10)).await;
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
        tokio::time::sleep(Duration::from_millis(10)).await;
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
    wait_for_response_b9(responses.clone(), 1, Duration::from_secs(3)).await;
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
    wait_for_response_b9(responses, 99, Duration::from_secs(2)).await;
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

/// notebookDocument/didSave – dispatch path wired; params arrive correctly.
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
        Duration::from_secs(2),
        "notebookDocument/didSave",
    )
    .await;
    assert_eq!(
        saved.notebook_document.uri.as_str(),
        "file:///save_test.ipynb"
    );

    shutdown_b9(tx, responses, server_handle, reader_handle).await;
}

/// notebookDocument/didClose – dispatch path wired; params arrive correctly.
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
        Duration::from_secs(2),
        "notebookDocument/didClose",
    )
    .await;
    assert_eq!(
        closed.notebook_document.uri.as_str(),
        "file:///close_test.ipynb"
    );

    shutdown_b9(tx, responses, server_handle, reader_handle).await;
}

/// window/workDoneProgress/cancel – dispatch path wired; token arrives.
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
        Duration::from_secs(2),
        "window/workDoneProgress/cancel",
    )
    .await;
    assert_eq!(
        cancel.token,
        lsp::NumberOrString::String("cancel-token-b9".to_string())
    );

    shutdown_b9(tx, responses, server_handle, reader_handle).await;
}

/// $/progress – dispatch path wired; token arrives.
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

    let prog = wait_for_event_b9(&events.progress, Duration::from_secs(2), "$/progress").await;
    assert_eq!(
        prog.token,
        lsp::NumberOrString::String("progress-b9".to_string())
    );

    shutdown_b9(tx, responses, server_handle, reader_handle).await;
}

/// $/setTrace – dispatch path wired; trace value parsed as Messages.
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

    let trace = wait_for_event_b9(&events.set_trace, Duration::from_secs(2), "$/setTrace").await;
    assert_eq!(trace.value, lsp::TraceValue::Messages);

    shutdown_b9(tx, responses, server_handle, reader_handle).await;
}

// ══════════════════════════════════════════════════════════════════════════════
// Batch 8 — workspace/did*Files, workspace/willRenameFiles,
//            workspace/willDeleteFiles, workspace/textDocumentContent,
//            notebookDocument/didOpen, notebookDocument/didChange
// ══════════════════════════════════════════════════════════════════════════════

#[derive(Clone, Default)]
struct Batch8Events {
    did_create_files: Arc<Mutex<Option<lsp::CreateFilesParams>>>,
    did_rename_files: Arc<Mutex<Option<lsp::RenameFilesParams>>>,
    did_delete_files: Arc<Mutex<Option<lsp::DeleteFilesParams>>>,
    did_open_notebook: Arc<Mutex<Option<lsp::DidOpenNotebookDocumentParams>>>,
    did_change_notebook: Arc<Mutex<Option<lsp::DidChangeNotebookDocumentParams>>>,
}

struct Batch8Backend {
    #[allow(dead_code)]
    client: Client,
    events: Batch8Events,
}

#[tower_lsp_max::async_trait]
impl LanguageServer for Batch8Backend {
    async fn initialize(&self, _: lsp::InitializeParams) -> Result<lsp::InitializeResult> {
        Ok(lsp::InitializeResult::default())
    }
    async fn initialized(&self, _: lsp::InitializedParams) {}
    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }

    async fn text_document_content(
        &self,
        _params: lsp318::TextDocumentContentParams,
    ) -> Result<lsp318::TextDocumentContentResult> {
        Ok(lsp318::TextDocumentContentResult {
            text: "batch8-stub".to_string(),
        })
    }

    async fn did_create_files(&self, params: lsp::CreateFilesParams) {
        *self.events.did_create_files.lock().unwrap() = Some(params);
    }
    async fn did_rename_files(&self, params: lsp::RenameFilesParams) {
        *self.events.did_rename_files.lock().unwrap() = Some(params);
    }
    async fn did_delete_files(&self, params: lsp::DeleteFilesParams) {
        *self.events.did_delete_files.lock().unwrap() = Some(params);
    }
    async fn did_open_notebook_document(&self, params: lsp::DidOpenNotebookDocumentParams) {
        *self.events.did_open_notebook.lock().unwrap() = Some(params);
    }
    async fn did_change_notebook_document(&self, params: lsp::DidChangeNotebookDocumentParams) {
        *self.events.did_change_notebook.lock().unwrap() = Some(params);
    }
}

async fn batch8_start(
    events: Batch8Events,
) -> (
    tokio::io::DuplexStream,
    tokio::io::BufReader<tokio::io::DuplexStream>,
) {
    let ev = events.clone();
    let (service, socket) = LspService::new(|client| Batch8Backend { client, events: ev });
    let (client_tx, server_rx) = tokio::io::duplex(1024 * 1024);
    let (server_tx, client_rx) = tokio::io::duplex(1024 * 1024);
    let server = Server::new(server_rx, server_tx, socket);
    tokio::spawn(server.serve(service));
    let mut tx = client_tx;
    let mut reader = tokio::io::BufReader::new(client_rx);

    let init = serde_json::json!({
        "jsonrpc":"2.0","id":1,"method":"initialize","params":{"capabilities":{}}
    });
    tx.write_all(&encode_message(&init)).await.unwrap();
    let _r = tokio::time::timeout(Duration::from_secs(5), read_message(&mut reader))
        .await
        .unwrap()
        .unwrap();
    let ntf = serde_json::json!({"jsonrpc":"2.0","method":"initialized","params":{}});
    tx.write_all(&encode_message(&ntf)).await.unwrap();
    (tx, reader)
}

async fn batch8_request(
    tx: &mut tokio::io::DuplexStream,
    reader: &mut tokio::io::BufReader<tokio::io::DuplexStream>,
    method: &str,
    params: serde_json::Value,
) -> serde_json::Value {
    let req = serde_json::json!({"jsonrpc":"2.0","id":2,"method":method,"params":params});
    tx.write_all(&encode_message(&req)).await.unwrap();
    loop {
        let msg = tokio::time::timeout(Duration::from_secs(5), read_message(reader))
            .await
            .unwrap()
            .unwrap();
        if msg.get("id").and_then(|i| i.as_i64()) == Some(2) {
            return msg;
        }
    }
}

async fn wait_flag_b8<T: Clone>(flag: &Arc<Mutex<Option<T>>>, label: &str) -> T {
    let start = std::time::Instant::now();
    loop {
        if start.elapsed() > Duration::from_secs(3) {
            panic!("Timeout waiting for {}", label);
        }
        if let Some(v) = flag.lock().unwrap().clone() {
            return v;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
}

/// workspace/willRenameFiles — dispatched; default impl returns method_not_found error.
#[tokio::test(flavor = "current_thread")]
async fn test_workspace_will_rename_files_dispatch() {
    let params = serde_json::json!({
        "files": [{ "oldUri": "file:///a.rs", "newUri": "file:///b.rs" }]
    });
    let resp = roundtrip("workspace/willRenameFiles", params).await;
    assert_well_formed(&resp);
}

/// workspace/willDeleteFiles — dispatched; default impl returns method_not_found error.
#[tokio::test(flavor = "current_thread")]
async fn test_workspace_will_delete_files_dispatch() {
    let params = serde_json::json!({
        "files": [{ "uri": "file:///dead.rs" }]
    });
    let resp = roundtrip("workspace/willDeleteFiles", params).await;
    assert_well_formed(&resp);
}

/// workspace/textDocumentContent — dispatched; Batch8Backend returns Ok("batch8-stub").
#[tokio::test(flavor = "current_thread")]
async fn test_workspace_text_document_content_dispatch() {
    let events = Batch8Events::default();
    let (mut tx, mut reader) = batch8_start(events).await;
    let params = serde_json::json!({ "textDocument": { "uri": "file:///batch8.rs" } });
    let resp = batch8_request(
        &mut tx,
        &mut reader,
        "workspace/textDocumentContent",
        params,
    )
    .await;
    assert_eq!(resp.get("id").and_then(|i| i.as_i64()), Some(2));
    assert!(resp.get("error").is_none(), "Unexpected error: {:?}", resp);
    let result = resp.get("result").expect("Expected result field");
    assert_eq!(
        result.get("text").and_then(|t| t.as_str()),
        Some("batch8-stub")
    );
}

/// workspace/didCreateFiles — notification dispatched and captured by handler.
#[tokio::test(flavor = "current_thread")]
async fn test_workspace_did_create_files_dispatch() {
    let events = Batch8Events::default();
    let flag = events.did_create_files.clone();
    let (mut tx, _reader) = batch8_start(events).await;

    let ntf = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "workspace/didCreateFiles",
        "params": { "files": [{ "uri": "file:///created.rs" }] }
    });
    tx.write_all(&encode_message(&ntf)).await.unwrap();

    let got = wait_flag_b8(&flag, "workspace/didCreateFiles").await;
    assert_eq!(got.files.len(), 1);
    assert_eq!(got.files[0].uri, "file:///created.rs");
}

/// workspace/didRenameFiles — notification dispatched and captured by handler.
#[tokio::test(flavor = "current_thread")]
async fn test_workspace_did_rename_files_dispatch() {
    let events = Batch8Events::default();
    let flag = events.did_rename_files.clone();
    let (mut tx, _reader) = batch8_start(events).await;

    let ntf = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "workspace/didRenameFiles",
        "params": { "files": [{ "oldUri": "file:///x.rs", "newUri": "file:///y.rs" }] }
    });
    tx.write_all(&encode_message(&ntf)).await.unwrap();

    let got = wait_flag_b8(&flag, "workspace/didRenameFiles").await;
    assert_eq!(got.files.len(), 1);
    assert_eq!(got.files[0].old_uri, "file:///x.rs");
    assert_eq!(got.files[0].new_uri, "file:///y.rs");
}

/// workspace/didDeleteFiles — notification dispatched and captured by handler.
#[tokio::test(flavor = "current_thread")]
async fn test_workspace_did_delete_files_dispatch() {
    let events = Batch8Events::default();
    let flag = events.did_delete_files.clone();
    let (mut tx, _reader) = batch8_start(events).await;

    let ntf = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "workspace/didDeleteFiles",
        "params": { "files": [{ "uri": "file:///gone.rs" }] }
    });
    tx.write_all(&encode_message(&ntf)).await.unwrap();

    let got = wait_flag_b8(&flag, "workspace/didDeleteFiles").await;
    assert_eq!(got.files.len(), 1);
    assert_eq!(got.files[0].uri, "file:///gone.rs");
}

/// notebookDocument/didOpen — notification dispatched and captured by handler.
#[tokio::test(flavor = "current_thread")]
async fn test_notebook_document_did_open_dispatch() {
    let events = Batch8Events::default();
    let flag = events.did_open_notebook.clone();
    let (mut tx, _reader) = batch8_start(events).await;

    let ntf = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "notebookDocument/didOpen",
        "params": {
            "notebookDocument": {
                "uri": "file:///batch8.ipynb",
                "notebookType": "jupyter",
                "version": 1,
                "metadata": {},
                "cells": []
            },
            "cellTextDocuments": []
        }
    });
    tx.write_all(&encode_message(&ntf)).await.unwrap();

    let got = wait_flag_b8(&flag, "notebookDocument/didOpen").await;
    assert_eq!(got.notebook_document.uri.as_str(), "file:///batch8.ipynb");
    assert_eq!(got.notebook_document.notebook_type, "jupyter");
}

/// notebookDocument/didChange — notification dispatched and captured by handler.
#[tokio::test(flavor = "current_thread")]
async fn test_notebook_document_did_change_dispatch() {
    let events = Batch8Events::default();
    let flag = events.did_change_notebook.clone();
    let (mut tx, _reader) = batch8_start(events).await;

    let ntf = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "notebookDocument/didChange",
        "params": {
            "notebookDocument": {
                "version": 2,
                "uri": "file:///batch8.ipynb"
            },
            "change": {}
        }
    });
    tx.write_all(&encode_message(&ntf)).await.unwrap();

    let got = wait_flag_b8(&flag, "notebookDocument/didChange").await;
    assert_eq!(got.notebook_document.uri.as_str(), "file:///batch8.ipynb");
    assert_eq!(got.notebook_document.version, 2);
}

// ---------------------------------------------------------------------------
// textDocument/inlineValue — dispatch path fires and returns well-formed JSON-RPC
// ---------------------------------------------------------------------------

/// textDocument/inlineValue — dispatched, default impl returns method_not_found
/// or null result. Verifies the dispatch path fires and response is well-formed.
#[tokio::test(flavor = "current_thread")]
async fn test_inline_value_dispatch() {
    let params_typed = lsp::InlineValueParams {
        work_done_progress_params: lsp::WorkDoneProgressParams::default(),
        text_document: lsp::TextDocumentIdentifier {
            uri: lsp::Uri::from_str("file:///test.rs").unwrap(),
        },
        range: lsp::Range {
            start: lsp::Position {
                line: 0,
                character: 0,
            },
            end: lsp::Position {
                line: 20,
                character: 0,
            },
        },
        context: lsp::InlineValueContext {
            frame_id: 1,
            stopped_location: lsp::Range {
                start: lsp::Position {
                    line: 5,
                    character: 0,
                },
                end: lsp::Position {
                    line: 5,
                    character: 0,
                },
            },
        },
    };
    let serialized = serde_json::to_value(&params_typed).unwrap();
    assert!(serialized.is_object());

    let params = serde_json::json!({
        "textDocument": { "uri": "file:///test.rs" },
        "range": {
            "start": { "line": 0, "character": 0 },
            "end":   { "line": 20, "character": 0 }
        },
        "context": {
            "frameId": 1,
            "stoppedLocation": {
                "start": { "line": 5, "character": 0 },
                "end":   { "line": 5, "character": 0 }
            }
        }
    });
    let resp = roundtrip("textDocument/inlineValue", params).await;
    assert_well_formed(&resp);
}
