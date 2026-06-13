use clap_noun_verb_macros::verb;
use lsp_max::{Client, LanguageServer, LspService, Server};
use lsp_types_max::*;
use parking_lot::RwLock;
use regex::Regex;
use std::collections::HashMap;
use std::sync::OnceLock;

// ── Axum-specific rules (inline — no external TOML files) ─────────────────────

struct AxumRule {
    id: &'static str,
    severity: DiagnosticSeverity,
    re: fn() -> &'static Regex,
    message: &'static str,
}

fn unwrap_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"\.unwrap\(\)").unwrap())
}

fn blocking_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"std::thread::sleep|std::fs::read|std::fs::write").unwrap())
}

const AXUM_RULES: &[AxumRule] = &[
    AxumRule {
        id: "AXUM-001",
        severity: DiagnosticSeverity::ERROR,
        re: unwrap_re,
        message: "unwrap() in handler — use ? or map_err instead",
    },
    AxumRule {
        id: "AXUM-002",
        severity: DiagnosticSeverity::WARNING,
        re: blocking_re,
        message: "blocking call in async context — use tokio equivalents",
    },
];

// ── Backend ───────────────────────────────────────────────────────────────────

struct AxumBackend {
    client: Client,
    /// uri → document text. Read-heavy.
    docs: RwLock<HashMap<Url, String>>,
}

impl AxumBackend {
    fn new(client: Client) -> Self {
        Self {
            client,
            docs: RwLock::new(HashMap::new()),
        }
    }

    async fn analyze_and_publish(&self, uri: Url, text: String) {
        let mut diags = Vec::new();
        for (line_idx, line) in text.lines().enumerate() {
            for rule in AXUM_RULES {
                let re = (rule.re)();
                if let Some(mat) = re.find(line) {
                    diags.push(Diagnostic {
                        range: Range {
                            start: Position {
                                line: line_idx as u32,
                                character: mat.start() as u32,
                            },
                            end: Position {
                                line: line_idx as u32,
                                character: mat.end() as u32,
                            },
                        },
                        severity: Some(rule.severity),
                        code: Some(NumberOrString::String(rule.id.to_string())),
                        source: Some("axum-lsp".to_string()),
                        message: rule.message.to_string(),
                        ..Default::default()
                    });
                }
            }
        }
        self.docs.write().insert(uri.clone(), text);
        self.client.publish_diagnostics(uri, diags, None).await;
    }
}

#[lsp_max::async_trait]
impl LanguageServer for AxumBackend {
    async fn initialize(&self, _: InitializeParams) -> lsp_max::jsonrpc::Result<InitializeResult> {
        Ok(InitializeResult {
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Kind(
                    TextDocumentSyncKind::FULL,
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
        self.analyze_and_publish(params.text_document.uri, params.text_document.text)
            .await;
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
}

// ── CLI entry point ───────────────────────────────────────────────────────────

#[verb("start")]
fn start_server(_stdio: bool) -> clap_noun_verb::Result<()> {
    tokio::runtime::Runtime::new().unwrap().block_on(async {
        let stdin = tokio::io::stdin();
        let stdout = tokio::io::stdout();
        let (service, socket) = LspService::new(AxumBackend::new);
        let _ = Server::new(stdin, stdout, socket).serve(service).await;
    });
    Ok(())
}

fn main() -> clap_noun_verb::Result<()> {
    clap_noun_verb::run()
}
