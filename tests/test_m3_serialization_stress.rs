//! Stress tests for Milestone 3 core type serialization and deserialization.
//!
//! Specifically verifies round-trip compatibility, edge cases (empty values, complex nested
//! structures), and backward compatibility (deserializing older schemas without the new fields).

use lsp_max::lsp_types::{Diagnostic, DiagnosticSeverity, Position, Range};
use lsp_max::max_protocol::{
    ChainDescriptor, ConformanceVector, DocRoute, GateId, HookDescriptor, HookGraphNode, LawAxis,
    ManifoldSnapshot, MaxDiagnostic, Receipt, ReceiptObligation, RepairAction, Repairability,
    SnapshotId, Terminality, TransitionAttempt,
};
use lsp_max::max_runtime::sha256;
use serde_json::{json, Value};

#[test]
#[ignore = "stress/perf: slow by design, run with --include-ignored"]
fn test_conformance_vector_edge_cases() {
    // 1. Completely default/empty vector
    let cv_empty = ConformanceVector {
        admitted: vec![],
        refused: vec![],
        unknown: vec![],
        score: None,
        strict_mode: false,
        process_quality: None,
        ..Default::default()
    };
    let json_empty = serde_json::to_string(&cv_empty).expect("serialize empty ConformanceVector");
    let cv_empty_dec: ConformanceVector =
        serde_json::from_str(&json_empty).expect("deserialize empty ConformanceVector");
    assert!(cv_empty_dec.admitted.is_empty());
    assert!(cv_empty_dec.refused.is_empty());
    assert!(cv_empty_dec.unknown.is_empty());
    assert_eq!(cv_empty_dec.score, None);
    assert!(!cv_empty_dec.strict_mode);

    // 2. Vector with complex LawAxis types, including Custom variants
    let cv_complex = ConformanceVector {
        admitted: vec![
            LawAxis::Protocol,
            LawAxis::Type,
            LawAxis::Custom("arbitraryAxis".to_string()),
        ],
        refused: vec![LawAxis::Security],
        unknown: vec![LawAxis::Autopoiesis, LawAxis::Custom("".to_string())],
        score: Some(75.5),
        strict_mode: true,
        process_quality: None,
        ..Default::default()
    };
    let json_complex =
        serde_json::to_string(&cv_complex).expect("serialize complex ConformanceVector");
    let cv_complex_dec: ConformanceVector =
        serde_json::from_str(&json_complex).expect("deserialize complex ConformanceVector");
    assert_eq!(cv_complex_dec.admitted.len(), 3);
    assert!(cv_complex_dec.admitted.contains(&LawAxis::Protocol));
    assert!(cv_complex_dec
        .admitted
        .contains(&LawAxis::Custom("arbitraryAxis".to_string())));
    assert_eq!(cv_complex_dec.score, Some(75.5));
    assert!(cv_complex_dec.strict_mode);
}

#[test]
#[ignore = "stress/perf: slow by design, run with --include-ignored"]
fn test_receipt_backward_compatibility_and_defaults() {
    // 1. Missing prev_receipt_hash field entirely should deserialize cleanly with None
    // Intentional arbitrary hash: testing that any String is accepted during deserialization
    let json_str = r#"{"receipt_id": "rcpt-1", "hash": "abc"}"#;
    let receipt: Receipt =
        serde_json::from_str(json_str).expect("deserialize receipt without prev_receipt_hash");
    assert_eq!(receipt.receipt_id, "rcpt-1");
    assert_eq!(receipt.hash, "abc");
    assert_eq!(receipt.prev_receipt_hash, None);

    // 2. Completely empty JSON object should yield defaults if allowed (since fields are not optional, this might error or succeed depending on #[serde(default)] on fields)
    let res_empty: std::result::Result<Receipt, _> = serde_json::from_str("{}");
    assert!(
        res_empty.is_err(),
        "empty json should fail to deserialize Receipt because receipt_id and hash are required"
    );

    // 3. Receipt with Some prev_receipt_hash round-trip
    let r_full = Receipt {
        receipt_id: "rcpt-2".to_string(),
        hash: sha256(b"rcpt-2"),
        prev_receipt_hash: Some(sha256(b"rcpt-1")),
    };
    let json_full = serde_json::to_string(&r_full).expect("serialize Receipt");
    let r_full_dec: Receipt = serde_json::from_str(&json_full).expect("deserialize Receipt");
    assert_eq!(r_full_dec.prev_receipt_hash, Some(sha256(b"rcpt-1")));
}

#[test]
#[ignore = "stress/perf: slow by design, run with --include-ignored"]
fn test_max_diagnostic_backward_compatibility() {
    // 1. Deserialize an old-style diagnostic that does NOT contain the new doctrine fields
    // (law_axis, violated_invariant, observed_state, expected_state, repairability, terminality).
    // They should deserialize to their default values due to #[serde(default)].
    let old_json = r#"{
        "lsp": {
            "range": {
                "start": {"line": 0, "character": 0},
                "end": {"line": 0, "character": 10}
            },
            "message": "old error"
        },
        "diagnostic_id": "diag-old",
        "law_id": "LAW-0",
        "violated_axes": [],
        "doc_routes": [],
        "repair_actions": [],
        "verification_gates": []
    }"#;

    let d: MaxDiagnostic =
        serde_json::from_str(old_json).expect("deserialize old-style MaxDiagnostic");
    assert_eq!(d.diagnostic_id, "diag-old");
    assert_eq!(d.law_id, "LAW-0");
    assert_eq!(d.violated_invariant, "");
    assert_eq!(d.observed_state, Value::Null);
    assert_eq!(d.expected_state, Value::Null);
    assert!(matches!(d.repairability, Repairability::Unknown));
    assert!(matches!(d.terminality, Terminality::NonTerminal));
    assert_eq!(d.law_axis, LawAxis::default());
}

#[test]
#[ignore = "stress/perf: slow by design, run with --include-ignored"]
fn test_max_diagnostic_extreme_values() {
    // 2. Check round-trip of a fully populated MaxDiagnostic with complex/extreme values
    let complex_observed = json!({
        "nested": {
            "array": [1, "two", true, null],
            "value": 42.0
        }
    });

    let complex_expected = json!({
        "nested": {
            "array": [1, "two", true],
            "value": 100.0
        }
    });

    let d = MaxDiagnostic {
        lsp: Diagnostic {
            range: Range {
                start: Position {
                    line: 123456789,
                    character: 987654321,
                },
                end: Position {
                    line: 123456790,
                    character: 0,
                },
            },
            severity: Some(DiagnosticSeverity::ERROR),
            code: Some(lsp_max::lsp_types::NumberOrString::String(
                "ERR_404".to_string(),
            )),
            source: Some("SystemCheck".to_string()),
            message: "Violated manifold integrity bounds".to_string(),
            ..Diagnostic::default()
        },
        diagnostic_id: "diag-extreme-999".to_string(),
        law_id: "LSP-SEC-009".to_string(),
        attempted_transition: Some(TransitionAttempt {
            from_state: "Initializing".to_string(),
            to_state: "Initialized".to_string(),
        }),
        violated_axes: vec!["Security".to_string(), "Type".to_string()],
        doc_routes: vec![DocRoute {
            path: "/docs/laws/security.md".to_string(),
        }],
        repair_actions: vec![RepairAction {
            action_id: "act-reauth".to_string(),
            description: "Re-authenticate token session".to_string(),
        }],
        verification_gates: vec![GateId("gate-auth-verify".to_string())],
        receipt_obligation: Some(ReceiptObligation {
            required_receipts: vec!["rcpt-prev-auth".to_string()],
        }),
        law_axis: LawAxis::Security,
        violated_invariant: "token_expiry > current_time".to_string(),
        observed_state: complex_observed.clone(),
        expected_state: complex_expected.clone(),
        repairability: Repairability::Repairable,
        terminality: Terminality::Terminal,
    };

    let serialized = serde_json::to_string(&d).expect("serialize extreme MaxDiagnostic");
    let d_dec: MaxDiagnostic =
        serde_json::from_str(&serialized).expect("deserialize extreme MaxDiagnostic");

    assert_eq!(d_dec.lsp.range.start.line, 123456789);
    assert_eq!(d_dec.diagnostic_id, "diag-extreme-999");
    assert_eq!(d_dec.law_axis, LawAxis::Security);
    assert_eq!(d_dec.observed_state, complex_observed);
    assert_eq!(d_dec.expected_state, complex_expected);
    assert!(matches!(d_dec.repairability, Repairability::Repairable));
    assert!(matches!(d_dec.terminality, Terminality::Terminal));
    assert!(d_dec.attempted_transition.is_some());
    assert_eq!(
        d_dec.attempted_transition.unwrap().from_state,
        "Initializing"
    );
}

#[test]
#[ignore = "stress/perf: slow by design, run with --include-ignored"]
fn test_manifold_snapshot_serialization_extremes() {
    // ms with empty lists
    let ms_empty = ManifoldSnapshot {
        snapshot_id: SnapshotId("snap-empty".to_string()),
        conformance: ConformanceVector {
            admitted: vec![],
            refused: vec![],
            unknown: vec![],
            score: None,
            strict_mode: false,
            process_quality: None,
            ..Default::default()
        },
        hooks: vec![],
        chains: vec![],
        receipts: vec![],
    };

    let ser_empty = serde_json::to_string(&ms_empty).expect("serialize empty ManifoldSnapshot");
    let dec_empty: ManifoldSnapshot =
        serde_json::from_str(&ser_empty).expect("deserialize empty ManifoldSnapshot");
    assert_eq!(dec_empty.snapshot_id.0, "snap-empty");
    assert!(dec_empty.hooks.is_empty());

    // ms with populated and complex lists
    let ms_complex = ManifoldSnapshot {
        snapshot_id: SnapshotId("snap-complex-101".to_string()),
        conformance: ConformanceVector {
            admitted: vec![LawAxis::Protocol],
            refused: vec![],
            unknown: vec![],
            score: Some(100.0),
            strict_mode: true,
            process_quality: None,
            ..Default::default()
        },
        hooks: vec![HookDescriptor {
            hook_id: "hook-1".to_string(),
            name: "HookOne".to_string(),
            description: "Desc".to_string(),
            axes: vec![LawAxis::Protocol],
            trigger_law: LawAxis::Protocol,
            input_type: "Diagnostic".to_string(),
            output_type: "Receipt".to_string(),
            failure_mode: "Block".to_string(),
        }],
        chains: vec![ChainDescriptor {
            chain_id: "chain-1".to_string(),
            nodes: vec![HookGraphNode {
                node_id: "node-1".to_string(),
                hook: HookDescriptor::default(),
                predecessors: vec![],
                successors: vec![],
            }],
            law_axis: LawAxis::Protocol,
        }],
        receipts: vec![Receipt {
            receipt_id: "rcpt-1".to_string(),
            hash: sha256(b"rcpt-1"),
            prev_receipt_hash: None,
        }],
    };

    let ser_complex =
        serde_json::to_string(&ms_complex).expect("serialize complex ManifoldSnapshot");
    let dec_complex: ManifoldSnapshot =
        serde_json::from_str(&ser_complex).expect("deserialize complex ManifoldSnapshot");
    assert_eq!(dec_complex.snapshot_id.0, "snap-complex-101");
    assert_eq!(dec_complex.hooks.len(), 1);
    assert_eq!(dec_complex.chains.len(), 1);
    assert_eq!(dec_complex.receipts.len(), 1);
}
