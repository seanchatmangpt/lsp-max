use std::error::Error;

use lsp_max::{LspService, Server};
use ra_max::{run_demo, RaMaxServer};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    match std::env::args().nth(1).as_deref() {
        Some("serve") | Some("--stdio") => serve_stdio().await,
        Some("demo") | None => {
            let report = run_demo()?;
            println!("{}", serde_json::to_string_pretty(&report)?);
            Ok(())
        }
        Some(other) => Err(format!("unknown ra-max mode `{other}`; use `demo` or `serve`").into()),
    }
}

async fn serve_stdio() -> Result<(), Box<dyn Error>> {
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();
    let (service, socket) = LspService::build(RaMaxServer::new)
        .custom_method("max/raSnapshot", RaMaxServer::max_snapshot)
        .finish();
    Server::new(stdin, stdout, socket).serve(service).await?;
    Ok(())
}
