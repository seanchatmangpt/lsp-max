//! Handlers for core textDocument/* LSP request methods.

use crate::jsonrpc::{Error, Result};
use crate::{lock_registry, update_diagnostics};
use lsp_types_max::*;
use serde_json::Value;
use std::str::FromStr;
use url::Url;

/// Default: UNSUPPORTED — no materialized view backs this method; override in concrete servers.
pub async fn document_highlight(
    params: DocumentHighlightParams,
) -> Result<Option<Vec<DocumentHighlight>>> {
    let _ = params;
    Ok(None)
}

/// Default: UNSUPPORTED — no materialized view backs this method; override in concrete servers.
pub async fn document_link(params: DocumentLinkParams) -> Result<Option<Vec<DocumentLink>>> {
    let _ = params;
    Ok(None)
}

pub async fn document_link_resolve(params: DocumentLink) -> Result<DocumentLink> {
    Ok(params)
}

/// Default: UNSUPPORTED — no materialized view backs this method; override in concrete servers.
pub async fn code_lens(params: CodeLensParams) -> Result<Option<Vec<CodeLens>>> {
    let _ = params;
    Ok(None)
}

pub async fn code_lens_resolve(params: CodeLens) -> Result<CodeLens> {
    Ok(params)
}

/// Default: UNSUPPORTED — no materialized view backs this method; override in concrete servers.
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

/// Default: UNSUPPORTED — no materialized view backs this method; override in concrete servers.
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

/// Default: UNSUPPORTED — no materialized view backs this method; override in concrete servers.
pub async fn moniker(params: MonikerParams) -> Result<Option<Vec<Moniker>>> {
    let _ = params;
    Ok(None)
}

/// Default: UNSUPPORTED — no materialized view backs this method; override in concrete servers.
pub async fn completion(params: CompletionParams) -> Result<Option<CompletionResponse>> {
    let _ = params;
    Ok(None)
}

pub async fn completion_resolve(params: CompletionItem) -> Result<CompletionItem> {
    Ok(params)
}

/// Default: UNSUPPORTED — no materialized view backs this method; override in concrete servers.
pub async fn signature_help(params: SignatureHelpParams) -> Result<Option<SignatureHelp>> {
    let _ = params;
    Ok(None)
}

/// Returns repair code actions from ServerRegistry whose diagnostics intersect the requested range.
///
/// Repair plans are keyed by diagnostic ID. For each plan, if the code action's diagnostics list
/// contains at least one diagnostic whose range overlaps `params.range`, the action is included.
/// Workspace-level diagnostics (range (0,0)→(0,0)) are included for every request on any URI.
pub async fn code_action(params: CodeActionParams) -> Result<Option<CodeActionResponse>> {
    let mut registry = lock_registry()?;
    update_diagnostics(&mut registry);

    let request_range = params.range;
    let mut actions: Vec<CodeActionOrCommand> = Vec::new();

    for plans in registry.repair_plans.values() {
        for plan in plans {
            // Include the plan if any of its diagnostics overlap the requested range.
            // A diagnostic at the default (0,0)→(0,0) range is treated as workspace-wide
            // and is always included.
            let overlaps = plan.action.diagnostics.as_ref().is_none_or(|diags| {
                diags.iter().any(|d| {
                    let dr = d.range;
                    let zero = Range::default();
                    dr == zero || (dr.start <= request_range.end && dr.end >= request_range.start)
                })
            });
            if overlaps {
                actions.push(CodeActionOrCommand::CodeAction(plan.action.clone()));
            }
        }
    }

    if actions.is_empty() {
        Ok(None)
    } else {
        Ok(Some(actions))
    }
}

/// Default: UNSUPPORTED — no materialized view backs this method; override in concrete servers.
pub async fn document_color(params: DocumentColorParams) -> Result<Vec<ColorInformation>> {
    let _ = params;
    Ok(vec![])
}

/// Default: UNSUPPORTED — no materialized view backs this method; override in concrete servers.
pub async fn color_presentation(params: ColorPresentationParams) -> Result<Vec<ColorPresentation>> {
    let _ = params;
    Ok(vec![])
}

/// Default: UNSUPPORTED — no materialized view backs this method; override in concrete servers.
pub async fn rename(params: RenameParams) -> Result<Option<WorkspaceEdit>> {
    let _ = params;
    Ok(None)
}

/// Default: UNSUPPORTED — no materialized view backs this method; override in concrete servers.
pub async fn prepare_rename(
    params: TextDocumentPositionParams,
) -> Result<Option<PrepareRenameResponse>> {
    let _ = params;
    Ok(None)
}

/// Default: UNSUPPORTED — no materialized view backs this method; override in concrete servers.
pub async fn symbol(params: WorkspaceSymbolParams) -> Result<Option<Vec<SymbolInformation>>> {
    let _ = params;
    Ok(None)
}

/// Dispatches known repair command IDs through the repair transaction machinery.
///
/// Known command IDs: `repair-state-sync`, `repair-apply-security-patch`, `repair-generate-auth`.
/// Unknown command IDs return `Error::invalid_params` — never silent `None`.
pub async fn execute_command(params: ExecuteCommandParams) -> Result<Option<Value>> {
    let known = [
        "repair-state-sync",
        "repair-apply-security-patch",
        "repair-generate-auth",
    ];
    if !known.contains(&params.command.as_str()) {
        return Err(Error::invalid_params(format!(
            "Unknown command '{}'. Known commands: {}",
            params.command,
            known.join(", ")
        )));
    }

    // Find the repair plan matching this command ID and apply it.
    let matching_plan = {
        let mut registry = lock_registry()?;
        update_diagnostics(&mut registry);
        registry
            .repair_plans
            .values()
            .flat_map(|plans| plans.iter())
            .find(|plan| {
                plan.action
                    .diagnostics
                    .as_ref()
                    .and_then(|diags| diags.first())
                    .map(|_| true)
                    .unwrap_or(false)
                    && registry.diagnostics.values().any(|md| {
                        md.repair_actions
                            .iter()
                            .any(|ra| ra.action_id == params.command)
                    })
            })
            .cloned()
    };

    if let Some(plan) = matching_plan {
        let receipt = super::repair::max_apply_repair_transaction(plan).await?;
        Ok(Some(
            serde_json::to_value(receipt).map_err(|_| Error::internal_error())?,
        ))
    } else {
        // Command is known but no active repair plan matches — succeed with null.
        Ok(Some(Value::Null))
    }
}

/// Alias to goto_definition: declaration ≡ definition for languages without the distinction.
pub async fn goto_declaration(
    params: request::GotoDeclarationParams,
) -> Result<Option<request::GotoDeclarationResponse>> {
    let uri = &params.text_document_position_params.text_document.uri;
    let pos = params.text_document_position_params.position;
    let views = lsp_max_runtime::control_plane::views::get_views();
    let url = Url::parse(uri.as_str()).map_err(|_| Error::internal_error())?;
    if let Some(loc) = lsp_max_runtime::control_plane::views::lookup_definition(views, &url, pos) {
        Ok(Some(request::GotoDeclarationResponse::Scalar(loc)))
    } else {
        Ok(None)
    }
}

/// Projects from type hierarchy supertypes: TypeHierarchyItem → Location.
pub async fn goto_type_definition(
    params: request::GotoTypeDefinitionParams,
) -> Result<Option<request::GotoTypeDefinitionResponse>> {
    let uri = &params.text_document_position_params.text_document.uri;
    let pos = params.text_document_position_params.position;
    let views = lsp_max_runtime::control_plane::views::get_views();
    let url = Url::parse(uri.as_str()).map_err(|_| Error::internal_error())?;

    if let Some(items) =
        lsp_max_runtime::control_plane::views::lookup_type_hierarchy_supertypes(views, &url, pos)
    {
        let locs: Vec<Location> = items
            .into_iter()
            .filter_map(|item| {
                Uri::from_str(item.uri.as_str()).ok().map(|u| Location {
                    uri: u,
                    range: item.selection_range,
                })
            })
            .collect();
        if locs.is_empty() {
            Ok(None)
        } else {
            Ok(Some(request::GotoTypeDefinitionResponse::Array(locs)))
        }
    } else {
        Ok(None)
    }
}

/// Projects from type hierarchy subtypes: TypeHierarchyItem → Location.
pub async fn goto_implementation(
    params: request::GotoImplementationParams,
) -> Result<Option<request::GotoImplementationResponse>> {
    let uri = &params.text_document_position_params.text_document.uri;
    let pos = params.text_document_position_params.position;
    let views = lsp_max_runtime::control_plane::views::get_views();
    let url = Url::parse(uri.as_str()).map_err(|_| Error::internal_error())?;

    if let Some(items) =
        lsp_max_runtime::control_plane::views::lookup_type_hierarchy_subtypes(views, &url, pos)
    {
        let locs: Vec<Location> = items
            .into_iter()
            .filter_map(|item| {
                Uri::from_str(item.uri.as_str()).ok().map(|u| Location {
                    uri: u,
                    range: item.selection_range,
                })
            })
            .collect();
        if locs.is_empty() {
            Ok(None)
        } else {
            Ok(Some(request::GotoImplementationResponse::Array(locs)))
        }
    } else {
        Ok(None)
    }
}

/// Default: UNSUPPORTED — no materialized view backs this method; override in concrete servers.
pub async fn will_save_wait_until(
    params: WillSaveTextDocumentParams,
) -> Result<Option<Vec<TextEdit>>> {
    let _ = params;
    Ok(None)
}

/// Default: UNSUPPORTED — no materialized view backs this method; override in concrete servers.
pub async fn inline_completion(
    params: InlineCompletionParams,
) -> Result<Option<InlineCompletionResponse>> {
    let _ = params;
    Ok(None)
}
