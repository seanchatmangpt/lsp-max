//! JSON-RPC stdio language-server entrypoint (agent contract: `main.rs`).
//!
//! Wires initialize/didOpen/didChange/didSave lifecycle to
//! `BranchlessLspServer` over lsp-max's `Server`/`LspService` stdio
//! transport.

use branchless_lsp::BranchlessLspServer;
use lsp_max::{LspService, Server};
use tokio::io::{stdin, stdout};

fn main() {
    let runtime = tokio::runtime::Runtime::new().expect("tokio runtime");
    runtime.block_on(async {
        let (service, socket) = LspService::new(BranchlessLspServer::new);
        let _ = Server::new(stdin(), stdout(), socket).serve(service).await;
    });
}
