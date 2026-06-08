use serde::{Deserialize, Serialize};
use tower_lsp_max::jsonrpc::Result;
use tower_lsp_max::lsp_types::*;
use tower_lsp_max::{Client, LspService, Server};
use gc005_wasm4pm_adapter::analyze_ocel;

#[derive(Debug)]
struct Backend {
    client: Client,
}

#[tower_lsp_max::async_trait]
impl tower_lsp_max::LanguageServer for Backend {
    async fn initialize(&self, _: InitializeParams) -> Result<InitializeResult> {
        Ok(InitializeResult {
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Kind(TextDocumentSyncKind::FULL)),
                code_action_provider: Some(CodeActionProviderCapability::Simple(true)),
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

    async fn code_action(&self, params: CodeActionParams) -> Result<Option<CodeActionResponse>> {
        let mut actions = Vec::new();
        for diag in params.context.diagnostics {
            if let Some(NumberOrString::String(code)) = &diag.code {
                if code == "WASM4PM-VERDICT-FIT" {
                    actions.push(CodeActionOrCommand::CodeAction(CodeAction {
                        title: "Bind Conformance Receipt".to_string(),
                        kind: Some(CodeActionKind::QUICKFIX),
                        diagnostics: Some(vec![diag.clone()]),
                        command: Some(Command {
                            title: "Bind Conformance Receipt".to_string(),
                            command: "conformance-receipt.bind".to_string(),
                            arguments: Some(vec![serde_json::to_value(params.text_document.uri.clone()).unwrap()]),
                        }),
                        ..Default::default()
                    }));
                }
            }
        }
        Ok(Some(actions))
    }
}

impl Backend {
    async fn diagnose(&self, uri: Url, content: String) {
        let mut diags = Vec::new();
        
        if uri.path().as_str().ends_with(".ocel.json") {
            let issues = analyze_ocel(&content);
            for issue in issues {
                let severity = match issue.severity.as_str() {
                    "INFORMATION" => DiagnosticSeverity::INFORMATION,
                    _ => DiagnosticSeverity::ERROR,
                };
                diags.push(Diagnostic {
                    range: Range::default(),
                    severity: Some(severity),
                    code: Some(NumberOrString::String(issue.code)),
                    message: issue.message,
                    source: Some("wasm4pm-lsp".to_string()),
                    ..Default::default()
                });
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
