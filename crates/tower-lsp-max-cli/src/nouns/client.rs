use clap_noun_verb::Result;
use clap_noun_verb_macros::verb;
use serde::{Deserialize, Serialize};
use std::fs;
use std::process::{Command, Stdio};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

#[derive(Serialize)]
pub struct ClientResult {
    pub connected: bool,
}

#[derive(Serialize, Deserialize)]
struct ClientState {
    pid: u32,
    url: String,
}

fn get_state_dir() -> std::path::PathBuf {
    let dir = std::env::temp_dir().join("tower-lsp-max-cli");
    let _ = fs::create_dir_all(&dir);
    dir
}

fn get_client_state_file() -> std::path::PathBuf {
    get_state_dir().join("client.json")
}

fn is_pid_running(pid: u32) -> bool {
    if pid == 0 {
        return false;
    }
    let status = Command::new("kill")
        .arg("-0")
        .arg(pid.to_string())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();
    match status {
        Ok(s) => s.success(),
        Err(_) => false,
    }
}

fn kill_pid(pid: u32) -> bool {
    if pid == 0 {
        return false;
    }
    let status = Command::new("kill")
        .arg(pid.to_string())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();
    match status {
        Ok(s) => {
            if s.success() {
                true
            } else {
                let _ = Command::new("kill")
                    .arg("-9")
                    .arg(pid.to_string())
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .status();
                true
            }
        }
        Err(_) => false,
    }
}

fn clean_url(url: &str) -> &str {
    url.strip_prefix("tcp://").unwrap_or(url)
}

async fn read_message<R: tokio::io::AsyncRead + Unpin>(reader: &mut R) -> std::io::Result<String> {
    let mut header = Vec::new();
    loop {
        let mut byte = [0u8; 1];
        reader.read_exact(&mut byte).await?;
        header.push(byte[0]);
        if header.ends_with(b"\r\n\r\n") {
            break;
        }
    }
    let header_str = String::from_utf8_lossy(&header);
    let mut content_length = 0;
    for line in header_str.lines() {
        if line.to_lowercase().starts_with("content-length:") {
            if let Some(val) = line.split(':').nth(1) {
                content_length = val.trim().parse::<usize>().map_err(|_| {
                    std::io::Error::new(std::io::ErrorKind::InvalidData, "Invalid Content-Length")
                })?;
            }
        }
    }
    if content_length == 0 {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "Content-Length not found",
        ));
    }
    let mut body = vec![0u8; content_length];
    reader.read_exact(&mut body).await?;
    String::from_utf8(body)
        .map_err(|_| std::io::Error::new(std::io::ErrorKind::InvalidData, "Invalid UTF-8 body"))
}

async fn write_message<W: tokio::io::AsyncWrite + Unpin>(
    writer: &mut W,
    content: &str,
) -> std::io::Result<()> {
    let msg = format!("Content-Length: {}\r\n\r\n{}", content.len(), content);
    writer.write_all(msg.as_bytes()).await?;
    writer.flush().await?;
    Ok(())
}

pub async fn run_client_daemon(url: &str) -> std::io::Result<()> {
    let addr = clean_url(url);
    let mut stream = TcpStream::connect(addr).await?;
    let (mut read, mut write) = stream.split();

    // Perform LSP Client Handshake
    let init_req = r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"capabilities":{}}}"#;
    write_message(&mut write, init_req).await?;

    // Read response
    let _response = read_message(&mut read).await?;

    // Send initialized notification
    let init_notif = r#"{"jsonrpc":"2.0","method":"initialized","params":{}}"#;
    write_message(&mut write, init_notif).await?;

    // Loop and read incoming messages to keep the connection alive
    loop {
        let _ = read_message(&mut read).await?;
    }
}

#[verb("connect")]
pub fn cmd_connect(url: String) -> Result<ClientResult> {
    connect_client(url)
}

fn connect_client(url: String) -> Result<ClientResult> {
    let state_file = get_client_state_file();
    if state_file.exists() {
        if let Ok(content) = fs::read_to_string(&state_file) {
            if let Ok(state) = serde_json::from_str::<ClientState>(&content) {
                if is_pid_running(state.pid) {
                    return Ok(ClientResult { connected: true });
                }
            }
        }
        let _ = fs::remove_file(&state_file);
    }

    let exe =
        std::env::current_exe().map_err(|e| clap_noun_verb::NounVerbError::ExecutionError {
            message: format!("Failed to get current executable path: {}", e),
        })?;

    let child = Command::new(exe)
        .env("TOWER_LSP_MAX_RUN_CLIENT_DAEMON", &url)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .stdin(Stdio::null())
        .spawn()
        .map_err(|e| clap_noun_verb::NounVerbError::ExecutionError {
            message: format!("Failed to spawn background client: {}", e),
        })?;

    let pid = child.id();
    let state = ClientState {
        pid,
        url: url.clone(),
    };
    if let Ok(content) = serde_json::to_string(&state) {
        let _ = fs::write(&state_file, content);
    }

    // Wait slightly for connection to establish or just return connected: true
    std::thread::sleep(std::time::Duration::from_millis(200));

    if is_pid_running(pid) {
        Ok(ClientResult { connected: true })
    } else {
        let _ = fs::remove_file(&state_file);
        Err(clap_noun_verb::NounVerbError::ExecutionError {
            message: "Failed to establish background client connection".to_string(),
        })
    }
}

#[verb("disconnect")]
pub fn cmd_disconnect() -> Result<ClientResult> {
    disconnect_client()
}

fn disconnect_client() -> Result<ClientResult> {
    let state_file = get_client_state_file();
    if !state_file.exists() {
        return Ok(ClientResult { connected: false });
    }

    let content = fs::read_to_string(&state_file).map_err(|e| {
        clap_noun_verb::NounVerbError::ExecutionError {
            message: format!("Failed to read client state file: {}", e),
        }
    })?;

    let state = serde_json::from_str::<ClientState>(&content).map_err(|e| {
        clap_noun_verb::NounVerbError::ExecutionError {
            message: format!("Failed to parse client state file: {}", e),
        }
    })?;

    let _ = kill_pid(state.pid);
    let _ = fs::remove_file(&state_file);

    Ok(ClientResult { connected: false })
}
