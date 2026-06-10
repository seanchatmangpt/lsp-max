use std::sync::Arc;
use tower_lsp_max::auto_lsp::AutoLspAdapter;

pub struct BevyBackend {
    pub client: tower_lsp_max::Client,
    pub auto_lsp: Arc<AutoLspAdapter>,
}

#[tower_lsp_max::async_trait]
impl tower_lsp_max::LanguageServer for BevyBackend {
    async fn initialize(
        &self,
        _params: tower_lsp_max::lsp_types_max::InitializeParams,
    ) -> tower_lsp_max::jsonrpc::Result<tower_lsp_max::lsp_types_max::InitializeResult> {
        Ok(tower_lsp_max::lsp_types_max::InitializeResult {
            server_info: Some(tower_lsp_max::lsp_types_max::ServerInfo {
                name: "bevy-lsp".to_string(),
                version: Some(env!("CARGO_PKG_VERSION").to_string()),
            }),
            offset_encoding: None,
            capabilities: tower_lsp_max::lsp_types_max::ServerCapabilities {
                text_document_sync: Some(
                    tower_lsp_max::lsp_types_max::TextDocumentSyncCapability::Kind(
                        tower_lsp_max::lsp_types_max::TextDocumentSyncKind::INCREMENTAL,
                    ),
                ),
                // Dynamically injected capabilities based on ontology
                ..Default::default()
            },
        })
    }

    async fn shutdown(&self) -> tower_lsp_max::jsonrpc::Result<()> {
        Ok(())
    }

    async fn did_open(&self, params: tower_lsp_max::lsp_types_max::DidOpenTextDocumentParams) {
        let uri = params.text_document.uri.clone();
        let version = params.text_document.version;
        self.auto_lsp
            .handle_did_open(params, tree_sitter_rust::LANGUAGE.into());
        let diags = crate::semantics::dispatch_semantic_rules(&self.auto_lsp, &uri);
        self.client
            .publish_diagnostics(uri, diags, Some(version))
            .await;
    }

    async fn did_change(&self, params: tower_lsp_max::lsp_types_max::DidChangeTextDocumentParams) {
        let uri = params.text_document.uri.clone();
        let version = params.text_document.version;
        self.auto_lsp
            .handle_did_change(params, tree_sitter_rust::LANGUAGE.into());
        let diags = crate::semantics::dispatch_semantic_rules(&self.auto_lsp, &uri);
        self.client
            .publish_diagnostics(uri, diags, Some(version))
            .await;
    }
}
