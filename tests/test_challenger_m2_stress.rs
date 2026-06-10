use lsp_max::lsp_types::*;
use lsp_max::{LspService, Server};
use lsp_max_playground::handlers::completions::get_completions;
use lsp_max_playground::handlers::diagnostics::get_diagnostics;
use lsp_max_playground::Backend;
use std::str::FromStr;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

// ===========================================================================
// 1. COMPLETIONS CONTEXT BOUNDS CHECKS
// ===========================================================================

#[test]
fn test_completions_extreme_coordinates_bounds() {
    let inputs = vec![
        "",
        "\n",
        "fn main() {}",
        "impl LanguageServer for Backend {\n    async fn shutdown(&self) -> Result<()> {\n        Ok(())\n    }\n}",
    ];

    let positions = vec![
        Position::new(0, 0),
        Position::new(0, 1),
        Position::new(1, 0),
        Position::new(100, 100),
        Position::new(u32::MAX, u32::MAX),
        Position::new(0, u32::MAX),
        Position::new(u32::MAX, 0),
    ];

    for text in &inputs {
        for &pos in &positions {
            let result = std::panic::catch_unwind(|| {
                get_completions(pos, text);
            });
            assert!(
                result.is_ok(),
                "get_completions panicked on position {:?} with text: {:?}",
                pos,
                text
            );
        }
    }
}

// ===========================================================================
// 2. ROBUST #[rpc] ATTRIBUTE PARSING
// ===========================================================================

#[test]
fn test_rpc_attribute_parsing_stress() {
    let test_cases = [
        // Malformed name attributes
        r#"
        #[rpc]
        pub trait TestServer {
            #[rpc(name)]
            async fn foo(&self);
        }
        "#,
        r#"
        #[rpc]
        pub trait TestServer {
            #[rpc(name = 123)]
            async fn foo(&self);
        }
        "#,
        r#"
        #[rpc]
        pub trait TestServer {
            #[rpc(name = "textDocument/completion", flag)]
            async fn foo(&self);
        }
        "#,
        r#"
        #[rpc]
        pub trait TestServer {
            #[rpc(invalid_attr = "yes", name = "textDocument/completion")]
            async fn foo(&self);
        }
        "#,
        // Non-ASCII and emojis
        r#"
        #[rpc]
        pub trait TestServer {
            #[rpc(name = "textDocument/completion🚀")]
            async fn foo(&self);
        }
        "#,
        // Unclosed meta/quotes
        r#"
        #[rpc]
        pub trait TestServer {
            #[rpc(name = "textDocument/completion
        }
        "#,
        // Other attributes mixed in
        r#"
        #[rpc]
        pub trait TestServer {
            #[deprecated]
            #[must_use]
            #[rpc(name = "textDocument/completion")]
            async fn foo(&self);
        }
        "#,
    ];

    let url = Uri::from_str("file:///Users/sac/test.rs").unwrap();
    for (i, text) in test_cases.iter().enumerate() {
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            get_diagnostics(text, &url);
        }));
        assert!(
            result.is_ok(),
            "get_diagnostics panicked on test case {}: {:?}",
            i,
            text
        );
    }
}

// ===========================================================================
// 3. STACK-BASED BRACE PARSING TO PREVENT LEAKAGE
// ===========================================================================

#[test]
fn test_stack_based_brace_parsing_comments_strings() {
    // Comments containing braces
    let text_comment_braces = r#"
    impl LanguageServer for Backend {
        // {
        // Some comments with opening brace
        // }
        async fn shutdown(&self) -> Result<()> {
            Ok(())
        }
    }
    "#;

    // String literals containing braces
    let text_string_braces = r#"
    impl LanguageServer for Backend {
        fn hello() {
            let x = "{";
            let y = "}";
        }
        async fn shutdown(&self) -> Result<()> {
            Ok(())
        }
    }
    "#;

    // Deeply nested/mismatched braces
    let text_mismatched_braces = r#"
    impl LanguageServer for Backend {
        {{{
        async fn shutdown(&self) -> Result<()> {
            Ok(())
        }
        }}
    }
    "#;

    let url = Uri::from_str("file:///Users/sac/test.rs").unwrap();

    for (name, text) in &[
        ("comment_braces", text_comment_braces),
        ("string_braces", text_string_braces),
        ("mismatched_braces", text_mismatched_braces),
    ] {
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            get_diagnostics(text, &url);
        }));
        assert!(
            result.is_ok(),
            "get_diagnostics panicked on brace parsing test: {}",
            name
        );
    }
}

// ===========================================================================
// 4. NESTED DIAGNOSTICS DETECTION
// ===========================================================================

#[test]
fn test_nested_diagnostics_detection_options() {
    let url = Uri::from_str("file:///Users/sac/test.rs").unwrap();

    // 1. Valid nesting
    let text_valid = r#"
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
    let diags_valid = get_diagnostics(text_valid, &url);
    let has_warning_valid = diags_valid.iter().any(|d| {
        d.code == Some(NumberOrString::String("TLM002".to_string()))
            && d.message.contains("did_change_workspace_folders")
        // did_change_workspace_folders requires workspaceFolders capability
    });
    assert!(
        !has_warning_valid,
        "Should NOT trigger warning when capability is correctly declared nested"
    );

    // 2. Missing nested capability (workspaceFolders is None / not declared)
    let text_missing = r#"
    struct Dummy;
    impl LanguageServer for Dummy {
        async fn initialize(&self, params: InitializeParams) -> Result<InitializeResult> {
            Ok(InitializeResult {
                capabilities: ServerCapabilities {
                    workspace: Some(WorkspaceOptions {
                        workspace_folders: None,
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
    let diags_missing = get_diagnostics(text_missing, &url);
    let has_warning_missing = diags_missing.iter().any(|d| {
        d.code == Some(NumberOrString::String("TLM002".to_string()))
            && d.message.contains("did_change_workspace_folders")
    });
    assert!(
        has_warning_missing,
        "Should trigger warning when workspaceFolders capability is None"
    );
}

// ===========================================================================
// 5. BOUNDS CHECKING IN QUICK-FIX CODE ACTIONS
// ===========================================================================

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
async fn test_quick_fix_code_actions_bounds_stress() {
    // Set up the LspService
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
    let doc_uri = "file:///Users/sac/lsp-max/dummy.rs";
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

    // Construct a list of extreme/out-of-bounds diagnostic ranges to test
    let extreme_ranges = vec![
        Range {
            start: Position::new(0, 10000),
            end: Position::new(0, 10000),
        },
        Range {
            start: Position::new(10000, 0),
            end: Position::new(10000, 0),
        },
        Range {
            start: Position::new(u32::MAX, u32::MAX),
            end: Position::new(u32::MAX, u32::MAX),
        },
        Range {
            // End before start
            start: Position::new(5, 5),
            end: Position::new(1, 1),
        },
    ];

    for (idx, range) in extreme_ranges.into_iter().enumerate() {
        let code_action_req = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 10 + idx as i64,
            "method": "textDocument/codeAction",
            "params": {
                "textDocument": {
                    "uri": doc_uri
                },
                "range": range,
                "context": {
                    "diagnostics": [
                        {
                            "range": range,
                            "severity": 1,
                            "code": "TLM005",
                            "source": "lsp-max-playground",
                            "message": "`init` is not a valid method. Did you mean `initialize`?"
                        }
                    ]
                }
            }
        });
        write_msg(&client_tx_shared, code_action_req).await;

        let resp = wait_for_response(
            client_received_responses.clone(),
            10 + idx as i64,
            Duration::from_secs(2),
        )
        .await;

        assert!(
            resp.is_some(),
            "Server did not respond or crashed on out of bounds code action range index: {}",
            idx
        );
    }

    // Cleanup
    client_tx_shared.lock().await.take();
    let _ = client_reader_task.await;

    let server_result = server_handle.await;
    assert!(
        server_result.is_ok(),
        "Server thread should have completed successfully, not panicked"
    );
}
