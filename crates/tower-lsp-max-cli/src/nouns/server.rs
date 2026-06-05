use clap_noun_verb::Result;
use clap_noun_verb_macros::verb;
use serde::{Deserialize, Serialize};
use std::fs;
use std::process::{Command, Stdio};
use tower_lsp_max::jsonrpc::Result as LspResult;
use tower_lsp_max::lsp_types::*;
use tower_lsp_max::{Client, LanguageServer, LspService, Server};

#[derive(Serialize)]
pub struct ServerResult {
    pub success: bool,
    pub message: String,
}

#[derive(Serialize, Deserialize)]
struct ServerState {
    pid: u32,
    port: u16,
}

fn get_state_dir() -> std::path::PathBuf {
    let dir = std::env::temp_dir().join("tower-lsp-max-cli");
    let _ = fs::create_dir_all(&dir);
    dir
}

fn get_server_state_file() -> std::path::PathBuf {
    get_state_dir().join("server.json")
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

#[derive(Debug)]
pub struct CliBackend {
    pub client: Client,
}

#[tower_lsp_max::async_trait]
impl LanguageServer for CliBackend {
    async fn initialize(&self, params: InitializeParams) -> LspResult<InitializeResult> {
        let capabilities = ServerCapabilities {
            text_document_sync: Some(TextDocumentSyncCapability::Kind(TextDocumentSyncKind::FULL)),
            ..ServerCapabilities::default()
        };
        if let Ok(mut reg) = tower_lsp_max::get_registry().lock() {
            reg.client_capabilities = Some(params.capabilities);
            reg.server_capabilities = Some(capabilities.clone());
        }
        Ok(InitializeResult {
            server_info: Some(ServerInfo {
                name: "tower-lsp-max-cli-server".to_string(),
                version: Some("26.6.4".to_string()),
            }),
            capabilities,
        })
    }

    async fn initialized(&self, _: InitializedParams) {
        self.client
            .log_message(MessageType::INFO, "Server initialized")
            .await;
    }

    async fn shutdown(&self) -> LspResult<()> {
        Ok(())
    }
}

pub async fn run_server_daemon(port: u16) -> std::io::Result<()> {
    let addr = format!("127.0.0.1:{}", port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    loop {
        let (stream, _) = listener.accept().await?;
        let (read, write) = tokio::io::split(stream);
        let (service, socket) = LspService::new(|client| CliBackend { client });
        tokio::spawn(async move {
            let _ = Server::new(read, write, socket).serve(service).await;
        });
    }
}

#[verb("start")]
pub fn cmd_start(port: u16) -> Result<ServerResult> {
    start_server(port)
}

fn start_server(port: u16) -> Result<ServerResult> {
    let state_file = get_server_state_file();
    if state_file.exists() {
        if let Ok(content) = fs::read_to_string(&state_file) {
            if let Ok(state) = serde_json::from_str::<ServerState>(&content) {
                if is_pid_running(state.pid) {
                    return Ok(ServerResult {
                        success: false,
                        message: format!(
                            "Server is already running on port {} with PID {}",
                            state.port, state.pid
                        ),
                    });
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
        .env("TOWER_LSP_MAX_RUN_SERVER_DAEMON", port.to_string())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .stdin(Stdio::null())
        .spawn()
        .map_err(|e| clap_noun_verb::NounVerbError::ExecutionError {
            message: format!("Failed to spawn background server: {}", e),
        })?;

    let pid = child.id();
    let state = ServerState { pid, port };
    if let Ok(content) = serde_json::to_string(&state) {
        let _ = fs::write(&state_file, content);
    }

    Ok(ServerResult {
        success: true,
        message: format!("Started server on port {} (PID {})", port, pid),
    })
}

#[verb("stop")]
pub fn cmd_stop() -> Result<ServerResult> {
    stop_server()
}

fn stop_server() -> Result<ServerResult> {
    let state_file = get_server_state_file();
    if !state_file.exists() {
        return Ok(ServerResult {
            success: true,
            message: "Server is not running".to_string(),
        });
    }

    let content = fs::read_to_string(&state_file).map_err(|e| {
        clap_noun_verb::NounVerbError::ExecutionError {
            message: format!("Failed to read server state file: {}", e),
        }
    })?;

    let state = serde_json::from_str::<ServerState>(&content).map_err(|e| {
        clap_noun_verb::NounVerbError::ExecutionError {
            message: format!("Failed to parse server state file: {}", e),
        }
    })?;

    let stopped = kill_pid(state.pid);
    let _ = fs::remove_file(&state_file);

    if stopped {
        Ok(ServerResult {
            success: true,
            message: format!("Server stopped (PID {})", state.pid),
        })
    } else {
        Ok(ServerResult {
            success: true,
            message: format!(
                "Server was not running but cleaned up state file (PID {})",
                state.pid
            ),
        })
    }
}

#[verb("status")]
pub fn cmd_status() -> Result<ServerResult> {
    status_server()
}

fn status_server() -> Result<ServerResult> {
    let state_file = get_server_state_file();
    if !state_file.exists() {
        return Ok(ServerResult {
            success: false,
            message: "Server is inactive".to_string(),
        });
    }

    let content = fs::read_to_string(&state_file).map_err(|e| {
        clap_noun_verb::NounVerbError::ExecutionError {
            message: format!("Failed to read server state file: {}", e),
        }
    })?;

    let state = serde_json::from_str::<ServerState>(&content).map_err(|e| {
        clap_noun_verb::NounVerbError::ExecutionError {
            message: format!("Failed to parse server state file: {}", e),
        }
    })?;

    if is_pid_running(state.pid) {
        Ok(ServerResult {
            success: true,
            message: format!(
                "Server is active on port {} (PID {})",
                state.port, state.pid
            ),
        })
    } else {
        let _ = fs::remove_file(&state_file);
        Ok(ServerResult {
            success: false,
            message: "Server is inactive".to_string(),
        })
    }
}
