use crate::powl_types::PowlNode;
use crate::validators::validate_node;
use lsp_max::{Client, LanguageServer};
use lsp_types_max::*;

pub struct PowlLsp {
    client: Client,
}

impl PowlLsp {
    pub fn new(client: Client) -> Self {
        Self { client }
    }

    async fn check_and_publish(&self, uri: Uri, content: String) {
        let diagnostics = parse_and_validate(&content);
        self.client.publish_diagnostics(uri, diagnostics, None).await;
    }
}

fn parse_and_validate(content: &str) -> Vec<Diagnostic> {
    match serde_json::from_str::<PowlNode>(content) {
        Ok(node) => validate_node(&node),
        Err(e) => vec![Diagnostic {
            range: Range::new(Position::new(0, 0), Position::new(0, 0)),
            severity: Some(DiagnosticSeverity::ERROR),
            source: Some("powl-lsp".to_string()),
            message: format!("POWL parse error: {}", e),
            ..Diagnostic::default()
        }],
    }
}

#[lsp_max::async_trait]
impl LanguageServer for PowlLsp {
    async fn initialize(&self, _: InitializeParams) -> lsp_max::jsonrpc::Result<InitializeResult> {
        Ok(InitializeResult {
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Kind(
                    TextDocumentSyncKind::FULL,
                )),
                ..ServerCapabilities::default()
            },
            server_info: Some(ServerInfo {
                name: "powl-lsp".to_string(),
                version: Some("26.6.9".to_string()),
            }),
            offset_encoding: None,
        })
    }

    async fn shutdown(&self) -> lsp_max::jsonrpc::Result<()> {
        Ok(())
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        self.check_and_publish(params.text_document.uri, params.text_document.text)
            .await;
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        if let Some(change) = params.content_changes.into_iter().next() {
            self.check_and_publish(params.text_document.uri, change.text)
                .await;
        }
    }

    async fn did_save(&self, params: DidSaveTextDocumentParams) {
        if let Some(text) = params.text {
            self.check_and_publish(params.text_document.uri, text).await;
        }
    }
}
