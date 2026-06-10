mod backend;
mod semantics;

use crate::backend::BevyBackend;
use std::sync::Arc;
use tower_lsp_max::auto_lsp::AutoLspAdapter;
use tower_lsp_max::{LspService, Server};

fn main() {
    tokio::runtime::Runtime::new().unwrap().block_on(async {
        let stdin = tokio::io::stdin();
        let stdout = tokio::io::stdout();

        let (service, socket) = LspService::new(|client| BevyBackend {
            client,
            auto_lsp: Arc::new(AutoLspAdapter::new_default()),
        });

        let _ = Server::new(stdin, stdout, socket).serve(service).await;
    });
}
