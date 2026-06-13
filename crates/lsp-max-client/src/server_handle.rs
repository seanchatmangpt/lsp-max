use crate::client::ClientError;
use lsp_types::*;
use serde_json::json;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tokio::sync::{mpsc, oneshot, Mutex};

/// A handle to interact with the connected Language Server.
/// This acts as the outbound proxy for the downstream client.
#[derive(Debug, Clone)]
pub struct ServerHandle {
    pub(crate) sender: mpsc::Sender<Value>,
    pub(crate) pending: Arc<Mutex<HashMap<u64, oneshot::Sender<Value>>>>,
    pub(crate) next_id: Arc<AtomicU64>,
}

impl ServerHandle {
    pub fn new(
        sender: mpsc::Sender<Value>,
        pending: Arc<Mutex<HashMap<u64, oneshot::Sender<Value>>>>,
        next_id: Arc<AtomicU64>,
    ) -> Self {
        Self {
            sender,
            pending,
            next_id,
        }
    }

    /// Helper to send a notification
    pub async fn notify(&self, method: &str, params: impl serde::Serialize) {
        let msg = json!({
            "jsonrpc": "2.0",
            "method": method,
            "params": params,
        });
        let _ = self.sender.send(msg).await;
    }

    /// Helper to send a request and await a correlated response.
    async fn request<R>(
        &self,
        method: &str,
        params: impl serde::Serialize,
    ) -> Result<Option<R>, ClientError>
    where
        R: serde::de::DeserializeOwned,
    {
        let id = self.next_id.fetch_add(1, Ordering::SeqCst);
        let msg = json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": method,
            "params": params,
        });

        let (tx, rx) = oneshot::channel::<Value>();
        self.pending.lock().await.insert(id, tx);

        if self.sender.send(msg).await.is_err() {
            self.pending.lock().await.remove(&id);
            return Ok(None);
        }

        match tokio::time::timeout(std::time::Duration::from_secs(30), rx).await {
            Ok(Ok(result)) => {
                if result.is_null() {
                    return Ok(None);
                }
                match serde_json::from_value::<R>(result) {
                    Ok(v) => Ok(Some(v)),
                    Err(_) => Ok(None),
                }
            }
            Ok(Err(_)) => Ok(None), // sender dropped
            Err(_) => {
                // timeout — clean up
                self.pending.lock().await.remove(&id);
                Ok(None)
            }
        }
    }

    // --- Lifecycle ---

    pub async fn initialize(
        &self,
        params: InitializeParams,
    ) -> Result<InitializeResult, ClientError> {
        match self
            .request::<InitializeResult>("initialize", params)
            .await?
        {
            Some(r) => Ok(r),
            None => Ok(InitializeResult::default()),
        }
    }
    pub async fn initialized(&self, params: InitializedParams) {
        self.notify("initialized", params).await;
    }
    pub async fn shutdown(&self) -> Result<(), ClientError> {
        let _ = self.request::<()>("shutdown", ()).await;
        Ok(())
    }
    pub async fn exit(&self) {
        self.notify("exit", ()).await;
    }

    // --- Text Document Synchronization ---

    pub async fn did_open(&self, params: DidOpenTextDocumentParams) {
        self.notify("textDocument/didOpen", params).await;
    }
    pub async fn did_change(&self, params: DidChangeTextDocumentParams) {
        self.notify("textDocument/didChange", params).await;
    }
    pub async fn did_save(&self, params: DidSaveTextDocumentParams) {
        self.notify("textDocument/didSave", params).await;
    }
    pub async fn did_close(&self, params: DidCloseTextDocumentParams) {
        self.notify("textDocument/didClose", params).await;
    }

    // --- Language Features ---

    pub async fn hover(&self, params: HoverParams) -> Result<Option<Hover>, ClientError> {
        self.request("textDocument/hover", params).await
    }
    pub async fn completion(
        &self,
        params: CompletionParams,
    ) -> Result<Option<CompletionResponse>, ClientError> {
        self.request("textDocument/completion", params).await
    }
    pub async fn completion_resolve(
        &self,
        params: CompletionItem,
    ) -> Result<CompletionItem, ClientError> {
        match self
            .request::<CompletionItem>("completionItem/resolve", params.clone())
            .await?
        {
            Some(r) => Ok(r),
            None => Ok(params),
        }
    }

    pub async fn signature_help(
        &self,
        params: SignatureHelpParams,
    ) -> Result<Option<SignatureHelp>, ClientError> {
        self.request("textDocument/signatureHelp", params).await
    }

    pub async fn goto_definition(
        &self,
        params: GotoDefinitionParams,
    ) -> Result<Option<GotoDefinitionResponse>, ClientError> {
        self.request("textDocument/definition", params).await
    }
    pub async fn goto_declaration(
        &self,
        params: GotoDefinitionParams,
    ) -> Result<Option<GotoDefinitionResponse>, ClientError> {
        self.request("textDocument/declaration", params).await
    }
    pub async fn goto_implementation(
        &self,
        params: GotoDefinitionParams,
    ) -> Result<Option<GotoDefinitionResponse>, ClientError> {
        self.request("textDocument/implementation", params).await
    }
    pub async fn goto_type_definition(
        &self,
        params: GotoDefinitionParams,
    ) -> Result<Option<GotoDefinitionResponse>, ClientError> {
        self.request("textDocument/typeDefinition", params).await
    }

    pub async fn references(
        &self,
        params: ReferenceParams,
    ) -> Result<Option<Vec<Location>>, ClientError> {
        self.request("textDocument/references", params).await
    }
    pub async fn document_highlight(
        &self,
        params: DocumentHighlightParams,
    ) -> Result<Option<Vec<DocumentHighlight>>, ClientError> {
        self.request("textDocument/documentHighlight", params).await
    }
    pub async fn document_symbol(
        &self,
        params: DocumentSymbolParams,
    ) -> Result<Option<DocumentSymbolResponse>, ClientError> {
        self.request("textDocument/documentSymbol", params).await
    }

    pub async fn code_action(
        &self,
        params: CodeActionParams,
    ) -> Result<Option<CodeActionResponse>, ClientError> {
        self.request("textDocument/codeAction", params).await
    }
    pub async fn code_action_resolve(&self, params: CodeAction) -> Result<CodeAction, ClientError> {
        match self
            .request::<CodeAction>("codeAction/resolve", params.clone())
            .await?
        {
            Some(r) => Ok(r),
            None => Ok(params),
        }
    }

    pub async fn code_lens(
        &self,
        params: CodeLensParams,
    ) -> Result<Option<Vec<CodeLens>>, ClientError> {
        self.request("textDocument/codeLens", params).await
    }
    pub async fn code_lens_resolve(&self, params: CodeLens) -> Result<CodeLens, ClientError> {
        match self
            .request::<CodeLens>("codeLens/resolve", params.clone())
            .await?
        {
            Some(r) => Ok(r),
            None => Ok(params),
        }
    }

    pub async fn formatting(
        &self,
        params: DocumentFormattingParams,
    ) -> Result<Option<Vec<TextEdit>>, ClientError> {
        self.request("textDocument/formatting", params).await
    }
    pub async fn range_formatting(
        &self,
        params: DocumentRangeFormattingParams,
    ) -> Result<Option<Vec<TextEdit>>, ClientError> {
        self.request("textDocument/rangeFormatting", params).await
    }
    pub async fn rename(&self, params: RenameParams) -> Result<Option<WorkspaceEdit>, ClientError> {
        self.request("textDocument/rename", params).await
    }
    pub async fn prepare_rename(
        &self,
        params: TextDocumentPositionParams,
    ) -> Result<Option<PrepareRenameResponse>, ClientError> {
        self.request("textDocument/prepareRename", params).await
    }

    pub async fn semantic_tokens_full(
        &self,
        params: SemanticTokensParams,
    ) -> Result<Option<SemanticTokensResult>, ClientError> {
        self.request("textDocument/semanticTokens/full", params)
            .await
    }
    pub async fn semantic_tokens_full_delta(
        &self,
        params: SemanticTokensDeltaParams,
    ) -> Result<Option<SemanticTokensFullDeltaResult>, ClientError> {
        self.request("textDocument/semanticTokens/full/delta", params)
            .await
    }
    pub async fn semantic_tokens_range(
        &self,
        params: SemanticTokensRangeParams,
    ) -> Result<Option<SemanticTokensRangeResult>, ClientError> {
        self.request("textDocument/semanticTokens/range", params)
            .await
    }

    pub async fn inlay_hint(
        &self,
        params: InlayHintParams,
    ) -> Result<Option<Vec<InlayHint>>, ClientError> {
        self.request("textDocument/inlayHint", params).await
    }
    pub async fn inlay_hint_resolve(&self, params: InlayHint) -> Result<InlayHint, ClientError> {
        match self
            .request::<InlayHint>("inlayHint/resolve", params.clone())
            .await?
        {
            Some(r) => Ok(r),
            None => Ok(params),
        }
    }

    // --- Workspace Features ---

    pub async fn symbol(
        &self,
        params: WorkspaceSymbolParams,
    ) -> Result<Option<Vec<SymbolInformation>>, ClientError> {
        self.request("workspace/symbol", params).await
    }
    pub async fn execute_command(
        &self,
        params: ExecuteCommandParams,
    ) -> Result<Option<Value>, ClientError> {
        self.request("workspace/executeCommand", params).await
    }
    pub async fn did_change_configuration(&self, params: DidChangeConfigurationParams) {
        self.notify("workspace/didChangeConfiguration", params)
            .await;
    }
    pub async fn did_change_watched_files(&self, params: DidChangeWatchedFilesParams) {
        self.notify("workspace/didChangeWatchedFiles", params).await;
    }
    pub async fn did_change_workspace_folders(&self, params: DidChangeWorkspaceFoldersParams) {
        self.notify("workspace/didChangeWorkspaceFolders", params)
            .await;
    }
}
