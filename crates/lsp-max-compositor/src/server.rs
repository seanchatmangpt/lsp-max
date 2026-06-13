use crate::connections::ChildConnections;
use crate::diagnostic_buffer::DiagnosticBuffer;
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
    merge_ctx: Arc<MergeContext>,
    connections: Arc<ChildConnections>,
    #[allow(dead_code)]
    buffer: Arc<DiagnosticBuffer>,
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

    async fn did_close(&self, params: DidCloseTextDocumentParams) {
        let uri = params.text_document.uri.to_string();
        self.buffer.clear_uri(&uri);
        tracing::debug!(uri = %uri, "compositor: cleared diagnostic buffer on close");
    }
}

impl CompositorServer {
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
                source: Some(format!("compositor/{}", d.source_tier.as_str())),
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

pub async fn run_stdio(router: ExtensionRouter, merge_ctx: MergeContext) {
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();
    let connections = Arc::new(ChildConnections::new());
    let merge_ctx = Arc::new(merge_ctx);
    let buffer = Arc::new(DiagnosticBuffer::new(Arc::clone(&merge_ctx)));
    let (service, socket) = LspService::new(|client| CompositorServer {
        client,
        router,
        merge_ctx,
        connections,
        buffer,
    });
    let _ = Server::new(stdin, stdout, socket).serve(service).await;
}
