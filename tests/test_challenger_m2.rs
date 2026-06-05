use std::str::FromStr;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tower_lsp_max::lsp_types::*;
use tower_lsp_max::{LspService, Server};
use tower_lsp_max_playground::handlers::completions::get_completions;
use tower_lsp_max_playground::handlers::diagnostics::get_diagnostics;
use tower_lsp_max_playground::Backend;

#[test]
fn test_completion_empty_file_no_panic() {
    let pos = Position::new(0, 0);
    let result = std::panic::catch_unwind(|| get_completions(pos, ""));
    assert!(
        result.is_ok(),
        "Expected get_completions on empty file to not panic"
    );
    let items = result.unwrap();
    assert!(items.is_empty());
}

#[test]
fn test_completion_out_of_bounds_no_panic() {
    let pos = Position::new(100, 100);
    let result = std::panic::catch_unwind(|| get_completions(pos, "fn main() {}"));
    assert!(
        result.is_ok(),
        "Expected get_completions with out of bounds line to not panic"
    );
    let items = result.unwrap();
    assert!(items.is_empty());
}

#[test]
fn test_completion_multiple_rpc_arguments_success() {
    let text = r#"
#[rpc]
pub trait TestServer {
    async fn foo(&self);
    async fn bar(&self);
    async fn baz(&self);
    #[rpc(flag, name = "textDoc
}
"#;
    let pos = Position::new(6, 29);
    let items = get_completions(pos, text);

    let has_completion = items
        .iter()
        .any(|item| item.label == "textDocument/completion");

    assert!(
        has_completion,
        "Should suggest completion when name is in a list of parameters"
    );
}

#[test]
fn test_completion_server_capabilities_nested_brace_success() {
    let text = r#"
fn get_caps() -> ServerCapabilities {
    ServerCapabilities {
        workspace: Some(WorkspaceServerCapabilities {
            workspace_folders: Some(WorkspaceFoldersServerCapabilities {
                supported: Some(true),
            })
        }),
        comp
    }
}
"#;
    let pos = Position::new(8, 12);
    let items = get_completions(pos, text);

    let has_completion = items.iter().any(|item| item.label == "completion_provider");

    assert!(has_completion, "Should find completion_provider inside ServerCapabilities structure even with nested blocks");

    let text_exact_brace = r#"
fn get_caps() -> ServerCapabilities {
    ServerCapabilities {
        workspace: Some(WorkspaceServerCapabilities {
            workspace_folders: Some(WorkspaceFoldersServerCapabilities {
                supported: Some(true),
            }
            )
        }
        ),
        comp
    }
}
"#;
    let pos_exact = Position::new(11, 12);
    let items_exact = get_completions(pos_exact, text_exact_brace);
    let has_completion_exact = items_exact
        .iter()
        .any(|item| item.label == "completion_provider");
    assert!(
        has_completion_exact,
        "Should find completions even with formatting closing braces on separate lines"
    );
}

#[test]
fn test_completion_outside_impl_block_no_false_positive() {
    let text = r#"
impl LanguageServer for Backend {
    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }
}

fn helper_function() {
    textDoc
}
"#;
    let pos = Position::new(8, 11);
    let items = get_completions(pos, text);

    let has_completion = items
        .iter()
        .any(|item| item.label == "textDocument/completion");

    assert!(
        !has_completion,
        "Should not suggest trait methods when cursor is outside the impl block"
    );
}

#[test]
fn test_nested_capabilities_no_false_positive_warning() {
    let url = Uri::from_str("file:///Users/sac/test.rs").unwrap();
    let text = r#"
struct Dummy;
impl LanguageServer for Dummy {
    async fn initialize(&self, params: InitializeParams) -> Result<InitializeResult> {
        Ok(InitializeResult {
            capabilities: ServerCapabilities {
                workspace: Some(WorkspaceOptions {
                    workspace_folders: Some(WorkspaceFoldersServerCapabilities {
                        supported: Some(true),
                        change_notifications: Some(OneOf::Left(true)),
                    }),
                    ..Default::default()
                }),
                ..Default::default()
            },
            ..Default::default()
        })
    }
    async fn shutdown(&self) -> Result<()> { Ok(()) }
    async fn did_change_workspace_folders(&self, params: DidChangeWorkspaceFoldersParams) {}
}
"#;
    let diags = get_diagnostics(text, &url);

    let false_positive = diags.iter().any(|d| {
        d.code == Some(NumberOrString::String("TLM002".to_string()))
            && d.message.contains("did_change_workspace_folders")
    });

    assert!(
        !false_positive,
        "Should not trigger false positive warning for did_change_workspace_folders when nested capability is declared"
    );
}

// Helper functions for integration test
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
) -> Option<serde_json::Value> {
    let start = std::time::Instant::now();
    loop {
        if start.elapsed() > timeout {
            return None;
        }
        {
            let mut guard = received.lock().unwrap();
            if let Some(pos) = guard
                .iter()
                .position(|msg| msg.get("id").and_then(|i| i.as_i64()) == Some(id))
            {
                return Some(guard.remove(pos));
            }
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
}

async fn wait_for_notification(
    received: Arc<Mutex<Vec<serde_json::Value>>>,
    method: &str,
    timeout: Duration,
) -> Option<serde_json::Value> {
    let start = std::time::Instant::now();
    loop {
        if start.elapsed() > timeout {
            return None;
        }
        {
            let mut guard = received.lock().unwrap();
            if let Some(pos) = guard
                .iter()
                .position(|msg| msg.get("method").and_then(|m| m.as_str()) == Some(method))
            {
                return Some(guard.remove(pos));
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
async fn test_quick_fix_missing_moniker_notebook_sync() {
    // Set up the LspService
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
                Err(_) => break, // EOF/closed
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

    // Initialize
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

    // Send initialized
    let init_ntf = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "initialized",
        "params": {}
    });
    write_msg(&client_tx_shared, init_ntf).await;

    // Open doc with missing moniker capability override
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

    // Wait for diagnostics notification
    let diag_ntf = wait_for_notification(
        client_received_notifications.clone(),
        "textDocument/publishDiagnostics",
        Duration::from_secs(2),
    )
    .await
    .expect("Expected publishDiagnostics");

    let params = diag_ntf.get("params").unwrap();
    let diags = params.get("diagnostics").unwrap().as_array().unwrap();

    // Verify we got TLM002 warning for did_open_notebook_document missing capability
    let warning = diags
        .iter()
        .find(|d| d.get("code").and_then(|c| c.as_str()) == Some("TLM002"));
    assert!(warning.is_some(), "Expected TLM002 warning");

    // Request code action for the warning
    let code_action_req = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "textDocument/codeAction",
        "params": {
            "textDocument": {
                "uri": doc_uri
            },
            "range": warning.unwrap().get("range").unwrap(),
            "context": {
                "diagnostics": [
                    warning.unwrap()
                ]
            }
        }
    });
    write_msg(&client_tx_shared, code_action_req).await;

    let code_action_resp =
        wait_for_response(client_received_responses.clone(), 2, Duration::from_secs(2))
            .await
            .unwrap();
    println!("DEBUG: CODE ACTION RESPONSE = {:#?}", code_action_resp);

    // Check if result exists, if not maybe it's null (e.g. None code actions)
    let result = code_action_resp.get("result");
    if let Some(res_val) = result {
        if !res_val.is_null() {
            let action_result = res_val.as_array().unwrap();
            // The result should contain a quickfix for notebook_document_sync because it is in CAPABILITY_FIELDS
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

    // Shutdown
    let shutdown_req = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 3,
        "method": "shutdown",
        "params": null
    });
    write_msg(&client_tx_shared, shutdown_req).await;
    let _ = wait_for_response(client_received_responses.clone(), 3, Duration::from_secs(2)).await;

    // Exit
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

#[tokio::test(flavor = "current_thread")]
async fn test_code_action_out_of_bounds_no_panic_integration() {
    // 1. Set up the LspService
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
                Err(_) => break, // EOF/closed
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

    // Open a document with very short text
    let doc_uri = "file:///Users/sac/tower-lsp-max/dummy.rs";
    let code_text = "fn main() {}\n";
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

    // Send a code action request with an out-of-bounds range diagnostic
    // Diagnostic has code "TLM005" (misspelled method) and range lines far out of bounds.
    let code_action_req = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "textDocument/codeAction",
        "params": {
            "textDocument": {
                "uri": doc_uri
            },
            "range": {
                "start": { "line": 100, "character": 0 },
                "end": { "line": 100, "character": 0 }
            },
            "context": {
                "diagnostics": [
                    {
                        "range": {
                            "start": { "line": 100, "character": 0 },
                            "end": { "line": 100, "character": 0 }
                        },
                        "severity": 1,
                        "code": "TLM005",
                        "source": "tower-lsp-max-playground",
                        "message": "`init` is not a valid method. Did you mean `initialize`?"
                    }
                ]
            }
        }
    });
    write_msg(&client_tx_shared, code_action_req).await;

    // Wait for response. If it panics/crashes, server_handle will complete or the reader will EOF.
    let resp =
        wait_for_response(client_received_responses.clone(), 2, Duration::from_secs(2)).await;

    // We expect resp to be Some because the server thread did not crash!
    assert!(
        resp.is_some(),
        "Server should have handled out of bounds range gracefully and returned a response"
    );

    // Cleanup
    client_tx_shared.lock().await.take();
    let _ = client_reader_task.await;

    // server_handle should still be running or completed normally, but definitely NOT panicked.
    let server_result = server_handle.await;
    assert!(
        server_result.is_ok(),
        "Server thread should have completed successfully, not panicked"
    );
}

#[test]
fn test_completion_extreme_bounds_safety() {
    // Large line and character indices
    let pos_large = Position::new(u32::MAX, u32::MAX);
    let result_large = std::panic::catch_unwind(|| {
        get_completions(pos_large, "impl LanguageServer for Backend {}")
    });
    assert!(
        result_large.is_ok(),
        "Should not panic on large position bounds"
    );

    // Emojis and ZWJ sequences in text
    let text_unicode = "impl LanguageServer for Backend {\n    // 👨‍👩‍👧‍👦 unicode helper\n    sh\n}";
    let pos_unicode = Position::new(2, 6);
    let result_unicode = std::panic::catch_unwind(|| get_completions(pos_unicode, text_unicode));
    assert!(
        result_unicode.is_ok(),
        "Should not panic on unicode content"
    );
    let items = result_unicode.unwrap();
    assert!(
        items.iter().any(|item| item.label == "shutdown"),
        "Should suggest shutdown completion"
    );
}

#[test]
fn test_rpc_attribute_parsing_adversarial() {
    // Spaces inside attribute list
    let text_spaces = r#"
#[rpc]
pub trait TestServer {
    #[rpc(  name   =   "textDocument/comp
"#;
    let pos_spaces = Position::new(3, 37);
    let completions_spaces = get_completions(pos_spaces, text_spaces);
    assert!(
        completions_spaces
            .iter()
            .any(|item| item.label == "textDocument/completion"),
        "Should suggest completion with spaces in rpc attribute name parameter"
    );

    // Missing trailing quote
    let text_no_quote = r#"
#[rpc]
pub trait TestServer {
    #[rpc(name = "textDocument/comp
"#;
    let pos_no_quote = Position::new(3, 32);
    let completions_no_quote = get_completions(pos_no_quote, text_no_quote);
    assert!(
        completions_no_quote
            .iter()
            .any(|item| item.label == "textDocument/completion"),
        "Should suggest completion without trailing quote"
    );

    // Commented out name
    let text_comment = r#"
#[rpc]
pub trait TestServer {
    #[rpc(
        // name = "ignore",
        name = "textDocument/comp
"#;
    let pos_comment = Position::new(5, 33);
    let completions_comment = get_completions(pos_comment, text_comment);
    assert!(
        completions_comment
            .iter()
            .any(|item| item.label == "textDocument/completion"),
        "Should suggest completion when previous name is commented out"
    );
}

#[test]
fn test_brace_parsing_stack_mismatches() {
    // Unbalanced braces (missing closing brace)
    let text_unclosed = r#"
impl LanguageServer for Backend {
    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }
    sh
"#;
    let pos_unclosed = Position::new(5, 6);
    let completions_unclosed = get_completions(pos_unclosed, text_unclosed);
    assert!(
        completions_unclosed
            .iter()
            .any(|item| item.label == "shutdown"),
        "Should suggest shutdown even if impl block is not closed"
    );

    // Braces in comments - brace parser correctly ignores it
    let text_comments = r#"
impl LanguageServer for Backend {
    // } <- this brace is in comment
    sh
}
"#;
    let pos_comments = Position::new(3, 6);
    let completions_comments = get_completions(pos_comments, text_comments);
    assert!(
        completions_comments
            .iter()
            .any(|item| item.label == "shutdown"),
        "Should suggest shutdown even with closing brace in comments"
    );

    // Braces in string literals - brace parser correctly ignores it
    let text_strings = r#"
impl LanguageServer for Backend {
    let x = "}"; // brace in string literal
    sh
}
"#;
    let pos_strings = Position::new(3, 6);
    let completions_strings = get_completions(pos_strings, text_strings);
    assert!(
        completions_strings
            .iter()
            .any(|item| item.label == "shutdown"),
        "Should suggest shutdown even with closing brace in string literal"
    );

    // Impl keyword in comment - brace parser correctly ignores it
    let text_impl_comment = r#"
// impl LanguageServer for Dummy {
fn test() {
    sh
}
"#;
    let pos_impl_comment = Position::new(3, 6);
    let completions_impl_comment = get_completions(pos_impl_comment, text_impl_comment);
    assert!(
        !completions_impl_comment
            .iter()
            .any(|item| item.label == "shutdown"),
        "Should not suggest shutdown when impl block is in comment"
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

    let init_ntf = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "initialized",
        "params": {}
    });
    write_msg(&client_tx_shared, init_ntf).await;

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

    let code_action_req = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "textDocument/codeAction",
        "params": {
            "textDocument": {
                "uri": doc_uri
            },
            "range": warning.unwrap().get("range").unwrap(),
            "context": {
                "diagnostics": [
                    warning.unwrap()
                ]
            }
        }
    });
    write_msg(&client_tx_shared, code_action_req).await;

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

    // Shutdown
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

    let doc_uri = "file:///Users/sac/tower-lsp-max/dummy_adv_ranges.rs";
    let code_text = "fn main() {}\n";
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

    // Send codeAction with start > end (invalid range)
    let code_action_req_invalid = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "textDocument/codeAction",
        "params": {
            "textDocument": {
                "uri": doc_uri
            },
            "range": {
                "start": { "line": 0, "character": 10 },
                "end": { "line": 0, "character": 5 }
            },
            "context": {
                "diagnostics": [
                    {
                        "range": {
                            "start": { "line": 0, "character": 10 },
                            "end": { "line": 0, "character": 5 }
                        },
                        "severity": 1,
                        "code": "TLM005",
                        "source": "tower-lsp-max-playground",
                        "message": "`init` is not a valid method. Did you mean `initialize`?"
                    }
                ]
            }
        }
    });
    write_msg(&client_tx_shared, code_action_req_invalid).await;
    let resp_invalid =
        wait_for_response(client_received_responses.clone(), 2, Duration::from_secs(2)).await;
    assert!(
        resp_invalid.is_some(),
        "Should respond to invalid range start > end gracefully"
    );

    // Send codeAction with extreme line index (out of bounds)
    let code_action_req_oob = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 3,
        "method": "textDocument/codeAction",
        "params": {
            "textDocument": {
                "uri": doc_uri
            },
            "range": {
                "start": { "line": u32::MAX, "character": u32::MAX },
                "end": { "line": u32::MAX, "character": u32::MAX }
            },
            "context": {
                "diagnostics": [
                    {
                        "range": {
                            "start": { "line": u32::MAX, "character": u32::MAX },
                            "end": { "line": u32::MAX, "character": u32::MAX }
                        },
                        "severity": 1,
                        "code": "TLM005",
                        "source": "tower-lsp-max-playground",
                        "message": "`init` is not a valid method. Did you mean `initialize`?"
                    }
                ]
            }
        }
    });
    write_msg(&client_tx_shared, code_action_req_oob).await;
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
