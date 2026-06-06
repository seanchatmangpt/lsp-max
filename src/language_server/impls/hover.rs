use crate::jsonrpc::{Error, Result};
use lsp_types_max::{Hover, HoverParams};
use url::Url;

/// Asks the server for hover information of a symbol.
pub async fn hover(params: HoverParams) -> Result<Option<Hover>> {
    let uri = &params.text_document_position_params.text_document.uri;
    let pos = params.text_document_position_params.position;
    let views = tower_lsp_max_runtime::control_plane::views::get_views();
    let url = Url::parse(uri.as_str()).map_err(|_| Error::internal_error())?;

    if let Some(h) = tower_lsp_max_runtime::control_plane::views::lookup_hover(views, &url, pos) {
        Ok(Some(h))
    } else {
        Ok(None)
    }
}
