use crate::jsonrpc::{Error, Result};
use lsp_types_max::{Location, ReferenceParams};
use url::Url;

/// Asks the server for all references to a symbol.
pub async fn references(params: ReferenceParams) -> Result<Option<Vec<Location>>> {
    let uri = &params.text_document_position.text_document.uri;
    let pos = params.text_document_position.position;
    let views = tower_lsp_max_runtime::control_plane::views::get_views();
    let url = Url::parse(uri.as_str()).map_err(|_| Error::internal_error())?;

    if let Some(locs) =
        tower_lsp_max_runtime::control_plane::views::lookup_references(views, &url, pos)
    {
        Ok(Some(locs))
    } else {
        Ok(None)
    }
}
