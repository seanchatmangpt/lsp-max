use crate::client::LanguageClient;
use crate::server_handle::ServerHandle;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::atomic::AtomicU64;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt, BufReader};
use tokio::sync::{mpsc, oneshot, Mutex};

/// Builder for constructing a client connection to a Language Server.
pub struct ClientBuilder {
    // Configuration options could go here
}

impl ClientBuilder {
    /// Create a new ClientBuilder.
    pub fn new() -> Self {
        Self {}
    }

    /// Build the client connection. Takes a type implementing `LanguageClient`
    /// to handle inbound server messages, and the I/O streams for the transport.
    /// `input` is the server's stdout (we read responses from it).
    /// `output` is the server's stdin (we write requests to it).
    /// Returns the `ServerHandle` which can be used to send outbound requests.
    pub fn build<C, I, O>(self, _client: C, input: I, output: O) -> ServerHandle
    where
        C: LanguageClient + Send + 'static,
        I: AsyncRead + Unpin + Send + 'static,
        O: AsyncWrite + Unpin + Send + 'static,
    {
        let pending: Arc<Mutex<HashMap<u64, oneshot::Sender<Value>>>> =
            Arc::new(Mutex::new(HashMap::new()));
        let next_id = Arc::new(AtomicU64::new(1));
        let (tx, rx) = mpsc::channel::<Value>(64);

        let handle = ServerHandle::new(tx, Arc::clone(&pending), Arc::clone(&next_id));

        // Spawn write loop: read from rx, serialize as LSP framing, write to output
        tokio::spawn(write_loop(rx, output));

        // Spawn read loop: read LSP-framed messages from input, dispatch responses/notifications
        tokio::spawn(read_loop(input, Arc::clone(&pending), _client));

        handle
    }
}

impl Default for ClientBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Write loop: dequeues outbound messages and writes them with LSP Content-Length framing.
async fn write_loop<O: AsyncWrite + Unpin>(mut rx: mpsc::Receiver<Value>, mut output: O) {
    while let Some(msg) = rx.recv().await {
        match serde_json::to_vec(&msg) {
            Ok(body) => {
                let header = format!("Content-Length: {}\r\n\r\n", body.len());
                if output.write_all(header.as_bytes()).await.is_err() {
                    break;
                }
                if output.write_all(&body).await.is_err() {
                    break;
                }
            }
            Err(e) => {
                tracing::warn!(
                    "lsp-max-client: failed to serialize outbound message: {}",
                    e
                );
            }
        }
    }
}

/// Read loop: reads LSP-framed messages from input, dispatches responses to pending map
/// and notifications to the LanguageClient handler.
async fn read_loop<I: AsyncRead + Unpin, C: LanguageClient>(
    input: I,
    pending: Arc<Mutex<HashMap<u64, oneshot::Sender<Value>>>>,
    _client: C,
) {
    let mut reader = BufReader::new(input);

    loop {
        // Parse headers: read lines until blank line (\r\n)
        let mut content_length: Option<usize> = None;
        loop {
            let mut line = String::new();
            match reader.read_line(&mut line).await {
                Ok(0) => return, // EOF
                Err(e) => {
                    tracing::debug!("lsp-max-client: read_line error: {}", e);
                    return;
                }
                Ok(_) => {}
            }
            let trimmed = line.trim_end_matches(['\r', '\n']);
            if trimmed.is_empty() {
                // Blank line — end of headers
                break;
            }
            if let Some(rest) = trimmed.strip_prefix("Content-Length:") {
                match rest.trim().parse::<usize>() {
                    Ok(n) => content_length = Some(n),
                    Err(e) => {
                        tracing::warn!("lsp-max-client: bad Content-Length: {}", e);
                    }
                }
            }
        }

        let length = match content_length {
            Some(n) => n,
            None => {
                tracing::warn!("lsp-max-client: no Content-Length header found, skipping");
                continue;
            }
        };

        // Read body
        let mut body = vec![0u8; length];
        if let Err(e) = reader.read_exact(&mut body).await {
            tracing::debug!("lsp-max-client: read_exact error: {}", e);
            return;
        }

        let msg: Value = match serde_json::from_slice(&body) {
            Ok(v) => v,
            Err(e) => {
                tracing::warn!("lsp-max-client: failed to parse JSON body: {}", e);
                continue;
            }
        };

        // Dispatch: response vs notification
        let has_id = msg.get("id").map(|v| !v.is_null()).unwrap_or(false);
        let has_method = msg.get("method").is_some();

        if has_id && !has_method {
            // JSON-RPC response
            if let Some(id) = msg["id"].as_u64() {
                let result = msg.get("result").cloned().unwrap_or(Value::Null);
                if let Some(tx) = pending.lock().await.remove(&id) {
                    let _ = tx.send(result);
                }
            }
        } else if has_method {
            // Notification or server-to-client request
            let method = msg["method"].as_str().unwrap_or("").to_owned();
            tracing::debug!("lsp-max-client: notification from server: {}", method);
            // Future: dispatch to _client trait methods based on method name
        }
    }
}
