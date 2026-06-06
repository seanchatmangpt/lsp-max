//! Handlers for formatting and linked editing range LSP request methods.

use crate::jsonrpc::{Error, Result};
use lsp_types_max::{
    DocumentFormattingParams, DocumentOnTypeFormattingParams, DocumentRangeFormattingParams,
    LinkedEditingRangeParams, LinkedEditingRanges, TextEdit,
};
use url::Url;

/// Handles the `textDocument/formatting` request.
pub async fn formatting(params: DocumentFormattingParams) -> Result<Option<Vec<TextEdit>>> {
    let uri = &params.text_document.uri;
    let _options = params.options;

    // Following the pattern in text_document.rs but prepared for integration
    // with the runtime's materialized views once formatting lookups are implemented.
    Ok(None)
}

/// Handles the `textDocument/rangeFormatting` request.
pub async fn range_formatting(
    params: DocumentRangeFormattingParams,
) -> Result<Option<Vec<TextEdit>>> {
    let uri = &params.text_document.uri;
    let _range = params.range;
    let _options = params.options;

    // Range formatting would typically query the runtime for specific edits
    // within the provided range.
    Ok(None)
}

/// Handles the `textDocument/onTypeFormatting` request.
pub async fn on_type_formatting(
    params: DocumentOnTypeFormattingParams,
) -> Result<Option<Vec<TextEdit>>> {
    let uri = &params.text_document_position.text_document.uri;
    let _pos = params.text_document_position.position;
    let _ch = params.ch;
    let _options = params.options;

    Ok(None)
}

/// Handles the `textDocument/linkedEditingRange` request.
pub async fn linked_editing_range(
    params: LinkedEditingRangeParams,
) -> Result<Option<LinkedEditingRanges>> {
    let uri = &params.text_document_position_params.text_document.uri;
    let _pos = params.text_document_position_params.position;

    // Linked editing ranges allow for simultaneous editing of related ranges (e.g. HTML tags).
    Ok(None)
}
