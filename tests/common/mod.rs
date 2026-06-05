use serde_json::Value;
use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWriteExt};

/// Shared alias for the sender end of a client-side duplex stream.
pub type TxShared = Arc<tokio::sync::Mutex<Option<tokio::io::DuplexStream>>>;

/// Shared alias for the thread-safe JSON-RPC message log.
pub type RxLog = Arc<std::sync::Mutex<Vec<Value>>>;

/// Reads a single frame-based LSP/JSON-RPC message from a stream.
pub async fn read_message<R: AsyncRead + Unpin>(reader: &mut R) -> std::io::Result<Value> {
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
            format!("Invalid Content-Length header: {}", len_line),
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

/// Encodes a JSON value as a well-formed Content-Length prefixed LSP/JSON-RPC message.
pub fn encode_message(msg: &Value) -> Vec<u8> {
    let payload = serde_json::to_string(msg).unwrap();
    format!("Content-Length: {}\r\n\r\n{}", payload.len(), payload).into_bytes()
}

/// Writes a JSON-RPC message to a shared duplex stream.
pub async fn write_msg(tx_shared: &TxShared, msg: Value) {
    let mut guard = tx_shared.lock().await;
    if let Some(ref mut tx) = *guard {
        tx.write_all(&encode_message(&msg)).await.unwrap();
    }
}

/// Waits for a response with a specific message ID from the received log.
pub async fn wait_for_response(received: RxLog, id: i64, timeout: Duration) -> Value {
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

/// Waits for a notification with a specific method from the received log.
#[allow(dead_code)]
pub async fn wait_for_notification(received: RxLog, method: &str, timeout: Duration) -> Value {
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

/// Utility helper to clean up receipt files generated during testing.
pub fn cleanup_receipts() {
    if let Ok(reg) = tower_lsp_max::get_registry().lock() {
        let temp_path = reg.root_path.clone();
        let _ = std::fs::remove_file(temp_path.join("admission.receipt"));
        let _ = std::fs::remove_file(temp_path.join("security.receipt"));
        let _ = std::fs::remove_file(temp_path.join("auth.receipt"));
    }
}
