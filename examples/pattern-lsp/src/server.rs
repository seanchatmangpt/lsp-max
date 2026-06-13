use clap_noun_verb::Result;
use clap_noun_verb_macros::verb;
use lsp_max::{Client, LanguageServer, LspService, Server};
use lsp_types_max::*;
use parking_lot::RwLock;
use serde::Serialize;
use std::collections::HashMap;
use std::sync::OnceLock;

use crate::scanner::scan_document;

// ── Compiled-once severity map ────────────────────────────────────────────────

fn severity_map() -> &'static HashMap<&'static str, DiagnosticSeverity> {
    static MAP: OnceLock<HashMap<&'static str, DiagnosticSeverity>> = OnceLock::new();
    MAP.get_or_init(|| {
        let mut m = HashMap::new();
        m.insert("error", DiagnosticSeverity::ERROR);
        m.insert("warning", DiagnosticSeverity::WARNING);
        m.insert("info", DiagnosticSeverity::INFORMATION);
        m.insert("hint", DiagnosticSeverity::HINT);
        m
    })
}

// ── Backend ───────────────────────────────────────────────────────────────────

struct PatternLsp {
    client: Client,
    /// uri → document text. Read-heavy: written on open/change, read on diagnostics.
    docs: RwLock<HashMap<Url, String>>,
}

impl PatternLsp {
    fn new(client: Client) -> Self {
        Self {
            client,
            docs: RwLock::new(HashMap::new()),
        }
    }

    async fn analyze_and_publish(&self, uri: Url, text: String) {
        let findings = {
            let uri_str = uri.to_string();
            scan_document(&uri_str, &text).unwrap_or_default()
        };

        let diags: Vec<Diagnostic> = findings
            .into_iter()
            .map(|f| {
                let severity = severity_map()
                    .get(f.severity.to_lowercase().as_str())
                    .copied()
                    .unwrap_or(DiagnosticSeverity::WARNING);
                let line = (f.line.saturating_sub(1)) as u32;
                let col = (f.column.saturating_sub(1)) as u32;
                Diagnostic {
                    range: Range {
                        start: Position {
                            line,
                            character: col,
                        },
                        end: Position {
                            line,
                            character: col + f.matched_text.len() as u32,
                        },
                    },
                    severity: Some(severity),
                    code: Some(NumberOrString::String(f.rule_id)),
                    source: Some("pattern-lsp".to_string()),
                    message: f.matched_text,
                    ..Default::default()
                }
            })
            .collect();

        self.docs.write().insert(uri.clone(), text);
        self.client.publish_diagnostics(uri, diags, None).await;
    }
}

#[lsp_max::async_trait]
impl LanguageServer for PatternLsp {
    async fn initialize(&self, _: InitializeParams) -> lsp_max::jsonrpc::Result<InitializeResult> {
        Ok(InitializeResult {
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Kind(
                    TextDocumentSyncKind::FULL,
                )),
                diagnostic_provider: Some(DiagnosticServerCapabilities::Options(
                    DiagnosticOptions {
                        inter_file_dependencies: false,
                        workspace_diagnostics: false,
                        ..Default::default()
                    },
                )),
                ..Default::default()
            },
            ..Default::default()
        })
    }

    async fn shutdown(&self) -> lsp_max::jsonrpc::Result<()> {
        Ok(())
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        let uri = params.text_document.uri;
        let text = params.text_document.text;
        self.analyze_and_publish(uri, text).await;
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        let uri = params.text_document.uri;
        if let Some(change) = params.content_changes.into_iter().last() {
            self.analyze_and_publish(uri, change.text).await;
        }
    }

    async fn did_close(&self, params: DidCloseTextDocumentParams) {
        let uri = params.text_document.uri;
        self.docs.write().remove(&uri);
        self.client.publish_diagnostics(uri, vec![], None).await;
    }

    async fn did_save(&self, params: DidSaveTextDocumentParams) {
        let uri = params.text_document.uri;
        let text = {
            let docs = self.docs.read();
            docs.get(&uri).cloned()
        };
        if let Some(t) = text.or(params.text) {
            self.analyze_and_publish(uri, t).await;
        }
    }
}

// ── CLI verb ──────────────────────────────────────────────────────────────────

#[derive(Serialize)]
pub struct ServerResult {
    pub success: bool,
}

/// Start the pattern LSP server
#[verb("serve")]
pub fn cmd_serve(stdio: bool) -> Result<ServerResult> {
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        if stdio {
            let stdin = tokio::io::stdin();
            let stdout = tokio::io::stdout();
            let (service, socket) = LspService::new(PatternLsp::new);
            Server::new(stdin, stdout, socket)
                .serve(service)
                .await
                .unwrap();
        }
    });
    Ok(ServerResult { success: true })
}
