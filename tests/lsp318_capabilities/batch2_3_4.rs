/// Batch 2/3/4 — default-impl dispatch tests for text document methods,
/// document highlight/symbol/codeAction/codeLens/documentLink, and
/// documentColor/formatting/rename/foldingRange.
use super::common::{assert_well_formed, roundtrip, td_pos};

fn call_hierarchy_item_json(uri: &str) -> serde_json::Value {
    serde_json::json!({
        "name": "myFunc",
        "kind": 12,
        "uri": uri,
        "range": {
            "start": { "line": 0, "character": 0 },
            "end":   { "line": 0, "character": 10 }
        },
        "selectionRange": {
            "start": { "line": 0, "character": 0 },
            "end":   { "line": 0, "character": 6 }
        }
    })
}

fn type_hierarchy_item_json(uri: &str) -> serde_json::Value {
    serde_json::json!({
        "name": "MyType",
        "kind": 5,
        "uri": uri,
        "range": {
            "start": { "line": 0, "character": 0 },
            "end":   { "line": 10, "character": 1 }
        },
        "selectionRange": {
            "start": { "line": 0, "character": 0 },
            "end":   { "line": 0, "character": 6 }
        }
    })
}

// ── Batch 2 ────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "current_thread")]
async fn test_completion_item_resolve_dispatch() {
    let params = serde_json::json!({ "label": "myFunction" });
    let resp = roundtrip("completionItem/resolve", params).await;
    assert_well_formed(&resp);
}

#[tokio::test(flavor = "current_thread")]
async fn test_hover_dispatch() {
    let resp = roundtrip("textDocument/hover", td_pos("file:///test.rs")).await;
    assert_well_formed(&resp);
}

#[tokio::test(flavor = "current_thread")]
async fn test_signature_help_dispatch() {
    let params = serde_json::json!({
        "textDocument": { "uri": "file:///test.rs" },
        "position": { "line": 0, "character": 0 },
        "context": { "triggerKind": 1, "isRetrigger": false }
    });
    let resp = roundtrip("textDocument/signatureHelp", params).await;
    assert_well_formed(&resp);
}

#[tokio::test(flavor = "current_thread")]
async fn test_declaration_dispatch() {
    let resp = roundtrip("textDocument/declaration", td_pos("file:///test.rs")).await;
    assert_well_formed(&resp);
}

#[tokio::test(flavor = "current_thread")]
async fn test_definition_dispatch() {
    let resp = roundtrip("textDocument/definition", td_pos("file:///test.rs")).await;
    assert_well_formed(&resp);
}

#[tokio::test(flavor = "current_thread")]
async fn test_type_definition_dispatch() {
    let resp = roundtrip("textDocument/typeDefinition", td_pos("file:///test.rs")).await;
    assert_well_formed(&resp);
}

#[tokio::test(flavor = "current_thread")]
async fn test_implementation_dispatch() {
    let resp = roundtrip("textDocument/implementation", td_pos("file:///test.rs")).await;
    assert_well_formed(&resp);
}

#[tokio::test(flavor = "current_thread")]
async fn test_references_dispatch() {
    let params = serde_json::json!({
        "textDocument": { "uri": "file:///test.rs" },
        "position": { "line": 0, "character": 0 },
        "context": { "includeDeclaration": true }
    });
    let resp = roundtrip("textDocument/references", params).await;
    assert_well_formed(&resp);
}

// ── Batch 6 hierarchy ──────────────────────────────────────────────────────

#[tokio::test(flavor = "current_thread")]
async fn test_inline_completion_dispatch() {
    let params = serde_json::json!({
        "textDocument": { "uri": "file:///test.rs" },
        "position": { "line": 0, "character": 5 },
        "context": { "triggerKind": 1 }
    });
    let resp = roundtrip("textDocument/inlineCompletion", params).await;
    assert_well_formed(&resp);
}

#[tokio::test(flavor = "current_thread")]
async fn test_prepare_call_hierarchy_dispatch() {
    let resp = roundtrip(
        "textDocument/prepareCallHierarchy",
        td_pos("file:///test.rs"),
    )
    .await;
    assert_well_formed(&resp);
}

#[tokio::test(flavor = "current_thread")]
async fn test_incoming_calls_dispatch() {
    let params = serde_json::json!({ "item": call_hierarchy_item_json("file:///test.rs") });
    let resp = roundtrip("callHierarchy/incomingCalls", params).await;
    assert_well_formed(&resp);
}

#[tokio::test(flavor = "current_thread")]
async fn test_outgoing_calls_dispatch() {
    let params = serde_json::json!({ "item": call_hierarchy_item_json("file:///test.rs") });
    let resp = roundtrip("callHierarchy/outgoingCalls", params).await;
    assert_well_formed(&resp);
}

#[tokio::test(flavor = "current_thread")]
async fn test_prepare_type_hierarchy_dispatch() {
    let resp = roundtrip(
        "textDocument/prepareTypeHierarchy",
        td_pos("file:///test.rs"),
    )
    .await;
    assert_well_formed(&resp);
}

#[tokio::test(flavor = "current_thread")]
async fn test_type_hierarchy_supertypes_dispatch() {
    let params = serde_json::json!({ "item": type_hierarchy_item_json("file:///test.rs") });
    let resp = roundtrip("typeHierarchy/supertypes", params).await;
    assert_well_formed(&resp);
}

#[tokio::test(flavor = "current_thread")]
async fn test_type_hierarchy_subtypes_dispatch() {
    let params = serde_json::json!({ "item": type_hierarchy_item_json("file:///test.rs") });
    let resp = roundtrip("typeHierarchy/subtypes", params).await;
    assert_well_formed(&resp);
}

#[tokio::test(flavor = "current_thread")]
async fn test_moniker_dispatch() {
    let resp = roundtrip("textDocument/moniker", td_pos("file:///test.rs")).await;
    assert_well_formed(&resp);
}

// ── Batch 3 — document highlight, symbol, code action, code lens, document link

#[tokio::test(flavor = "current_thread")]
async fn test_document_highlight_dispatch() {
    let resp = roundtrip("textDocument/documentHighlight", td_pos("file:///test.rs")).await;
    assert_well_formed(&resp);
}

#[tokio::test(flavor = "current_thread")]
async fn test_document_symbol_dispatch() {
    let params = serde_json::json!({ "textDocument": { "uri": "file:///test.rs" } });
    let resp = roundtrip("textDocument/documentSymbol", params).await;
    assert_well_formed(&resp);
}

#[tokio::test(flavor = "current_thread")]
async fn test_code_action_dispatch() {
    let params = serde_json::json!({
        "textDocument": { "uri": "file:///test.rs" },
        "range": {
            "start": { "line": 0, "character": 0 },
            "end":   { "line": 0, "character": 5 }
        },
        "context": { "diagnostics": [], "triggerKind": 1 }
    });
    let resp = roundtrip("textDocument/codeAction", params).await;
    assert_well_formed(&resp);
}

#[tokio::test(flavor = "current_thread")]
async fn test_code_action_resolve_dispatch() {
    let params = serde_json::json!({ "title": "resolve_me" });
    let resp = roundtrip("codeAction/resolve", params).await;
    assert_well_formed(&resp);
}

#[tokio::test(flavor = "current_thread")]
async fn test_code_lens_dispatch() {
    let params = serde_json::json!({ "textDocument": { "uri": "file:///test.rs" } });
    let resp = roundtrip("textDocument/codeLens", params).await;
    assert_well_formed(&resp);
}

#[tokio::test(flavor = "current_thread")]
async fn test_code_lens_resolve_dispatch() {
    let params = serde_json::json!({
        "range": {
            "start": { "line": 0, "character": 0 },
            "end":   { "line": 0, "character": 5 }
        }
    });
    let resp = roundtrip("codeLens/resolve", params).await;
    assert_well_formed(&resp);
}

#[tokio::test(flavor = "current_thread")]
async fn test_document_link_dispatch() {
    let params = serde_json::json!({ "textDocument": { "uri": "file:///test.rs" } });
    let resp = roundtrip("textDocument/documentLink", params).await;
    assert_well_formed(&resp);
}

#[tokio::test(flavor = "current_thread")]
async fn test_document_link_resolve_dispatch() {
    let params = serde_json::json!({
        "range": {
            "start": { "line": 0, "character": 0 },
            "end":   { "line": 0, "character": 10 }
        },
        "target": "https://example.com/"
    });
    let resp = roundtrip("documentLink/resolve", params).await;
    assert_well_formed(&resp);
}

// ── Batch 4 — documentColor, formatting, rename, foldingRange ──────────────

#[tokio::test(flavor = "current_thread")]
async fn test_document_color_dispatch() {
    let params = serde_json::json!({ "textDocument": { "uri": "file:///test.rs" } });
    let resp = roundtrip("textDocument/documentColor", params).await;
    assert_well_formed(&resp);
}

#[tokio::test(flavor = "current_thread")]
async fn test_color_presentation_dispatch() {
    let params = serde_json::json!({
        "textDocument": { "uri": "file:///test.rs" },
        "color": { "red": 1.0, "green": 0.0, "blue": 0.0, "alpha": 1.0 },
        "range": {
            "start": { "line": 0, "character": 0 },
            "end":   { "line": 0, "character": 7 }
        }
    });
    let resp = roundtrip("textDocument/colorPresentation", params).await;
    assert_well_formed(&resp);
}

#[tokio::test(flavor = "current_thread")]
async fn test_formatting_dispatch() {
    let params = serde_json::json!({
        "textDocument": { "uri": "file:///test.rs" },
        "options": { "tabSize": 4, "insertSpaces": true }
    });
    let resp = roundtrip("textDocument/formatting", params).await;
    assert_well_formed(&resp);
}

#[tokio::test(flavor = "current_thread")]
async fn test_range_formatting_dispatch() {
    let params = serde_json::json!({
        "textDocument": { "uri": "file:///test.rs" },
        "range": {
            "start": { "line": 0, "character": 0 },
            "end":   { "line": 5, "character": 0 }
        },
        "options": { "tabSize": 4, "insertSpaces": true }
    });
    let resp = roundtrip("textDocument/rangeFormatting", params).await;
    assert_well_formed(&resp);
}

#[tokio::test(flavor = "current_thread")]
async fn test_on_type_formatting_dispatch() {
    let params = serde_json::json!({
        "textDocument": { "uri": "file:///test.rs" },
        "position": { "line": 3, "character": 1 },
        "ch": ";",
        "options": { "tabSize": 4, "insertSpaces": true }
    });
    let resp = roundtrip("textDocument/onTypeFormatting", params).await;
    assert_well_formed(&resp);
}

#[tokio::test(flavor = "current_thread")]
async fn test_rename_dispatch() {
    let params = serde_json::json!({
        "textDocument": { "uri": "file:///test.rs" },
        "position": { "line": 1, "character": 5 },
        "newName": "newSymbol"
    });
    let resp = roundtrip("textDocument/rename", params).await;
    assert_well_formed(&resp);
}

#[tokio::test(flavor = "current_thread")]
async fn test_prepare_rename_dispatch() {
    let resp = roundtrip("textDocument/prepareRename", td_pos("file:///test.rs")).await;
    assert_well_formed(&resp);
}

#[tokio::test(flavor = "current_thread")]
async fn test_folding_range_dispatch() {
    let params = serde_json::json!({ "textDocument": { "uri": "file:///test.rs" } });
    let resp = roundtrip("textDocument/foldingRange", params).await;
    assert_well_formed(&resp);
}
