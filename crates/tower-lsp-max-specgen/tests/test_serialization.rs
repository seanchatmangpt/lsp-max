// Verify serialization and deserialization of the new LSP 3.18.0 structures.

#[path = "../../../generated/lsp_minimal.rs"]
mod lsp_minimal;

#[path = "../../../generated/lsp_3_18.rs"]
mod lsp_3_18;

use serde_json::json;

#[test]
fn test_minimal_serialization() {
    use lsp_minimal::{DocumentDiagnosticParams, TextDocumentIdentifier};

    let params = DocumentDiagnosticParams {
        text_document: TextDocumentIdentifier {
            uri: "file:///workspace/test.rs".to_string(),
        },
    };

    // Serialize to serde_json::Value
    let serialized =
        serde_json::to_value(&params).expect("Failed to serialize DocumentDiagnosticParams");
    let expected = json!({
        "textDocument": {
            "uri": "file:///workspace/test.rs"
        }
    });
    assert_eq!(serialized, expected);

    // Deserialize back to struct
    let deserialized: DocumentDiagnosticParams =
        serde_json::from_value(serialized).expect("Failed to deserialize DocumentDiagnosticParams");
    assert_eq!(deserialized, params);
}

#[test]
fn test_3_18_position_and_range() {
    use lsp_3_18::{Position, Range};

    let range = Range {
        start: Position {
            line: 10,
            character: 5,
        },
        end: Position {
            line: 11,
            character: 0,
        },
    };

    let serialized = serde_json::to_value(&range).expect("Failed to serialize Range");
    let expected = json!({
        "start": {
            "line": 10,
            "character": 5
        },
        "end": {
            "line": 11,
            "character": 0
        }
    });
    assert_eq!(serialized, expected);

    let deserialized: Range =
        serde_json::from_value(serialized).expect("Failed to deserialize Range");
    assert_eq!(deserialized, range);
}

#[test]
fn test_3_18_markup_content() {
    use lsp_3_18::{MarkupContent, MarkupKind};

    let doc = MarkupContent {
        kind: MarkupKind::Markdown,
        value: "# Title\nThis is some *markdown* text.".to_string(),
    };

    let serialized = serde_json::to_value(&doc).expect("Failed to serialize MarkupContent");
    let expected = json!({
        "kind": "markdown",
        "value": "# Title\nThis is some *markdown* text."
    });
    assert_eq!(serialized, expected);

    let deserialized: MarkupContent =
        serde_json::from_value(serialized).expect("Failed to deserialize MarkupContent");
    assert_eq!(deserialized, doc);
}

#[test]
fn test_3_18_client_info() {
    use lsp_3_18::ClientInfo;

    let client = ClientInfo {
        name: "test-client".to_string(),
        version: Some("1.2.3".to_string()),
    };

    let serialized = serde_json::to_value(&client).expect("Failed to serialize ClientInfo");
    let expected = json!({
        "name": "test-client",
        "version": "1.2.3"
    });
    assert_eq!(serialized, expected);

    let deserialized: ClientInfo =
        serde_json::from_value(serialized).expect("Failed to deserialize ClientInfo");
    assert_eq!(deserialized, client);
}

#[test]
fn test_3_18_apply_workspace_edit_result() {
    use lsp_3_18::ApplyWorkspaceEditResult;

    let res = ApplyWorkspaceEditResult {
        applied: false,
        failure_reason: Some("concurrent edits".to_string()),
        failed_change: Some(2),
    };

    let serialized =
        serde_json::to_value(&res).expect("Failed to serialize ApplyWorkspaceEditResult");
    let expected = json!({
        "applied": false,
        "failureReason": "concurrent edits",
        "failedChange": 2
    });
    assert_eq!(serialized, expected);

    let deserialized: ApplyWorkspaceEditResult =
        serde_json::from_value(serialized).expect("Failed to deserialize ApplyWorkspaceEditResult");
    assert_eq!(deserialized, res);
}
