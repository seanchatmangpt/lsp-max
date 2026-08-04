use serde::{Deserialize, Serialize};

use lsp_max::{Client, LanguageServer};
use lsp_types_max::{
    Diagnostic, DiagnosticSeverity, DidChangeTextDocumentParams, DidCloseTextDocumentParams,
    DidOpenTextDocumentParams, DocumentUri, InitializeParams, InitializeResult, InitializedParams,
    NumberOrString, Position, Range, ServerCapabilities, ServerInfo, TextDocumentSyncCapability,
    TextDocumentSyncKind,
};
use parking_lot::{Mutex, RwLock};

use crate::identity::{digest_bytes, ProjectAdmission, SemanticSubject};
use crate::receipt::{Outcome, Receipt, ReceiptChain, ReceiptKind};
use crate::semantic::{SemanticEngine, SemanticSnapshot, TreeSitterRustEngine};
use crate::WorkspaceState;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ServerSnapshot {
    pub admission: Option<ProjectAdmission>,
    pub semantic: Option<SemanticSnapshot>,
    pub receipts: Vec<Receipt>,
    pub receipt_chain_valid: bool,
}

/// LSP surface for the RFC 0006 vertical slice.
///
/// Document notifications update admitted observations only. Semantic analysis
/// is immutable, and diagnostics are published from a bound snapshot.
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

    fn manufacture_snapshot(&self) -> ServerSnapshot {
        let workspace = self.workspace.read().clone();
        if workspace.files().is_empty() {
            let receipts = self.receipts.lock();
            return ServerSnapshot {
                admission: None,
                semantic: None,
                receipts: receipts.receipts().to_vec(),
                receipt_chain_valid: receipts.verify(),
            };
        }

        let subject = SemanticSubject::tree_sitter_vertical_slice();
        let admission = ProjectAdmission::from_files(
            subject.clone(),
            "lsp://ra-max-workspace",
            workspace.files(),
        );
        let semantic = self.engine.analyze(&admission, workspace.files());
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
        ServerSnapshot {
            admission: Some(admission),
            semantic: Some(semantic),
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
                range: Range {
                    start: Position {
                        line: diagnostic.range.start_line,
                        character: diagnostic.range.start_column,
                    },
                    end: Position {
                        line: diagnostic.range.end_line,
                        character: diagnostic.range.end_column,
                    },
                },
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

    pub async fn max_snapshot(&self) -> lsp_max::jsonrpc::Result<ServerSnapshot> {
        Ok(self.manufacture_snapshot())
    }
}

#[lsp_max::async_trait]
impl LanguageServer for RaMaxServer {
    async fn initialize(&self, _: InitializeParams) -> lsp_max::jsonrpc::Result<InitializeResult> {
        Ok(InitializeResult {
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Kind(
                    TextDocumentSyncKind::FULL,
                )),
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
                lsp_types_max::MessageType::INFO,
                "ra-max initialized with receipt-bound Tree-sitter Rust semantics",
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
}
