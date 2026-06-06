//! Definition lookup implementations.

use crate::jsonrpc::{Error, Result};
use lsp_types_max::{GotoDefinitionParams, GotoDefinitionResponse};
use url::Url;

/// Asks the server for the definition location of a symbol.
pub async fn goto_definition(
    params: GotoDefinitionParams,
) -> Result<Option<GotoDefinitionResponse>> {
    let uri = &params.text_document_position_params.text_document.uri;
    let pos = params.text_document_position_params.position;
    let views = tower_lsp_max_runtime::control_plane::views::get_views();
    let url = Url::parse(uri.as_str()).map_err(|_| Error::internal_error())?;
    if let Some(loc) =
        tower_lsp_max_runtime::control_plane::views::lookup_definition(views, &url, pos)
    {
        Ok(Some(GotoDefinitionResponse::Scalar(loc)))
    } else {
        Ok(None)
    }
}
