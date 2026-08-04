mod powl_types;
mod server;
mod validators;

use crate::server::PowlLsp;
use clap_noun_verb_macros::verb;
use lsp_max::{LspService, Server};

#[verb("start", "server")]
fn start_server() -> clap_noun_verb::Result<()> {
    tokio::runtime::Runtime::new().unwrap().block_on(async {
        let stdin = tokio::io::stdin();
        let stdout = tokio::io::stdout();
        let (service, socket) = LspService::new(PowlLsp::new);
        let _ = Server::new(stdin, stdout, socket).serve(service).await;
    });
    Ok(())
}

fn main() -> clap_noun_verb::Result<()> {
    clap_noun_verb::run()
}
