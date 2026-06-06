use crate::jsonrpc::{Error, Result};
use async_trait::async_trait;
use auto_impl::auto_impl;
use lsp_types_max::request::{
    GotoDeclarationParams, GotoDeclarationResponse, GotoImplementationParams,
    GotoImplementationResponse, GotoTypeDefinitionParams, GotoTypeDefinitionResponse,
};
use lsp_types_max::*;
use serde_json::Value;
use tower_lsp_max_macros::rpc;

pub(crate) mod impls;

/// Trait implemented by language server backends to handle LSP requests and notifications.
#[rpc]
#[async_trait]
#[auto_impl(Arc, Box)]
pub trait LanguageServer: Send + Sync + 'static {
    /// Handler for the `initialize` endpoint.
    #[rpc(name = "initialize")]
    async fn initialize(&self, params: InitializeParams) -> Result<InitializeResult>;
    /// Handler for the `initialized` endpoint.
    #[rpc(name = "initialized")]
    async fn initialized(&self, params: InitializedParams) {
        impls::initialized(params).await;
    }
    /// Handler for the `shutdown` endpoint.
    #[rpc(name = "shutdown")]
    async fn shutdown(&self) -> Result<()>;
    /// Handler for the `did_open` endpoint.
    #[rpc(name = "textDocument/didOpen")]
    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        impls::did_open(params).await;
    }
    /// Handler for the `did_change` endpoint.
    #[rpc(name = "textDocument/didChange")]
    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        impls::did_change(params).await;
    }
    /// Handler for the `will_save` endpoint.
    #[rpc(name = "textDocument/willSave")]
    async fn will_save(&self, params: WillSaveTextDocumentParams) {
        impls::will_save(params).await;
    }
    /// Handler for the `will_save_wait_until` endpoint.
    #[rpc(name = "textDocument/willSaveWaitUntil")]
    async fn will_save_wait_until(
        &self,
        params: WillSaveTextDocumentParams,
    ) -> Result<Option<Vec<TextEdit>>> {
        impls::will_save_wait_until(params).await
    }
    /// Handler for the `did_save` endpoint.
    #[rpc(name = "textDocument/didSave")]
    async fn did_save(&self, params: DidSaveTextDocumentParams) {
        impls::did_save(params).await;
    }
    /// Handler for the `did_close` endpoint.
    #[rpc(name = "textDocument/didClose")]
    async fn did_close(&self, params: DidCloseTextDocumentParams) {
        impls::did_close(params).await;
    }
    /// Handler for the `goto_declaration` endpoint.
    #[rpc(name = "textDocument/declaration")]
    async fn goto_declaration(
        &self,
        params: GotoDeclarationParams,
    ) -> Result<Option<GotoDeclarationResponse>> {
        impls::goto_declaration(params).await
    }
    /// Handler for the `goto_definition` endpoint.
    #[rpc(name = "textDocument/definition")]
    async fn goto_definition(
        &self,
        params: GotoDefinitionParams,
    ) -> Result<Option<GotoDefinitionResponse>> {
        impls::goto_definition(params).await
    }
    /// Handler for the `goto_type_definition` endpoint.
    #[rpc(name = "textDocument/typeDefinition")]
    async fn goto_type_definition(
        &self,
        params: GotoTypeDefinitionParams,
    ) -> Result<Option<GotoTypeDefinitionResponse>> {
        impls::goto_type_definition(params).await
    }
    /// Handler for the `goto_implementation` endpoint.
    #[rpc(name = "textDocument/implementation")]
    async fn goto_implementation(
        &self,
        params: GotoImplementationParams,
    ) -> Result<Option<GotoImplementationResponse>> {
        impls::goto_implementation(params).await
    }
    /// Handler for the `references` endpoint.
    #[rpc(name = "textDocument/references")]
    async fn references(&self, params: ReferenceParams) -> Result<Option<Vec<Location>>> {
        impls::references(params).await
    }
    /// Handler for the `prepare_call_hierarchy` endpoint.
    #[rpc(name = "textDocument/prepareCallHierarchy")]
    async fn prepare_call_hierarchy(
        &self,
        params: CallHierarchyPrepareParams,
    ) -> Result<Option<Vec<CallHierarchyItem>>> {
        impls::prepare_call_hierarchy(params).await
    }
    /// Handler for the `incoming_calls` endpoint.
    #[rpc(name = "callHierarchy/incomingCalls")]
    async fn incoming_calls(
        &self,
        params: CallHierarchyIncomingCallsParams,
    ) -> Result<Option<Vec<CallHierarchyIncomingCall>>> {
        impls::incoming_calls(params).await
    }
    /// Handler for the `outgoing_calls` endpoint.
    #[rpc(name = "callHierarchy/outgoingCalls")]
    async fn outgoing_calls(
        &self,
        params: CallHierarchyOutgoingCallsParams,
    ) -> Result<Option<Vec<CallHierarchyOutgoingCall>>> {
        impls::outgoing_calls(params).await
    }
    /// Handler for the `prepare_type_hierarchy` endpoint.
    #[rpc(name = "textDocument/prepareTypeHierarchy")]
    async fn prepare_type_hierarchy(
        &self,
        params: TypeHierarchyPrepareParams,
    ) -> Result<Option<Vec<TypeHierarchyItem>>> {
        impls::prepare_type_hierarchy(params).await
    }
    /// Handler for the `supertypes` endpoint.
    #[rpc(name = "typeHierarchy/supertypes")]
    async fn supertypes(
        &self,
        params: TypeHierarchySupertypesParams,
    ) -> Result<Option<Vec<TypeHierarchyItem>>> {
        impls::supertypes(params).await
    }
    /// Handler for the `subtypes` endpoint.
    #[rpc(name = "typeHierarchy/subtypes")]
    async fn subtypes(
        &self,
        params: TypeHierarchySubtypesParams,
    ) -> Result<Option<Vec<TypeHierarchyItem>>> {
        impls::subtypes(params).await
    }
    /// Handler for the `document_highlight` endpoint.
    #[rpc(name = "textDocument/documentHighlight")]
    async fn document_highlight(
        &self,
        params: DocumentHighlightParams,
    ) -> Result<Option<Vec<DocumentHighlight>>> {
        impls::document_highlight(params).await
    }
    /// Handler for the `document_link` endpoint.
    #[rpc(name = "textDocument/documentLink")]
    async fn document_link(&self, params: DocumentLinkParams) -> Result<Option<Vec<DocumentLink>>> {
        impls::document_link(params).await
    }
    /// Handler for the `document_link_resolve` endpoint.
    #[rpc(name = "documentLink/resolve")]
    async fn document_link_resolve(&self, params: DocumentLink) -> Result<DocumentLink> {
        impls::document_link_resolve(params).await
    }
    /// Handler for the `hover` endpoint.
    #[rpc(name = "textDocument/hover")]
    async fn hover(&self, params: HoverParams) -> Result<Option<Hover>> {
        impls::hover(params).await
    }
    /// Handler for the `code_lens` endpoint.
    #[rpc(name = "textDocument/codeLens")]
    async fn code_lens(&self, params: CodeLensParams) -> Result<Option<Vec<CodeLens>>> {
        impls::code_lens(params).await
    }
    /// Handler for the `code_lens_resolve` endpoint.
    #[rpc(name = "codeLens/resolve")]
    async fn code_lens_resolve(&self, params: CodeLens) -> Result<CodeLens> {
        impls::code_lens_resolve(params).await
    }
    /// Handler for the `folding_range` endpoint.
    #[rpc(name = "textDocument/foldingRange")]
    async fn folding_range(&self, params: FoldingRangeParams) -> Result<Option<Vec<FoldingRange>>> {
        impls::folding_range(params).await
    }
    /// Handler for the `selection_range` endpoint.
    #[rpc(name = "textDocument/selectionRange")]
    async fn selection_range(
        &self,
        params: SelectionRangeParams,
    ) -> Result<Option<Vec<SelectionRange>>> {
        impls::selection_range(params).await
    }
    /// Handler for the `document_symbol` endpoint.
    #[rpc(name = "textDocument/documentSymbol")]
    async fn document_symbol(
        &self,
        params: DocumentSymbolParams,
    ) -> Result<Option<DocumentSymbolResponse>> {
        impls::document_symbol(params).await
    }
    /// Handler for the `semantic_tokens_full` endpoint.
    #[rpc(name = "textDocument/semanticTokens/full")]
    async fn semantic_tokens_full(
        &self,
        params: SemanticTokensParams,
    ) -> Result<Option<SemanticTokensResult>> {
        impls::semantic_tokens_full(params).await
    }
    /// Handler for the `semantic_tokens_full_delta` endpoint.
    #[rpc(name = "textDocument/semanticTokens/full/delta")]
    async fn semantic_tokens_full_delta(
        &self,
        params: SemanticTokensDeltaParams,
    ) -> Result<Option<SemanticTokensFullDeltaResult>> {
        impls::semantic_tokens_full_delta(params).await
    }
    /// Handler for the `semantic_tokens_range` endpoint.
    #[rpc(name = "textDocument/semanticTokens/range")]
    async fn semantic_tokens_range(
        &self,
        params: SemanticTokensRangeParams,
    ) -> Result<Option<SemanticTokensRangeResult>> {
        impls::semantic_tokens_range(params).await
    }
    /// Handler for the `inline_value` endpoint.
    #[rpc(name = "textDocument/inlineValue")]
    async fn inline_value(&self, params: InlineValueParams) -> Result<Option<Vec<InlineValue>>> {
        impls::hints_ext::inline_value(params).await
    }
    /// Handler for the `inlay_hint` endpoint.
    #[rpc(name = "textDocument/inlayHint")]
    async fn inlay_hint(&self, params: InlayHintParams) -> Result<Option<Vec<InlayHint>>> {
        impls::hints_ext::inlay_hint(params).await
    }
    /// Handler for the `inlay_hint_resolve` endpoint.
    #[rpc(name = "inlayHint/resolve")]
    async fn inlay_hint_resolve(&self, params: InlayHint) -> Result<InlayHint> {
        impls::hints_ext::inlay_hint_resolve(params).await
    }
    /// Handler for the `moniker` endpoint.
    #[rpc(name = "textDocument/moniker")]
    async fn moniker(&self, params: MonikerParams) -> Result<Option<Vec<Moniker>>> {
        impls::moniker(params).await
    }
    /// Handler for the `completion` endpoint.
    #[rpc(name = "textDocument/completion")]
    async fn completion(&self, params: CompletionParams) -> Result<Option<CompletionResponse>> {
        impls::completion(params).await
    }
    /// Handler for the `completion_resolve` endpoint.
    #[rpc(name = "completionItem/resolve")]
    async fn completion_resolve(&self, params: CompletionItem) -> Result<CompletionItem> {
        impls::completion_resolve(params).await
    }
    /// Handler for the `diagnostic` endpoint.
    #[rpc(name = "textDocument/diagnostic")]
    async fn diagnostic(
        &self,
        params: DocumentDiagnosticParams,
    ) -> Result<DocumentDiagnosticReportResult> {
        impls::diag_ext::diagnostic(params).await
    }
    /// Handler for the `workspace_diagnostic` endpoint.
    #[rpc(name = "workspace/diagnostic")]
    async fn workspace_diagnostic(
        &self,
        params: WorkspaceDiagnosticParams,
    ) -> Result<WorkspaceDiagnosticReportResult> {
        impls::diag_ext::workspace_diagnostic(params).await
    }
    /// Handler for the `signature_help` endpoint.
    #[rpc(name = "textDocument/signatureHelp")]
    async fn signature_help(&self, params: SignatureHelpParams) -> Result<Option<SignatureHelp>> {
        impls::signature_help(params).await
    }
    /// Handler for the `code_action` endpoint.
    #[rpc(name = "textDocument/codeAction")]
    async fn code_action(&self, params: CodeActionParams) -> Result<Option<CodeActionResponse>> {
        impls::code_action(params).await
    }
    /// Handler for the `code_action_resolve` endpoint.
    #[rpc(name = "codeAction/resolve")]
    async fn code_action_resolve(&self, params: CodeAction) -> Result<CodeAction> {
        impls::symbols_ext::code_action_resolve(params).await
    }
    /// Handler for the `document_color` endpoint.
    #[rpc(name = "textDocument/documentColor")]
    async fn document_color(&self, params: DocumentColorParams) -> Result<Vec<ColorInformation>> {
        impls::document_color(params).await
    }
    /// Handler for the `color_presentation` endpoint.
    #[rpc(name = "textDocument/colorPresentation")]
    async fn color_presentation(
        &self,
        params: ColorPresentationParams,
    ) -> Result<Vec<ColorPresentation>> {
        impls::color_presentation(params).await
    }
    /// Handler for the `formatting` endpoint.
    #[rpc(name = "textDocument/formatting")]
    async fn formatting(&self, params: DocumentFormattingParams) -> Result<Option<Vec<TextEdit>>> {
        impls::formatting(params).await
    }
    /// Handler for the `range_formatting` endpoint.
    #[rpc(name = "textDocument/rangeFormatting")]
    async fn range_formatting(
        &self,
        params: DocumentRangeFormattingParams,
    ) -> Result<Option<Vec<TextEdit>>> {
        impls::range_formatting(params).await
    }
    /// Handler for the `on_type_formatting` endpoint.
    #[rpc(name = "textDocument/onTypeFormatting")]
    async fn on_type_formatting(
        &self,
        params: DocumentOnTypeFormattingParams,
    ) -> Result<Option<Vec<TextEdit>>> {
        impls::on_type_formatting(params).await
    }
    /// Handler for the `rename` endpoint.
    #[rpc(name = "textDocument/rename")]
    async fn rename(&self, params: RenameParams) -> Result<Option<WorkspaceEdit>> {
        impls::rename(params).await
    }
    /// Handler for the `prepare_rename` endpoint.
    #[rpc(name = "textDocument/prepareRename")]
    async fn prepare_rename(
        &self,
        params: TextDocumentPositionParams,
    ) -> Result<Option<PrepareRenameResponse>> {
        impls::prepare_rename(params).await
    }
    /// Handler for the `linked_editing_range` endpoint.
    #[rpc(name = "textDocument/linkedEditingRange")]
    async fn linked_editing_range(
        &self,
        params: LinkedEditingRangeParams,
    ) -> Result<Option<LinkedEditingRanges>> {
        impls::fmt_ext::linked_editing_range(params).await
    }
    /// Handler for the `symbol` endpoint.
    #[rpc(name = "workspace/symbol")]
    async fn symbol(
        &self,
        params: WorkspaceSymbolParams,
    ) -> Result<Option<Vec<SymbolInformation>>> {
        impls::symbol(params).await
    }
    /// Handler for the `symbol_resolve` endpoint.
    #[rpc(name = "workspaceSymbol/resolve")]
    async fn symbol_resolve(&self, params: WorkspaceSymbol) -> Result<WorkspaceSymbol> {
        impls::symbols_ext::symbol_resolve(params).await
    }
    /// Handler for the `did_change_configuration` endpoint.
    #[rpc(name = "workspace/didChangeConfiguration")]
    async fn did_change_configuration(&self, params: DidChangeConfigurationParams) {
        impls::did_change_configuration(params).await;
    }
    /// Handler for the `did_change_workspace_folders` endpoint.
    #[rpc(name = "workspace/didChangeWorkspaceFolders")]
    async fn did_change_workspace_folders(&self, params: DidChangeWorkspaceFoldersParams) {
        impls::did_change_workspace_folders(params).await;
    }
    /// Handler for the `will_create_files` endpoint.
    #[rpc(name = "workspace/willCreateFiles")]
    async fn will_create_files(&self, params: CreateFilesParams) -> Result<Option<WorkspaceEdit>> {
        impls::will_create_files(params).await
    }
    /// Handler for the `did_create_files` endpoint.
    #[rpc(name = "workspace/didCreateFiles")]
    async fn did_create_files(&self, params: CreateFilesParams) {
        impls::did_create_files(params).await;
    }
    /// Handler for the `will_rename_files` endpoint.
    #[rpc(name = "workspace/willRenameFiles")]
    async fn will_rename_files(&self, params: RenameFilesParams) -> Result<Option<WorkspaceEdit>> {
        impls::will_rename_files(params).await
    }
    /// Handler for the `did_rename_files` endpoint.
    #[rpc(name = "workspace/didRenameFiles")]
    async fn did_rename_files(&self, params: RenameFilesParams) {
        impls::did_rename_files(params).await;
    }
    /// Handler for the `will_delete_files` endpoint.
    #[rpc(name = "workspace/willDeleteFiles")]
    async fn will_delete_files(&self, params: DeleteFilesParams) -> Result<Option<WorkspaceEdit>> {
        impls::will_delete_files(params).await
    }
    /// Handler for the `did_delete_files` endpoint.
    #[rpc(name = "workspace/didDeleteFiles")]
    async fn did_delete_files(&self, params: DeleteFilesParams) {
        impls::did_delete_files(params).await;
    }
    /// Handler for the `did_change_watched_files` endpoint.
    #[rpc(name = "workspace/didChangeWatchedFiles")]
    async fn did_change_watched_files(&self, params: DidChangeWatchedFilesParams) {
        impls::did_change_watched_files(params).await;
    }
    /// Handler for the `execute_command` endpoint.
    #[rpc(name = "workspace/executeCommand")]
    async fn execute_command(&self, params: ExecuteCommandParams) -> Result<Option<Value>> {
        impls::execute_command(params).await
    }
    /// Handler for the `max_snapshot` endpoint.
    #[rpc(name = "max/snapshot")]
    async fn max_snapshot(&self) -> Result<max_protocol::SnapshotId> {
        impls::max_snapshot().await
    }
    /// Handler for the `max_conformance_vector` endpoint.
    #[rpc(name = "max/conformanceVector")]
    async fn max_conformance_vector(
        &self,
        params: Option<max_protocol::SnapshotId>,
    ) -> Result<max_protocol::ConformanceVector> {
        impls::max_conformance_vector(params).await
    }
    /// Handler for the `max_explain_diagnostic` endpoint.
    #[rpc(name = "max/explainDiagnostic")]
    async fn max_explain_diagnostic(&self, params: String) -> Result<max_protocol::MaxDiagnostic> {
        impls::max_explain_diagnostic(params).await
    }
    /// Handler for the `max_repair_plan` endpoint.
    #[rpc(name = "max/repairPlan")]
    async fn max_repair_plan(&self, params: String) -> Result<Vec<max_protocol::MaxCodeAction>> {
        impls::max_repair_plan(params).await
    }
    /// Handler for the `max_apply_repair_transaction` endpoint.
    #[rpc(name = "max/applyRepairTransaction")]
    async fn max_apply_repair_transaction(
        &self,
        params: max_protocol::MaxCodeAction,
    ) -> Result<max_protocol::Receipt> {
        impls::max_apply_repair_transaction(params).await
    }
    /// Handler for the `max_export_analysis_bundle` endpoint.
    #[rpc(name = "max/exportAnalysisBundle")]
    async fn max_export_analysis_bundle(
        &self,
        params: max_protocol::SnapshotId,
    ) -> Result<max_protocol::AnalysisBundle> {
        impls::max_export_analysis_bundle(params).await
    }
    /// Handler for the `max_run_gate` endpoint.
    #[rpc(name = "max/runGate")]
    async fn max_run_gate(&self, params: max_protocol::GateId) -> Result<bool> {
        impls::max_run_gate(params).await
    }
    /// Handler for the `max_clear_diagnostic` endpoint.
    #[rpc(name = "max/clearDiagnostic")]
    async fn max_clear_diagnostic(&self, params: String) -> Result<()> {
        impls::max_clear_diagnostic(params).await
    }
    /// Handler for the `max_receipt` endpoint.
    #[rpc(name = "max/receipt")]
    async fn max_receipt(&self, params: String) -> Result<max_protocol::Receipt> {
        impls::max_receipt(params).await
    }
    /// Handler for the `max_release_actuation` endpoint.
    #[rpc(name = "max/releaseActuation")]
    async fn max_release_actuation(&self, params: Value) -> Result<Value> {
        impls::max_release_actuation(params).await
    }
    /// Handler for the `max_admission` endpoint.
    #[rpc(name = "max/admission")]
    async fn max_admission(&self) -> Result<serde_json::Value> {
        impls::max_admission().await
    }
    /// Handler for the `max_autonomic_loop` endpoint.
    #[rpc(name = "max/autonomicLoop")]
    async fn max_autonomic_loop(&self) -> Result<serde_json::Value> {
        impls::max_autonomic_loop().await
    }
    /// Handler for the `max_chain` endpoint.
    #[rpc(name = "max/chain")]
    async fn max_chain(&self) -> Result<serde_json::Value> {
        impls::max_chain().await
    }
    /// Handler for the `max_hook` endpoint.
    #[rpc(name = "max/hook")]
    async fn max_hook(&self) -> Result<serde_json::Value> {
        impls::max_hook().await
    }
    /// Handler for the `max_hook_graph` endpoint.
    #[rpc(name = "max/hookGraph")]
    async fn max_hook_graph(&self) -> Result<serde_json::Value> {
        impls::max_hook_graph().await
    }
    /// Handler for the `max_lawful_transition` endpoint.
    #[rpc(name = "max/lawfulTransition")]
    async fn max_lawful_transition(&self, params: String) -> Result<serde_json::Value> {
        impls::max_lawful_transition(params).await
    }
    /// Handler for the `max_ledger_report` endpoint.
    #[rpc(name = "max/ledgerReport")]
    async fn max_ledger_report(&self) -> Result<String> {
        impls::max_ledger_report().await
    }
    /// Handler for the `max_manifold_snapshot` endpoint.
    #[rpc(name = "max/manifoldSnapshot")]
    async fn max_manifold_snapshot(&self) -> Result<serde_json::Value> {
        impls::max_manifold_snapshot().await
    }
    /// Handler for the `max_propagate` endpoint.
    #[rpc(name = "max/propagate")]
    async fn max_propagate(&self, params: max_protocol::Receipt) -> Result<serde_json::Value> {
        impls::max_propagate(params).await
    }
    /// Handler for the `max_refusal` endpoint.
    #[rpc(name = "max/refusal")]
    async fn max_refusal(&self, params: String) -> Result<serde_json::Value> {
        impls::max_refusal(params).await
    }
    /// Handler for the `max_replay` endpoint.
    #[rpc(name = "max/replay")]
    async fn max_replay(&self) -> Result<serde_json::Value> {
        impls::max_replay().await
    }
    /// Handler for the `max_verify_ledger` endpoint.
    #[rpc(name = "max/verifyLedger")]
    async fn max_verify_ledger(&self) -> Result<serde_json::Value> {
        impls::max_verify_ledger().await
    }
    /// Handler for the `max_conformance_delta` endpoint.
    #[rpc(name = "max/conformanceDelta")]
    async fn max_conformance_delta(&self, params: serde_json::Value) -> Result<serde_json::Value> {
        impls::max_conformance_delta(params).await
    }
    /// Handler for the `inline_completion` endpoint.
    #[rpc(name = "textDocument/inlineCompletion")]
    async fn inline_completion(
        &self,
        params: InlineCompletionParams,
    ) -> Result<Option<InlineCompletionResponse>> {
        let _ = params;
        Err(Error::method_not_found())
    }
    /// Handler for the `text_document_content` endpoint.
    #[rpc(name = "workspace/textDocumentContent")]
    async fn text_document_content(
        &self,
        params: max_protocol::lsp_3_18::TextDocumentContentParams,
    ) -> Result<max_protocol::lsp_3_18::TextDocumentContentResult> {
        let _ = params;
        Err(Error::method_not_found())
    }
    /// Handler for the `max_dump_state` endpoint.
    #[rpc(name = "max/dumpState")]
    async fn max_dump_state(&self) -> Result<serde_json::Value> {
        impls::max_dump_state().await
    }
    /// Handler for the `max_restore_state` endpoint.
    #[rpc(name = "max/restoreState")]
    async fn max_restore_state(&self, params: serde_json::Value) -> Result<()> {
        impls::max_restore_state(params).await
    }
    /// Handler for the `ranges_formatting` endpoint.
    #[rpc(name = "textDocument/rangesFormatting")]
    async fn ranges_formatting(
        &self,
        params: max_protocol::lsp_3_18::DocumentRangesFormattingParams,
    ) -> Result<Option<Vec<max_protocol::lsp_3_18::TextEdit>>> {
        let _ = params;
        Ok(None)
    }
    /// Handler for the `did_open_notebook_document` endpoint.
    #[rpc(name = "notebookDocument/didOpen")]
    async fn did_open_notebook_document(&self, params: DidOpenNotebookDocumentParams) {
        impls::notebook_ext::did_open_notebook(params).await
    }
    /// Handler for the `did_change_notebook_document` endpoint.
    #[rpc(name = "notebookDocument/didChange")]
    async fn did_change_notebook_document(&self, params: DidChangeNotebookDocumentParams) {
        impls::notebook_ext::did_change_notebook(params).await
    }
    /// Handler for the `did_save_notebook_document` endpoint.
    #[rpc(name = "notebookDocument/didSave")]
    async fn did_save_notebook_document(&self, params: DidSaveNotebookDocumentParams) {
        impls::notebook_ext::did_save_notebook(params).await
    }
    /// Handler for the `did_close_notebook_document` endpoint.
    #[rpc(name = "notebookDocument/didClose")]
    async fn did_close_notebook_document(&self, params: DidCloseNotebookDocumentParams) {
        impls::notebook_ext::did_close_notebook(params).await
    }
    /// Handler for the `work_done_progress_cancel` endpoint.
    #[rpc(name = "window/workDoneProgress/cancel")]
    async fn work_done_progress_cancel(&self, params: WorkDoneProgressCancelParams) {
        impls::work_done_progress_cancel(params).await;
    }
    /// Handler for the `set_trace` endpoint.
    #[rpc(name = "$/setTrace")]
    async fn set_trace(&self, params: SetTraceParams) {
        impls::set_trace(params).await;
    }
    /// Handler for the `progress` endpoint.
    #[rpc(name = "$/progress")]
    async fn progress(&self, params: ProgressParams) {
        impls::progress(params).await;
    }
    /// Handler for the `max_instance_list` endpoint.
    #[rpc(name = "max/instanceList")]
    async fn max_instance_list(&self) -> Result<Value> {
        impls::max_instance_list().await
    }
    /// Handler for the `max_reset` endpoint.
    #[rpc(name = "max/reset")]
    async fn max_reset(&self) -> Result<()> {
        impls::max_reset().await
    }
    /// Handler for the `max_lsif` endpoint.
    #[rpc(name = "max/lsif")]
    async fn max_lsif(&self) -> Result<String> {
        impls::max_lsif().await
    }
}
