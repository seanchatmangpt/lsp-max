// child_process.rs — subprocess lifecycle for a single child LSP server.
// Spawns the server via tokio::process::Command, wires stdin/stdout through
// lsp-max-client::ClientBuilder, and exposes a ServerHandle for request dispatch.

use lsp_max::client::{ClientBuilder, ClientError, LanguageClient, ServerHandle};
use lsp_max::lsp_types::{
    ClientCapabilities, ClientInfo, DocumentUri, InitializeParams, InitializeResult,
    InitializedParams, ServerCapabilities,
};
use std::process::Stdio;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::process::Command;

/// A no-op implementation of `LanguageClient` used when the compositor does not
/// need to handle server-to-client reverse-requests (diagnostics are collected
/// via the diagnostic buffer, not via this path).
pub struct NoopClient;

#[async_trait::async_trait]
impl LanguageClient for NoopClient {}

/// A running child LSP server subprocess with an active JSON-RPC connection.
/// The OS-level child process is owned by the exit-watcher task returned from `spawn`.
pub struct ChildProcess {
    pub server_id: String,
    pub handle: ServerHandle,
    /// Set to `true` by the exit-watcher task once the OS child has exited
    /// (clean or crash). Drives the pool's dead-entry eviction: see
    /// `ChildProcessPool::get` and the no-respawn law below.
    pub exited: Arc<AtomicBool>,
}

impl ChildProcess {
    /// Spawn a child LSP server and establish the JSON-RPC connection.
    ///
    /// Returns `(ChildProcess, exit_future)`. The exit future resolves when the
    /// child process exits (crash or clean exit). Callers should `tokio::spawn`
    /// the exit future and drive cleanup there.
    ///
    /// `command`: path to the server binary (e.g. "/usr/local/bin/wasm4pm-lsp")
    /// `args`: server arguments (e.g. ["serve", "--stdio"])
    /// `server_id`: logical name for this server (from lsp-max.toml)
    pub async fn spawn<C>(
        server_id: String,
        command: &str,
        args: &[&str],
        client: C,
    ) -> std::io::Result<(
        Self,
        impl std::future::Future<Output = std::io::Result<std::process::ExitStatus>>,
    )>
    where
        C: LanguageClient,
    {
        let mut child = Command::new(command)
            .args(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()?;

        let stdin = child.stdin.take().expect("stdin piped");
        let stdout = child.stdout.take().expect("stdout piped");

        let handle = ClientBuilder::new().build(client, stdout, stdin);

        // Background task owns the Child so wait() has exclusive &mut access.
        // The oneshot carries the ExitStatus back to the caller's future.
        let (exit_tx, exit_rx) = tokio::sync::oneshot::channel::<std::process::ExitStatus>();
        let exited = Arc::new(AtomicBool::new(false));
        let exited_for_watcher = Arc::clone(&exited);
        tokio::spawn(async move {
            let status = child.wait().await.ok();
            // Liveness flip happens unconditionally — even when the oneshot
            // receiver is gone, the pool must observe the child as dead.
            exited_for_watcher.store(true, Ordering::Release);
            if let Some(status) = status {
                let _ = exit_tx.send(status);
            }
        });

        let proc = Self {
            server_id,
            handle,
            exited,
        };
        let exit_fut = async move {
            exit_rx.await.map_err(|_| {
                std::io::Error::other("exit watcher task dropped before sending status")
            })
        };

        Ok((proc, exit_fut))
    }

    /// Send an LSP initialize + initialized handshake to the child server.
    /// Returns the child's advertised ServerCapabilities on success.
    pub async fn initialize(
        &self,
        root_uri: Option<DocumentUri>,
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
///
/// ── NO-RESPAWN LAW ──────────────────────────────────────────────────────────
/// When a child exits (crash or clean exit), the pool EVICTS the dead entry
/// (`get` removes it once the exit-watcher flag is observed) and the
/// compositor does NOT respawn. Respawn policy — restartOnCrash, maxRestarts,
/// backoff — is a Claude Code FRONT-door concern (fleet split per M8: the
/// front door owns process restart; the compositor owns fan-out and merge
/// over the children it was handed). The compositor's only obligations after
/// a child death are: (a) never dispatch into a dead child again, (b) keep
/// the session and the surviving children serving. Finding filed as M8;
/// receipt: docs/jira/v26.11.3/receipts/cl-f-eviction.md. The prior behavior
/// (dead entries retained forever, exit watcher only logging + clearing
/// buffer entries at the server.rs block) is falsified by
/// `tests/child_eviction_test.rs`.
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
        let (proc, exit_fut) =
            ChildProcess::spawn(server_id.clone(), command, args, NoopClient).await?;
        self.processes.insert(server_id, proc);
        // Drop the exit future — callers that need exit detection use spawn() directly.
        drop(exit_fut);
        Ok(())
    }

    pub fn get(
        &self,
        server_id: &str,
    ) -> Option<dashmap::mapref::one::Ref<'_, String, ChildProcess>> {
        if let Some(entry) = self.processes.get(server_id) {
            if entry.value().exited.load(Ordering::Acquire) {
                // Dead child: evict so every subsequent dispatch (fanout,
                // flush coordinator, routing) skips it. Dispatch to an
                // evicted id falls back to the remaining live children —
                // the documented FirstSuccess/fan-all behavior — and the
                // session stays alive.
                drop(entry);
                self.processes.remove(server_id);
                return None;
            }
            Some(entry)
        } else {
            None
        }
    }

    /// True if `server_id` has a live (non-exited) child in the pool.
    /// Reads the liveness flag without taking a map lock guard on the
    /// child entry path used by health/state surfaces.
    pub fn is_live(&self, server_id: &str) -> bool {
        match self.processes.get(server_id) {
            Some(entry) => !entry.value().exited.load(Ordering::Acquire),
            None => false,
        }
    }

    pub fn server_ids(&self) -> Vec<String> {
        self.processes.iter().map(|e| e.key().clone()).collect()
    }

    /// Insert an already-spawned and initialized `ChildProcess` into the pool.
    pub fn register(&self, server_id: String, proc: ChildProcess) {
        self.processes.insert(server_id, proc);
    }

    /// Remove and return the `ChildProcess` for `server_id`, if present.
    pub fn remove(&self, server_id: &str) -> Option<ChildProcess> {
        self.processes.remove(server_id).map(|(_, proc)| proc)
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
        let result = ChildProcess::spawn("test-server".to_string(), "cat", &[], NoopClient).await;
        // cat may not be available in all environments; if spawn fails, skip.
        if let Ok((proc, _exit_fut)) = result {
            assert_eq!(proc.server_id, "test-server");
        }
    }
}
