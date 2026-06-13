use crate::connections::ChildConnections;
use crate::{ExtensionRouter, MergeContext};
use lsp_max::jsonrpc::Result;
use lsp_max::lsp_types::*;
use lsp_max::{Client, LspService, Server};
use std::sync::Arc;

pub struct CompositorServer {
    #[allow(dead_code)]
    client: Client,
    router: ExtensionRouter,
    #[allow(dead_code)]
    merge_ctx: MergeContext,
    connections: Arc<ChildConnections>,
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
    async fn initialize(&self, _: InitializeParams) -> Result<InitializeResult> {
        Ok(InitializeResult {
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Kind(
                    TextDocumentSyncKind::INCREMENTAL,
                )),
                ..Default::default()
            },
            server_info: Some(ServerInfo {
                name: "lsp-max-compositor".to_string(),
                version: Some(env!("CARGO_PKG_VERSION").to_string()),
            }),
            ..Default::default()
        })
    }

    async fn initialized(&self, _: InitializedParams) {
        tracing::info!("compositor initialized");
    }

    async fn shutdown(&self) -> Result<()> {
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
    }
}

pub async fn run_stdio(router: ExtensionRouter, merge_ctx: MergeContext) {
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();
    let connections = Arc::new(ChildConnections::new());
    let (service, socket) = LspService::new(|client| CompositorServer {
        client,
        router,
        merge_ctx,
        connections,
    });
    let _ = Server::new(stdin, stdout, socket).serve(service).await;
}
