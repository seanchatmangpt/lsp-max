use anti_llm_lsp::server::AntiLlmServer;
use clap::{Parser, Subcommand};
use tower_lsp_max::{LspService, Server};

#[derive(Parser)]
#[command(name = "anti-llm-lsp")]
#[command(about = "Admissibility server detecting LLM-generated mistakes", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Start the LSP server over stdio
    Serve {
        #[arg(long)]
        stdio: bool,
    },
    /// Run a raw scan on the workspace directory
    Scan {
        #[arg(long, default_value = ".")]
        dir: String,
    },
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Serve { stdio } => {
            if stdio {
                let stdin = tokio::io::stdin();
                let stdout = tokio::io::stdout();
                let (service, socket) = LspService::new(AntiLlmServer::new);
                let _ = Server::new(stdin, stdout, socket).serve(service).await;
            } else {
                eprintln!("Error: --stdio flag is required for LSP serve");
                std::process::exit(1);
            }
        }
        Commands::Scan { dir } => {
            let _ = anti_llm_lsp::ocel::write_ocel_outputs(&dir);
            let obs = anti_llm_lsp::engine::scan_directory(&dir);
            let diags = anti_llm_lsp::engine::evaluate_diagnostics(&obs);
            println!("--- Anti-LLM Admissibility Scan Findings ---");
            println!("Observations: {}", obs.len());
            println!("Diagnostics emitted: {}", diags.len());
            for d in diags {
                println!("  - [{}] {}:{}: {}", d.code, d.file_path, d.line, d.message);
            }
        }
    }

    Ok(())
}
