//! Handlers for textDocument/* LSP 3.18 request methods.

use lsp_types::*;
use serde_json::Value;
use crate::jsonrpc::{Error, Result};

// ── textDocument/documentHighlight ───────────────────────────────────────────

pub async fn document_highlight(
    params: DocumentHighlightParams,
) -> Result<Option<Vec<DocumentHighlight>>> {
    let _ = params;
    Ok(None)
}

// ── textDocument/documentLink ────────────────────────────────────────────────

pub async fn document_link(
    params: DocumentLinkParams,
) -> Result<Option<Vec<DocumentLink>>> {
    let _ = params;
    Ok(None)
}

pub async fn document_link_resolve(params: DocumentLink) -> Result<DocumentLink> {
    Ok(params)
}

// ── textDocument/codeLens ────────────────────────────────────────────────────

pub async fn code_lens(params: CodeLensParams) -> Result<Option<Vec<CodeLens>>> {
    let _ = params;
    Ok(None)
}

pub async fn code_lens_resolve(params: CodeLens) -> Result<CodeLens> {
    Ok(params)
}

// ── textDocument/foldingRange ────────────────────────────────────────────────

pub async fn folding_range(
    params: FoldingRangeParams,
) -> Result<Option<Vec<FoldingRange>>> {
    let _ = params;
    Ok(None)
}

// ── textDocument/selectionRange ───────────────────────────────────────────────

pub async fn selection_range(
    params: SelectionRangeParams,
) -> Result<Option<Vec<SelectionRange>>> {
    let positions = &params.positions;
    let text_document = &params.text_document;
    let _ = text_document;
    let ranges: Vec<SelectionRange> = positions
        .iter()
        .map(|pos| SelectionRange {
            range: Range {
                start: *pos,
                end: *pos,
            },
            parent: None,
        })
        .collect();
    Ok(Some(ranges))
}

// ── textDocument/documentSymbol ───────────────────────────────────────────────

pub async fn document_symbol(
    params: DocumentSymbolParams,
) -> Result<Option<DocumentSymbolResponse>> {
    let _ = params;
    Ok(None)
}

// ── textDocument/semanticTokens/* ────────────────────────────────────────────

pub async fn semantic_tokens_full(
    params: SemanticTokensParams,
) -> Result<Option<SemanticTokensResult>> {
    let _ = params;
    Ok(Some(SemanticTokensResult::Tokens(SemanticTokens {
        result_id: None,
        data: vec![],
    })))
}

pub async fn semantic_tokens_full_delta(
    params: SemanticTokensDeltaParams,
) -> Result<Option<SemanticTokensFullDeltaResult>> {
    let _ = params;
    Ok(Some(SemanticTokensFullDeltaResult::TokensDelta(
        SemanticTokensDelta {
            result_id: None,
            edits: vec![],
        },
    )))
}

pub async fn semantic_tokens_range(
    params: SemanticTokensRangeParams,
) -> Result<Option<SemanticTokensRangeResult>> {
    let _ = params;
    Ok(Some(SemanticTokensRangeResult::Tokens(SemanticTokens {
        result_id: None,
        data: vec![],
    })))
}

// ── textDocument/inlineValue ─────────────────────────────────────────────────

pub async fn inline_value(
    params: InlineValueParams,
) -> Result<Option<Vec<InlineValue>>> {
    let _ = params;
    Ok(None)
}

// ── textDocument/inlayHint ───────────────────────────────────────────────────

pub async fn inlay_hint(params: InlayHintParams) -> Result<Option<Vec<InlayHint>>> {
    let _ = params;
    Ok(None)
}

pub async fn inlay_hint_resolve(params: InlayHint) -> Result<InlayHint> {
    Ok(params)
}

// ── textDocument/moniker ─────────────────────────────────────────────────────

pub async fn moniker(params: MonikerParams) -> Result<Option<Vec<Moniker>>> {
    let _ = params;
    Ok(None)
}

// ── textDocument/completion ───────────────────────────────────────────────────

pub async fn completion(
    params: CompletionParams,
) -> Result<Option<CompletionResponse>> {
    let _ = params;
    Ok(Some(CompletionResponse::Array(vec![])))
}

pub async fn completion_resolve(params: CompletionItem) -> Result<CompletionItem> {
    Ok(params)
}

// ── textDocument/diagnostic ───────────────────────────────────────────────────

pub async fn diagnostic(
    params: DocumentDiagnosticParams,
) -> Result<DocumentDiagnosticReportResult> {
    let _ = params;
    Ok(DocumentDiagnosticReportResult::Report(
        DocumentDiagnosticReport::Full(RelatedFullDocumentDiagnosticReport {
            related_documents: None,
            full_document_diagnostic_report: FullDocumentDiagnosticReport {
                result_id: None,
                items: vec![],
            },
        }),
    ))
}

pub async fn workspace_diagnostic(
    params: WorkspaceDiagnosticParams,
) -> Result<WorkspaceDiagnosticReportResult> {
    let _ = params;
    Ok(WorkspaceDiagnosticReportResult::Report(
        WorkspaceDiagnosticReport {
            items: vec![],
        },
    ))
}

// ── textDocument/signatureHelp ────────────────────────────────────────────────

pub async fn signature_help(
    params: SignatureHelpParams,
) -> Result<Option<SignatureHelp>> {
    let _ = params;
    Ok(None)
}

// ── textDocument/codeAction ───────────────────────────────────────────────────

pub async fn code_action(
    params: CodeActionParams,
) -> Result<Option<CodeActionResponse>> {
    let _ = params;
    Ok(Some(vec![]))
}

pub async fn code_action_resolve(params: CodeAction) -> Result<CodeAction> {
    Ok(params)
}

// ── textDocument/documentColor ────────────────────────────────────────────────

pub async fn document_color(
    params: DocumentColorParams,
) -> Result<Vec<ColorInformation>> {
    let _ = params;
    Ok(vec![])
}

pub async fn color_presentation(
    params: ColorPresentationParams,
) -> Result<Vec<ColorPresentation>> {
    let _ = params;
    Ok(vec![])
}

// ── textDocument/formatting ───────────────────────────────────────────────────

pub async fn formatting(
    params: DocumentFormattingParams,
) -> Result<Option<Vec<TextEdit>>> {
    let _ = params;
    Ok(None)
}

pub async fn range_formatting(
    params: DocumentRangeFormattingParams,
) -> Result<Option<Vec<TextEdit>>> {
    let _ = params;
    Ok(None)
}

pub async fn on_type_formatting(
    params: DocumentOnTypeFormattingParams,
) -> Result<Option<Vec<TextEdit>>> {
    let _ = params;
    Ok(None)
}

// ── textDocument/rename ───────────────────────────────────────────────────────

pub async fn rename(params: RenameParams) -> Result<Option<WorkspaceEdit>> {
    let _ = params;
    Ok(None)
}

pub async fn prepare_rename(
    params: TextDocumentPositionParams,
) -> Result<Option<PrepareRenameResponse>> {
    let _ = params;
    Ok(None)
}

// ── textDocument/linkedEditingRange ──────────────────────────────────────────

pub async fn linked_editing_range(
    params: LinkedEditingRangeParams,
) -> Result<Option<LinkedEditingRanges>> {
    let _ = params;
    Ok(None)
}

// ── textDocument/declaration / definition / implementation ────────────────────

pub async fn goto_declaration(
    params: request::GotoDeclarationParams,
) -> Result<Option<request::GotoDeclarationResponse>> {
    let _ = params;
    Ok(None)
}

pub async fn goto_type_definition(
    params: request::GotoTypeDefinitionParams,
) -> Result<Option<request::GotoTypeDefinitionResponse>> {
    let _ = params;
    Ok(None)
}

pub async fn goto_implementation(
    params: request::GotoImplementationParams,
) -> Result<Option<request::GotoImplementationResponse>> {
    let _ = params;
    Ok(None)
}

pub async fn will_save_wait_until(
    params: WillSaveTextDocumentParams,
) -> Result<Option<Vec<TextEdit>>> {
    let _ = params;
    Ok(None)
}

// ── workspace/* ──────────────────────────────────────────────────────────────

pub async fn symbol(
    params: WorkspaceSymbolParams,
) -> Result<Option<Vec<SymbolInformation>>> {
    let _ = params;
    Ok(None)
}

pub async fn symbol_resolve(params: WorkspaceSymbol) -> Result<WorkspaceSymbol> {
    Ok(params)
}

pub async fn execute_command(params: ExecuteCommandParams) -> Result<Option<Value>> {
    let _ = params;
    Ok(None)
}

pub async fn will_create_files(
    params: CreateFilesParams,
) -> Result<Option<WorkspaceEdit>> {
    let _ = params;
    Ok(None)
}

pub async fn will_rename_files(
    params: RenameFilesParams,
) -> Result<Option<WorkspaceEdit>> {
    let _ = params;
    Ok(None)
}

pub async fn will_delete_files(
    params: DeleteFilesParams,
) -> Result<Option<WorkspaceEdit>> {
    let _ = params;
    Ok(None)
}
