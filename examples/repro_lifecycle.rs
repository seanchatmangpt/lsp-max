
use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tower_lsp_max::{LanguageServer, LspService, Server};
use tower_lsp_max::jsonrpc::Result as RpcResult;
use tower_lsp_max::lsp_types as lsp;

struct TestBackend;

#[tower_lsp_max::async_trait]
impl LanguageServer for TestBackend {
    async fn initialize(&self, _: lsp::InitializeParams) -> RpcResult<lsp::InitializeResult> {
        Ok(lsp::InitializeResult::default())
    }
    async fn shutdown(&self) -> RpcResult<()> {
        Ok(())
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

#[tokio::main]
async fn main() {
    tower_lsp_max::reset_registry_for_tests();
    let (service, socket) = LspService::new(|_| TestBackend);

    let (mut client_tx, server_rx) = tokio::io::duplex(1024 * 1024);
    let (server_tx, mut client_rx) = tokio::io::duplex(1024 * 1024);

    tokio::spawn(async move {
        let _ = Server::new(server_rx, server_tx, socket)
            .serve(service)
            .await;
    });

    // Test max/snapshot BEFORE initialization
    let req = serde_json::json!({"jsonrpc":"2.0","id":1,"method":"max/snapshot"});
    client_tx.write_all(&encode_message(&req)).await.unwrap();
    
    let resp = read_message(&mut client_rx).await.unwrap();
    println!("max/snapshot response before init: {}", resp);

    // Test max/explainDiagnostic BEFORE initialization
    let req = serde_json::json!({"jsonrpc":"2.0","id":2,"method":"max/explainDiagnostic","params":"diag-missing-receipt"});
    client_tx.write_all(&encode_message(&req)).await.unwrap();
    
    let resp = read_message(&mut client_rx).await.unwrap();
    println!("max/explainDiagnostic response before init: {}", resp);
}
