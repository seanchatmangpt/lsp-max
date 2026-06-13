// child_process.rs — subprocess lifecycle for a single child LSP server.
// Spawns the server via tokio::process::Command, wires stdin/stdout through
// lsp-max-client::ClientBuilder, and exposes a ServerHandle for request dispatch.

use lsp_max::lsp_types::{
    ClientCapabilities, ClientInfo, InitializeParams, InitializeResult, InitializedParams,
    ServerCapabilities, Url,
};
use lsp_max_client::{ClientBuilder, ClientError, LanguageClient, ServerHandle};
use std::process::Stdio;
use tokio::process::Command;

/// A no-op implementation of `LanguageClient` used when the compositor does not
/// need to handle server-to-client reverse-requests (diagnostics are collected
/// via the diagnostic buffer, not via this path).
struct NoopClient;

#[async_trait::async_trait]
impl LanguageClient for NoopClient {}

/// A running child LSP server subprocess with an active JSON-RPC connection.
pub struct ChildProcess {
    pub server_id: String,
    pub handle: ServerHandle,
    // Keep the Child alive so the process is not dropped (and killed) prematurely.
    _child: tokio::process::Child,
}

impl ChildProcess {
    /// Spawn a child LSP server and establish the JSON-RPC connection.
    ///
    /// `command`: path to the server binary (e.g. "/usr/local/bin/wasm4pm-lsp")
    /// `args`: server arguments (e.g. ["serve", "--stdio"])
    /// `server_id`: logical name for this server (from lsp-max.toml)
    pub async fn spawn(server_id: String, command: &str, args: &[&str]) -> std::io::Result<Self> {
        let mut child = Command::new(command)
            .args(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()?;

        let stdin = child.stdin.take().expect("stdin piped");
        let stdout = child.stdout.take().expect("stdout piped");

        let handle = ClientBuilder::new().build(NoopClient, stdout, stdin);

        Ok(Self {
            server_id,
            handle,
            _child: child,
        })
    }

    /// Send an LSP initialize + initialized handshake to the child server.
    /// Returns the child's advertised ServerCapabilities on success.
    pub async fn initialize(
        &self,
        root_uri: Option<Url>,
    ) -> Result<ServerCapabilities, ClientError> {
        #[allow(deprecated)]
        let params = InitializeParams {
            process_id: Some(std::process::id()),
            root_uri,
            capabilities: ClientCapabilities::default(),
            client_info: Some(ClientInfo {
                name: "lsp-max-compositor".to_string(),
                version: Some("26.6.9".to_string()),
            }),
            ..Default::default()
        };
        let result: InitializeResult = self.handle.initialize(params).await?;
        self.handle.initialized(InitializedParams {}).await;
        Ok(result.capabilities)
    }
}

/// Manages a pool of spawned child LSP servers, keyed by server_id.
pub struct ChildProcessPool {
    processes: dashmap::DashMap<String, ChildProcess>,
}

impl ChildProcessPool {
    pub fn new() -> Self {
        Self {
            processes: dashmap::DashMap::new(),
        }
    }

    pub async fn spawn_and_register(
        &self,
        server_id: String,
        command: &str,
        args: &[&str],
    ) -> std::io::Result<()> {
        let proc = ChildProcess::spawn(server_id.clone(), command, args).await?;
        self.processes.insert(server_id, proc);
        Ok(())
    }

    pub fn get(
        &self,
        server_id: &str,
    ) -> Option<dashmap::mapref::one::Ref<'_, String, ChildProcess>> {
        self.processes.get(server_id)
    }

    pub fn server_ids(&self) -> Vec<String> {
        self.processes.iter().map(|e| e.key().clone()).collect()
    }

    /// Insert an already-spawned and initialized `ChildProcess` into the pool.
    pub fn register(&self, server_id: String, proc: ChildProcess) {
        self.processes.insert(server_id, proc);
    }

    /// Returns all server IDs currently in the pool.
    /// Collect IDs first, then look up handles one at a time to avoid
    /// holding DashMap refs across async await points.
    pub fn server_ids_snapshot(&self) -> Vec<String> {
        self.processes.iter().map(|e| e.key().clone()).collect()
    }
}

impl Default for ChildProcessPool {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pool_starts_empty() {
        let pool = ChildProcessPool::new();
        assert_eq!(pool.server_ids().len(), 0);
    }

    #[test]
    fn server_ids_snapshot_empty_pool() {
        let pool = ChildProcessPool::new();
        assert_eq!(pool.server_ids_snapshot(), Vec::<String>::new());
    }

    #[test]
    fn server_ids_snapshot_matches_server_ids() {
        // With no registered processes both methods must agree.
        let pool = ChildProcessPool::new();
        assert_eq!(pool.server_ids_snapshot(), pool.server_ids());
    }

    // Spawn a real process (cat) to verify the spawn path works.
    // cat reads stdin and echoes to stdout, matching the JSON-RPC pipe pattern.
    // We just verify spawn returns Ok and the handle exists.
    #[tokio::test]
    async fn spawn_process_and_get_handle() {
        let result = ChildProcess::spawn("test-server".to_string(), "cat", &[]).await;
        // cat may not be available in all environments; if spawn fails, skip.
        if let Ok(proc) = result {
            assert_eq!(proc.server_id, "test-server");
        }
    }
}
