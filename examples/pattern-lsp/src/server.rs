use crate::diagnostics::Receipt;
use crate::scanner;
use clap_noun_verb::Result;
use clap_noun_verb_macros::verb;
use lsp_types_max::*;
use serde::Serialize;
use tokio::runtime::Runtime;
use tower_lsp_max::{Client, LanguageServer, LspService, Server};

#[derive(Serialize)]
pub struct ServerResult {
    pub success: bool,
}

struct PatternLsp {
    client: Client,
}

impl PatternLsp {
    pub fn new(client: Client) -> Self {
        Self { client }
    }

    async fn check_and_publish(&self, uri: Uri, content: String) {
        if let Ok(findings) = scanner::scan_document(uri.as_str(), &content) {
            let mut diagnostics = Vec::new();
            for f in findings {
                let severity = match f.severity.as_str() {
                    "error" => DiagnosticSeverity::ERROR,
                    "warning" => DiagnosticSeverity::WARNING,
                    "info" => DiagnosticSeverity::INFORMATION,
                    "hint" => DiagnosticSeverity::HINT,
                    _ => DiagnosticSeverity::WARNING,
                };

                let diagnostic = Diagnostic {
                    range: Range::new(
                        Position::new((f.line - 1) as u32, (f.column - 1) as u32),
                        Position::new(
                            (f.line - 1) as u32,
                            (f.column - 1 + f.matched_text.len()) as u32,
                        ),
                    ),
                    severity: Some(severity),
                    code: Some(NumberOrString::String(f.rule_id.clone())),
                    code_description: None,
                    source: Some("pattern-lsp".to_string()),
                    message: format!("Pattern matched: {}", f.matched_text),
                    related_information: None,
                    tags: None,
                    data: Some(serde_json::json!({
                        "rule_id": f.rule_id,
                        "workspace_root": f.workspace_root,
                    })),
                };

                let receipt: Receipt = f.into();
                println!("{}", serde_json::to_string(&receipt).unwrap());

                diagnostics.push(diagnostic);
            }
            self.client
                .publish_diagnostics(uri, diagnostics, None)
                .await;
        }
    }
}

#[tower_lsp_max::async_trait]
impl LanguageServer for PatternLsp {
    async fn initialize(
        &self,
        _: InitializeParams,
    ) -> tower_lsp_max::jsonrpc::Result<InitializeResult> {
        Ok(InitializeResult {
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Kind(
                    TextDocumentSyncKind::FULL,
                )),
                diagnostic_provider: Some(DiagnosticServerCapabilities::Options(
                    DiagnosticOptions {
                        identifier: Some("pattern-lsp".into()),
                        inter_file_dependencies: false,
                        workspace_diagnostics: false,
                        work_done_progress_options: WorkDoneProgressOptions {
                            work_done_progress: None,
                        },
                    },
                )),
                ..ServerCapabilities::default()
            },
            server_info: Some(ServerInfo {
                name: "pattern-lsp".to_string(),
                version: Some("0.1.0".to_string()),
            }),
            offset_encoding: None,
        })
    }

    async fn shutdown(&self) -> tower_lsp_max::jsonrpc::Result<()> {
        Ok(())
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        self.check_and_publish(params.text_document.uri, params.text_document.text)
            .await;
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        if let Some(change) = params.content_changes.into_iter().next() {
            self.check_and_publish(params.text_document.uri, change.text)
                .await;
        }
    }

    async fn did_save(&self, params: DidSaveTextDocumentParams) {
        if let Some(text) = params.text {
            self.check_and_publish(params.text_document.uri, text).await;
        }
    }

    async fn diagnostic(
        &self,
        _params: DocumentDiagnosticParams,
    ) -> tower_lsp_max::jsonrpc::Result<DocumentDiagnosticReportResult> {
        // As a simple example, we might not need to respond to the pull request because we do push based,
        // but if required, we can return the Full report. Let's return Full with empty items and rely on publish for now.
        Ok(DocumentDiagnosticReportResult::Report(
            DocumentDiagnosticReport::Full(RelatedFullDocumentDiagnosticReport {
                related_documents: None,
                full_document_diagnostic_report: FullDocumentDiagnosticReport {
                    result_id: None,
                    items: vec![],
                },
            }),
        ))
    }
}

/// Start the pattern LSP server
/// # Arguments
/// * `stdio` - Use stdio for LSP transport
#[verb("serve")]
pub fn cmd_serve(stdio: bool) -> Result<ServerResult> {
    let rt = Runtime::new().unwrap();
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
