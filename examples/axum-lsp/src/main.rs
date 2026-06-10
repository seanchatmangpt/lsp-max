use clap_noun_verb_macros::verb;
use lsp_max::ast::AutoLspAdapter;
use lsp_max::{LspService, Server};
use std::sync::Arc;

// This is a ggen-generated Framework LSP.
// It is structurally compliant with the wasm4pm-compat baseline admissibility laws.

pub struct AxumBackend {
    pub client: lsp_max::Client,
    pub lsp_max_ast: Arc<AutoLspAdapter>,
}

#[lsp_max::async_trait]
impl lsp_max::LanguageServer for AxumBackend {
    async fn initialize(
        &self,
        _params: lsp_max::lsp_types_max::InitializeParams,
    ) -> lsp_max::jsonrpc::Result<lsp_max::lsp_types_max::InitializeResult> {
        Ok(lsp_max::lsp_types_max::InitializeResult {
            server_info: Some(lsp_max::lsp_types_max::ServerInfo {
                name: "axum-lsp".to_string(),
                version: Some("1.0.0".to_string()),
            }),
            offset_encoding: None,
            capabilities: lsp_max::lsp_types_max::ServerCapabilities {
                text_document_sync: Some(lsp_max::lsp_types_max::TextDocumentSyncCapability::Kind(
                    lsp_max::lsp_types_max::TextDocumentSyncKind::INCREMENTAL,
                )),
                ..Default::default()
            },
        })
    }

    async fn shutdown(&self) -> lsp_max::jsonrpc::Result<()> {
        Ok(())
    }

    async fn did_open(&self, params: lsp_max::lsp_types_max::DidOpenTextDocumentParams) {
        self.lsp_max_ast
            .handle_did_open(params, tree_sitter_rust::LANGUAGE.into());
    }

    async fn did_change(&self, params: lsp_max::lsp_types_max::DidChangeTextDocumentParams) {
        self.lsp_max_ast
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
            lsp_max_ast: Arc::new(AutoLspAdapter::new_default()),
        });

        let _ = Server::new(stdin, stdout, socket).serve(service).await;
    });

    Ok(())
}

fn main() -> clap_noun_verb::Result<()> {
    clap_noun_verb::run()
}
