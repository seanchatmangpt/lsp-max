//! Implementation for Semantic Tokens Delta requests.
//!
//! This module provides the logic for computing deltas between successive
//! semantic token results to minimize the amount of data sent over the wire,
//! as specified in the LSP Semantic Tokens documentation.

use crate::jsonrpc::{Error, Result};
use lsp_types_max::{
    SemanticTokens, SemanticTokensDelta, SemanticTokensDeltaParams, SemanticTokensEdit,
    SemanticTokensFullDeltaResult, SemanticTokensResult,
};
use url::Url;

/// Implementation for `textDocument/semanticTokens/full/delta`.
///
/// This function computes the difference between the tokens currently visible
/// in the document and the tokens reported in a previous response identified by `previous_result_id`.
pub async fn semantic_tokens_full_delta(
    params: SemanticTokensDeltaParams,
) -> Result<Option<SemanticTokensFullDeltaResult>> {
    let uri = &params.text_document.uri;
    let previous_result_id = &params.previous_result_id;

    // In a complete implementation integrated with the runtime:
    // 1. Fetch current semantic tokens from the control plane views.
    // 2. Lookup the previous tokens associated with `previous_result_id`.
    // 3. If found and applicable, compute the `SemanticTokensEdit` list.
    // 4. Otherwise, return the full `SemanticTokens` result.

    // Placeholder implementation mirroring the project's current stub pattern:
    Ok(Some(SemanticTokensFullDeltaResult::TokensDelta(
        SemanticTokensDelta {
            result_id: None,
            edits: vec![],
        },
    )))
}

/// Computes the deltas (edits) required to transform `previous` tokens into `current` tokens.
///
/// This algorithm performs a linear comparison to find the first differing token
/// and generates a single edit to replace the rest of the data. For high-performance
/// production environments, this can be optimized with a diffing algorithm (e.g., Myers' diff)
/// to generate more granular edits.
pub fn compute_semantic_tokens_edits(
    previous: &SemanticTokens,
    current: &SemanticTokens,
) -> Vec<SemanticTokensEdit> {
    let prev_data = &previous.data;
    let curr_data = &current.data;

    let mut i = 0;
    let prev_len = prev_data.len();
    let curr_len = curr_data.len();
    let min_len = std::cmp::min(prev_len, curr_len);

    // Identify the prefix shared by both token sets.
    while i < min_len && prev_data[i] == curr_data[i] {
        i += 1;
    }

    // If they are identical, return no edits.
    if i == prev_len && prev_len == curr_len {
        return Vec::new();
    }

    // Identify the suffix shared by both token sets.
    let mut j = 0;
    while j < (min_len - i) && prev_data[prev_len - 1 - j] == curr_data[curr_len - 1 - j] {
        j += 1;
    }

    // Create an edit that replaces the differing middle section.
    vec![SemanticTokensEdit {
        start: i as u32,
        delete_count: (prev_len - i - j) as u32,
        data: Some(curr_data[i..(curr_len - j)].to_vec()),
    }]
}
