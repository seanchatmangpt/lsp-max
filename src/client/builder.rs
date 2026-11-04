use crate::client::server_handle::ServerHandle;
use crate::client::LanguageClient;
use crate::lsp_types::{
    LogMessageParams, ProgressParams, PublishDiagnosticsParams, ShowMessageParams,
};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt, BufReader};
use tokio::sync::{mpsc, oneshot, Mutex};

/// Inbound JSON-RPC id parsed through one enum so string ids are typed, not
/// silently dropped by an `as_u64()` miss. Outbound ids are always numeric
/// (the pending map stays u64-keyed); this enum covers the inbound surface.
#[derive(Debug, Clone, PartialEq)]
enum PendingId {
    Num(u64),
    Str(String),
}

impl PendingId {
    fn from_value(v: &Value) -> Option<PendingId> {
        if let Some(n) = v.as_u64() {
            Some(PendingId::Num(n))
        } else {
            v.as_str().map(|s| PendingId::Str(s.to_owned()))
        }
    }
}

/// Counters for server notifications that carry no LanguageClient handler.
/// Clone + interior mutability so the read loop can share them; surfaced for
/// health/state endpoints instead of vanishing into warn logs.
#[derive(Debug, Default)]
pub struct NotificationCounters {
    /// `$/progress` notifications acknowledged.
    pub progress: AtomicU64,
    /// Notifications (and canned-answerable requests) with no routed handler.
    pub not_routable: AtomicU64,
    /// Notifications whose params failed to deserialize.
    pub malformed: AtomicU64,
}

impl Clone for NotificationCounters {
    fn clone(&self) -> Self {
        NotificationCounters {
            progress: AtomicU64::new(self.progress.load(Ordering::Relaxed)),
            not_routable: AtomicU64::new(self.not_routable.load(Ordering::Relaxed)),
            malformed: AtomicU64::new(self.malformed.load(Ordering::Relaxed)),
        }
    }
}

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

        // Read-loop liveness: flipped false at every read-loop exit so
        // `ServerHandle::request` can fail typed immediately for requests
        // issued AFTER the child died (the pending drain only covers
        // requests already in flight).
        let (alive_tx, alive_rx) = tokio::sync::watch::channel(true);

        let handle = ServerHandle::new(
            tx.clone(),
            Arc::clone(&pending),
            Arc::clone(&next_id),
            alive_rx,
        );

        // Spawn write loop: read from rx, serialize as LSP framing, write to output
        tokio::spawn(write_loop(rx, output));

        // Spawn read loop: read LSP-framed messages from input, dispatch responses/notifications
        tokio::spawn(read_loop(
            input,
            Arc::clone(&pending),
            _client,
            tx,
            Arc::new(NotificationCounters::default()),
            alive_tx,
        ));

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
                // serde_json::Value serialization is infallible by
                // construction; reaching this arm means the Value invariant
                // itself is broken, so fail loud (PG-5 pick: panic with
                // context, not a warn that launders a broken write path).
                panic!("lsp-max-client: outbound Value failed to serialize: {e}");
            }
        }
    }
}

/// Sentinel JSON-RPC error object delivered to every pending request when the
/// read loop dies (stdout EOF or read error). `ServerHandle::request` maps
/// this to a typed `child-exited` failure instead of laundering child death
/// into a null success after the 30 s timeout.
fn transport_closed_error(reason: &str) -> Value {
    serde_json::json!({
        "jsonrpc": "2.0",
        "id": Value::Null,
        "error": {
            "code": -32603,
            "message": "transport closed",
            "data": reason,
        }
    })
}

/// Drain every pending request with the typed transport-closed sentinel.
async fn fail_pending(pending: Arc<Mutex<HashMap<u64, oneshot::Sender<Value>>>>, reason: &str) {
    let mut map = pending.lock().await;
    for (_, tx) in map.drain() {
        let _ = tx.send(transport_closed_error(reason));
    }
}

/// Read loop: reads LSP-framed messages from input, dispatches responses to
/// pending map and notifications to the LanguageClient handler. Server→client
/// requests are answered with canned results on the write path; unroutable
/// notifications increment [`NotificationCounters`].
async fn read_loop<I: AsyncRead + Unpin, C: LanguageClient>(
    input: I,
    pending: Arc<Mutex<HashMap<u64, oneshot::Sender<Value>>>>,
    client: C,
    outbound: mpsc::Sender<Value>,
    counters: Arc<NotificationCounters>,
    alive: tokio::sync::watch::Sender<bool>,
) {
    let mut reader = BufReader::new(input);

    loop {
        // Parse headers: read lines until blank line (\r\n)
        let mut content_length: Option<usize> = None;
        loop {
            let mut line = String::new();
            match reader.read_line(&mut line).await {
                Ok(0) => {
                    let _ = alive.send(false);
                    fail_pending(pending, "stdout EOF: server process closed stdout").await;
                    return;
                }
                Err(e) => {
                    tracing::debug!("lsp-max-client: read_line error: {}", e);
                    let _ = alive.send(false);
                    fail_pending(pending, "stdout read error").await;
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
            let _ = alive.send(false);
            fail_pending(pending, "stdout read_exact error mid-frame").await;
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
            if let Some(id) = PendingId::from_value(&msg["id"]) {
                // Preserve the JSON-RPC `error` object: without this, an error
                // response reaches the proxy as Null and `ServerHandle::request`
                // launders it into Ok(None). The full message is carried through
                // so the error surface stays typed at the handle.
                let result = if msg.get("error").is_some() {
                    msg.clone()
                } else {
                    msg.get("result").cloned().unwrap_or(Value::Null)
                };
                match id {
                    PendingId::Num(n) => {
                        if let Some(tx) = pending.lock().await.remove(&n) {
                            let _ = tx.send(result);
                        }
                    }
                    PendingId::Str(s) => {
                        // String ids never match our outbound numeric ids; a
                        // response with a string id cannot correlate to any
                        // pending request — count, don't correlate.
                        tracing::debug!(
                            "lsp-max-client: response with string id {s} has no pending entry"
                        );
                        counters.not_routable.fetch_add(1, Ordering::Relaxed);
                    }
                }
            }
        } else if has_method {
            // Notification or server-to-client request
            let method = msg["method"].as_str().unwrap_or("").to_owned();
            let params = msg.get("params").cloned().unwrap_or(Value::Null);
            if has_id {
                // Server-to-client request: answer canned on the write path so
                // spec-compliant servers get a response instead of silence.
                let id = msg["id"].clone();
                let response = canned_response(&method, &params);
                if outbound
                    .send(serde_json::json!({
                        "jsonrpc": "2.0",
                        "id": id,
                        "result": response,
                    }))
                    .await
                    .is_err()
                {
                    tracing::warn!(
                        "lsp-max-client: write path closed; cannot answer server request {}",
                        method
                    );
                    counters.not_routable.fetch_add(1, Ordering::Relaxed);
                }
            } else {
                dispatch_notification(&client, &method, params, &counters).await;
            }
        }
    }
}

/// Canned result for a server→client request. `workspace/configuration` returns
/// one `Null` per requested item (the lawful "client supports no per-item
/// config" answer); every other method gets a `Null` result.
fn canned_response(method: &str, params: &Value) -> Value {
    match method {
        "workspace/configuration" => {
            let items = params
                .get("items")
                .and_then(|v| v.as_array())
                .map(|a| a.len())
                .unwrap_or(0);
            Value::Array(vec![Value::Null; items])
        }
        _ => Value::Null,
    }
}

/// Route a server notification to the `LanguageClient` handler for its method.
/// Child diagnostics pass through unchanged — the compositor's attribution
/// (andon prefixes) is applied upstream and must not be rewritten here.
async fn dispatch_notification<C: LanguageClient>(
    client: &C,
    method: &str,
    params: Value,
    counters: &NotificationCounters,
) {
    match method {
        "textDocument/publishDiagnostics" => {
            match serde_json::from_value::<PublishDiagnosticsParams>(params) {
                Ok(p) => client.publish_diagnostics(p).await,
                Err(e) => {
                    counters.malformed.fetch_add(1, Ordering::Relaxed);
                    tracing::warn!(
                        "lsp-max-client: malformed textDocument/publishDiagnostics params: {}",
                        e
                    );
                }
            }
        }
        "window/logMessage" => match serde_json::from_value::<LogMessageParams>(params) {
            Ok(p) => client.log_message(p).await,
            Err(e) => {
                counters.malformed.fetch_add(1, Ordering::Relaxed);
                tracing::warn!("lsp-max-client: malformed window/logMessage params: {}", e);
            }
        },
        "window/showMessage" => match serde_json::from_value::<ShowMessageParams>(params) {
            Ok(p) => client.show_message(p).await,
            Err(e) => {
                counters.malformed.fetch_add(1, Ordering::Relaxed);
                tracing::warn!("lsp-max-client: malformed window/showMessage params: {}", e);
            }
        },
        "$/progress" => match serde_json::from_value::<ProgressParams>(params) {
            Ok(_) => {
                counters.progress.fetch_add(1, Ordering::Relaxed);
            }
            Err(e) => {
                counters.malformed.fetch_add(1, Ordering::Relaxed);
                tracing::warn!("lsp-max-client: malformed $/progress params: {}", e);
            }
        },
        "telemetry/event" => client.telemetry_event(params).await,
        _ => {
            counters.not_routable.fetch_add(1, Ordering::Relaxed);
            tracing::debug!("lsp-max-client: unknown notification dropped: {}", method);
        }
    }
}
