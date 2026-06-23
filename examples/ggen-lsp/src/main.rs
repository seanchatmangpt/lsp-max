//! ggen-lsp server entrypoint — CANDIDATE

use ggen_lsp::Backend;
use lsp_max::{LspService, Server};

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt().init();
    let (service, socket) = LspService::new(|client| Backend::new(client));
    Server::new(tokio::io::stdin(), tokio::io::stdout(), socket)
        .serve(service)
        .await;
}
