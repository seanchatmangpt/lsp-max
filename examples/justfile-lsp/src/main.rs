//! justfile-lsp — LSP server for Justfiles
//!
//! Emits diagnostics on:
//!   JUST-001  Recipe name uses underscore — prefer kebab-case
//!   JUST-002  Private recipe (`_prefix`) — hidden from `just --list`
//!   JUST-003  Recipe body line not prefixed with `@` — command echoed to stdout
//!   JUST-004  Unquoted variable expansion — potential word-splitting
//!   JUST-005  Victory language in comment or echo — use bounded status words
//!
//! Built on lsp-max. Never uses lsp-server.

mod parser;

use lsp_max::{
    primitives::{debounce, DebounceHandle, DiagnosticSink, DocumentStore},
    Client, LanguageServer, LspService, Server,
};
use lsp_types_max::*;
use parser::Severity;
use std::time::Duration;

// ── Backend ───────────────────────────────────────────────────────────────────

struct JustfileLsp {
    docs: DocumentStore,
    sink: DiagnosticSink,
}

impl JustfileLsp {
    fn new(client: Client) -> Self {
        Self {
            docs: DocumentStore::new(),
            sink: DiagnosticSink::new(client),
        }
    }

    async fn analyze_and_publish(&self, uri: Url) {
        let Some(text) = self.docs.get_content(&uri) else {
            return;
        };
        let raw_diags = parser::parse(&text);

        let diags: Vec<Diagnostic> = raw_diags
            .into_iter()
            .map(|d| {
                let severity = match d.severity {
                    Severity::Error => DiagnosticSeverity::ERROR,
                    Severity::Warning => DiagnosticSeverity::WARNING,
                    Severity::Info => DiagnosticSeverity::INFORMATION,
                };
                Diagnostic {
                    range: Range {
                        start: Position {
                            line: d.line,
                            character: d.column_start,
                        },
                        end: Position {
                            line: d.line,
                            character: d.column_end,
                        },
                    },
                    severity: Some(severity),
                    code: Some(NumberOrString::String(d.code.to_string())),
                    source: Some("justfile-lsp".to_string()),
                    message: d.message,
                    ..Default::default()
                }
            })
            .collect();

        self.sink.publish(uri, diags).await;
    }
}

#[lsp_max::async_trait]
impl LanguageServer for JustfileLsp {
    async fn initialize(&self, _: InitializeParams) -> lsp_max::jsonrpc::Result<InitializeResult> {
        Ok(InitializeResult {
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Kind(
                    TextDocumentSyncKind::FULL,
                )),
                ..Default::default()
            },
            server_info: Some(ServerInfo {
                name: "justfile-lsp".to_string(),
                version: Some(env!("CARGO_PKG_VERSION").to_string()),
            }),
            ..Default::default()
        })
    }

    async fn shutdown(&self) -> lsp_max::jsonrpc::Result<()> {
        Ok(())
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        let uri = params.text_document.uri;
        self.docs.open(
            uri.clone(),
            params.text_document.text,
            params.text_document.version,
        );
        self.analyze_and_publish(uri).await;
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        let uri = params.text_document.uri;
        self.docs
            .update(&uri, params.content_changes, params.text_document.version);
        self.analyze_and_publish(uri).await;
    }

    async fn did_save(&self, params: DidSaveTextDocumentParams) {
        let uri = params.text_document.uri;
        if self.docs.is_open(&uri) {
            self.analyze_and_publish(uri).await;
        }
    }

    async fn did_close(&self, params: DidCloseTextDocumentParams) {
        let uri = params.text_document.uri;
        self.docs.close(&uri);
        self.sink.clear(&uri).await;
    }
}

// ── Entry point ───────────────────────────────────────────────────────────────

#[tokio::main]
async fn main() {
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();
    let (service, socket) = LspService::new(JustfileLsp::new);
    Server::new(stdin, stdout, socket)
        .serve(service)
        .await
        .unwrap();
}

// Show that `debounce` compiles — an integration point for editors that send
// rapid `didChange` bursts.
#[allow(dead_code)]
fn _debounce_compiles_check(docs: DocumentStore, sink: DiagnosticSink, uri: Url) -> DebounceHandle {
    debounce(Duration::from_millis(150), move || {
        let docs = docs.clone();
        let sink = sink.clone();
        let uri = uri.clone();
        async move {
            let Some(text) = docs.get_content(&uri) else {
                return;
            };
            let diags = parser::parse(&text)
                .into_iter()
                .map(|_d| Diagnostic::default())
                .collect::<Vec<_>>();
            sink.publish(uri, diags).await;
        }
    })
}
