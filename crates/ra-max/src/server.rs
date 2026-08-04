use std::collections::HashMap;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

use lsp_max::{Client, LanguageServer};
use lsp_types_max::*;
use parking_lot::{Mutex, RwLock};

use crate::identity::{digest_bytes, ProjectAdmission, SemanticSubject};
use crate::intelligence::{CompletionCandidate, IntelligenceError, Occurrence, WorkspaceIndex};
use crate::receipt::{Outcome, Receipt, ReceiptChain, ReceiptKind};
use crate::semantic::{SemanticEngine, SemanticSnapshot, SourceRange, Symbol, TreeSitterRustEngine};
use crate::WorkspaceState;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct IndexSummary {
    pub index_hash: String,
    pub symbol_count: usize,
    pub occurrence_count: usize,
    pub verified: bool,
    pub lexical_only: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ServerSnapshot {
    pub admission: Option<ProjectAdmission>,
    pub semantic: Option<SemanticSnapshot>,
    pub index: Option<IndexSummary>,
    pub receipts: Vec<Receipt>,
    pub receipt_chain_valid: bool,
}

struct ManufacturedAnalysis {
    admission: ProjectAdmission,
    semantic: SemanticSnapshot,
    index: WorkspaceIndex,
}

/// LSP surface for the RFC 0006 80/20 implementation.
///
/// Document notifications update admitted observations only. Semantic analysis
/// and workspace indexing are immutable. Navigation and refactoring are
/// intentionally lexical: ambiguous Rust scope is refused rather than guessed.
pub struct RaMaxServer {
    client: Client,
    workspace: RwLock<WorkspaceState>,
    receipts: Mutex<ReceiptChain>,
    engine: TreeSitterRustEngine,
}

impl RaMaxServer {
    pub fn new(client: Client) -> Self {
        Self {
            client,
            workspace: RwLock::new(WorkspaceState::default()),
            receipts: Mutex::new(ReceiptChain::default()),
            engine: TreeSitterRustEngine,
        }
    }

    fn manufacture_analysis(&self) -> Option<ManufacturedAnalysis> {
        let workspace = self.workspace.read().clone();
        if workspace.files().is_empty() {
            return None;
        }

        let subject = SemanticSubject::tree_sitter_vertical_slice();
        let admission = ProjectAdmission::from_files(
            subject.clone(),
            "lsp://ra-max-workspace",
            workspace.files(),
        );
        let semantic = self.engine.analyze(&admission, workspace.files());
        let index = WorkspaceIndex::build(&admission, &semantic, workspace.files());
        let mut receipts = self.receipts.lock();
        receipts.append(
            ReceiptKind::Admission,
            subject.digest(),
            digest_bytes(b"lsp document observation"),
            admission.project_hash.clone(),
            Outcome::Admitted,
        );
        receipts.append(
            ReceiptKind::SemanticRevision,
            admission.project_hash.clone(),
            digest_bytes(self.engine.name().as_bytes()),
            semantic.revision_hash.clone(),
            Outcome::Executed,
        );

        Some(ManufacturedAnalysis {
            admission,
            semantic,
            index,
        })
    }

    fn manufacture_snapshot(&self) -> ServerSnapshot {
        let analysis = self.manufacture_analysis();
        let receipts = self.receipts.lock();
        let (admission, semantic, index) = match analysis {
            Some(analysis) => {
                let summary = IndexSummary {
                    index_hash: analysis.index.index_hash.clone(),
                    symbol_count: analysis.index.symbols.len(),
                    occurrence_count: analysis.index.occurrences.len(),
                    verified: analysis.index.verify(),
                    lexical_only: true,
                };
                (
                    Some(analysis.admission),
                    Some(analysis.semantic),
                    Some(summary),
                )
            }
            None => (None, None, None),
        };

        ServerSnapshot {
            admission,
            semantic,
            index,
            receipts: receipts.receipts().to_vec(),
            receipt_chain_valid: receipts.verify(),
        }
    }

    async fn publish_snapshot_diagnostics(&self, uri: DocumentUri, version: Option<i32>) {
        let snapshot = self.manufacture_snapshot();
        let uri_text = uri.as_str();
        let diagnostics = snapshot
            .semantic
            .into_iter()
            .flat_map(|semantic| semantic.diagnostics)
            .filter(|diagnostic| diagnostic.path == uri_text)
            .map(|diagnostic| Diagnostic {
                range: to_lsp_range(&diagnostic.range),
                severity: Some(DiagnosticSeverity::ERROR),
                code: Some(NumberOrString::String(diagnostic.code)),
                code_description: None,
                source: Some("ra-max".to_owned()),
                message: diagnostic.message,
                related_information: None,
                tags: None,
                data: None,
            })
            .collect();
        self.client
            .publish_diagnostics(uri, diagnostics, version)
            .await;
    }

    async fn log_refusal(&self, error: &IntelligenceError) {
        self.client
            .log_message(MessageType::WARNING, format!("ra-max refusal: {error}"))
            .await;
    }

    pub async fn max_snapshot(&self) -> lsp_max::jsonrpc::Result<ServerSnapshot> {
        Ok(self.manufacture_snapshot())
    }
}

#[lsp_max::async_trait]
impl LanguageServer for RaMaxServer {
    async fn initialize(&self, _: InitializeParams) -> lsp_max::jsonrpc::Result<InitializeResult> {
        Ok(InitializeResult {
            capabilities: ServerCapabilities {
                position_encoding: Some(PositionEncodingKind::UTF8),
                text_document_sync: Some(TextDocumentSyncCapability::Kind(
                    TextDocumentSyncKind::FULL,
                )),
                hover_provider: Some(HoverProviderCapability::Simple(true)),
                definition_provider: Some(OneOf::Left(true)),
                references_provider: Some(OneOf::Left(true)),
                completion_provider: Some(CompletionOptions {
                    resolve_provider: Some(false),
                    trigger_characters: Some(vec![".".to_owned(), ":".to_owned()]),
                    ..Default::default()
                }),
                document_symbol_provider: Some(OneOf::Left(true)),
                workspace_symbol_provider: Some(OneOf::Left(true)),
                rename_provider: Some(OneOf::Right(RenameOptions {
                    prepare_provider: Some(true),
                    work_done_progress_options: Default::default(),
                })),
                ..Default::default()
            },
            server_info: Some(ServerInfo {
                name: "ra-max".to_owned(),
                version: Some(env!("CARGO_PKG_VERSION").to_owned()),
            }),
            ..Default::default()
        })
    }

    async fn initialized(&self, _: InitializedParams) {
        self.client
            .log_message(
                MessageType::INFO,
                "ra-max initialized: receipt-bound structural Rust intelligence; lexical-only scope",
            )
            .await;
    }

    async fn shutdown(&self) -> lsp_max::jsonrpc::Result<()> {
        Ok(())
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        let uri = params.text_document.uri;
        let version = Some(params.text_document.version);
        self.workspace
            .write()
            .insert_observation(uri.as_str(), params.text_document.text);
        self.publish_snapshot_diagnostics(uri, version).await;
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        let uri = params.text_document.uri;
        let version = Some(params.text_document.version);
        if let Some(change) = params.content_changes.into_iter().last() {
            self.workspace
                .write()
                .insert_observation(uri.as_str(), change.text);
        }
        self.publish_snapshot_diagnostics(uri, version).await;
    }

    async fn did_close(&self, params: DidCloseTextDocumentParams) {
        let uri = params.text_document.uri;
        self.workspace.write().remove_observation(uri.as_str());
        self.client.publish_diagnostics(uri, Vec::new(), None).await;
    }

    async fn hover(&self, params: HoverParams) -> lsp_max::jsonrpc::Result<Option<Hover>> {
        let position = params.text_document_position_params.position;
        let path = params
            .text_document_position_params
            .text_document
            .uri
            .to_string();
        let Some(analysis) = self.manufacture_analysis() else {
            return Ok(None);
        };
        match analysis.index.hover_at(&path, position.line, position.character) {
            Ok(info) => Ok(Some(Hover {
                contents: HoverContents::Markup(MarkupContent {
                    kind: MarkupKind::Markdown,
                    value: format!(
                        "```rust\n{}\n```\n\n`{}` · lexical workspace resolution · revision `{}`",
                        info.symbol.signature, info.symbol.kind, analysis.semantic.revision_hash
                    ),
                }),
                range: Some(to_lsp_range(&info.occurrence.range)),
            })),
            Err(error) => {
                self.log_refusal(&error).await;
                Ok(None)
            }
        }
    }

    async fn goto_definition(
        &self,
        params: GotoDefinitionParams,
    ) -> lsp_max::jsonrpc::Result<Option<GotoDefinitionResponse>> {
        let position = params.text_document_position_params.position;
        let path = params
            .text_document_position_params
            .text_document
            .uri
            .to_string();
        let Some(analysis) = self.manufacture_analysis() else {
            return Ok(None);
        };
        match analysis.index.definition_at(&path, position.line, position.character) {
            Ok(symbol) => Ok(symbol_location(&symbol).map(GotoDefinitionResponse::Scalar)),
            Err(error) => {
                self.log_refusal(&error).await;
                Ok(None)
            }
        }
    }

    async fn references(
        &self,
        params: ReferenceParams,
    ) -> lsp_max::jsonrpc::Result<Option<Vec<Location>>> {
        let position = params.text_document_position.position;
        let path = params.text_document_position.text_document.uri.to_string();
        let Some(analysis) = self.manufacture_analysis() else {
            return Ok(None);
        };
        match analysis.index.references_at(
            &path,
            position.line,
            position.character,
            params.context.include_declaration,
        ) {
            Ok(references) => Ok(Some(
                references
                    .iter()
                    .filter_map(occurrence_location)
                    .collect(),
            )),
            Err(error) => {
                self.log_refusal(&error).await;
                Ok(None)
            }
        }
    }

    async fn completion(
        &self,
        params: CompletionParams,
    ) -> lsp_max::jsonrpc::Result<Option<CompletionResponse>> {
        let position = params.text_document_position.position;
        let path = params.text_document_position.text_document.uri.to_string();
        let Some(analysis) = self.manufacture_analysis() else {
            return Ok(None);
        };
        match analysis
            .index
            .completions(&path, position.line, position.character, 100)
        {
            Ok(candidates) => Ok(Some(CompletionResponse::Array(
                candidates.into_iter().map(completion_item).collect(),
            ))),
            Err(error) => {
                self.log_refusal(&error).await;
                Ok(None)
            }
        }
    }

    async fn document_symbol(
        &self,
        params: DocumentSymbolParams,
    ) -> lsp_max::jsonrpc::Result<Option<DocumentSymbolResponse>> {
        let path = params.text_document.uri.to_string();
        let Some(analysis) = self.manufacture_analysis() else {
            return Ok(None);
        };
        Ok(Some(DocumentSymbolResponse::Nested(
            analysis
                .index
                .document_symbols(&path)
                .into_iter()
                .map(document_symbol)
                .collect(),
        )))
    }

    async fn symbol(
        &self,
        params: WorkspaceSymbolParams,
    ) -> lsp_max::jsonrpc::Result<Option<Vec<SymbolInformation>>> {
        let Some(analysis) = self.manufacture_analysis() else {
            return Ok(None);
        };
        Ok(Some(
            analysis
                .index
                .workspace_symbols(&params.query, 200)
                .iter()
                .filter_map(symbol_information)
                .collect(),
        ))
    }

    async fn prepare_rename(
        &self,
        params: TextDocumentPositionParams,
    ) -> lsp_max::jsonrpc::Result<Option<PrepareRenameResponse>> {
        let path = params.text_document.uri.to_string();
        let position = params.position;
        let Some(analysis) = self.manufacture_analysis() else {
            return Ok(None);
        };
        match analysis
            .index
            .prepare_rename_at(&path, position.line, position.character)
        {
            Ok(occurrence) => Ok(Some(PrepareRenameResponse::RangeWithPlaceholder {
                range: to_lsp_range(&occurrence.range),
                placeholder: occurrence.name,
            })),
            Err(error) => {
                self.log_refusal(&error).await;
                Ok(None)
            }
        }
    }

    async fn rename(
        &self,
        params: RenameParams,
    ) -> lsp_max::jsonrpc::Result<Option<WorkspaceEdit>> {
        let path = params.text_document_position.text_document.uri.to_string();
        let position = params.text_document_position.position;
        let Some(analysis) = self.manufacture_analysis() else {
            return Ok(None);
        };
        match analysis.index.rename_plan(
            &path,
            position.line,
            position.character,
            &params.new_name,
        ) {
            Ok(plan) => {
                let changes: HashMap<_, _> = plan
                    .edits
                    .into_iter()
                    .filter_map(|(path, edits)| {
                        DocumentUri::from_str(&path).ok().map(|uri| {
                            (
                                uri,
                                edits
                                    .into_iter()
                                    .map(|edit| TextEdit {
                                        range: to_lsp_range(&edit.range),
                                        new_text: edit.new_text,
                                    })
                                    .collect(),
                            )
                        })
                    })
                    .collect();
                Ok(Some(WorkspaceEdit {
                    changes: Some(changes),
                    document_changes: None,
                    change_annotations: None,
                }))
            }
            Err(error) => {
                self.log_refusal(&error).await;
                Ok(None)
            }
        }
    }
}

fn to_lsp_range(range: &SourceRange) -> Range {
    Range {
        start: Position {
            line: range.start_line,
            character: range.start_column,
        },
        end: Position {
            line: range.end_line,
            character: range.end_column,
        },
    }
}

fn symbol_location(symbol: &Symbol) -> Option<Location> {
    let uri = DocumentUri::from_str(&symbol.path).ok()?;
    Some(Location {
        uri,
        range: to_lsp_range(&symbol.selection_range),
    })
}

fn occurrence_location(occurrence: &Occurrence) -> Option<Location> {
    let uri = DocumentUri::from_str(&occurrence.path).ok()?;
    Some(Location {
        uri,
        range: to_lsp_range(&occurrence.range),
    })
}

fn completion_item(candidate: CompletionCandidate) -> CompletionItem {
    CompletionItem {
        label: candidate.label.clone(),
        kind: Some(completion_kind(&candidate.kind)),
        detail: Some(format!(
            "{} [{}]",
            candidate.detail,
            if candidate.lexical_only {
                "lexical"
            } else {
                "semantic"
            }
        )),
        sort_text: Some(format!("{}:{}", candidate.kind, candidate.label)),
        filter_text: Some(candidate.label.clone()),
        insert_text: Some(candidate.label),
        ..Default::default()
    }
}

fn completion_kind(kind: &str) -> CompletionItemKind {
    match kind {
        "function" => CompletionItemKind::FUNCTION,
        "struct" => CompletionItemKind::STRUCT,
        "enum" => CompletionItemKind::ENUM,
        "trait" => CompletionItemKind::INTERFACE,
        "module" => CompletionItemKind::MODULE,
        "const" | "static" => CompletionItemKind::CONSTANT,
        "type_alias" => CompletionItemKind::TYPE_PARAMETER,
        "keyword" => CompletionItemKind::KEYWORD,
        _ => CompletionItemKind::TEXT,
    }
}

fn symbol_kind(kind: &str) -> SymbolKind {
    match kind {
        "function" => SymbolKind::FUNCTION,
        "struct" => SymbolKind::STRUCT,
        "enum" => SymbolKind::ENUM,
        "trait" => SymbolKind::INTERFACE,
        "module" => SymbolKind::MODULE,
        "const" | "static" => SymbolKind::CONSTANT,
        "type_alias" => SymbolKind::TYPE_PARAMETER,
        _ => SymbolKind::VARIABLE,
    }
}

#[allow(deprecated)]
fn document_symbol(symbol: Symbol) -> DocumentSymbol {
    DocumentSymbol {
        name: symbol.name,
        detail: Some(format!("{} · lexical", symbol.signature)),
        kind: symbol_kind(&symbol.kind),
        tags: None,
        deprecated: None,
        range: to_lsp_range(&symbol.range),
        selection_range: to_lsp_range(&symbol.selection_range),
        children: None,
    }
}

#[allow(deprecated)]
fn symbol_information(symbol: &Symbol) -> Option<SymbolInformation> {
    Some(SymbolInformation {
        name: symbol.name.clone(),
        kind: symbol_kind(&symbol.kind),
        tags: None,
        deprecated: None,
        location: symbol_location(symbol)?,
        container_name: Some("ra-max lexical workspace".to_owned()),
    })
}
