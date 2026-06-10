use gc005_wasm4pm_adapter::analyze_ocel;
use lsp_max::jsonrpc::Result;
use lsp_max::lsp_types::*;
use lsp_max::{Client, LspService, Server};

#[derive(Debug)]
struct Backend {
    client: Client,
}

#[lsp_max::async_trait]
impl lsp_max::LanguageServer for Backend {
    async fn initialize(&self, _: InitializeParams) -> Result<InitializeResult> {
        Ok(InitializeResult {
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Kind(
                    TextDocumentSyncKind::FULL,
                )),
                code_action_provider: Some(CodeActionProviderCapability::Simple(true)),
                ..Default::default()
            },
            ..Default::default()
        })
    }

    async fn initialized(&self, _: InitializedParams) {
        self.client
            .log_message(MessageType::INFO, "wasm4pm-lsp initialized")
            .await;
    }

    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        self.diagnose(params.text_document.uri, params.text_document.text)
            .await;
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        self.diagnose(
            params.text_document.uri,
            params.content_changes[0].text.clone(),
        )
        .await;
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
                            arguments: Some(vec![serde_json::to_value(
                                params.text_document.uri.clone(),
                            )
                            .unwrap()]),
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
        let path = uri.path().to_string();

        if path.ends_with(".ocel.json") {
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

        // CLAP-PACK-HANDLER-UNBOUND: cli.rs without clap-noun-verb handler binding
        if path.ends_with("cli.rs") {
            let has_verb_binding = content.contains("#[verb")
                || content.contains("clap_noun_verb")
                || content.contains("clap-noun-verb");
            if !has_verb_binding {
                diags.push(Diagnostic {
                    range: Range::default(),
                    severity: Some(DiagnosticSeverity::ERROR),
                    code: Some(NumberOrString::String(
                        "CLAP-PACK-HANDLER-UNBOUND".to_string(),
                    )),
                    message: "cli.rs has no clap-noun-verb handler binding".to_string(),
                    source: Some("wasm4pm-lsp".to_string()),
                    data: Some(serde_json::json!({"source_id": "clap_noun_verb_pack_lsp"})),
                    ..Default::default()
                });
            }
            // ggen projection state overlay when ggen:override marker present
            if content.contains("ggen:override") {
                diags.push(Diagnostic {
                    range: Range::default(),
                    severity: Some(DiagnosticSeverity::INFORMATION),
                    code: Some(NumberOrString::String(
                        "GGEN-PROJECTION-OVERLAY".to_string(),
                    )),
                    message: "ggen projection state overlay active".to_string(),
                    source: Some("wasm4pm-lsp".to_string()),
                    data: Some(serde_json::json!({"source_id": "ggen_lsp_observer"})),
                    ..Default::default()
                });
            }
        }

        // TOWER-PACK-UNGUARDED-MUTATION: .rs files with direct mutation functions
        if path.ends_with(".rs") && !path.ends_with("cli.rs") {
            let has_mutation = content.contains("write_to_disk")
                || content.contains("fn write_")
                || content.contains("fn delete_")
                || content.contains("fn mutate_")
                || content.contains("fn update_file");
            if has_mutation {
                diags.push(Diagnostic {
                    range: Range::default(),
                    severity: Some(DiagnosticSeverity::ERROR),
                    code: Some(NumberOrString::String(
                        "TOWER-PACK-UNGUARDED-MUTATION".to_string(),
                    )),
                    message: "LSP surface must be read-only; direct file mutation detected"
                        .to_string(),
                    source: Some("wasm4pm-lsp".to_string()),
                    data: Some(serde_json::json!({"source_id": "lsp_max_pack_lsp"})),
                    ..Default::default()
                });
            }
        }

        // GGEN-EVIDENCE-001: receipts.json must be valid receipt JSON
        if path.ends_with("receipts.json") || path.ends_with(".receipt.json") {
            let is_valid_receipt = serde_json::from_str::<serde_json::Value>(&content)
                .ok()
                .and_then(|v| v.get("digest").cloned())
                .is_some();
            if !is_valid_receipt {
                diags.push(Diagnostic {
                    range: Range::default(),
                    severity: Some(DiagnosticSeverity::ERROR),
                    code: Some(NumberOrString::String("GGEN-EVIDENCE-001".to_string())),
                    message: "receipts.json is not a valid receipt (missing digest field)"
                        .to_string(),
                    source: Some("wasm4pm-lsp".to_string()),
                    data: Some(serde_json::json!({"source_id": "ggen_lsp_observer"})),
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
    Server::new(stdin, stdout, socket)
        .serve(service)
        .await
        .unwrap();
}
