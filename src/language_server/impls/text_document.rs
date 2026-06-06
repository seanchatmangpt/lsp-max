//! Handlers for core textDocument/* LSP request methods.

use crate::jsonrpc::Result;
use lsp_types_max::*;
use serde_json::Value;

pub async fn document_highlight(
    params: DocumentHighlightParams,
) -> Result<Option<Vec<DocumentHighlight>>> {
    let _ = params;
    Ok(None)
}

pub async fn document_link(params: DocumentLinkParams) -> Result<Option<Vec<DocumentLink>>> {
    let _ = params;
    Ok(None)
}

pub async fn document_link_resolve(params: DocumentLink) -> Result<DocumentLink> {
    Ok(params)
}

pub async fn code_lens(params: CodeLensParams) -> Result<Option<Vec<CodeLens>>> {
    let _ = params;
    Ok(None)
}

pub async fn code_lens_resolve(params: CodeLens) -> Result<CodeLens> {
    Ok(params)
}

pub async fn folding_range(params: FoldingRangeParams) -> Result<Option<Vec<FoldingRange>>> {
    let _ = params;
    Ok(None)
}

pub async fn selection_range(params: SelectionRangeParams) -> Result<Option<Vec<SelectionRange>>> {
    let positions = &params.positions;
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

pub async fn document_symbol(
    params: DocumentSymbolParams,
) -> Result<Option<DocumentSymbolResponse>> {
    let _ = params;
    Ok(None)
}

pub async fn semantic_tokens_full(
    params: SemanticTokensParams,
) -> Result<Option<SemanticTokensResult>> {
    let _ = params;
    Ok(Some(SemanticTokensResult::Tokens(SemanticTokens {
        result_id: None,
        data: vec![],
    })))
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

pub async fn moniker(params: MonikerParams) -> Result<Option<Vec<Moniker>>> {
    let _ = params;
    Ok(None)
}

pub async fn completion(params: CompletionParams) -> Result<Option<CompletionResponse>> {
    let _ = params;
    Ok(None)
}

pub async fn completion_resolve(params: CompletionItem) -> Result<CompletionItem> {
    Ok(params)
}

pub async fn signature_help(params: SignatureHelpParams) -> Result<Option<SignatureHelp>> {
    let _ = params;
    Ok(None)
}

pub async fn code_action(params: CodeActionParams) -> Result<Option<CodeActionResponse>> {
    let _ = params;
    Ok(None)
}

pub async fn document_color(params: DocumentColorParams) -> Result<Vec<ColorInformation>> {
    let _ = params;
    Ok(vec![])
}

pub async fn color_presentation(
    params: ColorPresentationParams,
) -> Result<Vec<ColorPresentation>> {
    let _ = params;
    Ok(vec![])
}

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

pub async fn symbol(params: WorkspaceSymbolParams) -> Result<Option<Vec<SymbolInformation>>> {
    let _ = params;
    Ok(None)
}

pub async fn execute_command(params: ExecuteCommandParams) -> Result<Option<Value>> {
    let _ = params;
    Ok(None)
}

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

pub async fn inline_completion(
    params: InlineCompletionParams,
) -> Result<Option<InlineCompletionResponse>> {
    let _ = params;
    Ok(None)
}
