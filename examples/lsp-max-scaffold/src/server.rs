use crate::analyzer::DefaultAnalyzer;
use crate::diagnostics::ScaffoldDiagnostic;
use crate::law::ScaffoldConformanceVector;
use crate::verifiable::{VerifiableDiagnostic, VerifiableEngine};
use lsp_max::jsonrpc;
use lsp_max::lsp_types::*;
use lsp_max::{Client, LanguageServer};
use tokio::sync::Mutex;

/// Layer 2: Local LSP state surface.
///
/// Holds per-session law state. The LSP surface is read-only — this server
/// emits diagnostics, hovers, and code actions but never mutates files. Every
/// diagnostic it publishes carries a replay-verifiable receipt in `data`.
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

    /// Build proof-carrying diagnostics for a document, plus any gate diagnostic.
    fn diagnostics_for(text: &str) -> Vec<Diagnostic> {
        let analyzer = DefaultAnalyzer::new();
        let mut engine = VerifiableEngine::new(&analyzer);
        let mut out: Vec<Diagnostic> = engine
            .extend(text)
            .iter()
            .map(|vd| verifiable_to_lsp(text, vd))
            .collect();
        if let Some(gate_diag) = Self::gate_check() {
            out.push(scaffold_diagnostic_to_lsp(&gate_diag, zero_range()));
        }
        out
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

fn zero_range() -> Range {
    Range {
        start: Position::new(0, 0),
        end: Position::new(0, 0),
    }
}

/// Map a byte offset in `text` to an LSP (line, utf-16 character) position.
fn offset_to_position(text: &str, offset: usize) -> Position {
    let offset = offset.min(text.len());
    let mut line = 0u32;
    let mut line_start = 0usize;
    for (i, b) in text.bytes().enumerate() {
        if i >= offset {
            break;
        }
        if b == b'\n' {
            line += 1;
            line_start = i + 1;
        }
    }
    let character = text[line_start..offset].encode_utf16().count() as u32;
    Position::new(line, character)
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

/// Convert a verifiable diagnostic to LSP, embedding its proof in `data` so the
/// receipt travels on the wire and any client can re-verify it.
fn verifiable_to_lsp(text: &str, vd: &VerifiableDiagnostic) -> Diagnostic {
    let range = Range {
        start: offset_to_position(text, vd.witness.doc_span.0),
        end: offset_to_position(text, vd.witness.doc_span.1),
    };
    let data = serde_json::json!({
        "receipt": vd.receipt,
        "witness": vd.witness,
    });
    Diagnostic {
        range,
        severity: Some(DiagnosticSeverity::WARNING),
        code: Some(NumberOrString::String(vd.code.clone())),
        source: Some("lsp-max-scaffold/rvd".to_string()),
        message: vd.message.clone(),
        data: Some(data),
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
                "lsp-max-scaffold: CANDIDATE — diagnostics carry replay-verifiable receipts",
            )
            .await;
    }

    async fn shutdown(&self) -> jsonrpc::Result<()> {
        Ok(())
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        let uri = params.text_document.uri.clone();
        let diags = Self::diagnostics_for(&params.text_document.text);
        self.push_diagnostics(uri, diags).await;
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        let uri = params.text_document.uri;
        let text = params
            .content_changes
            .into_iter()
            .last()
            .map(|c| c.text)
            .unwrap_or_default();
        let diags = Self::diagnostics_for(&text);
        self.push_diagnostics(uri, diags).await;
    }

    async fn did_save(&self, params: DidSaveTextDocumentParams) {
        let uri = params.text_document.uri.clone();
        let text = params.text.unwrap_or_default();
        let diags = Self::diagnostics_for(&text);
        self.push_diagnostics(uri, diags).await;
    }

    async fn code_action(
        &self,
        params: CodeActionParams,
    ) -> jsonrpc::Result<Option<CodeActionResponse>> {
        let actions: Vec<CodeActionOrCommand> = params
            .context
            .diagnostics
            .iter()
            .filter_map(|d| {
                let code = match &d.code {
                    Some(NumberOrString::String(s)) => s.as_str(),
                    _ => return None,
                };
                let title = match code {
                    "RVD-FORK-001" => "Remove the forbidden fork reference (use lsp-max)",
                    "RVD-VICTORY-001" => "Replace victory language with a bounded status",
                    "SCAFFOLD-GATE-001" => {
                        "Resolve WASM4PM-* / GGEN-* diagnostics to clear the ANDON gate"
                    }
                    _ => return None,
                };
                Some(CodeActionOrCommand::CodeAction(CodeAction {
                    title: title.to_string(),
                    kind: Some(CodeActionKind::QUICKFIX),
                    diagnostics: Some(vec![d.clone()]),
                    ..Default::default()
                }))
            })
            .collect();

        if actions.is_empty() {
            Ok(None)
        } else {
            Ok(Some(actions))
        }
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
