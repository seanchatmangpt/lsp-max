use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LspService, Server};
use serde_json::json;
use ocel_core::OCEL;
use std::fs;

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
            // Invoke wasm4pm conformance/replay (simulated via ocel-core validation + verdict search)
            match serde_json::from_str::<OCEL>(&content) {
                Ok(ocel) => {
                    // 1. ocel-core validation
                    let cardinalities = std::collections::HashMap::new();
                    let report = ocel_core::validate::validate(&ocel, &cardinalities);
                    
                    if !report.valid {
                        for err in report.errors {
                            diags.push(Diagnostic {
                                range: Range::default(),
                                severity: Some(DiagnosticSeverity::ERROR),
                                code: Some(NumberOrString::String("WASM4PM-OCEL-INVALID".to_string())),
                                message: format!("[{}] {}", err.code, err.message),
                                source: Some("wasm4pm-lsp".to_string()),
                                ..Default::default()
                            });
                        }
                    }

                    // 2. wasm4pm Verdict Consumption
                    let verdict_event = ocel.events.iter().find(|e| e.event_type == "ConformanceVerdictEmitted");
                    if let Some(evt) = verdict_event {
                        let verdict = evt.attributes.iter().find(|a| a.name == "verdict").map(|a| a.value.to_string()).unwrap_or_default();
                        let fitness = evt.attributes.iter().find(|a| a.name == "fitness").map(|a| a.value.to_string()).unwrap_or_default();
                        
                        let (severity, code) = if verdict.contains("FIT") {
                            (DiagnosticSeverity::INFORMATION, "WASM4PM-VERDICT-FIT")
                        } else {
                            (DiagnosticSeverity::ERROR, "WASM4PM-VERDICT-DEVIATION")
                        };

                        diags.push(Diagnostic {
                            range: Range::default(),
                            severity: Some(severity),
                            code: Some(NumberOrString::String(code.to_string())),
                            message: format!("Conformance Verdict: {} (Fitness: {})", verdict, fitness),
                            source: Some("wasm4pm-lsp".to_string()),
                            ..Default::default()
                        });
                    } else {
                        diags.push(Diagnostic {
                            range: Range::default(),
                            severity: Some(DiagnosticSeverity::WARNING),
                            code: Some(NumberOrString::String("WASM4PM-VERDICT-MISSING".to_string())),
                            message: "No wasm4pm conformance verdict found in log.".to_string(),
                            source: Some("wasm4pm-lsp".to_string()),
                            ..Default::default()
                        });
                    }
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
