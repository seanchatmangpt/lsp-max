/// Batch 5 — selectionRange, linkedEditingRange, semanticTokens/*,
/// inlayHint, inlayHint/resolve, textDocument/diagnostic.
use std::str::FromStr;
use tower_lsp_max::lsp_types as lsp;

use super::common::{assert_well_formed, roundtrip, td_pos};

#[tokio::test(flavor = "current_thread")]
async fn test_selection_range_dispatch() {
    let params_typed = lsp::SelectionRangeParams {
        work_done_progress_params: lsp::WorkDoneProgressParams::default(),
        partial_result_params: lsp::PartialResultParams::default(),
        text_document: lsp::TextDocumentIdentifier {
            uri: lsp::Uri::from_str("file:///test.rs").unwrap(),
        },
        positions: vec![lsp::Position {
            line: 0,
            character: 0,
        }],
    };
    let serialized = serde_json::to_value(&params_typed).unwrap();
    assert!(serialized.is_object());

    let params = serde_json::json!({
        "textDocument": { "uri": "file:///test.rs" },
        "positions": [{ "line": 0, "character": 0 }]
    });
    let resp = roundtrip("textDocument/selectionRange", params).await;
    assert_well_formed(&resp);
}

#[tokio::test(flavor = "current_thread")]
async fn test_linked_editing_range_dispatch() {
    let params_typed = lsp::LinkedEditingRangeParams {
        text_document_position_params: lsp::TextDocumentPositionParams {
            text_document: lsp::TextDocumentIdentifier {
                uri: lsp::Uri::from_str("file:///test.rs").unwrap(),
            },
            position: lsp::Position {
                line: 1,
                character: 5,
            },
        },
        work_done_progress_params: lsp::WorkDoneProgressParams::default(),
    };
    let serialized = serde_json::to_value(&params_typed).unwrap();
    assert!(serialized.is_object());

    let resp = roundtrip("textDocument/linkedEditingRange", td_pos("file:///test.rs")).await;
    assert_well_formed(&resp);
}

#[tokio::test(flavor = "current_thread")]
async fn test_semantic_tokens_full_dispatch() {
    let params_typed = lsp::SemanticTokensParams {
        work_done_progress_params: lsp::WorkDoneProgressParams::default(),
        partial_result_params: lsp::PartialResultParams::default(),
        text_document: lsp::TextDocumentIdentifier {
            uri: lsp::Uri::from_str("file:///test.rs").unwrap(),
        },
    };
    let serialized = serde_json::to_value(&params_typed).unwrap();
    assert!(serialized.is_object());

    let params = serde_json::json!({ "textDocument": { "uri": "file:///test.rs" } });
    let resp = roundtrip("textDocument/semanticTokens/full", params).await;
    assert_well_formed(&resp);
}

#[tokio::test(flavor = "current_thread")]
async fn test_semantic_tokens_full_delta_dispatch() {
    let params_typed = lsp::SemanticTokensDeltaParams {
        work_done_progress_params: lsp::WorkDoneProgressParams::default(),
        partial_result_params: lsp::PartialResultParams::default(),
        text_document: lsp::TextDocumentIdentifier {
            uri: lsp::Uri::from_str("file:///test.rs").unwrap(),
        },
        previous_result_id: "prev-result-id".to_string(),
    };
    let serialized = serde_json::to_value(&params_typed).unwrap();
    assert!(serialized.is_object());

    let params = serde_json::json!({
        "textDocument": { "uri": "file:///test.rs" },
        "previousResultId": "prev-result-id"
    });
    let resp = roundtrip("textDocument/semanticTokens/full/delta", params).await;
    assert_well_formed(&resp);
}

#[tokio::test(flavor = "current_thread")]
async fn test_semantic_tokens_range_dispatch() {
    let params_typed = lsp::SemanticTokensRangeParams {
        work_done_progress_params: lsp::WorkDoneProgressParams::default(),
        partial_result_params: lsp::PartialResultParams::default(),
        text_document: lsp::TextDocumentIdentifier {
            uri: lsp::Uri::from_str("file:///test.rs").unwrap(),
        },
        range: lsp::Range {
            start: lsp::Position {
                line: 0,
                character: 0,
            },
            end: lsp::Position {
                line: 10,
                character: 0,
            },
        },
    };
    let serialized = serde_json::to_value(&params_typed).unwrap();
    assert!(serialized.is_object());

    let params = serde_json::json!({
        "textDocument": { "uri": "file:///test.rs" },
        "range": {
            "start": { "line": 0, "character": 0 },
            "end":   { "line": 10, "character": 0 }
        }
    });
    let resp = roundtrip("textDocument/semanticTokens/range", params).await;
    assert_well_formed(&resp);
}

#[tokio::test(flavor = "current_thread")]
async fn test_inlay_hint_dispatch() {
    let params_typed = lsp::InlayHintParams {
        work_done_progress_params: lsp::WorkDoneProgressParams::default(),
        text_document: lsp::TextDocumentIdentifier {
            uri: lsp::Uri::from_str("file:///test.rs").unwrap(),
        },
        range: lsp::Range {
            start: lsp::Position {
                line: 0,
                character: 0,
            },
            end: lsp::Position {
                line: 20,
                character: 0,
            },
        },
    };
    let serialized = serde_json::to_value(&params_typed).unwrap();
    assert!(serialized.is_object());

    let params = serde_json::json!({
        "textDocument": { "uri": "file:///test.rs" },
        "range": {
            "start": { "line": 0, "character": 0 },
            "end":   { "line": 20, "character": 0 }
        }
    });
    let resp = roundtrip("textDocument/inlayHint", params).await;
    assert_well_formed(&resp);
}

#[tokio::test(flavor = "current_thread")]
async fn test_inlay_hint_resolve_dispatch() {
    let hint = lsp::InlayHint {
        position: lsp::Position {
            line: 3,
            character: 10,
        },
        label: lsp::InlayHintLabel::String("i32".to_string()),
        kind: None,
        text_edits: None,
        tooltip: None,
        padding_left: None,
        padding_right: None,
        data: None,
    };
    let serialized = serde_json::to_value(&hint).unwrap();
    assert!(serialized.is_object());

    let params = serde_json::json!({
        "position": { "line": 3, "character": 10 },
        "label": "i32"
    });
    let resp = roundtrip("inlayHint/resolve", params).await;
    assert_well_formed(&resp);
}

#[tokio::test(flavor = "current_thread")]
async fn test_document_diagnostic_dispatch() {
    let params_typed = lsp::DocumentDiagnosticParams {
        work_done_progress_params: lsp::WorkDoneProgressParams::default(),
        partial_result_params: lsp::PartialResultParams::default(),
        text_document: lsp::TextDocumentIdentifier {
            uri: lsp::Uri::from_str("file:///test.rs").unwrap(),
        },
        identifier: None,
        previous_result_id: None,
    };
    let serialized = serde_json::to_value(&params_typed).unwrap();
    assert!(serialized.is_object());

    let params = serde_json::json!({ "textDocument": { "uri": "file:///test.rs" } });
    let resp = roundtrip("textDocument/diagnostic", params).await;
    assert_well_formed(&resp);
}
