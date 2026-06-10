/// textDocument/inlineValue dispatch test.
use std::str::FromStr;
use tower_lsp_max::lsp_types as lsp;

use super::common::{assert_well_formed, roundtrip};

#[tokio::test(flavor = "current_thread")]
async fn test_inline_value_dispatch() {
    let params_typed = lsp::InlineValueParams {
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
        context: lsp::InlineValueContext {
            frame_id: 1,
            stopped_location: lsp::Range {
                start: lsp::Position {
                    line: 5,
                    character: 0,
                },
                end: lsp::Position {
                    line: 5,
                    character: 0,
                },
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
        },
        "context": {
            "frameId": 1,
            "stoppedLocation": {
                "start": { "line": 5, "character": 0 },
                "end":   { "line": 5, "character": 0 }
            }
        }
    });
    let resp = roundtrip("textDocument/inlineValue", params).await;
    assert_well_formed(&resp);
}
