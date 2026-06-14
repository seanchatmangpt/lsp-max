//! `impl LanguageServer for ComposedServer` — delegates to route_request/route_notification.

use serde_json::Value;

use crate::jsonrpc::{Error, Result};
use crate::LanguageServer;
use lsp_types_max::request::*;
use lsp_types_max::*;

use super::server::ComposedServer;

#[async_trait::async_trait]
impl LanguageServer for ComposedServer {
    async fn initialize(&self, params: InitializeParams) -> Result<InitializeResult> {
        self.route_request("initialize", params)
            .await?
            .ok_or_else(Error::internal_error)
    }

    async fn initialized(&self, params: InitializedParams) {
        tracing::trace!("--- ComposedServer::initialized start");
        self.route_notification("initialized", params).await;
        tracing::trace!("--- ComposedServer::initialized end");
    }

    async fn shutdown(&self) -> Result<()> {
        self.route_request::<_, ()>("shutdown", ())
            .await?
            .ok_or_else(Error::internal_error)
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        self.route_notification("textDocument/didOpen", params)
            .await;
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        self.route_notification("textDocument/didChange", params)
            .await;
    }

    async fn will_save(&self, params: WillSaveTextDocumentParams) {
        self.route_notification("textDocument/willSave", params)
            .await;
    }

    async fn will_save_wait_until(
        &self,
        params: WillSaveTextDocumentParams,
    ) -> Result<Option<Vec<TextEdit>>> {
        self.route_request("textDocument/willSaveWaitUntil", params)
            .await
            .map(|o| o.flatten())
    }

    async fn did_save(&self, params: DidSaveTextDocumentParams) {
        self.route_notification("textDocument/didSave", params)
            .await;
    }

    async fn did_close(&self, params: DidCloseTextDocumentParams) {
        self.route_notification("textDocument/didClose", params)
            .await;
    }

    async fn goto_declaration(
        &self,
        params: GotoDeclarationParams,
    ) -> Result<Option<GotoDeclarationResponse>> {
        self.route_request("textDocument/declaration", params)
            .await
            .map(|o| o.flatten())
    }

    async fn goto_definition(
        &self,
        params: GotoDefinitionParams,
    ) -> Result<Option<GotoDefinitionResponse>> {
        self.route_request("textDocument/definition", params)
            .await
            .map(|o| o.flatten())
    }

    async fn goto_type_definition(
        &self,
        params: GotoTypeDefinitionParams,
    ) -> Result<Option<GotoTypeDefinitionResponse>> {
        self.route_request("textDocument/typeDefinition", params)
            .await
            .map(|o| o.flatten())
    }

    async fn goto_implementation(
        &self,
        params: GotoImplementationParams,
    ) -> Result<Option<GotoImplementationResponse>> {
        self.route_request("textDocument/implementation", params)
            .await
            .map(|o| o.flatten())
    }

    async fn references(&self, params: ReferenceParams) -> Result<Option<Vec<Location>>> {
        self.route_request("textDocument/references", params)
            .await
            .map(|o| o.flatten())
    }

    async fn prepare_call_hierarchy(
        &self,
        params: CallHierarchyPrepareParams,
    ) -> Result<Option<Vec<CallHierarchyItem>>> {
        self.route_request("textDocument/prepareCallHierarchy", params)
            .await
            .map(|o| o.flatten())
    }

    async fn incoming_calls(
        &self,
        params: CallHierarchyIncomingCallsParams,
    ) -> Result<Option<Vec<CallHierarchyIncomingCall>>> {
        self.route_request("callHierarchy/incomingCalls", params)
            .await
            .map(|o| o.flatten())
    }

    async fn outgoing_calls(
        &self,
        params: CallHierarchyOutgoingCallsParams,
    ) -> Result<Option<Vec<CallHierarchyOutgoingCall>>> {
        self.route_request("callHierarchy/outgoingCalls", params)
            .await
            .map(|o| o.flatten())
    }

    async fn prepare_type_hierarchy(
        &self,
        params: TypeHierarchyPrepareParams,
    ) -> Result<Option<Vec<TypeHierarchyItem>>> {
        self.route_request("textDocument/prepareTypeHierarchy", params)
            .await
            .map(|o| o.flatten())
    }

    async fn supertypes(
        &self,
        params: TypeHierarchySupertypesParams,
    ) -> Result<Option<Vec<TypeHierarchyItem>>> {
        self.route_request("typeHierarchy/supertypes", params)
            .await
            .map(|o| o.flatten())
    }

    async fn subtypes(
        &self,
        params: TypeHierarchySubtypesParams,
    ) -> Result<Option<Vec<TypeHierarchyItem>>> {
        self.route_request("typeHierarchy/subtypes", params)
            .await
            .map(|o| o.flatten())
    }

    async fn document_highlight(
        &self,
        params: DocumentHighlightParams,
    ) -> Result<Option<Vec<DocumentHighlight>>> {
        self.route_request("textDocument/documentHighlight", params)
            .await
            .map(|o| o.flatten())
    }

    async fn document_link(&self, params: DocumentLinkParams) -> Result<Option<Vec<DocumentLink>>> {
        self.route_request("textDocument/documentLink", params)
            .await
            .map(|o| o.flatten())
    }

    async fn document_link_resolve(&self, params: DocumentLink) -> Result<DocumentLink> {
        self.route_request("documentLink/resolve", params)
            .await?
            .ok_or_else(Error::internal_error)
    }

    async fn hover(&self, params: HoverParams) -> Result<Option<Hover>> {
        self.route_request("textDocument/hover", params)
            .await
            .map(|o| o.flatten())
    }

    async fn completion(&self, params: CompletionParams) -> Result<Option<CompletionResponse>> {
        self.route_request("textDocument/completion", params)
            .await
            .map(|o| o.flatten())
    }

    async fn completion_resolve(&self, params: CompletionItem) -> Result<CompletionItem> {
        self.route_request("completionItem/resolve", params)
            .await?
            .ok_or_else(Error::internal_error)
    }

    async fn semantic_tokens_full(
        &self,
        params: SemanticTokensParams,
    ) -> Result<Option<SemanticTokensResult>> {
        self.route_request("textDocument/semanticTokens/full", params)
            .await
            .map(|o| o.flatten())
    }

    async fn semantic_tokens_full_delta(
        &self,
        params: SemanticTokensDeltaParams,
    ) -> Result<Option<SemanticTokensFullDeltaResult>> {
        self.route_request("textDocument/semanticTokens/full/delta", params)
            .await
            .map(|o| o.flatten())
    }

    async fn semantic_tokens_range(
        &self,
        params: SemanticTokensRangeParams,
    ) -> Result<Option<SemanticTokensRangeResult>> {
        self.route_request("textDocument/semanticTokens/range", params)
            .await
            .map(|o| o.flatten())
    }

    async fn code_lens(&self, params: CodeLensParams) -> Result<Option<Vec<CodeLens>>> {
        self.route_request("textDocument/codeLens", params)
            .await
            .map(|o| o.flatten())
    }

    async fn code_lens_resolve(&self, params: CodeLens) -> Result<CodeLens> {
        self.route_request("codeLens/resolve", params)
            .await?
            .ok_or_else(Error::internal_error)
    }

    async fn folding_range(&self, params: FoldingRangeParams) -> Result<Option<Vec<FoldingRange>>> {
        self.route_request("textDocument/foldingRange", params)
            .await
            .map(|o| o.flatten())
    }

    async fn selection_range(
        &self,
        params: SelectionRangeParams,
    ) -> Result<Option<Vec<SelectionRange>>> {
        self.route_request("textDocument/selectionRange", params)
            .await
            .map(|o| o.flatten())
    }

    async fn document_symbol(
        &self,
        params: DocumentSymbolParams,
    ) -> Result<Option<DocumentSymbolResponse>> {
        self.route_request("textDocument/documentSymbol", params)
            .await
            .map(|o| o.flatten())
    }

    async fn symbol(
        &self,
        params: WorkspaceSymbolParams,
    ) -> Result<Option<Vec<SymbolInformation>>> {
        self.route_request("workspace/symbol", params)
            .await
            .map(|o| o.flatten())
    }

    async fn symbol_resolve(&self, params: WorkspaceSymbol) -> Result<WorkspaceSymbol> {
        self.route_request("workspaceSymbol/resolve", params)
            .await?
            .ok_or_else(Error::internal_error)
    }

    async fn did_change_configuration(&self, params: DidChangeConfigurationParams) {
        self.route_notification("workspace/didChangeConfiguration", params)
            .await;
    }

    async fn did_change_workspace_folders(&self, params: DidChangeWorkspaceFoldersParams) {
        self.route_notification("workspace/didChangeWorkspaceFolders", params)
            .await;
    }

    async fn will_create_files(&self, params: CreateFilesParams) -> Result<Option<WorkspaceEdit>> {
        self.route_request("workspace/willCreateFiles", params)
            .await
            .map(|o| o.flatten())
    }

    async fn did_create_files(&self, params: CreateFilesParams) {
        self.route_notification("workspace/didCreateFiles", params)
            .await;
    }

    async fn will_rename_files(&self, params: RenameFilesParams) -> Result<Option<WorkspaceEdit>> {
        self.route_request("workspace/willRenameFiles", params)
            .await
            .map(|o| o.flatten())
    }

    async fn did_rename_files(&self, params: RenameFilesParams) {
        self.route_notification("workspace/didRenameFiles", params)
            .await;
    }

    async fn will_delete_files(&self, params: DeleteFilesParams) -> Result<Option<WorkspaceEdit>> {
        self.route_request("workspace/willDeleteFiles", params)
            .await
            .map(|o| o.flatten())
    }

    async fn did_delete_files(&self, params: DeleteFilesParams) {
        self.route_notification("workspace/didDeleteFiles", params)
            .await;
    }

    async fn did_change_watched_files(&self, params: DidChangeWatchedFilesParams) {
        self.route_notification("workspace/didChangeWatchedFiles", params)
            .await;
    }

    async fn execute_command(&self, params: ExecuteCommandParams) -> Result<Option<Value>> {
        self.route_request("workspace/executeCommand", params)
            .await
            .map(|o| o.flatten())
    }

    async fn signature_help(&self, params: SignatureHelpParams) -> Result<Option<SignatureHelp>> {
        self.route_request("textDocument/signatureHelp", params)
            .await
            .map(|o| o.flatten())
    }

    async fn code_action(&self, params: CodeActionParams) -> Result<Option<CodeActionResponse>> {
        self.route_request("textDocument/codeAction", params)
            .await
            .map(|o| o.flatten())
    }

    async fn code_action_resolve(&self, params: CodeAction) -> Result<CodeAction> {
        self.route_request("codeAction/resolve", params)
            .await?
            .ok_or_else(Error::internal_error)
    }

    async fn document_color(&self, params: DocumentColorParams) -> Result<Vec<ColorInformation>> {
        self.route_request("textDocument/documentColor", params)
            .await
            .map(|o| o.unwrap_or_default())
    }

    async fn color_presentation(
        &self,
        params: ColorPresentationParams,
    ) -> Result<Vec<ColorPresentation>> {
        self.route_request("textDocument/colorPresentation", params)
            .await
            .map(|o| o.unwrap_or_default())
    }

    async fn formatting(&self, params: DocumentFormattingParams) -> Result<Option<Vec<TextEdit>>> {
        self.route_request("textDocument/formatting", params)
            .await
            .map(|o| o.flatten())
    }

    async fn range_formatting(
        &self,
        params: DocumentRangeFormattingParams,
    ) -> Result<Option<Vec<TextEdit>>> {
        self.route_request("textDocument/rangeFormatting", params)
            .await
            .map(|o| o.flatten())
    }

    async fn on_type_formatting(
        &self,
        params: DocumentOnTypeFormattingParams,
    ) -> Result<Option<Vec<TextEdit>>> {
        self.route_request("textDocument/onTypeFormatting", params)
            .await
            .map(|o| o.flatten())
    }

    async fn rename(&self, params: RenameParams) -> Result<Option<WorkspaceEdit>> {
        self.route_request("textDocument/rename", params)
            .await
            .map(|o| o.flatten())
    }

    async fn prepare_rename(
        &self,
        params: TextDocumentPositionParams,
    ) -> Result<Option<PrepareRenameResponse>> {
        self.route_request("textDocument/prepareRename", params)
            .await
            .map(|o| o.flatten())
    }

    async fn linked_editing_range(
        &self,
        params: LinkedEditingRangeParams,
    ) -> Result<Option<LinkedEditingRanges>> {
        self.route_request("textDocument/linkedEditingRange", params)
            .await
            .map(|o| o.flatten())
    }

    async fn moniker(&self, params: MonikerParams) -> Result<Option<Vec<Moniker>>> {
        self.route_request("textDocument/moniker", params)
            .await
            .map(|o| o.flatten())
    }

    async fn inlay_hint(&self, params: InlayHintParams) -> Result<Option<Vec<InlayHint>>> {
        self.route_request("textDocument/inlayHint", params)
            .await
            .map(|o| o.flatten())
    }

    async fn inlay_hint_resolve(&self, params: InlayHint) -> Result<InlayHint> {
        self.route_request("inlayHint/resolve", params)
            .await?
            .ok_or_else(Error::internal_error)
    }

    async fn inline_value(&self, params: InlineValueParams) -> Result<Option<Vec<InlineValue>>> {
        self.route_request("textDocument/inlineValue", params)
            .await
            .map(|o| o.flatten())
    }

    async fn diagnostic(
        &self,
        params: DocumentDiagnosticParams,
    ) -> Result<DocumentDiagnosticReportResult> {
        self.route_request("textDocument/diagnostic", params)
            .await?
            .ok_or_else(Error::internal_error)
    }

    async fn workspace_diagnostic(
        &self,
        params: WorkspaceDiagnosticParams,
    ) -> Result<WorkspaceDiagnosticReportResult> {
        self.route_request("workspace/diagnostic", params)
            .await?
            .ok_or_else(Error::internal_error)
    }

    async fn inline_completion(
        &self,
        params: InlineCompletionParams,
    ) -> Result<Option<InlineCompletionResponse>> {
        self.route_request("textDocument/inlineCompletion", params)
            .await
            .map(|o| o.flatten())
    }

    async fn text_document_content(
        &self,
        params: max_protocol::lsp_3_18::TextDocumentContentParams,
    ) -> Result<max_protocol::lsp_3_18::TextDocumentContentResult> {
        self.route_request("workspace/textDocumentContent", params)
            .await?
            .ok_or_else(Error::internal_error)
    }

    async fn ranges_formatting(
        &self,
        params: max_protocol::lsp_3_18::DocumentRangesFormattingParams,
    ) -> Result<Option<Vec<max_protocol::lsp_3_18::TextEdit>>> {
        self.route_request("textDocument/rangesFormatting", params)
            .await
            .map(|o| o.flatten())
    }

    async fn did_open_notebook_document(&self, params: DidOpenNotebookDocumentParams) {
        self.route_notification("notebookDocument/didOpen", params)
            .await;
    }

    async fn did_change_notebook_document(&self, params: DidChangeNotebookDocumentParams) {
        self.route_notification("notebookDocument/didChange", params)
            .await;
    }

    async fn did_save_notebook_document(&self, params: DidSaveNotebookDocumentParams) {
        self.route_notification("notebookDocument/didSave", params)
            .await;
    }

    async fn did_close_notebook_document(&self, params: DidCloseNotebookDocumentParams) {
        self.route_notification("notebookDocument/didClose", params)
            .await;
    }

    async fn work_done_progress_cancel(&self, params: WorkDoneProgressCancelParams) {
        self.route_notification("window/workDoneProgress/cancel", params)
            .await;
    }

    async fn set_trace(&self, params: SetTraceParams) {
        self.route_notification("$/setTrace", params).await;
    }

    async fn progress(&self, params: ProgressParams) {
        self.route_notification("$/progress", params).await;
    }
}
