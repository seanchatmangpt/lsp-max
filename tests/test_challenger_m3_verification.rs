use lsp_max_protocol::custom_methods::*;
use lsp_max_protocol::{
    ChainDescriptor, ConformanceVector, HookDescriptor, HookGraphNode, LawAxis, ManifoldSnapshot,
    MaxDiagnostic, Receipt, SnapshotId,
};
use lsp_max_runtime::MaxMethod;
use lsp_types::request::Request;
use lsp_types_max as lsp_types;

#[test]
fn test_serialization_conformance_vector() {
    // 1. Default ConformanceVector
    let default_cv = ConformanceVector::default();
    let serialized = serde_json::to_string(&default_cv).unwrap();
    let deserialized: ConformanceVector = serde_json::from_str(&serialized).unwrap();
    assert_eq!(default_cv.admitted.len(), deserialized.admitted.len());
    assert_eq!(default_cv.refused.len(), deserialized.refused.len());
    assert_eq!(default_cv.unknown.len(), deserialized.unknown.len());
    assert_eq!(default_cv.score, deserialized.score);
    assert_eq!(default_cv.strict_mode, deserialized.strict_mode);

    // 2. Full/Custom ConformanceVector with different LawAxis variants
    let custom_cv = ConformanceVector {
        admitted: vec![
            LawAxis::Protocol,
            LawAxis::Type,
            LawAxis::Custom("custom_axis".to_string()),
        ],
        refused: vec![LawAxis::Security, LawAxis::Release],
        unknown: vec![LawAxis::Domain],
        score: Some(50.0),
        strict_mode: false,
    };
    let serialized = serde_json::to_string(&custom_cv).unwrap();
    let deserialized: ConformanceVector = serde_json::from_str(&serialized).unwrap();
    assert_eq!(custom_cv.admitted, deserialized.admitted);
    assert_eq!(custom_cv.refused, deserialized.refused);
    assert_eq!(custom_cv.unknown, deserialized.unknown);
    assert_eq!(custom_cv.score, deserialized.score);
    assert_eq!(custom_cv.strict_mode, deserialized.strict_mode);
}

#[test]
fn test_serialization_receipt() {
    // 1. Default Receipt
    let default_receipt = Receipt::default();
    let serialized = serde_json::to_string(&default_receipt).unwrap();
    let deserialized: Receipt = serde_json::from_str(&serialized).unwrap();
    assert_eq!(default_receipt.receipt_id, deserialized.receipt_id);
    assert_eq!(default_receipt.hash, deserialized.hash);
    assert_eq!(
        default_receipt.prev_receipt_hash,
        deserialized.prev_receipt_hash
    );

    // 2. Custom Receipt
    let custom_receipt = Receipt {
        receipt_id: "rec_123".to_string(),
        hash: "hash_xyz".to_string(),
        prev_receipt_hash: Some("prev_hash_123".to_string()),
    };
    let serialized = serde_json::to_string(&custom_receipt).unwrap();
    let deserialized: Receipt = serde_json::from_str(&serialized).unwrap();
    assert_eq!(custom_receipt.receipt_id, deserialized.receipt_id);
    assert_eq!(custom_receipt.hash, deserialized.hash);
    assert_eq!(
        custom_receipt.prev_receipt_hash,
        deserialized.prev_receipt_hash
    );
}

#[test]
fn test_serialization_max_diagnostic() {
    // 1. Default MaxDiagnostic
    let default_diag = MaxDiagnostic::default();
    let serialized = serde_json::to_string(&default_diag).unwrap();
    let deserialized: MaxDiagnostic = serde_json::from_str(&serialized).unwrap();
    assert_eq!(default_diag.diagnostic_id, deserialized.diagnostic_id);
    assert_eq!(default_diag.law_id, deserialized.law_id);

    // 2. Custom MaxDiagnostic
    let custom_diag = MaxDiagnostic {
        lsp: lsp_types::Diagnostic {
            range: lsp_types::Range {
                start: lsp_types::Position {
                    line: 1,
                    character: 2,
                },
                end: lsp_types::Position {
                    line: 3,
                    character: 4,
                },
            },
            severity: Some(lsp_types::DiagnosticSeverity::ERROR),
            code: Some(lsp_types::NumberOrString::String("E001".to_string())),
            code_description: None,
            source: Some("lsp-max".to_string()),
            message: "Test violation".to_string(),
            related_information: None,
            tags: None,
            data: None,
        },
        diagnostic_id: "diag_123".to_string(),
        law_id: "law_axis_violation".to_string(),
        attempted_transition: None,
        violated_axes: vec!["Protocol".to_string()],
        doc_routes: vec![],
        repair_actions: vec![],
        verification_gates: vec![],
        receipt_obligation: None,
        law_axis: LawAxis::Protocol,
        violated_invariant: "invariant_x".to_string(),
        observed_state: serde_json::json!({"state": "bad"}),
        expected_state: serde_json::json!({"state": "good"}),
        repairability: lsp_max_protocol::Repairability::Repairable,
        terminality: lsp_max_protocol::Terminality::Terminal,
    };
    let serialized = serde_json::to_string(&custom_diag).unwrap();
    let deserialized: MaxDiagnostic = serde_json::from_str(&serialized).unwrap();
    assert_eq!(custom_diag.diagnostic_id, deserialized.diagnostic_id);
    assert_eq!(custom_diag.law_id, deserialized.law_id);
    assert_eq!(
        custom_diag.violated_invariant,
        deserialized.violated_invariant
    );
    assert_eq!(custom_diag.observed_state, deserialized.observed_state);
    assert_eq!(custom_diag.expected_state, deserialized.expected_state);
}

#[test]
fn test_serialization_manifold_snapshot() {
    // 1. Default ManifoldSnapshot
    let default_snapshot = ManifoldSnapshot::default();
    let serialized = serde_json::to_string(&default_snapshot).unwrap();
    let deserialized: ManifoldSnapshot = serde_json::from_str(&serialized).unwrap();
    assert_eq!(default_snapshot.snapshot_id.0, deserialized.snapshot_id.0);
    assert_eq!(
        default_snapshot.conformance.strict_mode,
        deserialized.conformance.strict_mode
    );

    // 2. Custom ManifoldSnapshot
    let custom_snapshot = ManifoldSnapshot {
        snapshot_id: SnapshotId("snap_456".to_string()),
        conformance: ConformanceVector {
            admitted: vec![LawAxis::Type],
            refused: vec![],
            unknown: vec![],
            score: Some(100.0),
            strict_mode: true,
        },
        hooks: vec![HookDescriptor {
            hook_id: "hook_1".to_string(),
            name: "test_hook".to_string(),
            description: "test hook desc".to_string(),
            axes: vec![LawAxis::Hook],
            trigger_law: LawAxis::Hook,
            input_type: "input".to_string(),
            output_type: "output".to_string(),
            failure_mode: "fail".to_string(),
        }],
        chains: vec![ChainDescriptor {
            chain_id: "chain_1".to_string(),
            nodes: vec![HookGraphNode {
                node_id: "node_1".to_string(),
                hook: HookDescriptor::default(),
                predecessors: vec![],
                successors: vec![],
            }],
            law_axis: LawAxis::Hook,
        }],
        receipts: vec![Receipt {
            receipt_id: "rec_1".to_string(),
            hash: "hash_1".to_string(),
            prev_receipt_hash: None,
        }],
    };
    let serialized = serde_json::to_string(&custom_snapshot).unwrap();
    let deserialized: ManifoldSnapshot = serde_json::from_str(&serialized).unwrap();
    assert_eq!(custom_snapshot.snapshot_id.0, deserialized.snapshot_id.0);
    assert_eq!(custom_snapshot.hooks.len(), deserialized.hooks.len());
    assert_eq!(custom_snapshot.chains.len(), deserialized.chains.len());
    assert_eq!(custom_snapshot.receipts.len(), deserialized.receipts.len());
}

#[test]
fn test_custom_lsp_request_method_mappings() {
    // Assert that the constants exported match the Request::METHOD on the structs
    assert_eq!(MaxHook::METHOD, METHOD_HOOK);
    assert_eq!(MaxHookGraph::METHOD, METHOD_HOOK_GRAPH);
    assert_eq!(MaxChain::METHOD, METHOD_CHAIN);
    assert_eq!(MaxPropagate::METHOD, METHOD_PROPAGATE);
    assert_eq!(MaxAutonomicLoop::METHOD, METHOD_AUTONOMIC_LOOP);
    assert_eq!(MaxManifoldSnapshot::METHOD, METHOD_MANIFOLD_SNAPSHOT);
    assert_eq!(MaxLawfulTransition::METHOD, METHOD_LAWFUL_TRANSITION);
    assert_eq!(MaxAdmission::METHOD, METHOD_ADMISSION);
    assert_eq!(MaxRefusal::METHOD, METHOD_REFUSAL);
    assert_eq!(MaxReplay::METHOD, METHOD_REPLAY);
    assert_eq!(MaxReleaseActuation::METHOD, METHOD_RELEASE_ACTUATION);

    // Other core custom requests from prior milestones
    assert_eq!(MaxSnapshot::METHOD, "max/snapshot");
    assert_eq!(MaxConformanceVector::METHOD, "max/conformanceVector");
    assert_eq!(MaxExplainDiagnostic::METHOD, "max/explainDiagnostic");
    assert_eq!(MaxRepairPlan::METHOD, "max/repairPlan");
    assert_eq!(
        MaxApplyRepairTransaction::METHOD,
        "max/applyRepairTransaction"
    );
    assert_eq!(MaxExportAnalysisBundle::METHOD, "max/exportAnalysisBundle");
    assert_eq!(MaxRunGate::METHOD, "max/runGate");
    assert_eq!(MaxClearDiagnostic::METHOD, "max/clearDiagnostic");
    assert_eq!(MaxReceipt::METHOD, "max/receipt");
}

#[test]
fn test_runtime_max_method_routing() {
    // Test parsing and formatting of method strings
    let methods = vec![
        (MaxMethod::Snapshot, "max/snapshot"),
        (MaxMethod::ConformanceVector, "max/conformanceVector"),
        (MaxMethod::ClearDiagnostic, "max/clearDiagnostic"),
        (MaxMethod::ExplainDiagnostic, "max/explainDiagnostic"),
        (MaxMethod::RepairPlan, "max/repairPlan"),
        (
            MaxMethod::ApplyRepairTransaction,
            "max/applyRepairTransaction",
        ),
        (MaxMethod::ExportAnalysisBundle, "max/exportAnalysisBundle"),
        (MaxMethod::RunGate, "max/runGate"),
        (MaxMethod::Receipt, "max/receipt"),
        (MaxMethod::Hook, "max/hook"),
        (MaxMethod::HookGraph, "max/hookGraph"),
        (MaxMethod::Chain, "max/chain"),
        (MaxMethod::Propagate, "max/propagate"),
        (MaxMethod::AutonomicLoop, "max/autonomicLoop"),
        (MaxMethod::ManifoldSnapshot, "max/manifoldSnapshot"),
        (MaxMethod::LawfulTransition, "max/lawfulTransition"),
        (MaxMethod::Admission, "max/admission"),
        (MaxMethod::Refusal, "max/refusal"),
        (MaxMethod::Replay, "max/replay"),
        (MaxMethod::ReleaseActuation, "max/releaseActuation"),
        (MaxMethod::InstanceList, "max/instanceList"),
        (MaxMethod::DumpState, "max/dumpState"),
        (MaxMethod::RestoreState, "max/restoreState"),
        (MaxMethod::Reset, "max/reset"),
        (MaxMethod::ConformanceDelta, "max/conformanceDelta"),
        (MaxMethod::VerifyLedger, "max/verifyLedger"),
        (MaxMethod::LedgerReport, "max/ledgerReport"),
    ];

    for (variant, method_str) in methods {
        assert_eq!(variant.as_str(), method_str);
        assert_eq!(MaxMethod::try_from(method_str), Ok(variant));
    }

    assert!(MaxMethod::try_from("invalid/method").is_err());
}
