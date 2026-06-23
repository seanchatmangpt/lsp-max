use lsp_max::{LspService, Server};
use lsp_max_meta::MetaBackend;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt().init();
    let (service, socket) = LspService::new(MetaBackend::new);
    let _ = Server::new(tokio::io::stdin(), tokio::io::stdout(), socket)
        .serve(service)
        .await;
}
