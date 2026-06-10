mod backend;
mod semantics;

use crate::backend::TexBackend;
use clap_noun_verb_macros::verb;
use lsp_max::ast::AutoLspAdapter;
use lsp_max::{LspService, Server};
use std::process::Command;
use std::sync::Arc;

#[verb("start", "server")]
fn start_server() -> clap_noun_verb::Result<()> {
    tokio::runtime::Runtime::new().unwrap().block_on(async {
        let stdin = tokio::io::stdin();
        let stdout = tokio::io::stdout();

        let (service, socket) = LspService::new(|client| TexBackend {
            client,
            lsp_max_ast: Arc::new(AutoLspAdapter::new_default()),
        });

        let _ = Server::new(stdin, stdout, socket).serve(service).await;
    });

    Ok(())
}

const THESIS_DIR: &str = "docs/thesis/periodic-table-of-reason";
const PDF_NAME: &str = "periodic-table-of-reason-v26.6.10.pdf";

fn run_pdflatex(jobname: &str) {
    let _ = Command::new("pdflatex")
        .current_dir(THESIS_DIR)
        .args(["-interaction=nonstopmode", &format!("-jobname={}", jobname), "main.tex"])
        .status();
}

fn run_bibtex(jobname: &str) {
    match Command::new("bibtex").current_dir(THESIS_DIR).arg(jobname).status() {
        Ok(s) if s.success() => println!("bibtex OK."),
        Ok(_) => println!("bibtex errors — check .blg."),
        Err(e) => println!("bibtex unavailable: {}", e),
    }
}

#[verb("compile", "pdf")]
fn compile_pdf() -> clap_noun_verb::Result<()> {
    let jobname = "periodic-table-of-reason-v26.6.10";
    println!("Pass 1/4: pdflatex...");
    Command::new("pdflatex")
        .current_dir(THESIS_DIR)
        .args(["-interaction=nonstopmode", &format!("-jobname={}", jobname), "main.tex"])
        .status()?;
    println!("Pass 2/4: bibtex...");
    run_bibtex(jobname);
    println!("Pass 3/4: pdflatex...");
    run_pdflatex(jobname);
    println!("Pass 4/4: pdflatex...");
    run_pdflatex(jobname);
    println!("Opening {}...", PDF_NAME);
    let _ = Command::new("open").current_dir(THESIS_DIR).arg(PDF_NAME).status();
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
        "docs/thesis/periodic-table-of-reason/chapters/01-unverifiability.tex",
        "docs/thesis/periodic-table-of-reason/chapters/02-breed-category.tex",
        "docs/thesis/periodic-table-of-reason/chapters/03-oracle-theorem.tex",
        "docs/thesis/periodic-table-of-reason/chapters/04-ocel-conformance.tex",
        "docs/thesis/periodic-table-of-reason/chapters/05-epistemological-space.tex",
        "docs/thesis/periodic-table-of-reason/chapters/06-bvc.tex",
        "docs/thesis/periodic-table-of-reason/chapters/07-fourth-constraint.tex",
        "docs/thesis/periodic-table-of-reason/chapters/08-speed-transitions.tex",
        "docs/thesis/periodic-table-of-reason/chapters/09-receipt-chain.tex",
        "docs/thesis/periodic-table-of-reason/chapters/10-fortune5.tex",
        "docs/thesis/periodic-table-of-reason/chapters/11-main-theorems.tex",
        "docs/thesis/periodic-table-of-reason/chapters/12-verification-falsifiers.tex",
        "docs/thesis/periodic-table-of-reason/chapters/13-conclusion.tex",
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
        let uri: lsp_max::lsp_types_max::Url = url.as_str().parse().unwrap();

        adapter.handle_did_open(
            lsp_max::lsp_types_max::DidOpenTextDocumentParams {
                text_document: lsp_max::lsp_types_max::TextDocumentItem {
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
            .any(|d| d.severity == Some(lsp_max::lsp_types_max::DiagnosticSeverity::ERROR));

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
                        lsp_max::lsp_types_max::NumberOrString::Number(n) => n.to_string(),
                        lsp_max::lsp_types_max::NumberOrString::String(s) => s.clone(),
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
