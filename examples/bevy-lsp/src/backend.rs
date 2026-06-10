use lsp_max::ast::AutoLspAdapter;
use std::sync::Arc;

pub struct BevyBackend {
    pub client: lsp_max::Client,
    pub lsp_max_ast: Arc<AutoLspAdapter>,
}

#[lsp_max::async_trait]
impl lsp_max::LanguageServer for BevyBackend {
    async fn initialize(
        &self,
        _params: lsp_max::lsp_types_max::InitializeParams,
    ) -> lsp_max::jsonrpc::Result<lsp_max::lsp_types_max::InitializeResult> {
        Ok(lsp_max::lsp_types_max::InitializeResult {
            server_info: Some(lsp_max::lsp_types_max::ServerInfo {
                name: "bevy-lsp".to_string(),
                version: Some(env!("CARGO_PKG_VERSION").to_string()),
            }),
            offset_encoding: None,
            capabilities: lsp_max::lsp_types_max::ServerCapabilities {
                text_document_sync: Some(lsp_max::lsp_types_max::TextDocumentSyncCapability::Kind(
                    lsp_max::lsp_types_max::TextDocumentSyncKind::INCREMENTAL,
                )),
                // Dynamically injected capabilities based on ontology
                ..Default::default()
            },
        })
    }

    async fn shutdown(&self) -> lsp_max::jsonrpc::Result<()> {
        Ok(())
    }

    async fn did_open(&self, params: lsp_max::lsp_types_max::DidOpenTextDocumentParams) {
        let uri = params.text_document.uri.clone();
        let version = params.text_document.version;
        self.lsp_max_ast
            .handle_did_open(params, tree_sitter_rust::LANGUAGE.into());
        let diags = crate::semantics::dispatch_semantic_rules(&self.lsp_max_ast, &uri);
        self.client
            .publish_diagnostics(uri, diags, Some(version))
            .await;
    }

    async fn did_change(&self, params: lsp_max::lsp_types_max::DidChangeTextDocumentParams) {
        let uri = params.text_document.uri.clone();
        let version = params.text_document.version;
        self.lsp_max_ast
            .handle_did_change(params, tree_sitter_rust::LANGUAGE.into());
        let diags = crate::semantics::dispatch_semantic_rules(&self.lsp_max_ast, &uri);
        self.client
            .publish_diagnostics(uri, diags, Some(version))
            .await;
    }
}
