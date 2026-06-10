//! Handlers for Inlay Hints and Inline Values.

use crate::jsonrpc::Result;
use lsp_types_max::{InlayHint, InlayHintParams, InlineValue, InlineValueParams};

/// Default: UNSUPPORTED — no materialized view backs this method; override in concrete servers.
pub async fn inlay_hint(params: InlayHintParams) -> Result<Option<Vec<InlayHint>>> {
    let _uri = &params.text_document.uri;
    let _range = &params.range;
    Ok(None)
}

/// Handles the `inlayHint/resolve` request.
pub async fn inlay_hint_resolve(params: InlayHint) -> Result<InlayHint> {
    Ok(params)
}

/// Default: UNSUPPORTED — no materialized view backs this method; override in concrete servers.
pub async fn inline_value(params: InlineValueParams) -> Result<Option<Vec<InlineValue>>> {
    let _uri = &params.text_document.uri;
    let _range = &params.range;
    let _context = &params.context;
    Ok(None)
}
