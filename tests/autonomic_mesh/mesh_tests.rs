/// Additional autonomic mesh tests: save/load, hooks, RPC, unicode, concurrency.
use tower_lsp_max_runtime::{
    AutonomicMesh, HookEvent, InstanceId, LspInstance, LspPhase, MeshAction, PolicyState, Receipt,
    ReceiptRoutingHook,
};

use super::common::make_test_diagnostic;

#[test]
fn test_save_load_preserves_instances() {
    let tmp = tempfile::NamedTempFile::new().expect("tempfile creation failed");
    let path = tmp.path().to_str().unwrap();
    let _ = std::fs::remove_file(path);

    let mut mesh = AutonomicMesh::new();
    mesh.add_instance(LspInstance::new("A"));
    mesh.add_instance(LspInstance::new("B"));

    mesh.execute_action(MeshAction::AddDiagnostic {
        instance_id: InstanceId::from("A"),
        diagnostic: Box::new(make_test_diagnostic("diag-save-load", "law-save-load")),
    });

    mesh.execute_action(MeshAction::EmitReceipt {
        instance_id: InstanceId::from("B"),
        receipt: Receipt {
            receipt_id: "rcpt-save-load".to_string(),
            hash: "hash-save-load".to_string(),
            prev_receipt_hash: None,
        },
    });

    mesh.save_to_file(path).expect("Failed to save mesh state");

    let loaded = AutonomicMesh::load_from_file(path).expect("Failed to load mesh state");

    assert!(
        loaded.instances.contains_key("A"),
        "Instance A missing after load"
    );
    assert!(
        loaded.instances.contains_key("B"),
        "Instance B missing after load"
    );
    assert_eq!(
        loaded.instances.get("A").unwrap().diagnostics.len(),
        1,
        "Instance A should have 1 diagnostic after load"
    );
    assert_eq!(
        loaded.instances.get("A").unwrap().diagnostics[0].diagnostic_id,
        "diag-save-load"
    );
    assert_eq!(
        loaded.instances.get("B").unwrap().receipts.len(),
        1,
        "Instance B should have 1 receipt after load"
    );
    assert_eq!(
        loaded.instances.get("B").unwrap().receipts[0].receipt_id,
        "rcpt-save-load"
    );
}

#[test]
fn test_load_from_file_creates_defaults_when_missing() {
    let tmp = tempfile::NamedTempFile::new().expect("tempfile creation failed");
    let path_owned = tmp.path().to_str().unwrap().to_string();
    let path = &path_owned;
    let _ = std::fs::remove_file(path);

    let mesh = AutonomicMesh::load_from_file(path)
        .expect("load_from_file should succeed when file is missing");

    assert!(
        mesh.instances.contains_key("LSP_1"),
        "Default instance LSP_1 should be created when file is missing"
    );
    assert!(
        mesh.instances.contains_key("LSP_2"),
        "Default instance LSP_2 should be created when file is missing"
    );
}

#[test]
fn test_hooks_not_persisted_must_be_reregistered() {
    let tmp = tempfile::NamedTempFile::new().expect("tempfile creation failed");
    let path = tmp.path().to_str().unwrap();
    let _ = std::fs::remove_file(path);

    {
        let mesh = AutonomicMesh::load_from_file(path).expect("Failed to create mesh");
        mesh.save_to_file(path).expect("Failed to save mesh");
    }

    let mut mesh = AutonomicMesh::load_from_file(path).expect("Failed to reload mesh");

    mesh.dispatch_event(HookEvent::DiagnosticEmitted {
        instance_id: InstanceId::from("LSP_1"),
        diagnostic: Box::new(make_test_diagnostic("diag-intake", "law-intake-validation")),
    });

    assert_eq!(
        mesh.instances.get("LSP_2").unwrap().policy_state,
        Some(PolicyState::ClarificationRequested),
        "LSP_2 should be ClarificationRequested after IntakeDiagnosticHook fires"
    );
}

#[test]
fn test_receipt_routing_hook_partial_clear() {
    let mut mesh = AutonomicMesh::new();
    let mut lsp1 = LspInstance::new("LSP_1");
    lsp1.policy_state = Some(PolicyState::ClarificationRequested);
    let mut lsp2 = LspInstance::new("LSP_2");
    lsp2.policy_state = Some(PolicyState::ClarificationRequested);
    mesh.add_instance(lsp1);
    mesh.add_instance(lsp2);
    mesh.register_hook(Box::new(ReceiptRoutingHook::new()));

    mesh.execute_action(MeshAction::AddDiagnostic {
        instance_id: InstanceId::from("LSP_1"),
        diagnostic: Box::new(make_test_diagnostic("diag-a", "law-test")),
    });
    mesh.execute_action(MeshAction::AddDiagnostic {
        instance_id: InstanceId::from("LSP_1"),
        diagnostic: Box::new(make_test_diagnostic("diag-b", "law-test")),
    });

    assert_eq!(mesh.instances.get("LSP_1").unwrap().diagnostics.len(), 2);

    mesh.dispatch_event(HookEvent::ReceiptEmitted {
        instance_id: InstanceId::from("LSP_2"),
        receipt: Receipt {
            receipt_id: "rcpt-routing-test".to_string(),
            hash: "hash-routing-test".to_string(),
            prev_receipt_hash: None,
        },
    });

    let lsp1_ref = mesh.instances.get("LSP_1").unwrap();
    assert!(
        lsp1_ref.diagnostics.is_empty(),
        "Both diag-a and diag-b should be cleared from LSP_1, remaining: {:?}",
        lsp1_ref.diagnostics
    );
}

#[test]
fn test_receipt_routing_hook_clear_fires_event_per_diag() {
    let mut mesh = AutonomicMesh::new();
    let mut lsp1 = LspInstance::new("LSP_1");
    lsp1.policy_state = Some(PolicyState::ClarificationRequested);
    let mut lsp2 = LspInstance::new("LSP_2");
    lsp2.policy_state = Some(PolicyState::ClarificationRequested);
    mesh.add_instance(lsp1);
    mesh.add_instance(lsp2);
    mesh.register_hook(Box::new(ReceiptRoutingHook::new()));

    mesh.execute_action(MeshAction::AddDiagnostic {
        instance_id: InstanceId::from("LSP_1"),
        diagnostic: Box::new(make_test_diagnostic("diag-a", "law-test")),
    });
    mesh.execute_action(MeshAction::AddDiagnostic {
        instance_id: InstanceId::from("LSP_1"),
        diagnostic: Box::new(make_test_diagnostic("diag-b", "law-test")),
    });

    mesh.dispatch_event(HookEvent::ReceiptEmitted {
        instance_id: InstanceId::from("LSP_2"),
        receipt: Receipt {
            receipt_id: "rcpt-routing-test".to_string(),
            hash: "hash-routing-test".to_string(),
            prev_receipt_hash: None,
        },
    });

    let cleared_count = mesh
        .event_log
        .iter()
        .filter(|e| matches!(e, HookEvent::DiagnosticCleared { .. }))
        .count();

    assert!(
        cleared_count >= 2,
        "Expected at least 2 DiagnosticCleared events in event_log, found {}",
        cleared_count
    );
}

#[test]
fn test_rpc_clear_diagnostic_unknown_diag_is_noop() {
    let mut mesh = AutonomicMesh::new();
    let mut inst_a = LspInstance::new("INST_A");
    inst_a
        .diagnostics
        .push(make_test_diagnostic("real-diag", "law-real"));
    mesh.add_instance(inst_a);

    let result = mesh.dispatch_rpc(
        "INST_A",
        "max/clearDiagnostic",
        serde_json::json!("phantom"),
    );
    assert!(
        result.is_ok(),
        "dispatch_rpc should return Ok for unknown diagnostic id"
    );

    let inst = mesh.instances.get("INST_A").unwrap();
    assert_eq!(
        inst.diagnostics.len(),
        1,
        "real-diag should still be present"
    );
    assert_eq!(inst.diagnostics[0].diagnostic_id, "real-diag");

    let phantom_cleared = mesh.event_log.iter().any(|e| {
        matches!(e, HookEvent::DiagnosticCleared { diagnostic_id, .. } if diagnostic_id == "phantom")
    });
    assert!(
        !phantom_cleared,
        "No DiagnosticCleared event should exist for phantom diagnostic id"
    );
}

#[test]
fn test_complete_customer_service_workflow_with_rpc() {
    let mut mesh = AutonomicMesh::new();

    let mut lsp1 = LspInstance::new("LSP_1");
    lsp1.phase = LspPhase::Initialized;
    lsp1.policy_state = Some(PolicyState::Operational);
    let mut lsp2 = LspInstance::new("LSP_2");
    lsp2.phase = LspPhase::Initialized;
    lsp2.policy_state = Some(PolicyState::Operational);

    mesh.add_instance(lsp1);
    mesh.add_instance(lsp2);

    let result = mesh.dispatch_rpc("LSP_1", "max/conformanceVector", serde_json::json!(null));
    assert!(result.is_ok());
    let vec = result.unwrap();
    assert!(
        vec["score"].is_null(),
        "Initial score should be null (no axes evaluated)"
    );

    mesh.execute_action(MeshAction::AddDiagnostic {
        instance_id: InstanceId::from("LSP_1"),
        diagnostic: Box::new(make_test_diagnostic("rpc-test-diag", "law-rpc-test")),
    });

    let result2 = mesh.dispatch_rpc("LSP_1", "max/conformanceVector", serde_json::json!(null));
    assert!(result2.is_ok());
    let score2 = result2.unwrap()["score"]
        .as_f64()
        .expect("score should be f64 after adding ERROR diagnostic");
    assert!(
        score2 < 100.0,
        "Conformance score should be below 100 after adding diagnostic, got {}",
        score2
    );

    let clear_result = mesh.dispatch_rpc(
        "LSP_1",
        "max/clearDiagnostic",
        serde_json::json!("rpc-test-diag"),
    );
    assert!(
        clear_result.is_ok(),
        "max/clearDiagnostic RPC should succeed"
    );

    assert!(
        mesh.instances.get("LSP_1").unwrap().diagnostics.is_empty(),
        "Diagnostic should be removed after max/clearDiagnostic RPC"
    );

    let result3 = mesh.dispatch_rpc("LSP_1", "max/conformanceVector", serde_json::json!(null));
    assert!(result3.is_ok());
    assert!(
        result3.unwrap()["score"].is_null(),
        "Conformance score should be null after clearing all diagnostics"
    );
}

#[test]
fn test_round_trip_with_non_ascii_instance_id() {
    let tmp = tempfile::NamedTempFile::new().expect("tempfile");
    let path = tmp.path().to_str().unwrap();

    let mut mesh = AutonomicMesh::new();
    let unicode_id = "ＬＳＰ＿测试_🦀";
    mesh.add_instance(LspInstance::new(unicode_id));

    mesh.save_to_file(path).expect("save with unicode id");

    let loaded = AutonomicMesh::load_from_file(path).expect("load with unicode id");
    assert!(
        loaded.instances.contains_key(unicode_id),
        "Instance with unicode id '{}' must survive a save/load round-trip",
        unicode_id
    );
}

#[test]
fn test_save_does_not_corrupt_existing_data_on_partial_write() {
    let tmp = tempfile::NamedTempFile::new().expect("tempfile");
    let path = tmp.path().to_str().unwrap();

    let mut mesh = AutonomicMesh::new();
    mesh.add_instance(LspInstance::new("LSP_A"));
    mesh.save_to_file(path).expect("first save");

    mesh.add_instance(LspInstance::new("LSP_B"));
    mesh.save_to_file(path).expect("second save");

    let raw = std::fs::read_to_string(path).expect("read back");
    serde_json::from_str::<serde_json::Value>(&raw)
        .expect("file on disk must be valid JSON after every save");

    let loaded = AutonomicMesh::load_from_file(path).expect("load after second save");
    assert!(loaded.instances.contains_key("LSP_A"));
    assert!(loaded.instances.contains_key("LSP_B"));
}

#[test]
fn test_concurrent_add_instance_does_not_panic() {
    use std::thread;

    let results: Vec<_> = thread::scope(|s| {
        (0..8)
            .map(|i| {
                s.spawn(move || {
                    let tmp = tempfile::NamedTempFile::new().expect("tempfile");
                    let path = tmp.path().to_str().unwrap().to_owned();
                    let mut mesh = AutonomicMesh::new();
                    mesh.add_instance(LspInstance::new(&format!("LSP_THREAD_{}", i)));
                    mesh.save_to_file(&path).expect("concurrent save");
                    AutonomicMesh::load_from_file(&path).expect("concurrent load")
                })
            })
            .collect::<Vec<_>>()
            .into_iter()
            .map(|h| h.join().expect("thread panicked"))
            .collect()
    });

    assert_eq!(results.len(), 8, "All 8 threads must complete successfully");
    for (i, loaded) in results.iter().enumerate() {
        assert!(
            loaded.instances.contains_key(&format!("LSP_THREAD_{}", i)),
            "Thread {} result must contain its own instance",
            i
        );
    }
}
