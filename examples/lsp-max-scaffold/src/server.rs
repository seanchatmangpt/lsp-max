use crate::diagnostics::ScaffoldDiagnostic;
use crate::law::ScaffoldConformanceVector;
use lsp_max::jsonrpc;
use lsp_max::lsp_types::*;
use lsp_max::{Client, LanguageServer};
use tokio::sync::Mutex;

/// Layer 2: Local LSP state surface.
///
/// Holds per-session law state. The LSP surface is read-only — this server
/// emits diagnostics, hovers, and intents but never mutates files directly.
pub struct ScaffoldServer {
    client: Client,
    conformance: Mutex<ScaffoldConformanceVector>,
}

impl ScaffoldServer {
    pub fn new(client: Client) -> Self {
        Self {
            client,
            conformance: Mutex::new(ScaffoldConformanceVector::new()),
        }
    }

    async fn push_diagnostics(&self, uri: Url, diagnostics: Vec<Diagnostic>) {
        self.client
            .publish_diagnostics(uri, diagnostics, None)
            .await;
    }

    fn gate_check() -> Option<ScaffoldDiagnostic> {
        let path = gate_file_path();
        if path.exists() {
            let blocked = std::fs::read(&path)
                .ok()
                .and_then(|b| b.first().copied())
                .map(|b| b == b'1')
                .unwrap_or(false);
            if blocked {
                return Some(ScaffoldDiagnostic::gate_blocked(
                    &path.display().to_string(),
                ));
            }
        }
        None
    }
}

fn gate_file_path() -> std::path::PathBuf {
    let workspace = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("/"));
    let hash = fnv1a(workspace.to_string_lossy().as_bytes());
    let dir = std::env::var("XDG_RUNTIME_DIR")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| std::path::PathBuf::from("/tmp"));
    dir.join(format!("lsp-max-gate-{hash:016x}"))
}

fn fnv1a(bytes: &[u8]) -> u64 {
    let mut hash: u64 = 0xcbf29ce484222325;
    for &b in bytes {
        hash ^= b as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

fn scaffold_diagnostic_to_lsp(d: &ScaffoldDiagnostic, range: Range) -> Diagnostic {
    Diagnostic {
        range,
        severity: Some(DiagnosticSeverity::WARNING),
        code: Some(NumberOrString::String(d.code.to_string())),
        source: Some("lsp-max-scaffold".to_string()),
        message: d.message.clone(),
        ..Default::default()
    }
}

#[lsp_max::async_trait]
impl LanguageServer for ScaffoldServer {
    async fn initialize(&self, _: InitializeParams) -> jsonrpc::Result<InitializeResult> {
        Ok(InitializeResult {
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Options(
                    TextDocumentSyncOptions {
                        open_close: Some(true),
                        change: Some(TextDocumentSyncKind::FULL),
                        save: Some(TextDocumentSyncSaveOptions::SaveOptions(SaveOptions {
                            include_text: Some(true),
                        })),
                        ..Default::default()
                    },
                )),
                hover_provider: Some(HoverProviderCapability::Simple(true)),
                diagnostic_provider: Some(DiagnosticServerCapabilities::Options(
                    DiagnosticOptions {
                        identifier: Some("lsp-max-scaffold".to_string()),
                        inter_file_dependencies: false,
                        workspace_diagnostics: false,
                        work_done_progress_options: WorkDoneProgressOptions {
                            work_done_progress: None,
                        },
                    },
                )),
                code_action_provider: Some(CodeActionProviderCapability::Simple(true)),
                ..Default::default()
            },
            server_info: Some(ServerInfo {
                name: "lsp-max-scaffold".to_string(),
                version: Some(env!("CARGO_PKG_VERSION").to_string()),
            }),
            offset_encoding: None,
        })
    }

    async fn initialized(&self, _: InitializedParams) {
        self.client
            .log_message(
                MessageType::INFO,
                "lsp-max-scaffold: CANDIDATE — law axes UNKNOWN until receipts are produced",
            )
            .await;
    }

    async fn shutdown(&self) -> jsonrpc::Result<()> {
        Ok(())
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        let uri = params.text_document.uri;
        let mut lsp_diags: Vec<Diagnostic> = vec![];

        if let Some(gate_diag) = Self::gate_check() {
            let range = Range {
                start: Position::new(0, 0),
                end: Position::new(0, 0),
            };
            lsp_diags.push(scaffold_diagnostic_to_lsp(&gate_diag, range));
        }

        self.push_diagnostics(uri, lsp_diags).await;
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        let uri = params.text_document.uri;
        let content = params
            .content_changes
            .into_iter()
            .last()
            .map(|c| c.text)
            .unwrap_or_default();

        let mut lsp_diags: Vec<Diagnostic> = vec![];

        if let Some(gate_diag) = Self::gate_check() {
            let range = Range {
                start: Position::new(0, 0),
                end: Position::new(0, 0),
            };
            lsp_diags.push(scaffold_diagnostic_to_lsp(&gate_diag, range));
        }

        let conformance = self.conformance.lock().await;
        if conformance.unknown.is_empty() && conformance.admitted.is_empty() {
            let d = crate::diagnostics::ScaffoldDiagnostic::unknown_collapsed("all axes");
            let range = Range {
                start: Position::new(0, 0),
                end: Position::new(0, content.lines().count().saturating_sub(1) as u32),
            };
            lsp_diags.push(scaffold_diagnostic_to_lsp(&d, range));
        }

        self.push_diagnostics(uri, lsp_diags).await;
    }

    async fn hover(&self, params: HoverParams) -> jsonrpc::Result<Option<Hover>> {
        let conformance = self.conformance.lock().await;
        let label = conformance.status_label();
        let score = conformance
            .score()
            .map(|s| format!("{:.0}%", s * 100.0))
            .unwrap_or_else(|| "N/A".to_string());

        let _ = params;
        Ok(Some(Hover {
            contents: HoverContents::Markup(MarkupContent {
                kind: MarkupKind::Markdown,
                value: format!(
                    "**lsp-max-scaffold** conformance\n\n\
                     Status: `{label}`  \n\
                     Score: `{score}`  \n\
                     Admitted: {}  \n\
                     Refused: {}  \n\
                     Unknown: {}",
                    conformance.admitted.len(),
                    conformance.refused.len(),
                    conformance.unknown.len(),
                ),
            }),
            range: None,
        }))
    }
}
