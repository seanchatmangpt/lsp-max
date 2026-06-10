use std::sync::Mutex;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tower_lsp_max::jsonrpc::Result;
use tower_lsp_max::lsp_types as lsp;
use tower_lsp_max::{LspService, Server};

// ── StubBackend (used by batches 2-7) ──────────────────────────────────────

pub struct StubBackend;

#[tower_lsp_max::async_trait]
impl tower_lsp_max::LanguageServer for StubBackend {
    async fn initialize(&self, _: lsp::InitializeParams) -> Result<lsp::InitializeResult> {
        Ok(lsp::InitializeResult::default())
    }
    async fn initialized(&self, _: lsp::InitializedParams) {}
    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }
}

// ── Wire helpers ────────────────────────────────────────────────────────────

pub async fn read_message<R: tokio::io::AsyncRead + Unpin>(
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

pub fn encode_message(msg: &serde_json::Value) -> Vec<u8> {
    let payload = serde_json::to_string(msg).unwrap();
    format!("Content-Length: {}\r\n\r\n{}", payload.len(), payload).into_bytes()
}

pub fn td_pos(uri: &str) -> serde_json::Value {
    serde_json::json!({
        "textDocument": { "uri": uri },
        "position":     { "line": 0, "character": 0 }
    })
}

pub fn assert_well_formed(resp: &serde_json::Value) {
    assert_eq!(resp.get("id").and_then(|i| i.as_i64()), Some(2));
    assert!(
        resp.get("result").is_some() || resp.get("error").is_some(),
        "Response had neither result nor error: {:?}",
        resp
    );
}

pub async fn roundtrip(method: &str, params: serde_json::Value) -> serde_json::Value {
    let (service, socket) = LspService::new(|_client| StubBackend);
    let (mut client_tx, server_rx) = tokio::io::duplex(1024 * 1024);
    let (server_tx, mut client_rx) = tokio::io::duplex(1024 * 1024);
    let server = Server::new(server_rx, server_tx, socket);
    tokio::spawn(server.serve(service));

    let init = serde_json::json!({
        "jsonrpc":"2.0","id":1,"method":"initialize","params":{"capabilities":{}}
    });
    client_tx.write_all(&encode_message(&init)).await.unwrap();

    let mut reader = tokio::io::BufReader::new(&mut client_rx);
    let _init_resp = tokio::time::timeout(Duration::from_secs(5), read_message(&mut reader))
        .await
        .expect("timeout waiting for initialize response")
        .unwrap();

    let req = serde_json::json!({
        "jsonrpc":"2.0","id":2,"method":method,"params":params
    });
    client_tx.write_all(&encode_message(&req)).await.unwrap();

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

pub async fn roundtrip_notification_then_shutdown(method: &str, params: serde_json::Value) {
    let (service, socket) = LspService::new(|_client| StubBackend);
    let (mut client_tx, server_rx) = tokio::io::duplex(1024 * 1024);
    let (server_tx, mut client_rx) = tokio::io::duplex(1024 * 1024);
    let server = Server::new(server_rx, server_tx, socket);
    tokio::spawn(server.serve(service));

    let mut reader = tokio::io::BufReader::new(&mut client_rx);

    let init = serde_json::json!({
        "jsonrpc":"2.0","id":1,"method":"initialize","params":{"capabilities":{}}
    });
    client_tx.write_all(&encode_message(&init)).await.unwrap();
    let _init_resp = tokio::time::timeout(Duration::from_secs(5), read_message(&mut reader))
        .await
        .expect("timeout waiting for initialize response")
        .unwrap();

    let notif = serde_json::json!({"jsonrpc":"2.0","method":method,"params":params});
    client_tx.write_all(&encode_message(&notif)).await.unwrap();

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

pub async fn wait_for_response_b1(
    received: std::sync::Arc<Mutex<Vec<serde_json::Value>>>,
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

pub async fn write_msg_b1(
    tx_shared: &std::sync::Arc<tokio::sync::Mutex<Option<tokio::io::DuplexStream>>>,
    msg: serde_json::Value,
) {
    let mut guard = tx_shared.lock().await;
    if let Some(ref mut tx) = *guard {
        tx.write_all(&encode_message(&msg)).await.unwrap();
    }
}
