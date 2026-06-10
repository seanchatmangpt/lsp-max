mod backend;
mod semantics;

use crate::backend::BevyBackend;
use lsp_max::ast::AutoLspAdapter;
use lsp_max::{LspService, Server};
use std::sync::Arc;

fn main() {
    tokio::runtime::Runtime::new().unwrap().block_on(async {
        let stdin = tokio::io::stdin();
        let stdout = tokio::io::stdout();

        let (service, socket) = LspService::new(|client| BevyBackend {
            client,
            lsp_max_ast: Arc::new(AutoLspAdapter::new_default()),
        });

        let _ = Server::new(stdin, stdout, socket).serve(service).await;
    });
}
