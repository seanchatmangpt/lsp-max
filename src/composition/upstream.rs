//! Upstream connection types for composition.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use serde_json::{json, Value};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::sync::{broadcast, oneshot, Mutex};

#[derive(Debug, Clone)]
pub struct UpstreamNotification {
    pub source_id: String,
    pub message: serde_json::Value,
}

// ── Persistent Upstream Connection ─────────────────────────────────────────────

/// Reads a single LSP message from a byte reader (header + body).
async fn read_lsp_message<R: AsyncReadExt + Unpin>(
    reader: &mut R,
) -> std::io::Result<Option<Vec<u8>>> {
    let mut header_buf = Vec::new();
    let mut byte = [0u8; 1];
    loop {
        match reader.read_exact(&mut byte).await {
            Ok(_) => {
                header_buf.push(byte[0]);
                if header_buf.ends_with(b"\r\n\r\n") {
                    break;
                }
                if header_buf.len() > 4096 {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        "Header too long",
                    ));
                }
            }
            Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => return Ok(None),
            Err(e) => return Err(e),
        }
    }
    let header_str = String::from_utf8_lossy(&header_buf);
    let content_length: usize = header_str
        .lines()
        .find(|l| l.starts_with("Content-Length:"))
        .and_then(|l| l.split(':').nth(1))
        .and_then(|v| v.trim().parse().ok())
        .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::InvalidData, "No Content-Length"))?;
    let mut body = vec![0u8; content_length];
    reader.read_exact(&mut body).await?;
    Ok(Some(body))
}

/// Writes a single LSP message to a byte writer (with Content-Length header).
async fn write_lsp_message<W: AsyncWriteExt + Unpin>(
    writer: &mut W,
    body: &[u8],
) -> std::io::Result<()> {
    let header = format!("Content-Length: {}\r\n\r\n", body.len());
    writer.write_all(header.as_bytes()).await?;
    writer.write_all(body).await?;
    writer.flush().await
}

type PendingMap = Arc<Mutex<HashMap<i64, oneshot::Sender<Value>>>>;

/// A persistent connection to an upstream LSP server.
/// Maintains a background reader that forwards:
/// - Responses to pending requests (correlated by id)
/// - Server-initiated notifications/requests (no id or id not pending) → `unsolicited_tx`
pub struct PersistentUpstream {
    pub source_id: String,
    pub address: String,
    write_tx: tokio::sync::mpsc::Sender<Vec<u8>>,
    pending: PendingMap,
    _unsolicited_rx: Arc<Mutex<broadcast::Receiver<UpstreamNotification>>>,
    next_id: Arc<Mutex<i64>>,
}

impl std::fmt::Debug for PersistentUpstream {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PersistentUpstream")
            .field("source_id", &self.source_id)
            .field("address", &self.address)
            .finish_non_exhaustive()
    }
}

impl PersistentUpstream {
    /// Connect to `address` and start background reader/writer tasks.
    /// Returns `None` if connection fails within `timeout_ms`.
    pub async fn connect(
        source_id: String,
        address: String,
        timeout_ms: u64,
        unsolicited_tx: broadcast::Sender<UpstreamNotification>,
    ) -> Option<Self> {
        let stream = tokio::time::timeout(
            Duration::from_millis(timeout_ms),
            TcpStream::connect(&address),
        )
        .await
        .ok()?
        .ok()?;

        let (reader_half, writer_half) = tokio::io::split(stream);
        let pending: PendingMap = Arc::new(Mutex::new(HashMap::new()));
        let (write_tx, mut write_rx) = tokio::sync::mpsc::channel::<Vec<u8>>(64);

        // Background writer task
        let mut writer_half = writer_half;
        let source_id_writer = source_id.clone();
        tokio::spawn(async move {
            while let Some(msg) = write_rx.recv().await {
                if let Ok(msg_str) = String::from_utf8(msg.clone()) {
                    tracing::trace!(
                        "--- PersistentUpstream [{}] writing: {}",
                        source_id_writer,
                        msg_str.trim()
                    );
                }
                if write_lsp_message(&mut writer_half, &msg).await.is_err() {
                    tracing::trace!("--- PersistentUpstream [{}] write failed", source_id_writer);
                    break;
                }
            }
        });

        // Background reader task
        let pending_clone = pending.clone();
        let unsolicited_tx_clone = unsolicited_tx.clone();
        let mut reader_half = reader_half;
        let source_id_clone = source_id.clone();
        tokio::spawn(async move {
            loop {
                match read_lsp_message(&mut reader_half).await {
                    Ok(Some(bytes)) => {
                        let msg: Value = match serde_json::from_slice(&bytes) {
                            Ok(v) => v,
                            Err(_) => continue,
                        };
                        tracing::trace!(
                            "--- PersistentUpstream [{}] read message: {}",
                            source_id_clone,
                            msg
                        );
                        // Check if this is a response to a pending request
                        if let Some(id) = msg.get("id").and_then(|v| v.as_i64()) {
                            let mut map = pending_clone.lock().await;
                            if let Some(tx) = map.remove(&id) {
                                tracing::trace!(
                                    "--- PersistentUpstream [{}] matched pending response id: {}",
                                    source_id_clone,
                                    id
                                );
                                let _ = tx.send(msg);
                                continue;
                            }
                        }
                        // Not a matched response: it's an unsolicited notification or request
                        tracing::trace!(
                            "--- PersistentUpstream [{}] forwarding unsolicited message",
                            source_id_clone
                        );
                        let _ = unsolicited_tx_clone.send(UpstreamNotification {
                            source_id: source_id_clone.clone(),
                            message: msg,
                        });
                    }
                    _ => {
                        tracing::trace!(
                            "--- PersistentUpstream [{}] read loop terminated",
                            source_id_clone
                        );
                        break; // Connection closed or error
                    }
                }
            }
        });

        let _unsolicited_rx = Arc::new(Mutex::new(unsolicited_tx.subscribe()));
        Some(Self {
            source_id,
            address,
            write_tx,
            pending,
            _unsolicited_rx,
            next_id: Arc::new(Mutex::new(1)),
        })
    }

    /// Send a request and wait for its response.
    pub async fn request(
        &self,
        method: &str,
        params: Value,
        timeout_ms: u64,
    ) -> std::result::Result<Value, String> {
        let id = {
            let mut n = self.next_id.lock().await;
            let id = *n;
            *n += 1;
            id
        };
        let req = json!({"jsonrpc":"2.0","id":id,"method":method,"params":params});
        let body = serde_json::to_vec(&req).map_err(|e| e.to_string())?;
        let (resp_tx, resp_rx) = oneshot::channel();
        self.pending.lock().await.insert(id, resp_tx);
        tracing::trace!(
            "--- PersistentUpstream [{}] sending request id: {}, method: {}",
            self.source_id,
            id,
            method
        );
        self.write_tx.send(body).await.map_err(|e| e.to_string())?;
        tracing::trace!(
            "--- PersistentUpstream [{}] awaiting response for id: {}",
            self.source_id,
            id
        );
        let resp = tokio::time::timeout(Duration::from_millis(timeout_ms), resp_rx)
            .await
            .map_err(|_| {
                format!(
                    "Timeout waiting for response to {method} from {}",
                    self.address
                )
            })?
            .map_err(|_| "Response channel closed".to_string())?;
        tracing::trace!(
            "--- PersistentUpstream [{}] got response for id: {}",
            self.source_id,
            id
        );
        if let Some(err) = resp.get("error") {
            return Err(format!("Upstream error: {err}"));
        }
        Ok(resp.get("result").cloned().unwrap_or(Value::Null))
    }

    /// Send a notification (no response expected).
    pub async fn notify(&self, method: &str, params: Value) -> std::result::Result<(), String> {
        let notif = json!({"jsonrpc":"2.0","method":method,"params":params});
        let body = serde_json::to_vec(&notif).map_err(|e| e.to_string())?;
        tracing::trace!(
            "--- PersistentUpstream [{}] sending notification method: {}",
            self.source_id,
            method
        );
        self.write_tx.send(body).await.map_err(|e| e.to_string())
    }

    pub async fn send_raw(&self, msg: serde_json::Value) -> std::result::Result<(), String> {
        let body = serde_json::to_vec(&msg).map_err(|e| e.to_string())?;
        tracing::trace!(
            "--- PersistentUpstream [{}] sending raw msg: {}",
            self.source_id,
            msg
        );
        self.write_tx.send(body).await.map_err(|e| e.to_string())
    }
}
