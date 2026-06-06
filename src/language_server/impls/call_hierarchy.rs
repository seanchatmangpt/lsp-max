use crate::jsonrpc::{Error, Result};
use lsp_types_max::{
    CallHierarchyIncomingCall, CallHierarchyIncomingCallsParams, CallHierarchyItem,
    CallHierarchyOutgoingCall, CallHierarchyOutgoingCallsParams, CallHierarchyPrepareParams,
};
use url::Url;

/// Prepares the call hierarchy for a symbol.
pub async fn prepare_call_hierarchy(
    params: CallHierarchyPrepareParams,
) -> Result<Option<Vec<CallHierarchyItem>>> {
    let uri = &params.text_document_position_params.text_document.uri;
    let pos = params.text_document_position_params.position;
    let views = tower_lsp_max_runtime::control_plane::views::get_views();
    let url = Url::parse(uri.as_str()).map_err(|_| Error::internal_error())?;

    if let Some(items) =
        tower_lsp_max_runtime::control_plane::views::lookup_call_hierarchy_prepare(views, &url, pos)
    {
        Ok(Some(items))
    } else {
        Ok(None)
    }
}

/// Resolves incoming calls for a call hierarchy item.
pub async fn incoming_calls(
    params: CallHierarchyIncomingCallsParams,
) -> Result<Option<Vec<CallHierarchyIncomingCall>>> {
    let uri = &params.item.uri;
    let pos = params.item.selection_range.start;
    let views = tower_lsp_max_runtime::control_plane::views::get_views();
    let url = Url::parse(uri.as_str()).map_err(|_| Error::internal_error())?;

    if let Some(calls) = tower_lsp_max_runtime::control_plane::views::lookup_call_hierarchy_incoming(
        views, &url, pos,
    ) {
        Ok(Some(calls))
    } else {
        Ok(None)
    }
}

/// Resolves outgoing calls for a call hierarchy item.
pub async fn outgoing_calls(
    params: CallHierarchyOutgoingCallsParams,
) -> Result<Option<Vec<CallHierarchyOutgoingCall>>> {
    let uri = &params.item.uri;
    let pos = params.item.selection_range.start;
    let views = tower_lsp_max_runtime::control_plane::views::get_views();
    let url = Url::parse(uri.as_str()).map_err(|_| Error::internal_error())?;

    if let Some(calls) = tower_lsp_max_runtime::control_plane::views::lookup_call_hierarchy_outgoing(
        views, &url, pos,
    ) {
        Ok(Some(calls))
    } else {
        Ok(None)
    }
}
