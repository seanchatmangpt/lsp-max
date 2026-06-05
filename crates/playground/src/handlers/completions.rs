use tower_lsp_max::lsp_types::*;

use crate::Backend;

// ---------------------------------------------------------------------------
// Static method table
// ---------------------------------------------------------------------------

/// One entry per `LanguageServer` trait method.
pub struct MethodEntry {
    pub fn_name: &'static str,
    pub lsp_method: &'static str,
    pub params_type: &'static str,
    pub return_type: &'static str,
    pub capability_field: Option<&'static str>,
    pub domain: Domain,
}

#[derive(Clone, Copy, PartialEq)]
pub enum Domain {
    Lifecycle,
    TextSync,
    Navigation,
    Symbols,
    Editing,
    Diagnostics,
    CodeLens,
    SemanticTokens,
    Workspace,
    Window,
    Max,
}

/// Master capability-method table derived from the `LanguageServer` trait.
pub static METHODS: &[MethodEntry] = &[
    // --- LIFECYCLE ---
    MethodEntry {
        fn_name: "initialize",
        lsp_method: "initialize",
        params_type: "InitializeParams",
        return_type: "Result<InitializeResult>",
        capability_field: None,
        domain: Domain::Lifecycle,
    },
    MethodEntry {
        fn_name: "initialized",
        lsp_method: "initialized",
        params_type: "InitializedParams",
        return_type: "()",
        capability_field: None,
        domain: Domain::Lifecycle,
    },
    MethodEntry {
        fn_name: "shutdown",
        lsp_method: "shutdown",
        params_type: "()",
        return_type: "Result<()>",
        capability_field: None,
        domain: Domain::Lifecycle,
    },
    // --- TEXT SYNC ---
    MethodEntry {
        fn_name: "did_open",
        lsp_method: "textDocument/didOpen",
        params_type: "DidOpenTextDocumentParams",
        return_type: "()",
        capability_field: Some("text_document_sync"),
        domain: Domain::TextSync,
    },
    MethodEntry {
        fn_name: "did_change",
        lsp_method: "textDocument/didChange",
        params_type: "DidChangeTextDocumentParams",
        return_type: "()",
        capability_field: Some("text_document_sync"),
        domain: Domain::TextSync,
    },
    MethodEntry {
        fn_name: "did_save",
        lsp_method: "textDocument/didSave",
        params_type: "DidSaveTextDocumentParams",
        return_type: "()",
        capability_field: Some("text_document_sync"),
        domain: Domain::TextSync,
    },
    MethodEntry {
        fn_name: "did_close",
        lsp_method: "textDocument/didClose",
        params_type: "DidCloseTextDocumentParams",
        return_type: "()",
        capability_field: Some("text_document_sync"),
        domain: Domain::TextSync,
    },
    MethodEntry {
        fn_name: "will_save",
        lsp_method: "textDocument/willSave",
        params_type: "WillSaveTextDocumentParams",
        return_type: "()",
        capability_field: Some("text_document_sync"),
        domain: Domain::TextSync,
    },
    MethodEntry {
        fn_name: "will_save_wait_until",
        lsp_method: "textDocument/willSaveWaitUntil",
        params_type: "WillSaveTextDocumentParams",
        return_type: "Result<Option<Vec<TextEdit>>>",
        capability_field: Some("text_document_sync"),
        domain: Domain::TextSync,
    },
    MethodEntry {
        fn_name: "did_open_notebook_document",
        lsp_method: "notebookDocument/didOpen",
        params_type: "max_protocol::lsp_3_18::DidOpenNotebookDocumentParams",
        return_type: "()",
        capability_field: Some("notebook_document_sync"),
        domain: Domain::TextSync,
    },
    MethodEntry {
        fn_name: "did_change_notebook_document",
        lsp_method: "notebookDocument/didChange",
        params_type: "max_protocol::lsp_3_18::DidChangeNotebookDocumentParams",
        return_type: "()",
        capability_field: Some("notebook_document_sync"),
        domain: Domain::TextSync,
    },
    MethodEntry {
        fn_name: "did_save_notebook_document",
        lsp_method: "notebookDocument/didSave",
        params_type: "max_protocol::lsp_3_18::DidSaveNotebookDocumentParams",
        return_type: "()",
        capability_field: Some("notebook_document_sync"),
        domain: Domain::TextSync,
    },
    MethodEntry {
        fn_name: "did_close_notebook_document",
        lsp_method: "notebookDocument/didClose",
        params_type: "max_protocol::lsp_3_18::DidCloseNotebookDocumentParams",
        return_type: "()",
        capability_field: Some("notebook_document_sync"),
        domain: Domain::TextSync,
    },
    // --- NAVIGATION ---
    MethodEntry {
        fn_name: "goto_definition",
        lsp_method: "textDocument/definition",
        params_type: "GotoDefinitionParams",
        return_type: "Result<Option<GotoDefinitionResponse>>",
        capability_field: Some("definition_provider"),
        domain: Domain::Navigation,
    },
    MethodEntry {
        fn_name: "goto_declaration",
        lsp_method: "textDocument/declaration",
        params_type: "GotoDeclarationParams",
        return_type: "Result<Option<GotoDeclarationResponse>>",
        capability_field: Some("declaration_provider"),
        domain: Domain::Navigation,
    },
    MethodEntry {
        fn_name: "goto_type_definition",
        lsp_method: "textDocument/typeDefinition",
        params_type: "GotoTypeDefinitionParams",
        return_type: "Result<Option<GotoTypeDefinitionResponse>>",
        capability_field: Some("type_definition_provider"),
        domain: Domain::Navigation,
    },
    MethodEntry {
        fn_name: "goto_implementation",
        lsp_method: "textDocument/implementation",
        params_type: "GotoImplementationParams",
        return_type: "Result<Option<GotoImplementationResponse>>",
        capability_field: Some("implementation_provider"),
        domain: Domain::Navigation,
    },
    MethodEntry {
        fn_name: "references",
        lsp_method: "textDocument/references",
        params_type: "ReferenceParams",
        return_type: "Result<Option<Vec<Location>>>",
        capability_field: Some("references_provider"),
        domain: Domain::Navigation,
    },
    MethodEntry {
        fn_name: "document_highlight",
        lsp_method: "textDocument/documentHighlight",
        params_type: "DocumentHighlightParams",
        return_type: "Result<Option<Vec<DocumentHighlight>>>",
        capability_field: Some("document_highlight_provider"),
        domain: Domain::Navigation,
    },
    MethodEntry {
        fn_name: "prepare_call_hierarchy",
        lsp_method: "textDocument/prepareCallHierarchy",
        params_type: "CallHierarchyPrepareParams",
        return_type: "Result<Option<Vec<CallHierarchyItem>>>",
        capability_field: Some("call_hierarchy_provider"),
        domain: Domain::Navigation,
    },
    MethodEntry {
        fn_name: "incoming_calls",
        lsp_method: "callHierarchy/incomingCalls",
        params_type: "CallHierarchyIncomingCallsParams",
        return_type: "Result<Option<Vec<CallHierarchyIncomingCall>>>",
        capability_field: Some("call_hierarchy_provider"),
        domain: Domain::Navigation,
    },
    MethodEntry {
        fn_name: "outgoing_calls",
        lsp_method: "callHierarchy/outgoingCalls",
        params_type: "CallHierarchyOutgoingCallsParams",
        return_type: "Result<Option<Vec<CallHierarchyOutgoingCall>>>",
        capability_field: Some("call_hierarchy_provider"),
        domain: Domain::Navigation,
    },
    MethodEntry {
        fn_name: "moniker",
        lsp_method: "textDocument/moniker",
        params_type: "MonikerParams",
        return_type: "Result<Option<Vec<Moniker>>>",
        capability_field: Some("moniker_provider"),
        domain: Domain::Navigation,
    },
    // --- SYMBOLS ---
    MethodEntry {
        fn_name: "document_symbol",
        lsp_method: "textDocument/documentSymbol",
        params_type: "DocumentSymbolParams",
        return_type: "Result<Option<DocumentSymbolResponse>>",
        capability_field: Some("document_symbol_provider"),
        domain: Domain::Symbols,
    },
    MethodEntry {
        fn_name: "symbol",
        lsp_method: "workspace/symbol",
        params_type: "WorkspaceSymbolParams",
        return_type: "Result<Option<Vec<SymbolInformation>>>",
        capability_field: Some("workspace_symbol_provider"),
        domain: Domain::Symbols,
    },
    MethodEntry {
        fn_name: "prepare_type_hierarchy",
        lsp_method: "textDocument/prepareTypeHierarchy",
        params_type: "TypeHierarchyPrepareParams",
        return_type: "Result<Option<Vec<TypeHierarchyItem>>>",
        capability_field: Some("type_hierarchy_provider"),
        domain: Domain::Symbols,
    },
    MethodEntry {
        fn_name: "supertypes",
        lsp_method: "typeHierarchy/supertypes",
        params_type: "TypeHierarchySupertypesParams",
        return_type: "Result<Option<Vec<TypeHierarchyItem>>>",
        capability_field: Some("type_hierarchy_provider"),
        domain: Domain::Symbols,
    },
    MethodEntry {
        fn_name: "subtypes",
        lsp_method: "typeHierarchy/subtypes",
        params_type: "TypeHierarchySubtypesParams",
        return_type: "Result<Option<Vec<TypeHierarchyItem>>>",
        capability_field: Some("type_hierarchy_provider"),
        domain: Domain::Symbols,
    },
    // --- EDITING ---
    MethodEntry {
        fn_name: "completion",
        lsp_method: "textDocument/completion",
        params_type: "CompletionParams",
        return_type: "Result<Option<CompletionResponse>>",
        capability_field: Some("completion_provider"),
        domain: Domain::Editing,
    },
    MethodEntry {
        fn_name: "completion_resolve",
        lsp_method: "completionItem/resolve",
        params_type: "CompletionItem",
        return_type: "Result<CompletionItem>",
        capability_field: Some("completion_provider"),
        domain: Domain::Editing,
    },
    MethodEntry {
        fn_name: "hover",
        lsp_method: "textDocument/hover",
        params_type: "HoverParams",
        return_type: "Result<Option<Hover>>",
        capability_field: Some("hover_provider"),
        domain: Domain::Editing,
    },
    MethodEntry {
        fn_name: "signature_help",
        lsp_method: "textDocument/signatureHelp",
        params_type: "SignatureHelpParams",
        return_type: "Result<Option<SignatureHelp>>",
        capability_field: Some("signature_help_provider"),
        domain: Domain::Editing,
    },
    MethodEntry {
        fn_name: "code_action",
        lsp_method: "textDocument/codeAction",
        params_type: "CodeActionParams",
        return_type: "Result<Option<CodeActionResponse>>",
        capability_field: Some("code_action_provider"),
        domain: Domain::Editing,
    },
    MethodEntry {
        fn_name: "code_action_resolve",
        lsp_method: "codeAction/resolve",
        params_type: "CodeAction",
        return_type: "Result<CodeAction>",
        capability_field: Some("code_action_provider"),
        domain: Domain::Editing,
    },
    MethodEntry {
        fn_name: "rename",
        lsp_method: "textDocument/rename",
        params_type: "RenameParams",
        return_type: "Result<Option<WorkspaceEdit>>",
        capability_field: Some("rename_provider"),
        domain: Domain::Editing,
    },
    MethodEntry {
        fn_name: "prepare_rename",
        lsp_method: "textDocument/prepareRename",
        params_type: "TextDocumentPositionParams",
        return_type: "Result<Option<PrepareRenameResponse>>",
        capability_field: Some("rename_provider"),
        domain: Domain::Editing,
    },
    MethodEntry {
        fn_name: "formatting",
        lsp_method: "textDocument/formatting",
        params_type: "DocumentFormattingParams",
        return_type: "Result<Option<Vec<TextEdit>>>",
        capability_field: Some("document_formatting_provider"),
        domain: Domain::Editing,
    },
    MethodEntry {
        fn_name: "range_formatting",
        lsp_method: "textDocument/rangeFormatting",
        params_type: "DocumentRangeFormattingParams",
        return_type: "Result<Option<Vec<TextEdit>>>",
        capability_field: Some("document_range_formatting_provider"),
        domain: Domain::Editing,
    },
    MethodEntry {
        fn_name: "on_type_formatting",
        lsp_method: "textDocument/onTypeFormatting",
        params_type: "DocumentOnTypeFormattingParams",
        return_type: "Result<Option<Vec<TextEdit>>>",
        capability_field: Some("document_on_type_formatting_provider"),
        domain: Domain::Editing,
    },
    MethodEntry {
        fn_name: "inline_completion",
        lsp_method: "textDocument/inlineCompletion",
        params_type: "InlineCompletionParams",
        return_type: "Result<Option<InlineCompletionResponse>>",
        capability_field: Some("inline_completion_provider"),
        domain: Domain::Editing,
    },
    MethodEntry {
        fn_name: "linked_editing_range",
        lsp_method: "textDocument/linkedEditingRange",
        params_type: "LinkedEditingRangeParams",
        return_type: "Result<Option<LinkedEditingRanges>>",
        capability_field: Some("linked_editing_range_provider"),
        domain: Domain::Editing,
    },
    MethodEntry {
        fn_name: "document_color",
        lsp_method: "textDocument/documentColor",
        params_type: "DocumentColorParams",
        return_type: "Result<Vec<ColorInformation>>",
        capability_field: Some("color_provider"),
        domain: Domain::Editing,
    },
    MethodEntry {
        fn_name: "color_presentation",
        lsp_method: "textDocument/colorPresentation",
        params_type: "ColorPresentationParams",
        return_type: "Result<Vec<ColorPresentation>>",
        capability_field: Some("color_provider"),
        domain: Domain::Editing,
    },
    MethodEntry {
        fn_name: "ranges_formatting",
        lsp_method: "textDocument/rangesFormatting",
        params_type: "max_protocol::lsp_3_18::DocumentRangesFormattingParams",
        return_type: "Result<Option<Vec<max_protocol::lsp_3_18::TextEdit>>>",
        capability_field: Some("document_formatting_provider"),
        domain: Domain::Editing,
    },
    // --- DIAGNOSTICS ---
    MethodEntry {
        fn_name: "diagnostic",
        lsp_method: "textDocument/diagnostic",
        params_type: "DocumentDiagnosticParams",
        return_type: "Result<DocumentDiagnosticReportResult>",
        capability_field: Some("diagnostic_provider"),
        domain: Domain::Diagnostics,
    },
    MethodEntry {
        fn_name: "workspace_diagnostic",
        lsp_method: "workspace/diagnostic",
        params_type: "WorkspaceDiagnosticParams",
        return_type: "Result<WorkspaceDiagnosticReport>",
        capability_field: Some("diagnostic_provider"),
        domain: Domain::Diagnostics,
    },
    // --- CODE LENS & LINKS ---
    MethodEntry {
        fn_name: "code_lens",
        lsp_method: "textDocument/codeLens",
        params_type: "CodeLensParams",
        return_type: "Result<Option<Vec<CodeLens>>>",
        capability_field: Some("code_lens_provider"),
        domain: Domain::CodeLens,
    },
    MethodEntry {
        fn_name: "code_lens_resolve",
        lsp_method: "codeLens/resolve",
        params_type: "CodeLens",
        return_type: "Result<CodeLens>",
        capability_field: Some("code_lens_provider"),
        domain: Domain::CodeLens,
    },
    MethodEntry {
        fn_name: "document_link",
        lsp_method: "textDocument/documentLink",
        params_type: "DocumentLinkParams",
        return_type: "Result<Option<Vec<DocumentLink>>>",
        capability_field: Some("document_link_provider"),
        domain: Domain::CodeLens,
    },
    MethodEntry {
        fn_name: "document_link_resolve",
        lsp_method: "documentLink/resolve",
        params_type: "DocumentLink",
        return_type: "Result<DocumentLink>",
        capability_field: Some("document_link_provider"),
        domain: Domain::CodeLens,
    },
    MethodEntry {
        fn_name: "inlay_hint",
        lsp_method: "textDocument/inlayHint",
        params_type: "InlayHintParams",
        return_type: "Result<Option<Vec<InlayHint>>>",
        capability_field: Some("inlay_hint_provider"),
        domain: Domain::CodeLens,
    },
    MethodEntry {
        fn_name: "inlay_hint_resolve",
        lsp_method: "inlayHint/resolve",
        params_type: "InlayHint",
        return_type: "Result<InlayHint>",
        capability_field: Some("inlay_hint_provider"),
        domain: Domain::CodeLens,
    },
    MethodEntry {
        fn_name: "folding_range",
        lsp_method: "textDocument/foldingRange",
        params_type: "FoldingRangeParams",
        return_type: "Result<Option<Vec<FoldingRange>>>",
        capability_field: Some("folding_range_provider"),
        domain: Domain::CodeLens,
    },
    MethodEntry {
        fn_name: "selection_range",
        lsp_method: "textDocument/selectionRange",
        params_type: "SelectionRangeParams",
        return_type: "Result<Option<Vec<SelectionRange>>>",
        capability_field: Some("selection_range_provider"),
        domain: Domain::CodeLens,
    },
    MethodEntry {
        fn_name: "inline_value",
        lsp_method: "textDocument/inlineValue",
        params_type: "InlineValueParams",
        return_type: "Result<Option<Vec<InlineValue>>>",
        capability_field: Some("inline_value_provider"),
        domain: Domain::CodeLens,
    },
    // --- SEMANTIC TOKENS ---
    MethodEntry {
        fn_name: "semantic_tokens_full",
        lsp_method: "textDocument/semanticTokens/full",
        params_type: "SemanticTokensParams",
        return_type: "Result<Option<SemanticTokensResult>>",
        capability_field: Some("semantic_tokens_provider"),
        domain: Domain::SemanticTokens,
    },
    MethodEntry {
        fn_name: "semantic_tokens_full_delta",
        lsp_method: "textDocument/semanticTokens/full/delta",
        params_type: "SemanticTokensDeltaParams",
        return_type: "Result<Option<SemanticTokensFullDeltaResult>>",
        capability_field: Some("semantic_tokens_provider"),
        domain: Domain::SemanticTokens,
    },
    MethodEntry {
        fn_name: "semantic_tokens_range",
        lsp_method: "textDocument/semanticTokens/range",
        params_type: "SemanticTokensRangeParams",
        return_type: "Result<Option<SemanticTokensRangeResult>>",
        capability_field: Some("semantic_tokens_provider"),
        domain: Domain::SemanticTokens,
    },
    // --- WORKSPACE ---
    MethodEntry {
        fn_name: "execute_command",
        lsp_method: "workspace/executeCommand",
        params_type: "ExecuteCommandParams",
        return_type: "Result<Option<Value>>",
        capability_field: Some("execute_command_provider"),
        domain: Domain::Workspace,
    },
    MethodEntry {
        fn_name: "symbol_resolve",
        lsp_method: "workspaceSymbol/resolve",
        params_type: "WorkspaceSymbol",
        return_type: "Result<WorkspaceSymbol>",
        capability_field: Some("workspace_symbol_provider"),
        domain: Domain::Workspace,
    },
    MethodEntry {
        fn_name: "did_change_configuration",
        lsp_method: "workspace/didChangeConfiguration",
        params_type: "DidChangeConfigurationParams",
        return_type: "()",
        capability_field: None,
        domain: Domain::Workspace,
    },
    MethodEntry {
        fn_name: "did_change_workspace_folders",
        lsp_method: "workspace/didChangeWorkspaceFolders",
        params_type: "DidChangeWorkspaceFoldersParams",
        return_type: "()",
        capability_field: Some("workspace.workspaceFolders"),
        domain: Domain::Workspace,
    },
    MethodEntry {
        fn_name: "did_change_watched_files",
        lsp_method: "workspace/didChangeWatchedFiles",
        params_type: "DidChangeWatchedFilesParams",
        return_type: "()",
        capability_field: None,
        domain: Domain::Workspace,
    },
    MethodEntry {
        fn_name: "will_create_files",
        lsp_method: "workspace/willCreateFiles",
        params_type: "CreateFilesParams",
        return_type: "Result<Option<WorkspaceEdit>>",
        capability_field: Some("workspace.fileOperations"),
        domain: Domain::Workspace,
    },
    MethodEntry {
        fn_name: "did_create_files",
        lsp_method: "workspace/didCreateFiles",
        params_type: "CreateFilesParams",
        return_type: "()",
        capability_field: Some("workspace.fileOperations"),
        domain: Domain::Workspace,
    },
    MethodEntry {
        fn_name: "will_rename_files",
        lsp_method: "workspace/willRenameFiles",
        params_type: "RenameFilesParams",
        return_type: "Result<Option<WorkspaceEdit>>",
        capability_field: Some("workspace.fileOperations"),
        domain: Domain::Workspace,
    },
    MethodEntry {
        fn_name: "did_rename_files",
        lsp_method: "workspace/didRenameFiles",
        params_type: "RenameFilesParams",
        return_type: "()",
        capability_field: Some("workspace.fileOperations"),
        domain: Domain::Workspace,
    },
    MethodEntry {
        fn_name: "will_delete_files",
        lsp_method: "workspace/willDeleteFiles",
        params_type: "DeleteFilesParams",
        return_type: "Result<Option<WorkspaceEdit>>",
        capability_field: Some("workspace.fileOperations"),
        domain: Domain::Workspace,
    },
    MethodEntry {
        fn_name: "did_delete_files",
        lsp_method: "workspace/didDeleteFiles",
        params_type: "DeleteFilesParams",
        return_type: "()",
        capability_field: Some("workspace.fileOperations"),
        domain: Domain::Workspace,
    },
    MethodEntry {
        fn_name: "text_document_content",
        lsp_method: "workspace/textDocumentContent",
        params_type: "TextDocumentContentParams",
        return_type: "Result<TextDocumentContentResult>",
        capability_field: Some("text_document_content_provider"),
        domain: Domain::Workspace,
    },
    MethodEntry {
        fn_name: "text_document_content_refresh",
        lsp_method: "workspace/textDocumentContent/refresh",
        params_type: "max_protocol::lsp_3_18::TextDocumentContentRefreshParams",
        return_type: "Result<()>",
        capability_field: Some("text_document_content_provider"),
        domain: Domain::Workspace,
    },
    MethodEntry {
        fn_name: "work_done_progress_cancel",
        lsp_method: "window/workDoneProgress/cancel",
        params_type: "max_protocol::lsp_3_18::WorkDoneProgressCancelParams",
        return_type: "()",
        capability_field: None,
        domain: Domain::Window,
    },
    MethodEntry {
        fn_name: "progress",
        lsp_method: "$/progress",
        params_type: "max_protocol::lsp_3_18::ProgressParams",
        return_type: "()",
        capability_field: None,
        domain: Domain::Window,
    },
    MethodEntry {
        fn_name: "set_trace",
        lsp_method: "$/setTrace",
        params_type: "max_protocol::lsp_3_18::SetTraceParams",
        return_type: "()",
        capability_field: None,
        domain: Domain::Window,
    },
    // --- MAX EXTENSIONS ---
    MethodEntry {
        fn_name: "max_snapshot",
        lsp_method: "max/snapshot",
        params_type: "()",
        return_type: "Result<SnapshotId>",
        capability_field: None,
        domain: Domain::Max,
    },
    MethodEntry {
        fn_name: "max_conformance_vector",
        lsp_method: "max/conformanceVector",
        params_type: "MaxConformanceVectorParams",
        return_type: "Result<MaxConformanceVector>",
        capability_field: None,
        domain: Domain::Max,
    },
    MethodEntry {
        fn_name: "max_explain_diagnostic",
        lsp_method: "max/explainDiagnostic",
        params_type: "String",
        return_type: "Result<MaxDiagnostic>",
        capability_field: None,
        domain: Domain::Max,
    },
    MethodEntry {
        fn_name: "max_repair_plan",
        lsp_method: "max/repairPlan",
        params_type: "String",
        return_type: "Result<Vec<MaxCodeAction>>",
        capability_field: None,
        domain: Domain::Max,
    },
    MethodEntry {
        fn_name: "max_run_gate",
        lsp_method: "max/runGate",
        params_type: "GateId",
        return_type: "Result<bool>",
        capability_field: None,
        domain: Domain::Max,
    },
    MethodEntry {
        fn_name: "max_receipt",
        lsp_method: "max/receipt",
        params_type: "String",
        return_type: "Result<Receipt>",
        capability_field: None,
        domain: Domain::Max,
    },
    MethodEntry {
        fn_name: "max_clear_diagnostic",
        lsp_method: "max/clearDiagnostic",
        params_type: "String",
        return_type: "Result<()>",
        capability_field: None,
        domain: Domain::Max,
    },
    MethodEntry {
        fn_name: "max_apply_repair_transaction",
        lsp_method: "max/applyRepairTransaction",
        params_type: "MaxRepairTransaction",
        return_type: "Result<MaxRepairResult>",
        capability_field: None,
        domain: Domain::Max,
    },
    MethodEntry {
        fn_name: "max_export_analysis_bundle",
        lsp_method: "max/exportAnalysisBundle",
        params_type: "MaxExportBundleParams",
        return_type: "Result<MaxAnalysisBundle>",
        capability_field: None,
        domain: Domain::Max,
    },
    MethodEntry {
        fn_name: "max_dump_state",
        lsp_method: "max/dumpState",
        params_type: "()",
        return_type: "Result<serde_json::Value>",
        capability_field: None,
        domain: Domain::Max,
    },
    MethodEntry {
        fn_name: "max_restore_state",
        lsp_method: "max/restoreState",
        params_type: "serde_json::Value",
        return_type: "Result<()>",
        capability_field: None,
        domain: Domain::Max,
    },
];

// ---------------------------------------------------------------------------
// Capability fields table
// ---------------------------------------------------------------------------

/// `(field_name, type_str, &[handler_fn_names])`
pub static CAPABILITY_FIELDS: &[(&str, &str, &[&str])] = &[
    (
        "text_document_sync",
        "Option<TextDocumentSyncCapability>",
        &[
            "did_open",
            "did_change",
            "did_save",
            "did_close",
            "will_save",
            "will_save_wait_until",
        ],
    ),
    (
        "completion_provider",
        "Option<CompletionOptions>",
        &["completion", "completion_resolve"],
    ),
    (
        "hover_provider",
        "Option<HoverProviderCapability>",
        &["hover"],
    ),
    (
        "definition_provider",
        "Option<OneOf<bool, DefinitionOptions>>",
        &["goto_definition"],
    ),
    (
        "declaration_provider",
        "Option<DeclarationCapability>",
        &["goto_declaration"],
    ),
    (
        "type_definition_provider",
        "Option<TypeDefinitionProviderCapability>",
        &["goto_type_definition"],
    ),
    (
        "implementation_provider",
        "Option<ImplementationProviderCapability>",
        &["goto_implementation"],
    ),
    (
        "references_provider",
        "Option<OneOf<bool, ReferencesOptions>>",
        &["references"],
    ),
    (
        "document_highlight_provider",
        "Option<OneOf<bool, DocumentHighlightOptions>>",
        &["document_highlight"],
    ),
    (
        "document_symbol_provider",
        "Option<OneOf<bool, DocumentSymbolOptions>>",
        &["document_symbol"],
    ),
    (
        "workspace_symbol_provider",
        "Option<OneOf<bool, WorkspaceSymbolOptions>>",
        &["symbol", "symbol_resolve"],
    ),
    (
        "code_action_provider",
        "Option<CodeActionProviderCapability>",
        &["code_action", "code_action_resolve"],
    ),
    (
        "code_lens_provider",
        "Option<CodeLensOptions>",
        &["code_lens", "code_lens_resolve"],
    ),
    (
        "document_link_provider",
        "Option<DocumentLinkOptions>",
        &["document_link", "document_link_resolve"],
    ),
    (
        "document_formatting_provider",
        "Option<OneOf<bool, DocumentFormattingOptions>>",
        &["formatting"],
    ),
    (
        "document_range_formatting_provider",
        "Option<OneOf<bool, DocumentRangeFormattingOptions>>",
        &["range_formatting"],
    ),
    (
        "document_on_type_formatting_provider",
        "Option<DocumentOnTypeFormattingOptions>",
        &["on_type_formatting"],
    ),
    (
        "rename_provider",
        "Option<OneOf<bool, RenameOptions>>",
        &["rename", "prepare_rename"],
    ),
    (
        "semantic_tokens_provider",
        "Option<SemanticTokensServerCapabilities>",
        &[
            "semantic_tokens_full",
            "semantic_tokens_full_delta",
            "semantic_tokens_range",
        ],
    ),
    (
        "inlay_hint_provider",
        "Option<OneOf<bool, InlayHintServerCapabilities>>",
        &["inlay_hint", "inlay_hint_resolve"],
    ),
    (
        "diagnostic_provider",
        "Option<DiagnosticServerCapabilities>",
        &["diagnostic", "workspace_diagnostic"],
    ),
    (
        "execute_command_provider",
        "Option<ExecuteCommandOptions>",
        &["execute_command"],
    ),
    (
        "call_hierarchy_provider",
        "Option<CallHierarchyServerCapability>",
        &["prepare_call_hierarchy", "incoming_calls", "outgoing_calls"],
    ),
    (
        "type_hierarchy_provider",
        "Option<OneOf<bool, TypeHierarchyOptions>>",
        &["prepare_type_hierarchy", "supertypes", "subtypes"],
    ),
    (
        "folding_range_provider",
        "Option<FoldingRangeProviderCapability>",
        &["folding_range"],
    ),
    (
        "selection_range_provider",
        "Option<SelectionRangeProviderCapability>",
        &["selection_range"],
    ),
    (
        "inline_completion_provider",
        "Option<OneOf<bool, InlineCompletionOptions>>",
        &["inline_completion"],
    ),
    (
        "text_document_content_provider",
        "Option<TextDocumentContentOptions>",
        &["text_document_content"],
    ),
];

// ---------------------------------------------------------------------------
// Public entry point
// ---------------------------------------------------------------------------

/// Completion entry point called from `lib.rs`.
pub async fn compute(backend: &Backend, uri: &Url, pos: Position) -> Option<CompletionResponse> {
    let doc = backend.docs.get(uri)?;
    if doc.language_id != "rust" {
        return None;
    }
    let source = doc.text.to_string();
    drop(doc);

    let items = get_completions(pos, &source);
    if items.is_empty() {
        None
    } else {
        Some(CompletionResponse::Array(items))
    }
}

/// Determine completions based on cursor context. Public so tests can call it directly.
pub fn get_completions(pos: Position, text: &str) -> Vec<CompletionItem> {
    let ctx = detect_context(text, pos);
    match ctx {
        CompletionContext::ImplLanguageServerMethod { partial } => complete_ls_methods(&partial),
        CompletionContext::ServerCapabilitiesField { partial } => {
            complete_capability_fields(&partial)
        }
        CompletionContext::RpcName { partial } => complete_rpc_names(&partial),
        CompletionContext::LspServiceBuilder => complete_lsp_service_builder(),
        CompletionContext::MessageTypeVariant => complete_message_type_variants(),
        CompletionContext::Unknown => vec![],
    }
}

// ---------------------------------------------------------------------------
// Context detection
// ---------------------------------------------------------------------------

enum CompletionContext {
    ImplLanguageServerMethod { partial: String },
    ServerCapabilitiesField { partial: String },
    RpcName { partial: String },
    LspServiceBuilder,
    MessageTypeVariant,
    Unknown,
}

fn detect_context(source: &str, pos: Position) -> CompletionContext {
    let lines: Vec<&str> = source.lines().collect();
    let current_line = lines.get(pos.line as usize).copied().unwrap_or("");
    let char_pos = (pos.character as usize).min(current_line.len());
    let prefix = &current_line[..char_pos];

    if let Some(partial) = find_rpc_attribute_partial(prefix, &lines, pos.line as usize) {
        return CompletionContext::RpcName { partial };
    }

    // MessageType:: context
    if prefix.contains("MessageType::") {
        return CompletionContext::MessageTypeVariant;
    }

    // LspService builder: line starts with `.` after LspService::build chain
    if prefix.trim_start().starts_with('.') {
        for line in lines[..pos.line as usize].iter().rev().take(10) {
            if line.contains("LspService::build") {
                return CompletionContext::LspServiceBuilder;
            }
        }
    }

    // ServerCapabilities struct literal: scan upward for the struct name
    for line in lines[..=pos.line as usize].iter().rev() {
        if line.contains("ServerCapabilities") && line.contains('{') {
            let partial = prefix
                .split(|c: char| !c.is_alphanumeric() && c != '_')
                .next_back()
                .unwrap_or("")
                .to_string();
            return CompletionContext::ServerCapabilitiesField { partial };
        }
        // Stop scanning if we cross a closing brace at lower depth
        if line.trim() == "}" {
            break;
        }
    }

    // impl LanguageServer context: scan upward up to 200 lines
    for line in lines[..pos.line as usize].iter().rev().take(200) {
        if line.contains("impl LanguageServer for") {
            let partial = prefix
                .split(|c: char| !c.is_alphanumeric() && c != '_' && c != '/' && c != '$')
                .next_back()
                .unwrap_or("")
                .to_string();
            return CompletionContext::ImplLanguageServerMethod { partial };
        }
    }

    CompletionContext::Unknown
}

// ---------------------------------------------------------------------------
// Completion builders
// ---------------------------------------------------------------------------

fn complete_ls_methods(partial: &str) -> Vec<CompletionItem> {
    let domain_order = [
        Domain::Lifecycle,
        Domain::TextSync,
        Domain::Editing,
        Domain::Navigation,
        Domain::Symbols,
        Domain::Diagnostics,
        Domain::CodeLens,
        Domain::SemanticTokens,
        Domain::Workspace,
        Domain::Window,
        Domain::Max,
    ];

    let mut items = Vec::new();
    for domain in &domain_order {
        for entry in METHODS
            .iter()
            .filter(|m| m.domain == *domain)
            .filter(|m| {
                partial.is_empty()
                    || m.fn_name.starts_with(partial)
                    || m.lsp_method.starts_with(partial)
            })
        {
            let params_arg = if entry.params_type == "()" {
                "_: ()".to_string()
            } else {
                format!("params: {}", entry.params_type)
            };
            let stub_body = if entry.return_type.starts_with("Result<") {
                format!("Err(tower_lsp::jsonrpc::Error::method_not_found()) // TODO: implement {}", entry.fn_name)
            } else {
                format!("// TODO: implement {}", entry.fn_name)
            };
            let stub = format!(
                "async fn {}(&self, {}) -> {} {{\n    {}\n}}",
                entry.fn_name, params_arg, entry.return_type, stub_body
            );
            let cap_note = entry
                .capability_field
                .map(|c| format!("\n\n**Requires capability:** `ServerCapabilities::{c}`"))
                .unwrap_or_default();

            // Push the item for Rust function name
            if partial.is_empty() || entry.fn_name.starts_with(partial) {
                items.push(CompletionItem {
                    label: entry.fn_name.to_string(),
                    kind: Some(CompletionItemKind::METHOD),
                    detail: Some(format!("{} — {}", entry.lsp_method, domain_label(domain))),
                    documentation: Some(Documentation::MarkupContent(MarkupContent {
                        kind: MarkupKind::Markdown,
                        value: format!(
                            "**`{}`**\n\nParams: `{}`  \nReturns: `{}`{}",
                            entry.lsp_method, entry.params_type, entry.return_type, cap_note
                        ),
                    })),
                    insert_text: Some(stub.clone()),
                    insert_text_format: Some(InsertTextFormat::PLAIN_TEXT),
                    sort_text: Some(format!("{:02}_{}", domain_sort_key(domain), entry.fn_name)),
                    ..Default::default()
                });
            }

            // Push the item for standard LSP method name (e.g. textDocument/completion)
            if partial.is_empty() || entry.lsp_method.starts_with(partial) {
                items.push(CompletionItem {
                    label: entry.lsp_method.to_string(),
                    kind: Some(CompletionItemKind::METHOD),
                    detail: Some(format!("{} — {}", entry.fn_name, domain_label(domain))),
                    documentation: Some(Documentation::MarkupContent(MarkupContent {
                        kind: MarkupKind::Markdown,
                        value: format!(
                            "**`{}`**\n\nParams: `{}`  \nReturns: `{}`{}",
                            entry.lsp_method, entry.params_type, entry.return_type, cap_note
                        ),
                    })),
                    insert_text: Some(stub.clone()),
                    insert_text_format: Some(InsertTextFormat::PLAIN_TEXT),
                    sort_text: Some(format!("{:02}_{}", domain_sort_key(domain), entry.lsp_method)),
                    ..Default::default()
                });
            }
        }
    }
    items
}

fn complete_rpc_names(partial: &str) -> Vec<CompletionItem> {
    let mut items = Vec::new();
    for entry in METHODS {
        let method = entry.lsp_method;
        if partial.is_empty() || method.starts_with(partial) {
            items.push(CompletionItem {
                label: method.to_string(),
                kind: Some(CompletionItemKind::METHOD),
                detail: Some(format!(
                    "{} — {}",
                    entry.fn_name,
                    domain_label(&entry.domain)
                )),
                documentation: Some(Documentation::MarkupContent(MarkupContent {
                    kind: MarkupKind::Markdown,
                    value: format!(
                        "**`{}`**\n\nRust handler: `async fn {}(&self, params: {}) -> {}`",
                        entry.lsp_method,
                        entry.fn_name,
                        entry.params_type,
                        entry.return_type
                    ),
                })),
                insert_text: Some(method.to_string()),
                insert_text_format: Some(InsertTextFormat::PLAIN_TEXT),
                ..Default::default()
            });
        }
    }
    items
}

fn find_rpc_attribute_partial(prefix: &str, lines: &[&str], current_line_idx: usize) -> Option<String> {
    // 1. Try to match on the current line prefix first
    if let Some(pos) = find_rpc_in_line(prefix) {
        return Some(pos);
    }
    // 2. Otherwise, check if the current line prefix contains `name = "partial` (without `rpc` on the same line)
    // and scan upward for `#[rpc`
    if let Some(partial) = find_name_literal_in_line(prefix) {
        // scan upward up to 3 lines
        for i in (0..current_line_idx).rev().take(3) {
            if lines[i].contains("#[rpc") || lines[i].contains("rpc") {
                return Some(partial);
            }
        }
    }
    None
}

fn find_rpc_in_line(prefix: &str) -> Option<String> {
    let bytes = prefix.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i..].starts_with(b"rpc") {
            let mut j = i + 3;
            while j < bytes.len() && bytes[j].is_ascii_whitespace() {
                j += 1;
            }
            if j < bytes.len() && bytes[j] == b'(' {
                j += 1;
                while j < bytes.len() && bytes[j].is_ascii_whitespace() {
                    j += 1;
                }
                if bytes[j..].starts_with(b"name") {
                    j += 4;
                    while j < bytes.len() && bytes[j].is_ascii_whitespace() {
                        j += 1;
                    }
                    if j < bytes.len() && bytes[j] == b'=' {
                        j += 1;
                        while j < bytes.len() && bytes[j].is_ascii_whitespace() {
                            j += 1;
                        }
                        if j < bytes.len() && bytes[j] == b'"' {
                            j += 1;
                            let content_part = &prefix[j..];
                            if !content_part.contains('"') {
                                return Some(content_part.to_string());
                            }
                        }
                    }
                }
            }
        }
        i += 1;
    }
    None
}

fn find_name_literal_in_line(prefix: &str) -> Option<String> {
    let bytes = prefix.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i..].starts_with(b"name") {
            let mut j = i + 4;
            while j < bytes.len() && bytes[j].is_ascii_whitespace() {
                j += 1;
            }
            if j < bytes.len() && bytes[j] == b'=' {
                j += 1;
                while j < bytes.len() && bytes[j].is_ascii_whitespace() {
                    j += 1;
                }
                if j < bytes.len() && bytes[j] == b'"' {
                    j += 1;
                    let content_part = &prefix[j..];
                    if !content_part.contains('"') {
                        return Some(content_part.to_string());
                    }
                }
            }
        }
        i += 1;
    }
    None
}

fn complete_capability_fields(partial: &str) -> Vec<CompletionItem> {
    CAPABILITY_FIELDS
        .iter()
        .filter(|(name, _, _)| partial.is_empty() || name.starts_with(partial))
        .map(|(name, ty, methods)| {
            let methods_str = methods.join("`, `");
            CompletionItem {
                label: name.to_string(),
                kind: Some(CompletionItemKind::FIELD),
                detail: Some(ty.to_string()),
                documentation: Some(Documentation::MarkupContent(MarkupContent {
                    kind: MarkupKind::Markdown,
                    value: format!(
                        "**`ServerCapabilities::{name}`**\n\nType: `{ty}`\n\nEnables handler(s): `{methods_str}`"
                    ),
                })),
                insert_text: Some(format!("{name}: Some(${{1:...}}),\n")),
                insert_text_format: Some(InsertTextFormat::SNIPPET),
                ..Default::default()
            }
        })
        .collect()
}

fn complete_lsp_service_builder() -> Vec<CompletionItem> {
    vec![
        CompletionItem {
            label: "custom_method".to_string(),
            kind: Some(CompletionItemKind::METHOD),
            detail: Some("Add a custom JSON-RPC method to the service".to_string()),
            documentation: Some(Documentation::MarkupContent(MarkupContent {
                kind: MarkupKind::Markdown,
                value: "`.custom_method(\"namespace/method\", Backend::handler_fn)`\n\nThe handler must be `async fn(&self, params: P) -> jsonrpc::Result<R>` where `R: Serialize`.".to_string(),
            })),
            insert_text: Some("custom_method(\"${1:namespace/method}\", ${2:Backend::handler})".to_string()),
            insert_text_format: Some(InsertTextFormat::SNIPPET),
            ..Default::default()
        },
        CompletionItem {
            label: "finish".to_string(),
            kind: Some(CompletionItemKind::METHOD),
            detail: Some("Build the LspService".to_string()),
            insert_text: Some("finish()".to_string()),
            insert_text_format: Some(InsertTextFormat::PLAIN_TEXT),
            ..Default::default()
        },
    ]
}

fn complete_message_type_variants() -> Vec<CompletionItem> {
    [
        ("ERROR", "Error message — shown as error in client log"),
        ("WARNING", "Warning message"),
        ("INFO", "Informational message"),
        ("LOG", "Log/debug message"),
    ]
    .iter()
    .map(|(name, doc)| CompletionItem {
        label: name.to_string(),
        kind: Some(CompletionItemKind::ENUM_MEMBER),
        detail: Some(doc.to_string()),
        insert_text: Some(name.to_string()),
        insert_text_format: Some(InsertTextFormat::PLAIN_TEXT),
        ..Default::default()
    })
    .collect()
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn domain_label(d: &Domain) -> &'static str {
    match d {
        Domain::Lifecycle => "Lifecycle",
        Domain::TextSync => "Text Synchronization",
        Domain::Navigation => "Navigation",
        Domain::Symbols => "Symbols",
        Domain::Editing => "Editing",
        Domain::Diagnostics => "Diagnostics",
        Domain::CodeLens => "Code Lens & Links",
        Domain::SemanticTokens => "Semantic Tokens",
        Domain::Workspace => "Workspace",
        Domain::Window => "Window",
        Domain::Max => "tower-lsp-max Extensions",
    }
}

fn domain_sort_key(d: &Domain) -> u8 {
    match d {
        Domain::Lifecycle => 0,
        Domain::TextSync => 1,
        Domain::Editing => 2,
        Domain::Navigation => 3,
        Domain::Symbols => 4,
        Domain::Diagnostics => 5,
        Domain::CodeLens => 6,
        Domain::SemanticTokens => 7,
        Domain::Workspace => 8,
        Domain::Window => 9,
        Domain::Max => 10,
    }
}
