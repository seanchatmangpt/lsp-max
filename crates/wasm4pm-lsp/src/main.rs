use serde::{Deserialize, Serialize};
use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LspService, Server};
use ocel_core::OCEL;
use std::collections::{HashMap, HashSet};

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
                    let required = ["GALL-CHECKPOINT-001", "GALL-CHECKPOINT-002", "GALL-CHECKPOINT-003", "GALL-CHECKPOINT-004"];
                    let mut admitted = HashSet::new();
                    let mut previous_receipt_id = None;
                    let mut chain_broken = false;

                    for evt in &ocel.events {
                        if evt.event_type == "checkpoint.admitted" {
                            for rel in &evt.relationships {
                                admitted.insert(rel.object_id.clone());
                            }
                        }

                        if let Some(pr_attr) = evt.attributes.iter().find(|a| a.name == "previous_receipt") {
                            let pr = pr_attr.value.to_string();
                            if !pr.is_empty() && Some(pr) != previous_receipt_id {
                                chain_broken = true;
                            }
                        }
                        previous_receipt_id = Some(evt.id.clone());
                    }

                    let mut fitness = 1.0;
                    let mut deviations = Vec::new();
                    for req in required {
                        if !admitted.contains(req) {
                            fitness -= 0.25;
                            deviations.push(format!("Missing admission for {}", req));
                        }
                    }

                    let (severity, code, message) = if chain_broken {
                        (DiagnosticSeverity::ERROR, "WASM4PM-VERDICT-BLOCKED", "Conformance Verdict: BLOCKED (Chain Broken)".to_string())
                    } else if fitness == 1.0 {
                        (DiagnosticSeverity::INFORMATION, "WASM4PM-VERDICT-FIT", "Conformance Verdict: FIT (Fitness: 1.0)".to_string())
                    } else {
                        (DiagnosticSeverity::ERROR, "WASM4PM-VERDICT-DEVIATION", format!("Conformance Verdict: DEVIATION (Fitness: {}). {}", fitness, deviations.join(", ")))
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
