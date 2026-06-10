mod backend;
mod semantics;

use crate::backend::TexBackend;
use clap_noun_verb_macros::verb;
use std::process::Command;
use std::sync::Arc;
use tower_lsp_max::auto_lsp::AutoLspAdapter;
use tower_lsp_max::{LspService, Server};

#[verb("start", "server")]
fn start_server() -> clap_noun_verb::Result<()> {
    tokio::runtime::Runtime::new().unwrap().block_on(async {
        let stdin = tokio::io::stdin();
        let stdout = tokio::io::stdout();

        let (service, socket) = LspService::new(|client| TexBackend {
            client,
            auto_lsp: Arc::new(AutoLspAdapter::new_default()),
        });

        let _ = Server::new(stdin, stdout, socket).serve(service).await;
    });

    Ok(())
}

#[verb("compile", "pdf")]
fn compile_pdf() -> clap_noun_verb::Result<()> {
    println!("Compiling PDF with pdflatex...");
    let status = Command::new("pdflatex")
        .current_dir("docs/thesis/ggen")
        .arg("-interaction=nonstopmode")
        .arg("main.tex")
        .status()?;

    if status.success() {
        println!("Pass 1/3 compiled successfully.");
    }

    println!("Running bibtex...");
    let _ = Command::new("bibtex")
        .current_dir("docs/thesis/ggen")
        .arg("main")
        .status();

    println!("Running pdflatex (Pass 2)...");
    let _ = Command::new("pdflatex")
        .current_dir("docs/thesis/ggen")
        .arg("-interaction=nonstopmode")
        .arg("main.tex")
        .status();

    println!("Running pdflatex (Pass 3)...");
    let _ = Command::new("pdflatex")
        .current_dir("docs/thesis/ggen")
        .arg("-interaction=nonstopmode")
        .arg("main.tex")
        .status();

    println!("Opening main.pdf...");
    let _ = Command::new("open")
        .current_dir("docs/thesis/ggen")
        .arg("main.pdf")
        .status();

    Ok(())
}

#[verb("verify", "dissertation")]
fn verify_dissertation() -> clap_noun_verb::Result<()> {
    run_verification_domain_logic()?;
    Ok(())
}

fn run_verification_domain_logic() -> Result<bool, clap_noun_verb::error::NounVerbError> {
    let adapter = AutoLspAdapter::new_default();
    let chapters = [
        "docs/thesis/ggen/chapters/ch1_problem_formulation.tex",
        "docs/thesis/ggen/chapters/ch2_algebraic_boundary.tex",
        "docs/thesis/ggen/chapters/ch3_differential_calculus.tex",
        "docs/thesis/ggen/chapters/ch4_generative_functors.tex",
        "docs/thesis/ggen/chapters/ch5_operational_calculus.tex",
        "docs/thesis/ggen/chapters/ch6_swarm_defense.tex",
    ];

    println!("--- Natively Verifying Academic Rigor via tex-lsp ---");
    let mut all_passed = true;

    for path_str in chapters {
        let path = std::path::Path::new(path_str);
        if !path.exists() {
            println!("Skipping missing chapter: {}", path_str);
            continue;
        }

        let content =
            std::fs::read_to_string(path).map_err(clap_noun_verb::error::NounVerbError::from)?;
        let abs_path =
            std::fs::canonicalize(path).map_err(clap_noun_verb::error::NounVerbError::from)?;
        let url = url::Url::from_file_path(abs_path).map_err(|_| {
            clap_noun_verb::error::NounVerbError::from(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "Bad path",
            ))
        })?;
        let uri: tower_lsp_max::lsp_types_max::Url = url.as_str().parse().unwrap();

        adapter.handle_did_open(
            tower_lsp_max::lsp_types_max::DidOpenTextDocumentParams {
                text_document: tower_lsp_max::lsp_types_max::TextDocumentItem {
                    uri: uri.clone(),
                    language_id: "latex".to_string(),
                    version: 1,
                    text: content,
                },
            },
            tree_sitter_latex::LANGUAGE.into(),
        );

        let diags = crate::semantics::dispatch_semantic_rules(&adapter, &uri);

        let has_errors = diags
            .iter()
            .any(|d| d.severity == Some(tower_lsp_max::lsp_types_max::DiagnosticSeverity::ERROR));

        if !has_errors {
            println!("✅ {} : PASSED rigor checks.", path_str);
        } else {
            all_passed = false;
            println!("❌ {} : REJECTED.", path_str);
            for d in diags {
                let code = d
                    .code
                    .as_ref()
                    .map(|c| match c {
                        tower_lsp_max::lsp_types_max::NumberOrString::Number(n) => n.to_string(),
                        tower_lsp_max::lsp_types_max::NumberOrString::String(s) => s.clone(),
                    })
                    .unwrap_or_else(|| "UNKNOWN".to_string());
                println!("  [{}] {}", code, d.message);
            }
        }
    }
    Ok(all_passed)
}

fn main() -> clap_noun_verb::Result<()> {
    clap_noun_verb::run()
}
