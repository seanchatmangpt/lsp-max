use serde::{Deserialize, Serialize};
use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LspService, Server};
use ocel_core::OCEL;
use wasm4pm_algos::gall::{check_gall_conformance, GallVerdict};

#[derive(Debug)]
struct Backend {
    client: Client,
}

#[tower_lsp::async_trait]
impl tower_lsp::LanguageServer for Backend {
    async fn initialize(&self, _: InitializeParams) -> Result<InitializeResult> {
        Ok(InitializeResult {
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Kind(TextDocumentSyncKind::FULL)),
                ..Default::default()
            },
            ..Default::default()
        })
    }

    async fn initialized(&self, _: InitializedParams) {
        self.client.log_message(MessageType::INFO, "wasm4pm-lsp initialized").await;
    }

    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        self.diagnose(params.text_document.uri, params.text_document.text).await;
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        self.diagnose(params.text_document.uri, params.content_changes[0].text.clone()).await;
    }
}

impl Backend {
    async fn diagnose(&self, uri: Url, content: String) {
        let mut diags = Vec::new();
        
        if uri.path().ends_with(".ocel.json") {
            match serde_json::from_str::<OCEL>(&content) {
                Ok(ocel) => {
                    // 1. Authoritative wasm4pm Replay
                    let verdict = check_gall_conformance(&ocel);

                    let (severity, code, message) = match verdict {
                        GallVerdict::Blocked { reason } => {
                            (DiagnosticSeverity::ERROR, "WASM4PM-VERDICT-BLOCKED", format!("Conformance Verdict: BLOCKED ({})", reason))
                        }
                        GallVerdict::Fit { fitness } => {
                            (DiagnosticSeverity::INFORMATION, "WASM4PM-VERDICT-FIT", format!("Conformance Verdict: FIT (Fitness: {:.1})", fitness))
                        }
                        GallVerdict::Deviation { fitness, missing } => {
                            (DiagnosticSeverity::ERROR, "WASM4PM-VERDICT-DEVIATION", format!("Conformance Verdict: DEVIATION (Fitness: {:.1}). Missing admission for: {}", fitness, missing.join(", ")))
                        }
                    };

                    diags.push(Diagnostic {
                        range: Range::default(),
                        severity: Some(severity),
                        code: Some(NumberOrString::String(code.to_string())),
                        message,
                        source: Some("wasm4pm-lsp".to_string()),
                        ..Default::default()
                    });
                }
                Err(e) => {
                    diags.push(Diagnostic {
                        range: Range::default(),
                        severity: Some(DiagnosticSeverity::ERROR),
                        code: Some(NumberOrString::String("WASM4PM-PARSE-FAILED".to_string())),
                        message: format!("Failed to parse OCEL: {}", e),
                        source: Some("wasm4pm-lsp".to_string()),
                        ..Default::default()
                    });
                }
            }
        }

        self.client.publish_diagnostics(uri, diags, None).await;
    }
}

#[tokio::main]
async fn main() {
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    let (service, socket) = LspService::new(|client| Backend { client });
    Server::new(stdin, stdout, socket).serve(service).await;
}
