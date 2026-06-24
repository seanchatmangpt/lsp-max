//! WasmLspBackend — CANDIDATE
//! Law-axis: CANDIDATE across all declared methods — receipt chain OPEN
//!
//! Three methods declared in schema/domain.ttl with their law-status:
//!   - initialize         : required lifecycle (not law-gated)
//!   - textDocument/hover : CANDIDATE — transcript + receipt OPEN
//!   - shutdown           : required lifecycle (not law-gated)
//!
//! hover and completion return Default to keep this stub read-only; the LSP
//! surface never mutates files.

use lsp_max::lsp_types_max::*;

pub struct WasmLspBackend {
    client: lsp_max::Client,
}

impl WasmLspBackend {
    pub fn new(client: lsp_max::Client) -> Self {
        Self { client }
    }
}

#[lsp_max::async_trait]
impl lsp_max::LanguageServer for WasmLspBackend {
    async fn initialize(
        &self,
        _params: InitializeParams,
    ) -> lsp_max::jsonrpc::Result<InitializeResult> {
        Ok(InitializeResult {
            capabilities: crate::capabilities::server_capabilities(),
            ..Default::default()
        })
    }

    async fn initialized(&self, _: InitializedParams) {
        self.client
            .log_message(MessageType::INFO, "wasm-lsp CANDIDATE")
            .await;
    }

    async fn shutdown(&self) -> lsp_max::jsonrpc::Result<()> {
        Ok(())
    }

    // CANDIDATE: hover — law:CANDIDATE in domain.ttl; receipt chain OPEN
    async fn hover(&self, _params: HoverParams) -> lsp_max::jsonrpc::Result<Option<Hover>> {
        Ok(Default::default())
    }

    // CANDIDATE: completion — not declared in domain.ttl yet; stub only
    async fn completion(
        &self,
        _params: CompletionParams,
    ) -> lsp_max::jsonrpc::Result<Option<CompletionResponse>> {
        Ok(Default::default())
    }
}
