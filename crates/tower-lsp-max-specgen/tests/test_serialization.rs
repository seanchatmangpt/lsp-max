// Verify serialization and deserialization of the new LSP 3.18.0 structures.

#[path = "../../../generated/lsp_minimal.rs"]
mod lsp_minimal;

#[path = "../../../generated/lsp_3_18.rs"]
mod lsp_3_18;

use serde_json::json;

// ---------------------------------------------------------------------------
// INN-10-06: serde round-trip tests for protocol types
// ---------------------------------------------------------------------------

#[test]
fn test_protocol_conformance_vector_round_trip() {
    use lsp_max_protocol::{ConformanceVector, LawAxis};
    let cv = ConformanceVector {
        admitted: vec![LawAxis::Protocol, LawAxis::Type],
        refused: vec![LawAxis::Hook],
        unknown: vec![LawAxis::Domain],
        score: Some(66.7),
        strict_mode: true,
    };
    let s = serde_json::to_string(&cv).expect("serialize ConformanceVector");
    let cv2: ConformanceVector = serde_json::from_str(&s).expect("deserialize ConformanceVector");
    assert_eq!(cv2.admitted.len(), 2);
    assert_eq!(cv2.refused.len(), 1);
    assert_eq!(cv2.unknown.len(), 1);
    assert_eq!(cv2.score, Some(66.7));
    assert!(cv2.strict_mode);
}

#[test]
fn test_protocol_admission_result_round_trip() {
    use lsp_max_protocol::{AdmissionDecision, AdmissionResult, LawAxis, Receipt};
    let ar = AdmissionResult {
        decision: AdmissionDecision::Admitted,
        law_axis: LawAxis::Protocol,
        rationale: "all checks pass".to_string(),
        receipt: Some(Receipt {
            receipt_id: "rcpt-001".to_string(),
            hash: "abc123".to_string(),
            prev_receipt_hash: None,
        }),
    };
    let s = serde_json::to_string(&ar).expect("serialize AdmissionResult");
    let ar2: AdmissionResult = serde_json::from_str(&s).expect("deserialize AdmissionResult");
    assert_eq!(ar2.decision, AdmissionDecision::Admitted);
    assert_eq!(ar2.rationale, "all checks pass");
    assert!(ar2.receipt.is_some());
}

#[test]
fn test_protocol_refusal_result_round_trip() {
    use lsp_max_protocol::{LawAxis, Receipt, RefusalResult, RepairAction};
    let rr = RefusalResult {
        law_axis: LawAxis::Security,
        rationale: "security gate failed".to_string(),
        receipt: Receipt {
            receipt_id: "rcpt-002".to_string(),
            hash: "def456".to_string(),
            prev_receipt_hash: Some("abc123".to_string()),
        },
        repair_actions: vec![RepairAction {
            action_id: "fix-security".to_string(),
            description: "Run security scan".to_string(),
        }],
    };
    let s = serde_json::to_string(&rr).expect("serialize RefusalResult");
    let rr2: RefusalResult = serde_json::from_str(&s).expect("deserialize RefusalResult");
    assert_eq!(rr2.rationale, "security gate failed");
    assert_eq!(rr2.repair_actions.len(), 1);
    assert_eq!(rr2.receipt.prev_receipt_hash, Some("abc123".to_string()));
}

#[test]
fn test_protocol_lawful_transition_result_round_trip() {
    use lsp_max_protocol::{LawfulTransitionResult, Receipt};
    let ltr = LawfulTransitionResult {
        from_phase: "Uninitialized".to_string(),
        to_phase: "Initializing".to_string(),
        lawful: true,
        violated_laws: vec![],
        receipt: Some(Receipt {
            receipt_id: "rcpt-003".to_string(),
            hash: "ghi789".to_string(),
            prev_receipt_hash: None,
        }),
    };
    let s = serde_json::to_string(&ltr).expect("serialize LawfulTransitionResult");
    let ltr2: LawfulTransitionResult =
        serde_json::from_str(&s).expect("deserialize LawfulTransitionResult");
    assert_eq!(ltr2.from_phase, "Uninitialized");
    assert_eq!(ltr2.to_phase, "Initializing");
    assert!(ltr2.lawful);
    assert!(ltr2.violated_laws.is_empty());
}

#[test]
fn test_protocol_replay_result_round_trip() {
    use lsp_max_protocol::{ConformanceVector, Receipt, ReplayResult};
    let rr = ReplayResult {
        replay_id: "replay-001".to_string(),
        events_replayed: 42,
        conformance: ConformanceVector::default(),
        receipts: vec![Receipt {
            receipt_id: "rcpt-004".to_string(),
            hash: "jkl012".to_string(),
            prev_receipt_hash: None,
        }],
    };
    let s = serde_json::to_string(&rr).expect("serialize ReplayResult");
    let rr2: ReplayResult = serde_json::from_str(&s).expect("deserialize ReplayResult");
    assert_eq!(rr2.replay_id, "replay-001");
    assert_eq!(rr2.events_replayed, 42);
    assert_eq!(rr2.receipts.len(), 1);
}

#[test]
fn test_protocol_release_actuation_result_round_trip() {
    use lsp_max_protocol::{ConformanceVector, Receipt, ReleaseActuationResult};
    let rar = ReleaseActuationResult {
        released: true,
        conformance: ConformanceVector::default(),
        blocking_axes: vec![],
        receipt: Some(Receipt {
            receipt_id: "rcpt-release".to_string(),
            hash: "mno345".to_string(),
            prev_receipt_hash: None,
        }),
    };
    let s = serde_json::to_string(&rar).expect("serialize ReleaseActuationResult");
    let rar2: ReleaseActuationResult =
        serde_json::from_str(&s).expect("deserialize ReleaseActuationResult");
    assert!(rar2.released);
    assert!(rar2.blocking_axes.is_empty());
    assert!(rar2.receipt.is_some());
}

#[test]
fn test_protocol_hook_descriptor_round_trip() {
    use lsp_max_protocol::{HookDescriptor, LawAxis};
    let hd = HookDescriptor {
        hook_id: "hook-001".to_string(),
        name: "TestHook".to_string(),
        description: "A test hook".to_string(),
        axes: vec![LawAxis::Protocol],
        trigger_law: LawAxis::Hook,
        input_type: "MaxDiagnostic".to_string(),
        output_type: "Receipt".to_string(),
        failure_mode: "Refused".to_string(),
    };
    let s = serde_json::to_string(&hd).expect("serialize HookDescriptor");
    let hd2: HookDescriptor = serde_json::from_str(&s).expect("deserialize HookDescriptor");
    assert_eq!(hd2.hook_id, "hook-001");
    assert_eq!(hd2.name, "TestHook");
    assert_eq!(hd2.failure_mode, "Refused");
}

#[test]
fn test_protocol_chain_descriptor_round_trip() {
    use lsp_max_protocol::{ChainDescriptor, HookDescriptor, HookGraphNode, LawAxis};
    let node = HookGraphNode {
        node_id: "node-001".to_string(),
        hook: HookDescriptor::default(),
        predecessors: vec![],
        successors: vec!["node-002".to_string()],
    };
    let cd = ChainDescriptor {
        chain_id: "chain-001".to_string(),
        nodes: vec![node],
        law_axis: LawAxis::Protocol,
    };
    let s = serde_json::to_string(&cd).expect("serialize ChainDescriptor");
    let cd2: ChainDescriptor = serde_json::from_str(&s).expect("deserialize ChainDescriptor");
    assert_eq!(cd2.chain_id, "chain-001");
    assert_eq!(cd2.nodes.len(), 1);
    assert_eq!(cd2.nodes[0].successors, vec!["node-002".to_string()]);
}

#[test]
fn test_protocol_propagation_result_round_trip() {
    use lsp_max_protocol::{PropagationResult, Receipt};
    let pr = PropagationResult {
        propagation_id: "prop-001".to_string(),
        affected_nodes: vec!["node-A".to_string(), "node-B".to_string()],
        receipts: vec![Receipt {
            receipt_id: "rcpt-prop".to_string(),
            hash: "pqr678".to_string(),
            prev_receipt_hash: None,
        }],
        success: true,
    };
    let s = serde_json::to_string(&pr).expect("serialize PropagationResult");
    let pr2: PropagationResult = serde_json::from_str(&s).expect("deserialize PropagationResult");
    assert_eq!(pr2.propagation_id, "prop-001");
    assert_eq!(pr2.affected_nodes.len(), 2);
    assert!(pr2.success);
}

#[test]
fn test_protocol_autonomic_loop_status_round_trip() {
    use lsp_max_protocol::{AutonomicLoopStatus, Receipt};
    let als = AutonomicLoopStatus {
        loop_id: "loop-001".to_string(),
        active: true,
        iteration_count: 99,
        last_receipt: Some(Receipt {
            receipt_id: "rcpt-loop".to_string(),
            hash: "stu901".to_string(),
            prev_receipt_hash: None,
        }),
    };
    let s = serde_json::to_string(&als).expect("serialize AutonomicLoopStatus");
    let als2: AutonomicLoopStatus =
        serde_json::from_str(&s).expect("deserialize AutonomicLoopStatus");
    assert_eq!(als2.loop_id, "loop-001");
    assert_eq!(als2.iteration_count, 99);
    assert!(als2.active);
    assert!(als2.last_receipt.is_some());
}

#[test]
fn test_protocol_manifold_snapshot_round_trip() {
    use lsp_max_protocol::{
        ChainDescriptor, ConformanceVector, HookDescriptor, ManifoldSnapshot, Receipt, SnapshotId,
    };
    let ms = ManifoldSnapshot {
        snapshot_id: SnapshotId("snap-001".to_string()),
        conformance: ConformanceVector::default(),
        hooks: vec![HookDescriptor::default()],
        chains: vec![ChainDescriptor::default()],
        receipts: vec![Receipt {
            receipt_id: "rcpt-snap".to_string(),
            hash: "vwx234".to_string(),
            prev_receipt_hash: None,
        }],
    };
    let s = serde_json::to_string(&ms).expect("serialize ManifoldSnapshot");
    let ms2: ManifoldSnapshot = serde_json::from_str(&s).expect("deserialize ManifoldSnapshot");
    assert_eq!(ms2.snapshot_id.0, "snap-001");
    assert_eq!(ms2.hooks.len(), 1);
    assert_eq!(ms2.chains.len(), 1);
    assert_eq!(ms2.receipts.len(), 1);
}

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

    // We check if it was deserialized as AnnotatedTextEdit.
    match deserialized {
        AnnotatedTextEditOrSnippetTextEditOrTextEdit::AnnotatedTextEdit(annotated) => {
            assert_eq!(annotated.text_edit_base.new_text, "hello");
            assert_eq!(annotated.annotation_id, "my-annotation-id");
        }
        _ => {
            panic!("Expected it to deserialize as AnnotatedTextEdit, but got another variant")
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

    // We expect it to deserialize as CallHierarchyRegistrationOptions since the order is fixed.
    match deserialized {
        BooleanOrCallHierarchyOptionsOrCallHierarchyRegistrationOptions::CallHierarchyRegistrationOptions(
            opts,
        ) => {
            assert!(opts
                .text_document_registration_options_base
                .document_selector
                .is_some());
        }
        _ => {
            panic!("Expected it to deserialize as CallHierarchyRegistrationOptions, but got another variant")
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

    // We expect it to deserialize as SelectionRangeRegistrationOptions since the order is fixed.
    match deserialized {
        BooleanOrSelectionRangeOptionsOrSelectionRangeRegistrationOptions::SelectionRangeRegistrationOptions(opts) => {
            assert!(opts.text_document_registration_options_base.document_selector.is_some());
        }
        _ => {
            panic!("Expected it to deserialize as SelectionRangeRegistrationOptions, but got another variant")
        }
    }
}

#[test]
fn test_3_18_validated_features() {
    use lsp_3_18::{
        Diagnostic, LogMessageParams, MarkupContent, MarkupContentOrString, MarkupKind,
        MessageType, PatternOrRelativePattern, Position, Range, RelativePattern,
        SemanticTokenTypes, TextDocumentFilterPattern, UriOrWorkspaceFolder,
    };

    // 1. Validate Diagnostic.message using MarkupContent
    let diag = Diagnostic {
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
        severity: None,
        code: None,
        code_description: None,
        source: None,
        message: MarkupContentOrString::MarkupContent(MarkupContent {
            kind: MarkupKind::Markdown,
            value: "**Bold Error**".to_string(),
        }),
        tags: None,
        related_information: None,
        data: None,
    };
    let diag_json = serde_json::to_value(&diag).expect("Failed to serialize Diagnostic");
    assert_eq!(
        diag_json.pointer("/message/kind").and_then(|v| v.as_str()),
        Some("markdown")
    );
    assert_eq!(
        diag_json.pointer("/message/value").and_then(|v| v.as_str()),
        Some("**Bold Error**")
    );
    let diag_deser: Diagnostic =
        serde_json::from_value(diag_json).expect("Failed to deserialize Diagnostic");
    assert_eq!(diag_deser, diag);

    // 2. Validate SemanticTokenTypes open-set support and LABEL constant
    assert_eq!(SemanticTokenTypes::LABEL, "label");
    let custom_token = SemanticTokenTypes("customTokenType".to_string());
    let token_json =
        serde_json::to_value(&custom_token).expect("Failed to serialize custom SemanticTokenTypes");
    assert_eq!(token_json.as_str(), Some("customTokenType"));
    let token_deser: SemanticTokenTypes =
        serde_json::from_value(token_json).expect("Failed to deserialize SemanticTokenTypes");
    assert_eq!(token_deser, custom_token);

    // 3. Validate MessageType::Debug
    let log_params = LogMessageParams {
        type_: MessageType::Debug,
        message: "debug message".to_string(),
    };
    let log_json = serde_json::to_value(&log_params).expect("Failed to serialize LogMessageParams");
    assert_eq!(log_json.pointer("/type"), Some(&serde_json::json!(5)));
    let log_deser: LogMessageParams =
        serde_json::from_value(log_json).expect("Failed to deserialize LogMessageParams");
    assert_eq!(log_deser.type_, MessageType::Debug);

    // 4. Validate Relative pattern support in GlobPattern / document selectors
    let relative = RelativePattern {
        base_uri: UriOrWorkspaceFolder::Uri("file:///base/path".to_string()),
        pattern: "src/**/*.rs".to_string(),
    };
    let filter = TextDocumentFilterPattern {
        language: None,
        scheme: None,
        pattern: PatternOrRelativePattern::RelativePattern(relative),
    };
    let filter_json =
        serde_json::to_value(&filter).expect("Failed to serialize TextDocumentFilterPattern");
    assert_eq!(
        filter_json
            .pointer("/pattern/baseUri")
            .and_then(|v| v.as_str()),
        Some("file:///base/path")
    );
    assert_eq!(
        filter_json
            .pointer("/pattern/pattern")
            .and_then(|v| v.as_str()),
        Some("src/**/*.rs")
    );
    let filter_deser: TextDocumentFilterPattern = serde_json::from_value(filter_json)
        .expect("Failed to deserialize TextDocumentFilterPattern");
    assert_eq!(filter_deser, filter);
}
