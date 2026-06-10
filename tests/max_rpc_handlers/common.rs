use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tower_lsp_max::jsonrpc::Result as RpcResult;
use tower_lsp_max::lsp_types as lsp;
use tower_lsp_max::{LanguageServer, LspService, Server};

pub type TxShared = Arc<tokio::sync::Mutex<Option<tokio::io::DuplexStream>>>;
pub type RxLog = Arc<std::sync::Mutex<Vec<serde_json::Value>>>;
pub type SerialGuard = tokio::sync::MutexGuard<'static, ()>;

// Serialise all tests in this module so they don't race on the shared global registry.
pub static TEST_MUTEX: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

pub struct TestBackend;

#[tower_lsp_max::async_trait]
impl LanguageServer for TestBackend {
    async fn initialize(&self, _: lsp::InitializeParams) -> RpcResult<lsp::InitializeResult> {
        Ok(lsp::InitializeResult::default())
    }
    async fn shutdown(&self) -> RpcResult<()> {
        Ok(())
    }
}

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

pub async fn write_msg(tx_shared: &TxShared, msg: serde_json::Value) {
    let mut guard = tx_shared.lock().await;
    if let Some(ref mut tx) = *guard {
        tx.write_all(&encode_message(&msg)).await.unwrap();
    }
}

pub async fn wait_for_response(received: RxLog, id: i64, timeout: Duration) -> serde_json::Value {
    let start = std::time::Instant::now();
    loop {
        if start.elapsed() > timeout {
            panic!("Timeout waiting for response id={}", id);
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

pub async fn boot_server() -> (TxShared, RxLog, tokio::task::JoinHandle<()>, SerialGuard) {
    let _guard = TEST_MUTEX.lock().await;
    tower_lsp_max::reset_registry_for_tests();
    let temp_dir = tempfile::tempdir().unwrap();
    let temp_path = temp_dir.path().to_path_buf();
    std::boxed::Box::leak(std::boxed::Box::new(temp_dir));
    if let Ok(mut reg) = tower_lsp_max::get_registry().lock() {
        reg.root_path = temp_path.clone();
    }
    let _ = std::fs::remove_file(temp_path.join("admission.receipt"));
    let _ = std::fs::remove_file(temp_path.join("security.receipt"));
    let _ = std::fs::remove_file(temp_path.join("auth.receipt"));

    let (service, socket) = LspService::new(|_| TestBackend);

    let (client_tx, server_rx) = tokio::io::duplex(1024 * 1024);
    let (server_tx, client_rx) = tokio::io::duplex(1024 * 1024);

    let server_handle = tokio::spawn(async move {
        let _ = Server::new(server_rx, server_tx, socket)
            .serve(service)
            .await;
    });

    let client_tx_shared: TxShared = Arc::new(tokio::sync::Mutex::new(Some(client_tx)));
    let received: RxLog = Arc::new(std::sync::Mutex::new(Vec::new()));
    let received_clone = received.clone();

    let mut client_rx_owned = client_rx;
    tokio::spawn(async move {
        while let Ok(msg) = read_message(&mut client_rx_owned).await {
            received_clone.lock().unwrap().push(msg);
        }
    });

    write_msg(
        &client_tx_shared,
        serde_json::json!({"jsonrpc":"2.0","id":0,"method":"initialize","params":{"capabilities":{}}}),
    )
    .await;
    wait_for_response(received.clone(), 0, Duration::from_secs(3)).await;

    (client_tx_shared, received, server_handle, _guard)
}

pub fn expect_result(resp: &serde_json::Value) -> &serde_json::Value {
    resp.get("result")
        .unwrap_or_else(|| panic!("expected result, got: {}", resp))
}

pub fn expect_error(resp: &serde_json::Value) -> &serde_json::Value {
    resp.get("error")
        .unwrap_or_else(|| panic!("expected error, got: {}", resp))
}

pub fn cleanup_receipts() {
    if let Ok(reg) = tower_lsp_max::get_registry().lock() {
        let temp_path = reg.root_path.clone();
        let _ = std::fs::remove_file(temp_path.join("admission.receipt"));
        let _ = std::fs::remove_file(temp_path.join("security.receipt"));
        let _ = std::fs::remove_file(temp_path.join("auth.receipt"));
    }
}
