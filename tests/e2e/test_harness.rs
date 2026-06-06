use crate::mock_server::MockServer;
use serde_json::{json, Value};
use tower_lsp_max::{ComposedServer, LspService, Server};

async fn read_message<R: tokio::io::AsyncReadExt + Unpin>(
    reader: &mut R,
) -> std::io::Result<Option<Vec<u8>>> {
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

async fn write_message<W: tokio::io::AsyncWriteExt + Unpin>(
    writer: &mut W,
    msg: &[u8],
) -> std::io::Result<()> {
    let header = format!("Content-Length: {}\r\n\r\n", msg.len());
    writer.write_all(header.as_bytes()).await?;
    writer.write_all(msg).await?;
    writer.flush().await?;
    Ok(())
}

pub struct TestClient {
    pub stream: tokio::io::DuplexStream,
    next_id: i64,
    pub queued_messages: std::collections::VecDeque<Value>,
}

impl TestClient {
    pub fn new(stream: tokio::io::DuplexStream) -> Self {
        Self {
            stream,
            next_id: 1,
            queued_messages: std::collections::VecDeque::new(),
        }
    }

    pub async fn send_request(&mut self, method: &str, params: Value) -> Value {
        let id = self.next_id;
        self.next_id += 1;
        let req = json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": method,
            "params": params
        });
        let msg = serde_json::to_vec(&req).unwrap();
        write_message(&mut self.stream, &msg).await.unwrap();

        loop {
            let resp_bytes = read_message(&mut self.stream).await.unwrap().expect("EOF");
            let resp: Value = serde_json::from_slice(&resp_bytes).unwrap();
            if resp.get("id") == Some(&json!(id)) {
                return resp;
            } else {
                self.queued_messages.push_back(resp);
            }
        }
    }

    pub async fn write_request(&mut self, method: &str, params: Value) -> i64 {
        let id = self.next_id;
        self.next_id += 1;
        let req = json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": method,
            "params": params
        });
        let msg = serde_json::to_vec(&req).unwrap();
        write_message(&mut self.stream, &msg).await.unwrap();
        id
    }

    pub async fn read_response(&mut self, id: i64) -> Value {
        let mut idx = None;
        for (i, msg) in self.queued_messages.iter().enumerate() {
            if msg.get("id") == Some(&json!(id)) {
                idx = Some(i);
                break;
            }
        }
        if let Some(i) = idx {
            return self.queued_messages.remove(i).unwrap();
        }

        loop {
            let resp_bytes = read_message(&mut self.stream).await.unwrap().expect("EOF");
            let resp: Value = serde_json::from_slice(&resp_bytes).unwrap();
            if resp.get("id") == Some(&json!(id)) {
                return resp;
            } else {
                self.queued_messages.push_back(resp);
            }
        }
    }

    pub async fn send_notification(&mut self, method: &str, params: Value) {
        let req = json!({
            "jsonrpc": "2.0",
            "method": method,
            "params": params
        });
        let msg = serde_json::to_vec(&req).unwrap();
        write_message(&mut self.stream, &msg).await.unwrap();
    }

    pub async fn read_message(&mut self) -> Option<Value> {
        if let Some(msg) = self.queued_messages.pop_front() {
            return Some(msg);
        }
        let bytes = read_message(&mut self.stream).await.ok()??;
        serde_json::from_slice(&bytes).ok()
    }
}

pub struct TestHarness {
    pub mock_servers: Vec<MockServer>,
    pub client: TestClient,
}

impl TestHarness {
    pub async fn new(num_mock_servers: usize) -> Self {
        let mut mock_servers = Vec::new();
        let mut upstreams: Vec<(String, String)> = Vec::new();
        for i in 0..num_mock_servers {
            let mock = MockServer::start().await;
            upstreams.push((format!("mock-{}", i), mock.addr.to_string()));
            mock_servers.push(mock);
        }

        let (client_io, server_io) = tokio::io::duplex(1024 * 1024);
        let (service, socket) = LspService::new(|client| ComposedServer::new(client, upstreams.clone()));
        let (reader, writer) = tokio::io::split(server_io);
        tokio::spawn(async move {
            let _ = Server::new(reader, writer, socket).serve(service).await;
        });

        let client = TestClient::new(client_io);

        Self {
            mock_servers,
            client,
        }
    }

    pub fn shutdown(mut self) {
        for mock in &mut self.mock_servers {
            mock.shutdown();
        }
    }
}
