use clap_noun_verb_macros::verb;
use std::sync::Arc;
use tower_lsp_max::auto_lsp::AutoLspAdapter;
use tower_lsp_max::{LspService, Server};

// This is a ggen-generated Framework LSP.
// It is structurally compliant with the wasm4pm-compat baseline admissibility laws.

pub struct AxumBackend {
    pub client: tower_lsp_max::Client,
    pub auto_lsp: Arc<AutoLspAdapter>,
}

#[tower_lsp_max::async_trait]
impl tower_lsp_max::LanguageServer for AxumBackend {
    async fn initialize(
        &self,
        _params: tower_lsp_max::lsp_types_max::InitializeParams,
    ) -> tower_lsp_max::jsonrpc::Result<tower_lsp_max::lsp_types_max::InitializeResult> {
        Ok(tower_lsp_max::lsp_types_max::InitializeResult {
            server_info: Some(tower_lsp_max::lsp_types_max::ServerInfo {
                name: "axum-lsp".to_string(),
                version: Some("1.0.0".to_string()),
            }),
            offset_encoding: None,
            capabilities: tower_lsp_max::lsp_types_max::ServerCapabilities {
                text_document_sync: Some(
                    tower_lsp_max::lsp_types_max::TextDocumentSyncCapability::Kind(
                        tower_lsp_max::lsp_types_max::TextDocumentSyncKind::INCREMENTAL,
                    ),
                ),
                ..Default::default()
            },
        })
    }

    async fn shutdown(&self) -> tower_lsp_max::jsonrpc::Result<()> {
        Ok(())
    }

    async fn did_open(&self, params: tower_lsp_max::lsp_types_max::DidOpenTextDocumentParams) {
        self.auto_lsp
            .handle_did_open(params, tree_sitter_rust::LANGUAGE.into());
    }

    async fn did_change(&self, params: tower_lsp_max::lsp_types_max::DidChangeTextDocumentParams) {
        self.auto_lsp
            .handle_did_change(params, tree_sitter_rust::LANGUAGE.into());
    }
}

#[verb("start")]
fn start_server(_stdio: bool) -> clap_noun_verb::Result<()> {
    tokio::runtime::Runtime::new().unwrap().block_on(async {
        let stdin = tokio::io::stdin();
        let stdout = tokio::io::stdout();

        let (service, socket) = LspService::new(|client| AxumBackend {
            client,
            auto_lsp: Arc::new(AutoLspAdapter::new_default()),
        });

        let _ = Server::new(stdin, stdout, socket).serve(service).await;
    });

    Ok(())
}

fn main() -> clap_noun_verb::Result<()> {
    clap_noun_verb::run()
}
