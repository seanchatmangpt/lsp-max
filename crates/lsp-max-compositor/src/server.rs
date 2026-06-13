use crate::child_process::ChildProcessPool;
use crate::connections::ChildConnections;
use crate::diagnostic_buffer::DiagnosticBuffer;
use crate::flush_coordinator::FlushCoordinator;
use crate::{CompositorConfig, ExtensionRouter, MergeContext};
use lsp_max::jsonrpc::Result;
use lsp_max::lsp_types::*;
use lsp_max::{Client, LspService, Server};
use lsp_max_client::ServerHandle as ChildServerHandle;
use std::sync::{Arc, RwLock};

pub struct CompositorServer {
    client: Client,
    router: ExtensionRouter,
    #[allow(dead_code)]
    merge_ctx: Arc<MergeContext>,
    connections: Arc<ChildConnections>,
    buffer: Arc<DiagnosticBuffer>,
    pool: Arc<ChildProcessPool>,
    config: Arc<CompositorConfig>,
    /// Shared FlushCoordinator — used by the exit watcher to push empty diagnostics
    /// to the editor after a child process exits.
    flush_coord: Arc<FlushCoordinator>,
    /// Merged ServerCapabilities stored after initialize() completes.
    /// None until the first initialize() call returns.
    merged_capabilities: Arc<RwLock<Option<lsp_max::lsp_types::ServerCapabilities>>>,
}

/// Extract the file extension (without leading dot) from a URI string.
/// Returns an empty string if no extension is found.
fn ext_from_uri(uri: &str) -> String {
    uri.rsplit('/')
        .next()
        .and_then(|name| name.rsplit('.').next().filter(|_| name.contains('.')))
        .unwrap_or("")
        .to_string()
}

#[lsp_max::async_trait]
impl lsp_max::LanguageServer for CompositorServer {
    async fn initialize(&self, params: InitializeParams) -> Result<InitializeResult> {
        #[allow(deprecated)]
        let root_uri = params.root_uri.clone();

        // Collect (tier, ServerCapabilities) pairs from each spawned child.
        let mut child_capabilities: Vec<(
            crate::registry::ChildTier,
            lsp_max::lsp_types::ServerCapabilities,
        )> = Vec::new();

        for entry in &self.config.server {
            if let Some(cmd) = &entry.command {
                let eff_args = entry.effective_args();
                let args: Vec<&str> = eff_args.iter().map(|s| s.as_str()).collect();
                match crate::child_process::ChildProcess::spawn(entry.id.clone(), cmd, &args).await
                {
                    Ok((proc, exit_fut)) => {
                        let tier = crate::registry::ChildTier::from_priority(&entry.priority);
                        match proc.initialize(root_uri.clone()).await {
                            Ok(caps) => {
                                tracing::info!(server_id = %entry.id, "compositor: child ADMITTED");
                                child_capabilities.push((tier, caps));
                            }
                            Err(e) => {
                                tracing::warn!(
                                    server_id = %entry.id,
                                    error = %e,
                                    "compositor: child initialize BLOCKED"
                                );
                            }
                        }
                        self.pool.register(entry.id.clone(), proc);

                        // Spawn exit watcher: clear buffer entries and push empty diagnostics
                        // to the editor for all URIs this server owned.
                        let sid = entry.id.clone();
                        let conns = Arc::clone(&self.connections);
                        let buf = Arc::clone(&self.buffer);
                        let flush = Arc::clone(&self.flush_coord);
                        tokio::spawn(async move {
                            match exit_fut.await {
                                Ok(status) => {
                                    tracing::warn!(
                                        server_id = %sid,
                                        exit_status = ?status,
                                        "compositor: child server exited — clearing URIs"
                                    );
                                }
                                Err(e) => {
                                    tracing::warn!(
                                        server_id = %sid,
                                        error = %e,
                                        "compositor: child exit watcher error"
                                    );
                                }
                            }
                            let uris = conns.uris_for_server(&sid);
                            for uri in &uris {
                                buf.clear_uri(uri);
                                flush.signal_flush(uri);
                            }
                            tracing::info!(
                                server_id = %sid,
                                uri_count = uris.len(),
                                "compositor: exit cleanup ADMITTED — empty diagnostics signalled"
                            );
                        });
                    }
                    Err(e) => {
                        tracing::warn!(
                            server_id = %entry.id,
                            error = %e,
                            "compositor: child spawn BLOCKED"
                        );
                    }
                }
            }
        }

        // Merge child capabilities.  The compositor always advertises FULL
        // text_document_sync regardless of what children report, because the
        // compositor itself normalises change notifications before fan-out.
        let mut merged = crate::capability_merge::merge_capabilities(&child_capabilities);
        merged.text_document_sync =
            Some(TextDocumentSyncCapability::Kind(TextDocumentSyncKind::FULL));

        // Store for introspection.
        if let Ok(mut guard) = self.merged_capabilities.write() {
            *guard = Some(merged.clone());
        }

        Ok(InitializeResult {
            capabilities: merged,
            server_info: Some(ServerInfo {
                name: "lsp-max-compositor".to_string(),
                version: Some(env!("CARGO_PKG_VERSION").to_string()),
            }),
            ..Default::default()
        })
    }

    async fn initialized(&self, params: InitializedParams) {
        tracing::info!("compositor: editor handshake complete — fanning initialized to children");

        // Fan `initialized` to all child servers.
        // Collect (server_id, ServerHandle clone) while holding each DashMap ref briefly,
        // then drop all refs before any await point to avoid holding shard locks across awaits.
        let child_ids = self.pool.server_ids_snapshot();
        let mut handles: Vec<(String, ChildServerHandle)> = Vec::with_capacity(child_ids.len());
        for id in &child_ids {
            if let Some(proc_ref) = self.pool.get(id) {
                // Clone the handle — ServerHandle is Clone — then let proc_ref drop.
                handles.push((id.clone(), proc_ref.handle.clone()));
            }
        }
        for (id, handle) in handles {
            tracing::debug!(server_id = %id, "compositor: forwarding initialized to child");
            handle.initialized(params).await;
        }

        // Backfill: flush any diagnostics that arrived before the editor was ready.
        let uris = self.buffer.buffered_uris();
        if !uris.is_empty() {
            tracing::info!(
                count = uris.len(),
                "compositor: backfill flushing buffered URIs"
            );
            for uri in &uris {
                self.push_diagnostics_to_client(uri).await;
            }
        }
    }

    async fn shutdown(&self) -> Result<()> {
        // Fan LSP shutdown to all child servers.
        // shutdown is a request (expects a response); exit is a notification.
        // Order: shutdown all first (collect results), then exit all.
        let child_ids = self.pool.server_ids_snapshot();

        // Collect handles while DashMap refs are held briefly, then drop all refs
        // before any await point to avoid holding shard locks across awaits.
        let mut handles: Vec<ChildServerHandle> = Vec::with_capacity(child_ids.len());
        for id in &child_ids {
            if let Some(proc_ref) = self.pool.get(id) {
                handles.push(proc_ref.handle.clone());
            }
        }

        // Send shutdown requests to all children — best-effort, 5 s timeout per child.
        for handle in &handles {
            let _ =
                tokio::time::timeout(tokio::time::Duration::from_secs(5), handle.shutdown()).await;
        }

        // Send exit notification to all children.
        for handle in &handles {
            handle.exit().await;
        }

        tracing::info!(
            count = handles.len(),
            "compositor: shutdown fan-out ADMITTED"
        );
        Ok(())
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        let uri = params.text_document.uri.to_string();
        let ext = ext_from_uri(&uri);
        let servers = self.router.servers_for(&ext);
        for srv in &servers {
            tracing::debug!(
                server_id = %srv.id,
                tier = ?srv.tier,
                uri = %uri,
                "fanout: did_open routed to child server"
            );
            self.connections.record_notification(&srv.id, &uri);
        }
        if servers.is_empty() {
            tracing::debug!(uri = %uri, ext = %ext, "did_open: no child servers registered for extension");
        }
        self.fanout_did_open(&uri, params).await;
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        let uri = params.text_document.uri.to_string();
        let ext = ext_from_uri(&uri);
        let servers = self.router.servers_for(&ext);
        for srv in &servers {
            tracing::debug!(
                server_id = %srv.id,
                tier = ?srv.tier,
                uri = %uri,
                "fanout: did_change routed to child server"
            );
            self.connections.record_notification(&srv.id, &uri);
        }
        if servers.is_empty() {
            tracing::debug!(uri = %uri, ext = %ext, "did_change: no child servers registered for extension");
        }
        self.fanout_did_change(&uri, params).await;
    }

    async fn did_close(&self, params: DidCloseTextDocumentParams) {
        let uri = params.text_document.uri.to_string();
        self.buffer.clear_uri(&uri);
        tracing::debug!(uri = %uri, "compositor: cleared diagnostic buffer on close");
        self.fanout_did_close(&uri, params).await;
    }
}

impl CompositorServer {
    /// Fan a didOpen notification to all child servers registered for this URI's extension.
    /// Collects handles while DashMap refs are held briefly, drops refs before awaiting.
    async fn fanout_did_open(&self, uri: &str, params: DidOpenTextDocumentParams) {
        let targets = crate::fanout::servers_for_uri(&self.router, uri);
        let mut handles: Vec<(String, ChildServerHandle)> = Vec::with_capacity(targets.len());
        for server in &targets {
            if let Some(proc_ref) = self.pool.get(&server.id) {
                handles.push((server.id.clone(), proc_ref.handle.clone()));
            }
        }
        for (id, handle) in handles {
            tracing::debug!(server_id = %id, uri = %uri, "compositor: fanout didOpen to child");
            handle.did_open(params.clone()).await;
        }
    }

    /// Fan a didChange notification to all child servers registered for this URI's extension.
    async fn fanout_did_change(&self, uri: &str, params: DidChangeTextDocumentParams) {
        let targets = crate::fanout::servers_for_uri(&self.router, uri);
        let mut handles: Vec<(String, ChildServerHandle)> = Vec::with_capacity(targets.len());
        for server in &targets {
            if let Some(proc_ref) = self.pool.get(&server.id) {
                handles.push((server.id.clone(), proc_ref.handle.clone()));
            }
        }
        for (id, handle) in handles {
            tracing::debug!(server_id = %id, uri = %uri, "compositor: fanout didChange to child");
            handle.did_change(params.clone()).await;
        }
    }

    /// Fan a didClose notification to all child servers registered for this URI's extension.
    async fn fanout_did_close(&self, uri: &str, params: DidCloseTextDocumentParams) {
        let targets = crate::fanout::servers_for_uri(&self.router, uri);
        let mut handles: Vec<(String, ChildServerHandle)> = Vec::with_capacity(targets.len());
        for server in &targets {
            if let Some(proc_ref) = self.pool.get(&server.id) {
                handles.push((server.id.clone(), proc_ref.handle.clone()));
            }
        }
        for (id, handle) in handles {
            tracing::debug!(server_id = %id, uri = %uri, "compositor: fanout didClose to child");
            handle.did_close(params.clone()).await;
        }
    }

    /// Return the current ANDON state for all buffered URIs.
    /// Non-destructive — does not clear the diagnostic buffer.
    pub async fn compositor_state(&self) -> crate::state_response::CompositorStateResponse {
        use crate::state_response::{CompositorStateResponse, UriAndonState};

        let buffered_uris = self.buffer.buffered_uris();
        let mut uri_states: Vec<UriAndonState> = Vec::with_capacity(buffered_uris.len());
        let mut global_andon_block = false;

        for uri in &buffered_uris {
            let result = self.buffer.flush(uri);
            let has_andon = result.has_andon_block;
            if has_andon {
                global_andon_block = true;
            }
            uri_states.push(UriAndonState {
                uri: uri.clone(),
                has_andon_block: has_andon,
                andon_codes: result.andon_codes().iter().map(|s| s.to_string()).collect(),
                diagnostic_count: result.diagnostics.len(),
            });
        }

        let child_server_count = self.pool.server_ids_snapshot().len();

        let query_timestamp_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);

        CompositorStateResponse {
            uris: uri_states,
            global_andon_block,
            child_server_count,
            query_timestamp_ms,
        }
    }

    /// Lightweight health snapshot. O(1) — does not iterate the diagnostic buffer.
    /// Returns pool size, registered IDs, buffered URI count, and the advisory ANDON flag.
    /// For authoritative current ANDON state use compositor_state().
    pub fn compositor_health(&self) -> crate::health_response::CompositorHealth {
        use crate::health_response::CompositorHealth;
        CompositorHealth {
            child_server_count: self.pool.server_ids_snapshot().len(),
            child_server_ids: self.pool.server_ids_snapshot(),
            buffered_uri_count: self.buffer.buffered_uri_count(),
            has_any_andon_block: self.buffer.last_andon_block(),
        }
    }

    /// Returns the ServerCapabilities advertised to the editor after the
    /// last initialize() call, serialized as JSON for introspection.
    /// Returns None if initialize() has not yet been called.
    pub fn compositor_capabilities(&self) -> Option<serde_json::Value> {
        self.merged_capabilities.read().ok().and_then(|guard| {
            guard
                .as_ref()
                .map(|caps| serde_json::to_value(caps).unwrap_or(serde_json::Value::Null))
        })
    }

    /// Flush the diagnostic buffer for a URI and return the merged result.
    /// Provides a testable entry point that exercises the full buffer→merge→MergeResult path.
    pub fn flush_uri(&self, uri: &str) -> crate::merge::MergeResult {
        self.buffer.flush(uri)
    }

    /// Flush the diagnostic buffer for a URI, convert to LSP diagnostics, and push to the editor client.
    /// Returns the MergeResult so callers can inspect has_andon_block.
    pub async fn push_diagnostics_to_client(&self, uri_str: &str) -> crate::merge::MergeResult {
        let result = self.buffer.flush(uri_str);

        if result.has_andon_block {
            tracing::warn!(
                uri = %uri_str,
                codes = ?result.andon_codes(),
                "compositor: ANDON block — merged diagnostics contain law violations"
            );
        }

        // Convert DiagnosticEntry → lsp_types::Diagnostic
        use lsp_max::lsp_types::{Diagnostic, DiagnosticSeverity, NumberOrString, Position, Range};
        let lsp_diags: Vec<Diagnostic> = result
            .diagnostics
            .iter()
            .map(|d| Diagnostic {
                range: Range {
                    start: Position {
                        line: d.line,
                        character: d.character,
                    },
                    end: Position {
                        line: d.line,
                        character: d.character,
                    },
                },
                severity: Some(match d.severity {
                    1 => DiagnosticSeverity::ERROR,
                    2 => DiagnosticSeverity::WARNING,
                    3 => DiagnosticSeverity::INFORMATION,
                    _ => DiagnosticSeverity::HINT,
                }),
                code: if d.code.is_empty() {
                    None
                } else {
                    Some(NumberOrString::String(d.code.clone()))
                },
                source: Some(match &d.server_id {
                    Some(sid) => format!("compositor/{}:{}", d.source_tier.as_str(), sid),
                    None => format!("compositor/{}", d.source_tier.as_str()),
                }),
                message: d.message.clone(),
                ..Default::default()
            })
            .collect();

        use std::str::FromStr;
        if let Ok(uri) = lsp_max::lsp_types::Uri::from_str(uri_str) {
            self.client.publish_diagnostics(uri, lsp_diags, None).await;
        }

        result
    }
}

pub async fn run_stdio(
    router: ExtensionRouter,
    merge_ctx: MergeContext,
    config: Arc<CompositorConfig>,
) {
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();
    let connections = Arc::new(ChildConnections::new());
    let merge_ctx = Arc::new(merge_ctx);
    let buffer = Arc::new(DiagnosticBuffer::new(Arc::clone(&merge_ctx)));
    let pool = Arc::new(ChildProcessPool::new());
    // Client is Clone — spawn the flush coordinator so child-server deposits automatically
    // trigger debounced publish_diagnostics calls to the editor.
    let buffer_for_coord = Arc::clone(&buffer);
    let merge_ctx_for_coord = Arc::clone(&merge_ctx);
    let pool_for_coord = Arc::clone(&pool);
    let (service, socket) = LspService::new(|client: Client| {
        let flush_coord = Arc::new(FlushCoordinator::spawn(
            Arc::clone(&buffer_for_coord),
            Arc::clone(&merge_ctx_for_coord),
            client.clone(),
            Arc::clone(&pool_for_coord),
        ));
        CompositorServer {
            client,
            router,
            merge_ctx,
            connections,
            buffer,
            pool: Arc::clone(&pool),
            config: Arc::clone(&config),
            flush_coord,
            merged_capabilities: Arc::new(RwLock::new(None)),
        }
    });
    let _ = Server::new(stdin, stdout, socket).serve(service).await;
}
