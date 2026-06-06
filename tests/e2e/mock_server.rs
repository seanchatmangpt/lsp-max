use serde_json::{json, Value};
use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::Mutex;

#[derive(Debug, Clone)]
pub struct MockServerState {
    pub custom_capabilities: Value,
    pub hover_response: Option<Value>,
    pub completion_response: Option<Value>,
    pub definition_response: Option<Value>,
    pub references_response: Option<Value>,
    pub rename_response: Option<Value>,
    pub formatting_response: Option<Value>,
    pub code_action_response: Option<Value>,
    pub delay: Duration,
    pub should_crash_on_request: Option<String>,
    pub received_requests: Vec<Value>,
    pub diagnostics_to_send: Vec<Value>,
    pub dynamic_registrations: Vec<Value>,
    pub dynamic_unregistrations: Vec<Value>,
    pub is_dead: bool,
}

impl Default for MockServerState {
    fn default() -> Self {
        Self {
            custom_capabilities: json!({
                "hoverProvider": true,
                "completionProvider": {},
                "definitionProvider": true,
                "referencesProvider": true,
                "textDocumentSync": 1,
                "renameProvider": true,
                "documentFormattingProvider": true
            }),
            hover_response: None,
            completion_response: None,
            definition_response: None,
            references_response: None,
            rename_response: None,
            formatting_response: None,
            code_action_response: None,
            delay: Duration::from_secs(0),
            should_crash_on_request: None,
            received_requests: Vec::new(),
            diagnostics_to_send: Vec::new(),
            dynamic_registrations: Vec::new(),
            dynamic_unregistrations: Vec::new(),
            is_dead: false,
        }
    }
}

pub struct MockServer {
    pub addr: String,
    pub state: Arc<Mutex<MockServerState>>,
    shutdown_tx: Option<tokio::sync::oneshot::Sender<()>>,
}

impl MockServer {
    pub async fn start() -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap().to_string();
        let state = Arc::new(Mutex::new(MockServerState::default()));
        let (shutdown_tx, mut shutdown_rx) = tokio::sync::oneshot::channel::<()>();

        let state_clone = state.clone();
        tokio::spawn(async move {
            loop {
                tokio::select! {
                    accept_res = listener.accept() => {
                        let (stream, _) = match accept_res {
                            Ok(res) => res,
                            Err(_) => break,
                        };
                        let state_conn = state_clone.clone();
                        tokio::spawn(async move {
                            let _ = handle_connection(stream, state_conn).await;
                        });
                    }
                    _ = &mut shutdown_rx => {
                        break;
                    }
                }
            }
            let mut s = state_clone.lock().await;
            s.is_dead = true;
        });

        Self {
            addr,
            state,
            shutdown_tx: Some(shutdown_tx),
        }
    }

    pub fn shutdown(&mut self) {
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(());
        }
    }
}

async fn read_message<R: AsyncReadExt + Unpin>(reader: &mut R) -> std::io::Result<Option<Vec<u8>>> {
    let mut header_buf = Vec::new();
    let mut buf = [0u8; 1];

    loop {
        match reader.read_exact(&mut buf).await {
            Ok(_) => {
                header_buf.push(buf[0]);
                if header_buf.ends_with(b"\r\n\r\n") {
                    break;
                }
                if header_buf.len() > 1024 {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        "Headers too long",
                    ));
                }
            }
            Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => {
                if header_buf.is_empty() {
                    return Ok(None);
                } else {
                    return Err(e);
                }
            }
            Err(e) => return Err(e),
        }
    }

    let header_str = String::from_utf8_lossy(&header_buf);
    let mut content_length = None;
    for line in header_str.split("\r\n") {
        if line.to_lowercase().starts_with("content-length:") {
            let parts: Vec<&str> = line.split(':').collect();
            if parts.len() == 2 {
                if let Ok(len) = parts[1].trim().parse::<usize>() {
                    content_length = Some(len);
                }
            }
        }
    }

    let len = match content_length {
        Some(l) => l,
        None => {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Missing Content-Length header",
            ));
        }
    };

    let mut body = vec![0u8; len];
    reader.read_exact(&mut body).await?;
    Ok(Some(body))
}

async fn write_message<W: AsyncWriteExt + Unpin>(
    writer: &mut W,
    msg: &[u8],
) -> std::io::Result<()> {
    let header = format!("Content-Length: {}\r\n\r\n", msg.len());
    writer.write_all(header.as_bytes()).await?;
    writer.write_all(msg).await?;
    writer.flush().await?;
    Ok(())
}

async fn handle_connection(
    stream: TcpStream,
    state: Arc<Mutex<MockServerState>>,
) -> std::io::Result<()> {
    let (reader, mut writer) = tokio::io::split(stream);
    let mut reader = BufReader::new(reader);
    let (tx, mut rx) = tokio::sync::mpsc::channel::<Vec<u8>>(32);

    // Background writer task
    tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            if let Err(_) = write_message(&mut writer, &msg).await {
                break;
            }
        }
    });

    // Drain initial pre-queued notifications
    {
        let mut s = state.lock().await;
        for diag in s.diagnostics_to_send.drain(..) {
            let msg = serde_json::to_vec(&diag).unwrap();
            let _ = tx.send(msg).await;
        }
        for reg in s.dynamic_registrations.drain(..) {
            let msg = serde_json::to_vec(&reg).unwrap();
            let _ = tx.send(msg).await;
        }
        for unreg in s.dynamic_unregistrations.drain(..) {
            let msg = serde_json::to_vec(&unreg).unwrap();
            let _ = tx.send(msg).await;
        }
    }

    // Background task: continuously poll state for new push notifications
    let state_watch = state.clone();
    let tx_watch = tx.clone();
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
            let mut s = state_watch.lock().await;
            if s.is_dead { break; }
            for diag in s.diagnostics_to_send.drain(..) {
                let msg = serde_json::to_vec(&diag).unwrap();
                if tx_watch.send(msg).await.is_err() { return; }
            }
            for reg in s.dynamic_registrations.drain(..) {
                let msg = serde_json::to_vec(&reg).unwrap();
                if tx_watch.send(msg).await.is_err() { return; }
            }
            for unreg in s.dynamic_unregistrations.drain(..) {
                let msg = serde_json::to_vec(&unreg).unwrap();
                if tx_watch.send(msg).await.is_err() { return; }
            }
        }
    });

    // Main request processing loop
    loop {
        let msg_bytes = match read_message(&mut reader).await {
            Ok(Some(bytes)) => bytes,
            Ok(None) => break,
            Err(_) => break,
        };

        let request: Value = match serde_json::from_slice(&msg_bytes) {
            Ok(val) => val,
            Err(_) => {
                let err_resp = json!({
                    "jsonrpc": "2.0",
                    "error": {
                        "code": -32700,
                        "message": "Parse error"
                    }
                });
                let _ = tx.send(serde_json::to_vec(&err_resp).unwrap()).await;
                continue;
            }
        };

        let method = request.get("method").and_then(|m| m.as_str()).unwrap_or("").to_string();
        let id = request.get("id").cloned();

        let mut s = state.lock().await;
        s.received_requests.push(request.clone());

        if let Some(ref crash_method) = s.should_crash_on_request.clone() {
            if method == *crash_method {
                break;
            }
        }

        if s.delay.as_millis() > 0 {
            let delay = s.delay;
            drop(s);
            tokio::time::sleep(delay).await;
            s = state.lock().await;
        }

        if let Some(id_val) = id {
            let resp = match method.as_str() {
                "initialize" => {
                    json!({
                        "jsonrpc": "2.0",
                        "id": id_val,
                        "result": {
                            "capabilities": s.custom_capabilities
                        }
                    })
                }
                "shutdown" => {
                    json!({
                        "jsonrpc": "2.0",
                        "id": id_val,
                        "result": null
                    })
                }
                "textDocument/hover" => {
                    let res = s.hover_response.clone().unwrap_or(Value::Null);
                    json!({
                        "jsonrpc": "2.0",
                        "id": id_val,
                        "result": res
                    })
                }
                "textDocument/completion" => {
                    let res = s.completion_response.clone().unwrap_or(Value::Null);
                    json!({
                        "jsonrpc": "2.0",
                        "id": id_val,
                        "result": res
                    })
                }
                "textDocument/definition" => {
                    let res = s.definition_response.clone().unwrap_or(Value::Null);
                    json!({
                        "jsonrpc": "2.0",
                        "id": id_val,
                        "result": res
                    })
                }
                "textDocument/references" => {
                    let res = s.references_response.clone().unwrap_or(Value::Null);
                    json!({
                        "jsonrpc": "2.0",
                        "id": id_val,
                        "result": res
                    })
                }
                "textDocument/rename" => {
                    let res = s.rename_response.clone().unwrap_or(Value::Null);
                    json!({
                        "jsonrpc": "2.0",
                        "id": id_val,
                        "result": res
                    })
                }
                "textDocument/formatting" => {
                    let res = s.formatting_response.clone().unwrap_or(Value::Null);
                    json!({
                        "jsonrpc": "2.0",
                        "id": id_val,
                        "result": res
                    })
                }
                "textDocument/codeAction" => {
                    let res = s.code_action_response.clone().unwrap_or(Value::Null);
                    json!({
                        "jsonrpc": "2.0",
                        "id": id_val,
                        "result": res
                    })
                }
                _ => {
                    json!({
                        "jsonrpc": "2.0",
                        "id": id_val,
                        "result": Value::Null
                    })
                }
            };
            let _ = tx.send(serde_json::to_vec(&resp).unwrap()).await;
        }
    }
    Ok(())
}
