//! Language Server Protocol (LSP) server for the `wasm4pm-compat` process evidence ecosystem.
//!
//! Provides validation of Refusal Laws and Format Covenants in Rust code.
//! Integrates both precise AST parsing (using `syn`) and robust fallback regex scanning.

use clap::Parser;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Mutex;
use syn::spanned::Spanned;
use syn::visit::Visit;
use syn::{Expr, ExprCall, ExprMethodCall, Ident, UseTree};
use tower_lsp_max::jsonrpc::Result;
use tower_lsp_max::lsp_types_max::*;
use tower_lsp_max::{Client, LanguageServer, LspService, Server};
use tracing::{error, info};

/// CLI arguments for the LSP server.
#[derive(Parser, Debug)]
#[command(
    name = "wasm4pm-compat-lsp",
    version = "26.6.5",
    about = "Language Server Protocol daemon for the wasm4pm-compat process-evidence framework"
)]
struct CliArgs {
    /// Run the server over standard input/output (default).
    #[arg(long, default_value_t = true)]
    stdio: bool,

    /// Run the server over a TCP port instead of stdio.
    #[arg(long)]
    port: Option<u16>,

    /// Host address to bind to when running via TCP.
    #[arg(long, default_value = "127.0.0.1")]
    host: String,

    /// Minimum logging level.
    #[arg(long, default_value = "info")]
    log_level: String,
}

/// The stateful LSP server backend.
struct Backend {
    client: Client,
    files: Mutex<HashMap<Url, String>>,
}

impl Backend {
    /// Create a new LSP backend instance.
    fn new(client: Client) -> Self {
        Self {
            client,
            files: Mutex::new(HashMap::new()),
        }
    }

    /// Update a file's content in the internal cache and trigger analysis.
    async fn update_and_analyze(&self, uri: Url, content: String) {
        {
            let mut files = self.files.lock().unwrap();
            files.insert(uri.clone(), content.clone());
        }

        let diagnostics = self.analyze_source(&uri, &content);
        self.client
            .publish_diagnostics(uri, diagnostics, None)
            .await;
    }

    /// Perform analysis using syn (AST parsing) or fallback to regex if syntax is incomplete.
    fn analyze_source(&self, _uri: &Url, content: &str) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();

        // 1. Try structural parsing with `syn`
        match syn::parse_str::<syn::File>(content) {
            Ok(file) => {
                let mut visitor = InvariantVisitor::new();
                visitor.visit_file(&file);
                diagnostics.extend(visitor.diagnostics);
            }
            Err(err) => {
                // If the file is currently being edited and has syntax errors, fallback to regex to keep giving feedback
                info!("AST parsing failed (falling back to regex): {}", err);
                diagnostics.extend(self.analyze_with_regex(content));
            }
        }

        // Integrate lifecycle and format laundering verification rules
        let lifecycle_diagnostics = wasm4pm_compat_lsp::rules::lifecycle::check_source(content);
        for d in lifecycle_diagnostics {
            let severity = match d.severity {
                wasm4pm_compat_lsp::rules::lifecycle::DiagnosticSeverity::Error => {
                    DiagnosticSeverity::ERROR
                }
                wasm4pm_compat_lsp::rules::lifecycle::DiagnosticSeverity::Warning => {
                    DiagnosticSeverity::WARNING
                }
                wasm4pm_compat_lsp::rules::lifecycle::DiagnosticSeverity::Info => {
                    DiagnosticSeverity::INFORMATION
                }
            };
            diagnostics.push(Diagnostic {
                range: Range {
                    start: Position {
                        line: (d.start_line as u32).saturating_sub(1),
                        character: (d.start_col as u32).saturating_sub(1),
                    },
                    end: Position {
                        line: (d.end_line as u32).saturating_sub(1),
                        character: (d.end_col as u32).saturating_sub(1),
                    },
                },
                severity: Some(severity),
                code: Some(NumberOrString::String(d.code)),
                source: Some("wasm4pm-compat-lsp".to_string()),
                message: d.message,
                ..Default::default()
            });
        }

        diagnostics
    }

    /// Fallback regex-based scanner for incomplete or non-compiling files.
    fn analyze_with_regex(&self, content: &str) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();

        // Check 1: Violation of the Refusal Law (detecting generic `InvalidInput` usages)
        if let Ok(invalid_input_re) = regex::Regex::new(r"\bInvalidInput\b") {
            for mat in invalid_input_re.find_iter(content) {
                let (line, col) = get_line_col(content, mat.start());
                let end_col = col + (mat.end() - mat.start());
                diagnostics.push(Diagnostic {
                    range: Range {
                        start: Position { line: line as u32, character: col as u32 },
                        end: Position { line: line as u32, character: end_col as u32 },
                    },
                    severity: Some(DiagnosticSeverity::WARNING),
                    code: Some(NumberOrString::String("W4PM-001".to_string())),
                    source: Some("wasm4pm-compat-lsp".to_string()),
                    message: "Refusal Law Violation: Avoid generic error types like `InvalidInput`. Always use a specific named law (e.g., `DanglingEventObjectLink`, `UnsoundWfNet`) to represent structural refusal.".to_string(),
                    ..Default::default()
                });
            }
        }

        // Check 2: Violation of the Format Covenant (detecting lossy projections without LossPolicy)
        if let Ok(projection_re) = regex::Regex::new(r"\b(flatten_ocel_to_xes|project|flatten)\b") {
            for (i, line) in content.lines().enumerate() {
                if projection_re.is_match(line)
                    && !line.contains("LossPolicy")
                    && !line.contains("policy")
                {
                    if let Some(mat) = projection_re.find(line) {
                        diagnostics.push(Diagnostic {
                            range: Range {
                                start: Position { line: i as u32, character: mat.start() as u32 },
                                end: Position { line: i as u32, character: mat.end() as u32 },
                            },
                            severity: Some(DiagnosticSeverity::ERROR),
                            code: Some(NumberOrString::String("W4PM-002".to_string())),
                            source: Some("wasm4pm-compat-lsp".to_string()),
                            message: "Format Covenant Violation: Lossy projections (like OCEL to XES flattening) must explicitly specify a `LossPolicy` argument.".to_string(),
                            ..Default::default()
                        });
                    }
                }
            }
        }

        diagnostics
    }
}

#[tower_lsp_max::async_trait]
impl LanguageServer for Backend {
    async fn initialize(&self, _: InitializeParams) -> Result<InitializeResult> {
        Ok(InitializeResult {
            server_info: Some(ServerInfo {
                name: "wasm4pm-compat-lsp".to_string(),
                version: Some("26.6.5".to_string()),
            }),
            offset_encoding: None,
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Kind(
                    TextDocumentSyncKind::FULL,
                )),
                code_action_provider: Some(CodeActionProviderCapability::Simple(true)),
                execute_command_provider: Some(ExecuteCommandOptions {
                    commands: vec![
                        "wasm4pm-compat.explainRefusal".to_string(),
                        "wasm4pm-compat.listRefusalLaws".to_string(),
                    ],
                    ..Default::default()
                }),
                ..Default::default()
            },
        })
    }

    async fn initialized(&self, _: InitializedParams) {
        info!("wasm4pm-compat LSP initialized successfully.");
    }

    async fn shutdown(&self) -> Result<()> {
        info!("wasm4pm-compat LSP shutting down.");
        Ok(())
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        self.update_and_analyze(params.text_document.uri, params.text_document.text)
            .await;
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        if let Some(change) = params.content_changes.first() {
            self.update_and_analyze(params.text_document.uri, change.text.clone())
                .await;
        }
    }

    async fn did_save(&self, params: DidSaveTextDocumentParams) {
        let uri = params.text_document.uri;
        let content = {
            let files = self.files.lock().unwrap();
            files.get(&uri).cloned()
        };
        if let Some(text) = content {
            self.update_and_analyze(uri, text).await;
        }
    }

    async fn code_action(&self, params: CodeActionParams) -> Result<Option<CodeActionResponse>> {
        let mut actions = Vec::new();
        let uri = params.text_document.uri;

        for diag in params.context.diagnostics {
            if let Some(NumberOrString::String(ref code)) = diag.code {
                match code.as_str() {
                    "W4PM-002" => {
                        // Quickfix: append LossPolicy parameter
                        #[allow(clippy::mutable_key_type)]
                        let mut changes = HashMap::new();
                        changes.insert(
                            uri.clone(),
                            vec![TextEdit {
                                range: diag.range,
                                new_text: "flatten(LossPolicy::KeepAll)".to_string(),
                            }],
                        );
                        let action = CodeAction {
                            title: "Format Covenant: Add LossPolicy::KeepAll".to_string(),
                            kind: Some(CodeActionKind::QUICKFIX),
                            diagnostics: Some(vec![diag.clone()]),
                            edit: Some(WorkspaceEdit {
                                changes: Some(changes),
                                ..Default::default()
                            }),
                            ..Default::default()
                        };
                        actions.push(CodeActionOrCommand::CodeAction(action));
                    }
                    "W4PM-001" => {
                        // Quickfix: Suggest replacing with an actual Refusal law
                        #[allow(clippy::mutable_key_type)]
                        let mut changes = HashMap::new();
                        changes.insert(
                            uri.clone(),
                            vec![TextEdit {
                                range: diag.range,
                                new_text: "DanglingEventObjectLink".to_string(),
                            }],
                        );
                        let action = CodeAction {
                            title: "Refusal Law: Replace with DanglingEventObjectLink".to_string(),
                            kind: Some(CodeActionKind::QUICKFIX),
                            diagnostics: Some(vec![diag.clone()]),
                            edit: Some(WorkspaceEdit {
                                changes: Some(changes),
                                ..Default::default()
                            }),
                            ..Default::default()
                        };
                        actions.push(CodeActionOrCommand::CodeAction(action));
                    }
                    _ => {}
                }
            }
        }

        if actions.is_empty() {
            Ok(None)
        } else {
            Ok(Some(actions))
        }
    }

    async fn execute_command(&self, params: ExecuteCommandParams) -> Result<Option<Value>> {
        info!("Executing workspace command: {}", params.command);
        match params.command.as_str() {
            "wasm4pm-compat.explainRefusal" => {
                let law = params
                    .arguments
                    .first()
                    .and_then(|v| v.as_str())
                    .unwrap_or("UnknownLaw");

                let explanation = match law {
                    "DanglingEventObjectLink" => "DanglingEventObjectLink: An event references an object that does not exist in the object registry.",
                    "UnqualifiedObjectRelation" => "UnqualifiedObjectRelation: An Event-to-Object (E2O) link lacks a mandatory relationship qualifier.",
                    "DuplicateObjectId" => "DuplicateObjectId: Multiple objects share the same Object ID, violating uniqueness constraints.",
                    "MissingFinalMarking" => "MissingFinalMarking: The Petri net execution path has no reachable final marking, making the workflow net unsound.",
                    "UnsoundWfNet" => "UnsoundWfNet: The workflow net soundness criterion is violated (liveness or boundedness check failed).",
                    "FlatteningLoss" => "FlatteningLoss: OCEL to XES conversion flattened relations without specifying how to handle multi-valued references.",
                    "MissingWitness" => "MissingWitness: A required external-witness registry record could not be found in the witness lattice.",
                    "UnreplayableClaim" => "UnreplayableClaim: The claim cannot be replayed using the supplied receipt chain.",
                    _ => "Unknown or generic refusal law. Custom laws must represent concrete structural process invariants.",
                };

                self.client
                    .show_message(MessageType::INFO, explanation)
                    .await;
                Ok(Some(Value::String(explanation.to_string())))
            }
            "wasm4pm-compat.listRefusalLaws" => {
                let laws = vec![
                    "DanglingEventObjectLink",
                    "UnqualifiedObjectRelation",
                    "DuplicateObjectId",
                    "MissingFinalMarking",
                    "UnsoundWfNet",
                    "FlatteningLoss",
                    "MissingWitness",
                    "UnreplayableClaim",
                ];
                let msg = format!(
                    "Supported named refusal laws in wasm4pm-compat:\n{}",
                    laws.join("\n")
                );
                self.client.show_message(MessageType::INFO, &msg).await;
                Ok(Some(Value::Array(
                    laws.into_iter()
                        .map(|s| Value::String(s.to_string()))
                        .collect(),
                )))
            }
            _ => {
                error!("LSP command not found: {}", params.command);
                Err(tower_lsp_max::jsonrpc::Error::method_not_found())
            }
        }
    }
}

/// Visitor that walks the AST of a Rust file looking for refusal law violations and missing loss policies.
struct InvariantVisitor {
    diagnostics: Vec<Diagnostic>,
}

impl InvariantVisitor {
    fn new() -> Self {
        Self {
            diagnostics: Vec::new(),
        }
    }
}

impl<'ast> Visit<'ast> for InvariantVisitor {
    fn visit_ident(&mut self, node: &'ast Ident) {
        if node == "InvalidInput" {
            self.diagnostics.push(Diagnostic {
                range: span_to_range(node.span()),
                severity: Some(DiagnosticSeverity::WARNING),
                code: Some(NumberOrString::String("W4PM-001".to_string())),
                source: Some("wasm4pm-compat-lsp".to_string()),
                message: "Refusal Law Violation: Avoid generic error types like `InvalidInput`. Always use a specific named law (e.g., `DanglingEventObjectLink`, `UnsoundWfNet`) to represent structural refusal.".to_string(),
                ..Default::default()
            });
        }
        syn::visit::visit_ident(self, node);
    }

    fn visit_use_tree(&mut self, node: &'ast UseTree) {
        // Continue traversing the import tree
        syn::visit::visit_use_tree(self, node);
    }

    fn visit_expr_call(&mut self, node: &'ast ExprCall) {
        if let Expr::Path(ref expr_path) = *node.func {
            if let Some(segment) = expr_path.path.segments.last() {
                let fn_name = segment.ident.to_string();
                if fn_name == "flatten_ocel_to_xes" || fn_name == "flatten" || fn_name == "project"
                {
                    let mut has_loss_policy = false;
                    for arg in &node.args {
                        let arg_str = format!("{:?}", arg);
                        if arg_str.contains("LossPolicy") || arg_str.contains("policy") {
                            has_loss_policy = true;
                        }
                    }
                    if !has_loss_policy {
                        self.diagnostics.push(Diagnostic {
                            range: span_to_range(node.span()),
                            severity: Some(DiagnosticSeverity::ERROR),
                            code: Some(NumberOrString::String("W4PM-002".to_string())),
                            source: Some("wasm4pm-compat-lsp".to_string()),
                            message: "Format Covenant Violation: Lossy projections (like OCEL to XES flattening) must explicitly specify a `LossPolicy` argument.".to_string(),
                            ..Default::default()
                        });
                    }
                }
            }
        }
        syn::visit::visit_expr_call(self, node);
    }

    fn visit_expr_method_call(&mut self, node: &'ast ExprMethodCall) {
        let method_name = node.method.to_string();
        if method_name == "flatten" || method_name == "project" {
            let mut has_loss_policy = false;
            for arg in &node.args {
                let arg_str = format!("{:?}", arg);
                if arg_str.contains("LossPolicy") || arg_str.contains("policy") {
                    has_loss_policy = true;
                }
            }
            if !has_loss_policy {
                self.diagnostics.push(Diagnostic {
                    range: span_to_range(node.span()),
                    severity: Some(DiagnosticSeverity::ERROR),
                    code: Some(NumberOrString::String("W4PM-002".to_string())),
                    source: Some("wasm4pm-compat-lsp".to_string()),
                    message: "Format Covenant Violation: Lossy method projection must explicitly specify a `LossPolicy`.".to_string(),
                    ..Default::default()
                });
            }
        }
        syn::visit::visit_expr_method_call(self, node);
    }
}

/// Convert a `proc_macro2::Span` into an LSP `Range`.
fn span_to_range(span: proc_macro2::Span) -> Range {
    let start = span.start();
    let end = span.end();
    Range {
        start: Position {
            line: (start.line as u32).saturating_sub(1),
            character: start.column as u32,
        },
        end: Position {
            line: (end.line as u32).saturating_sub(1),
            character: end.column as u32,
        },
    }
}

/// Calculate line and column from a character index (byte-offset representation).
fn get_line_col(content: &str, byte_offset: usize) -> (usize, usize) {
    let mut line = 0;
    let mut col = 0;
    for (i, c) in content.char_indices() {
        if i >= byte_offset {
            break;
        }
        if c == '\n' {
            line += 1;
            col = 0;
        } else {
            col += 1;
        }
    }
    (line, col)
}

#[tokio::main]
async fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let args = CliArgs::parse();

    // LSP logs must go to standard error because stdin/stdout are reserved for messages.
    let filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new(&args.log_level));
    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .with_env_filter(filter)
        .init();

    info!("Starting wasm4pm-compat-lsp server");

    if let Some(port) = args.port {
        let addr = format!("{}:{}", args.host, port);
        info!("LSP running in TCP mode, binding to {}", addr);
        let listener = tokio::net::TcpListener::bind(&addr).await?;
        let (stream, _) = listener.accept().await?;
        let (reader, writer) = tokio::io::split(stream);
        let (service, socket) = LspService::new(Backend::new);
        Server::new(reader, writer, socket)
            .serve(service)
            .await
            .unwrap();
    } else {
        info!("LSP running in Stdio mode");
        let stdin = tokio::io::stdin();
        let stdout = tokio::io::stdout();
        let (service, socket) = LspService::new(Backend::new);
        Server::new(stdin, stdout, socket)
            .serve(service)
            .await
            .unwrap();
    }

    Ok(())
}
