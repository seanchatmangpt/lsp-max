use lsp_types::*;
use serde_json::Value;
use crate::client::ClientError;

/// A handle to interact with the connected Language Server.
/// This acts as the outbound proxy for the downstream client.
#[derive(Debug, Clone)]
pub struct ServerHandle {
    // Inner transport channels (to be implemented via tower-service in the builder)
}

impl ServerHandle {
    // --- Lifecycle ---

    pub async fn initialize(&self, _params: InitializeParams) -> Result<InitializeResult, ClientError> { unimplemented!() }
    pub async fn initialized(&self, _params: InitializedParams) { unimplemented!() }
    pub async fn shutdown(&self) -> Result<(), ClientError> { unimplemented!() }
    pub async fn exit(&self) { unimplemented!() }

    // --- Text Document Synchronization ---

    pub async fn did_open(&self, _params: DidOpenTextDocumentParams) { unimplemented!() }
    pub async fn did_change(&self, _params: DidChangeTextDocumentParams) { unimplemented!() }
    pub async fn did_save(&self, _params: DidSaveTextDocumentParams) { unimplemented!() }
    pub async fn did_close(&self, _params: DidCloseTextDocumentParams) { unimplemented!() }

    // --- Language Features ---

    pub async fn hover(&self, _params: HoverParams) -> Result<Option<Hover>, ClientError> { unimplemented!() }
    pub async fn completion(&self, _params: CompletionParams) -> Result<Option<CompletionResponse>, ClientError> { unimplemented!() }
    pub async fn completion_resolve(&self, _params: CompletionItem) -> Result<CompletionItem, ClientError> { unimplemented!() }
    
    pub async fn signature_help(&self, _params: SignatureHelpParams) -> Result<Option<SignatureHelp>, ClientError> { unimplemented!() }
    
    // In lsp-types 0.94, Declaration, Implementation, and TypeDefinition use GotoDefinitionParams and GotoDefinitionResponse.
    pub async fn goto_definition(&self, _params: GotoDefinitionParams) -> Result<Option<GotoDefinitionResponse>, ClientError> { unimplemented!() }
    pub async fn goto_declaration(&self, _params: GotoDefinitionParams) -> Result<Option<GotoDefinitionResponse>, ClientError> { unimplemented!() }
    pub async fn goto_implementation(&self, _params: GotoDefinitionParams) -> Result<Option<GotoDefinitionResponse>, ClientError> { unimplemented!() }
    pub async fn goto_type_definition(&self, _params: GotoDefinitionParams) -> Result<Option<GotoDefinitionResponse>, ClientError> { unimplemented!() }
    
    pub async fn references(&self, _params: ReferenceParams) -> Result<Option<Vec<Location>>, ClientError> { unimplemented!() }
    pub async fn document_highlight(&self, _params: DocumentHighlightParams) -> Result<Option<Vec<DocumentHighlight>>, ClientError> { unimplemented!() }
    pub async fn document_symbol(&self, _params: DocumentSymbolParams) -> Result<Option<DocumentSymbolResponse>, ClientError> { unimplemented!() }
    
    pub async fn code_action(&self, _params: CodeActionParams) -> Result<Option<CodeActionResponse>, ClientError> { unimplemented!() }
    pub async fn code_action_resolve(&self, _params: CodeAction) -> Result<CodeAction, ClientError> { unimplemented!() }
    
    pub async fn code_lens(&self, _params: CodeLensParams) -> Result<Option<Vec<CodeLens>>, ClientError> { unimplemented!() }
    pub async fn code_lens_resolve(&self, _params: CodeLens) -> Result<CodeLens, ClientError> { unimplemented!() }
    
    pub async fn formatting(&self, _params: DocumentFormattingParams) -> Result<Option<Vec<TextEdit>>, ClientError> { unimplemented!() }
    pub async fn range_formatting(&self, _params: DocumentRangeFormattingParams) -> Result<Option<Vec<TextEdit>>, ClientError> { unimplemented!() }
    pub async fn rename(&self, _params: RenameParams) -> Result<Option<WorkspaceEdit>, ClientError> { unimplemented!() }
    pub async fn prepare_rename(&self, _params: TextDocumentPositionParams) -> Result<Option<PrepareRenameResponse>, ClientError> { unimplemented!() }
    
    pub async fn semantic_tokens_full(&self, _params: SemanticTokensParams) -> Result<Option<SemanticTokensResult>, ClientError> { unimplemented!() }
    pub async fn semantic_tokens_full_delta(&self, _params: SemanticTokensDeltaParams) -> Result<Option<SemanticTokensFullDeltaResult>, ClientError> { unimplemented!() }
    pub async fn semantic_tokens_range(&self, _params: SemanticTokensRangeParams) -> Result<Option<SemanticTokensRangeResult>, ClientError> { unimplemented!() }
    
    pub async fn inlay_hint(&self, _params: InlayHintParams) -> Result<Option<Vec<InlayHint>>, ClientError> { unimplemented!() }
    pub async fn inlay_hint_resolve(&self, _params: InlayHint) -> Result<InlayHint, ClientError> { unimplemented!() }

    // --- Workspace Features ---

    pub async fn symbol(&self, _params: WorkspaceSymbolParams) -> Result<Option<Vec<SymbolInformation>>, ClientError> { unimplemented!() }
    pub async fn execute_command(&self, _params: ExecuteCommandParams) -> Result<Option<Value>, ClientError> { unimplemented!() }
    pub async fn did_change_configuration(&self, _params: DidChangeConfigurationParams) { unimplemented!() }
    pub async fn did_change_watched_files(&self, _params: DidChangeWatchedFilesParams) { unimplemented!() }
    pub async fn did_change_workspace_folders(&self, _params: DidChangeWorkspaceFoldersParams) { unimplemented!() }
}
