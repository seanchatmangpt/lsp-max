use dashmap::DashMap;
use lsp_max::lsp_types::{
    DidChangeTextDocumentParams, DidCloseTextDocumentParams, DidOpenTextDocumentParams,
    InitializeParams, InitializeResult, InitializedParams, MessageType, ServerCapabilities,
    TextDocumentSyncCapability, TextDocumentSyncKind, Uri,
};
use lsp_max::{Client, LanguageServer};

use crate::{diagnostics, scanner};

pub struct MetaBackend {
    client: Client,
    /// uri string → document text; DashMap for lock-free concurrent access.
    documents: DashMap<String, String>,
}

impl MetaBackend {
    pub fn new(client: Client) -> Self {
        Self {
            client,
            documents: DashMap::new(),
        }
    }

    /// Scan `content`, convert violations to LSP diagnostics, and publish.
    async fn diagnose(&self, uri: &Uri, content: &str) {
        let violations = scanner::scan(content);
        let diags = violations
            .iter()
            .map(diagnostics::to_lsp_diagnostic)
            .collect();
        self.client
            .publish_diagnostics(uri.clone(), diags, None)
            .await;
    }
}

#[lsp_max::async_trait]
impl LanguageServer for MetaBackend {
    async fn initialize(&self, _: InitializeParams) -> lsp_max::jsonrpc::Result<InitializeResult> {
        Ok(InitializeResult {
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Kind(
                    TextDocumentSyncKind::FULL,
                )),
                ..Default::default()
            },
            ..Default::default()
        })
    }

    async fn initialized(&self, _: InitializedParams) {
        self.client
            .log_message(MessageType::INFO, "lsp-max-meta CANDIDATE")
            .await;
    }

    async fn shutdown(&self) -> lsp_max::jsonrpc::Result<()> {
        Ok(())
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        let uri = params.text_document.uri;
        let text = params.text_document.text;
        self.documents.insert(uri.to_string(), text.clone());
        self.diagnose(&uri, &text).await;
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        if let Some(change) = params.content_changes.into_iter().last() {
            let uri = params.text_document.uri;
            self.documents.insert(uri.to_string(), change.text.clone());
            self.diagnose(&uri, &change.text).await;
        }
    }

    async fn did_close(&self, params: DidCloseTextDocumentParams) {
        let uri = params.text_document.uri;
        self.documents.remove(&uri.to_string());
        self.client.publish_diagnostics(uri, vec![], None).await;
    }
}
