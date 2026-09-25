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

#[test]
fn test_3_18_selection_range() {
    use lsp_3_18::{Position, Range, SelectionRange};

    let range = SelectionRange {
        range: Range {
            start: Position {
                line: 0,
                character: 0,
            },
            end: Position {
                line: 0,
                character: 5,
            },
        },
        parent: Some(Box::new(SelectionRange {
            range: Range {
                start: Position {
                    line: 0,
                    character: 0,
                },
                end: Position {
                    line: 0,
                    character: 10,
                },
            },
            parent: None,
        })),
    };

    let serialized = serde_json::to_value(&range).expect("Failed to serialize SelectionRange");
    let deserialized: SelectionRange =
        serde_json::from_value(serialized).expect("Failed to deserialize SelectionRange");
    assert_eq!(deserialized, range);
}

#[test]
fn test_3_18_symbol_kind() {
    use lsp_3_18::SymbolKind;

    let kind = SymbolKind::Class;
    let serialized = serde_json::to_value(kind).expect("Failed to serialize SymbolKind");
    assert_eq!(serialized, serde_json::json!(5));

    let deserialized: SymbolKind =
        serde_json::from_value(serialized).expect("Failed to deserialize SymbolKind");
    assert_eq!(deserialized, kind);

    // Test out-of-range value
    let invalid_value = serde_json::json!(999);
    let deserialized_res: Result<SymbolKind, _> = serde_json::from_value(invalid_value);
    assert!(deserialized_res.is_err());
}

#[test]
fn test_3_18_map_keys() {
    use lsp_3_18::{
        DocumentDiagnosticReportPartialResult, FullDocumentDiagnosticReport,
        FullDocumentDiagnosticReportOrUnchangedDocumentDiagnosticReport,
    };

    let mut related_documents = std::collections::BTreeMap::new();
    let report = FullDocumentDiagnosticReportOrUnchangedDocumentDiagnosticReport::FullDocumentDiagnosticReport(
        FullDocumentDiagnosticReport {
            kind: "full".to_string(),
            result_id: None,
            items: vec![],
        }
    );
    related_documents.insert("file:///test.rs".to_string(), report);

    let partial = DocumentDiagnosticReportPartialResult { related_documents };

    let serialized = serde_json::to_value(&partial)
        .expect("Failed to serialize DocumentDiagnosticReportPartialResult");
    let expected = serde_json::json!({
        "relatedDocuments": {
            "file:///test.rs": {
                "kind": "full",
                "resultId": null,
                "items": []
            }
        }
    });
    assert_eq!(serialized, expected);

    let deserialized: DocumentDiagnosticReportPartialResult = serde_json::from_value(serialized)
        .expect("Failed to deserialize DocumentDiagnosticReportPartialResult");
    assert_eq!(deserialized.related_documents.len(), 1);
}

#[test]
fn test_3_18_untagged_enum_ordering_bug() {
    use lsp_3_18::AnnotatedTextEditOrSnippetTextEditOrTextEdit;

    let json_str = r#"{
        "range": {
            "start": {"line": 0, "character": 0},
            "end": {"line": 0, "character": 0}
        },
        "newText": "hello",
        "annotationId": "my-annotation-id"
    }"#;

    let deserialized: AnnotatedTextEditOrSnippetTextEditOrTextEdit =
        serde_json::from_str(json_str).expect("Failed to deserialize");

    // We check if it was deserialized as AnnotatedTextEdit or TextEdit.
    // If it deserializes as TextEdit, it means the ordering in the untagged enum
    // allows TextEdit to swallow the input and discard "annotationId".
    match deserialized {
        AnnotatedTextEditOrSnippetTextEditOrTextEdit::TextEdit(text_edit) => {
            // Asserting the current buggy behavior: it parses as TextEdit and discards annotationId
            assert_eq!(text_edit.new_text, "hello");
        }
        _ => {
            panic!("Expected it to deserialize as TextEdit due to the bug, but got another variant")
        }
    }
}

#[test]
fn test_3_18_call_hierarchy_ordering_bug() {
    use lsp_3_18::BooleanOrCallHierarchyOptionsOrCallHierarchyRegistrationOptions;

    // CallHierarchyRegistrationOptions has documentSelector, workDoneProgress, etc.
    let json_str = r#"{
        "workDoneProgress": true,
        "documentSelector": [{"language": "rust"}]
    }"#;

    let deserialized: BooleanOrCallHierarchyOptionsOrCallHierarchyRegistrationOptions =
        serde_json::from_str(json_str).expect("Failed to deserialize");

    // Due to the wrong order, it deserializes as CallHierarchyOptions (the simpler subset)
    // and ignores documentSelector.
    match deserialized {
        BooleanOrCallHierarchyOptionsOrCallHierarchyRegistrationOptions::CallHierarchyOptions(
            opts,
        ) => {
            assert!(opts
                .work_done_progress_options_mixin
                .work_done_progress
                .unwrap_or(false));
        }
        _ => {
            panic!("Expected it to deserialize as CallHierarchyOptions due to the bug, but got another variant")
        }
    }
}

#[test]
fn test_3_18_selection_range_ordering_bug() {
    use lsp_3_18::BooleanOrSelectionRangeOptionsOrSelectionRangeRegistrationOptions;

    // SelectionRangeRegistrationOptions has documentSelector, workDoneProgress, etc.
    let json_str = r#"{
        "workDoneProgress": true,
        "documentSelector": [{"language": "rust"}]
    }"#;

    let deserialized: BooleanOrSelectionRangeOptionsOrSelectionRangeRegistrationOptions =
        serde_json::from_str(json_str).expect("Failed to deserialize");

    // Due to the wrong order, it deserializes as SelectionRangeOptions (the simpler subset)
    // and ignores documentSelector.
    match deserialized {
        BooleanOrSelectionRangeOptionsOrSelectionRangeRegistrationOptions::SelectionRangeOptions(opts) => {
            assert!(opts.work_done_progress_options_mixin.work_done_progress.unwrap_or(false));
        }
        _ => {
            panic!("Expected it to deserialize as SelectionRangeOptions due to the bug, but got another variant")
        }
    }
}
