use tower_lsp_max_runtime::{
    AutonomicMesh, CustomerRequestClassifierHook, Hook, HookEvent, InstanceId, LspInstance,
    LspPhase, MaxDiagnostic, MeshAction, PolicyEvaluationHook, PolicyState, Receipt,
    ReceiptRoutingHook,
};

struct CustomerServiceWorkflowHook;

impl Hook for CustomerServiceWorkflowHook {
    fn name(&self) -> &str {
        "CustomerServiceWorkflowHook"
    }

    fn trigger(&self, event: &HookEvent) -> Vec<MeshAction> {
        let mut actions = Vec::new();
        match event {
            // Step 5: Hook propagates event to LSP_2.
            HookEvent::ReceiptEmitted {
                instance_id,
                receipt,
            } => {
                if instance_id == "LSP_1" && receipt.receipt_id == "rcpt-customer-proof" {
                    actions.push(MeshAction::EmitReceipt {
                        instance_id: InstanceId::from("LSP_2"),
                        receipt: Receipt {
                            receipt_id: "rcpt-customer-proof".to_string(),
                            hash: receipt.hash.clone(),
                            prev_receipt_hash: None,
                        },
                    });
                }

                // Step 8: The receipt event triggers the hook to notify LSP_1 to clear its diagnostic and mark state as successfully transitioned.
                if instance_id == "LSP_2" && receipt.receipt_id == "rcpt-refund-executed" {
                    actions.push(MeshAction::ClearDiagnostic {
                        instance_id: InstanceId::from("LSP_1"),
                        diagnostic_id: "damaged-proof".to_string(),
                    });
                    actions.push(MeshAction::TransitionPolicyState {
                        instance_id: InstanceId::from("LSP_1"),
                        new_state: PolicyState::RefundAuthorized,
                    });
                }
            }
            HookEvent::PolicyStateChanged {
                instance_id,
                from_state,
                to_state,
            } => {
                // If LSP_1 transitioned to ClarificationRequested, propagate state to LSP_2
                if instance_id == "LSP_1" && to_state == &PolicyState::ClarificationRequested {
                    actions.push(MeshAction::TransitionPolicyState {
                        instance_id: InstanceId::from("LSP_2"),
                        new_state: PolicyState::ClarificationRequested,
                    });
                }
                // When LSP_2 transitions to RefundAuthorized, we register a cryptographic receipt
                if instance_id == "LSP_2"
                    && from_state == &PolicyState::ClarificationRequested
                    && to_state == &PolicyState::RefundAuthorized
                {
                    actions.push(MeshAction::EmitReceipt {
                        instance_id: InstanceId::from("LSP_2"),
                        receipt: Receipt {
                            receipt_id: "rcpt-refund-executed".to_string(),
                            hash: "sha256-cryptographic-proof-of-refund-executed-successfully-0xdeadbeef".to_string(),
                            prev_receipt_hash: None,
                        },
                    });
                }
            }
            _ => {}
        }
        actions
    }
}

#[test]
fn test_customer_service_autonomic_mesh_workflow() {
    // Construct an AutonomicMesh
    let mut mesh = AutonomicMesh::new();

    // Register LSP_1 (customer language server)
    let mut lsp1 = LspInstance::new("LSP_1");
    // Ensure starting phase is Uninitialized to simulate Step 1's transition
    lsp1.phase = LspPhase::Uninitialized;
    lsp1.policy_state = Some(PolicyState::Operational);

    // Register LSP_2 (policy/ops server)
    let mut lsp2 = LspInstance::new("LSP_2");
    lsp2.phase = LspPhase::Initialized;
    lsp2.policy_state = Some(PolicyState::Operational);

    mesh.add_instance(lsp1);
    mesh.add_instance(lsp2);

    // Register concrete Hooks
    mesh.register_hook(Box::new(CustomerRequestClassifierHook::new()));
    mesh.register_hook(Box::new(PolicyEvaluationHook::new()));
    mesh.register_hook(Box::new(ReceiptRoutingHook::new()));
    mesh.register_hook(Box::new(CustomerServiceWorkflowHook));

    // Cleanup target files to ensure clean start
    let receipt_file = std::path::Path::new("/Users/sac/tower-lsp-max/refund_receipt.txt");
    if receipt_file.exists() {
        std::fs::remove_file(receipt_file).expect("Failed to clean up refund receipt file");
    }

    // 1. Emit customer language state in LSP_1.
    // Transition LSP_1 from Uninitialized to Initialized.
    mesh.dispatch_event(HookEvent::StateTransition {
        instance_id: InstanceId::from("LSP_1"),
        from_phase: "Uninitialized".to_string(),
        to_phase: "Initialized".to_string(),
    });
    // This transition registers in CustomerRequestClassifierHook, which sees missing proof and transitions LSP_1 to ClarificationRequested.
    // CustomerServiceWorkflowHook propagates the ClarificationRequested state to LSP_2.
    assert_eq!(
        mesh.instances.get("LSP_1").unwrap().policy_state,
        Some(PolicyState::ClarificationRequested)
    );
    assert_eq!(
        mesh.instances.get("LSP_2").unwrap().policy_state,
        Some(PolicyState::ClarificationRequested)
    );

    // 2. Old-AI parsing fails, raising a diagnostic indicating missing state (damaged item proof).
    let parse_fail_res = mesh.run_command(
        "diagnose LSP_1 damaged-proof law-damaged-proof-check error Missing state proof: damaged item proof is missing or unreadable"
    );
    assert!(
        parse_fail_res.is_ok(),
        "Failed to run diagnose command: {:?}",
        parse_fail_res
    );

    // Verify diagnostic is present on LSP_1
    {
        let lsp1_ref = mesh.instances.get("LSP_1").unwrap();
        assert_eq!(lsp1_ref.diagnostics.len(), 1);
        assert_eq!(lsp1_ref.diagnostics[0].diagnostic_id, "damaged-proof");
    }

    // 3. Hook triggers a clarifying question.
    // PolicyState is in ClarificationRequested for both LSP_1 and LSP_2.
    assert_eq!(
        mesh.instances.get("LSP_1").unwrap().policy_state,
        Some(PolicyState::ClarificationRequested)
    );
    assert_eq!(
        mesh.instances.get("LSP_2").unwrap().policy_state,
        Some(PolicyState::ClarificationRequested)
    );

    // 4. Customer submits the proof, triggering state transition in LSP_1.
    mesh.execute_action(MeshAction::EmitReceipt {
        instance_id: InstanceId::from("LSP_1"),
        receipt: Receipt {
            receipt_id: "rcpt-customer-proof".to_string(),
            hash: "sha256-cryptographic-customer-proof-of-damaged-item-verification-hash"
                .to_string(),
            prev_receipt_hash: None,
        },
    });

    // Submitting the proof triggers transition in LSP_1 to RefundAuthorized under PolicyEvaluationHook.
    assert_eq!(
        mesh.instances.get("LSP_1").unwrap().policy_state,
        Some(PolicyState::RefundAuthorized)
    );

    // 5. Hook propagates event to LSP_2.
    // Under CustomerServiceWorkflowHook, receipt is propagated to LSP_2.
    // 6. LSP_2 evaluates the state, transitions policy state to RefundAuthorized, and executes bounded action act-create-refund-receipt.
    // PolicyEvaluationHook transitions LSP_2 to RefundAuthorized when receipt arrives, and runs bounded action.
    assert_eq!(
        mesh.instances.get("LSP_2").unwrap().policy_state,
        Some(PolicyState::RefundAuthorized)
    );
    assert!(
        mesh.executed_bounded_actions
            .contains(&"act-create-refund-receipt".to_string()),
        "Bounded action act-create-refund-receipt was not executed"
    );

    // 7. The bounded action writes the receipt file (/Users/sac/tower-lsp-max/refund_receipt.txt) and registers a cryptographic receipt.
    assert!(receipt_file.exists(), "Refund receipt file was not created");
    let receipt_content =
        std::fs::read_to_string(receipt_file).expect("Failed to read refund receipt file");
    assert!(receipt_content.contains("REFUND RECEIPT"));
    assert!(receipt_content.contains("Instance: LSP_2"));
    assert!(receipt_content.contains("Status: Executed"));

    // Check that cryptographic receipt was registered on LSP_2
    {
        let lsp2_ref = mesh.instances.get("LSP_2").unwrap();
        assert!(
            lsp2_ref
                .receipts
                .iter()
                .any(|r| r.receipt_id == "rcpt-refund-executed"),
            "Cryptographic receipt was not registered on LSP_2"
        );
    }

    // 8. The receipt event triggers the hook to notify LSP_1 to clear its diagnostic and mark state as successfully transitioned.
    // Under CustomerServiceWorkflowHook, when LSP_2 emits rcpt-refund-executed, it clears diagnostic "damaged-proof" on LSP_1 and transitions LSP_1 to RefundAuthorized.
    {
        let lsp1_ref = mesh.instances.get("LSP_1").unwrap();
        assert!(
            lsp1_ref.diagnostics.is_empty(),
            "Diagnostics on LSP_1 were not cleared: {:?}",
            lsp1_ref.diagnostics
        );
        assert_eq!(
            lsp1_ref.policy_state,
            Some(PolicyState::RefundAuthorized),
            "LSP_1 policy state not successfully transitioned to RefundAuthorized"
        );
    }
}

fn make_test_diagnostic(id: &str, law_id: &str) -> MaxDiagnostic {
    MaxDiagnostic {
        lsp: lsp_types::Diagnostic {
            range: lsp_types::Range::default(),
            severity: Some(lsp_types::DiagnosticSeverity::ERROR),
            code: None,
            code_description: None,
            source: Some("test".to_string()),
            message: format!("Test diagnostic {}", id),
            related_information: None,
            tags: None,
            data: None,
        },
        diagnostic_id: id.to_string(),
        law_id: law_id.to_string(),
        attempted_transition: None,
        violated_axes: vec![],
        doc_routes: vec![],
        repair_actions: vec![],
        verification_gates: vec![],
        receipt_obligation: None,
        ..Default::default()
    }
}

#[test]
fn test_save_load_preserves_instances() {
    let path = "/tmp/mesh_test_state_save_load.json";
    let _ = std::fs::remove_file(path);

    let mut mesh = AutonomicMesh::new();
    mesh.add_instance(LspInstance::new("A"));
    mesh.add_instance(LspInstance::new("B"));

    // Add a diagnostic to "A"
    mesh.execute_action(MeshAction::AddDiagnostic {
        instance_id: InstanceId::from("A"),
        diagnostic: Box::new(make_test_diagnostic("diag-save-load", "law-save-load")),
    });

    // Add a receipt to "B"
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
    let path = "/tmp/mesh_missing_UNIQUE_TEST.json";
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
    let path = "/tmp/mesh_hooks_reregister_test.json";
    let _ = std::fs::remove_file(path);

    // Create and save a mesh (load_from_file auto-registers IntakeDiagnosticHook)
    {
        let mesh = AutonomicMesh::load_from_file(path).expect("Failed to create mesh");
        mesh.save_to_file(path).expect("Failed to save mesh");
    }

    // Reload — load_from_file must re-register hooks
    let mut mesh = AutonomicMesh::load_from_file(path).expect("Failed to reload mesh");

    // Dispatch DiagnosticEmitted on LSP_1 with law-intake-validation
    // IntakeDiagnosticHook should transition LSP_2 to ClarificationRequested
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

    // Add two diagnostics to LSP_1 via execute_action so they are registered and tracked
    mesh.execute_action(MeshAction::AddDiagnostic {
        instance_id: InstanceId::from("LSP_1"),
        diagnostic: Box::new(make_test_diagnostic("diag-a", "law-test")),
    });
    mesh.execute_action(MeshAction::AddDiagnostic {
        instance_id: InstanceId::from("LSP_1"),
        diagnostic: Box::new(make_test_diagnostic("diag-b", "law-test")),
    });

    assert_eq!(mesh.instances.get("LSP_1").unwrap().diagnostics.len(), 2);

    // Dispatch ReceiptEmitted from LSP_2 — ReceiptRoutingHook should clear both diags on LSP_1
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

    // Conformance score should be 100.0 initially (no diagnostics)
    let result = mesh.dispatch_rpc("LSP_1", "max/conformanceVector", serde_json::json!(null));
    assert!(result.is_ok());
    let vec = result.unwrap();
    // No diagnostics → no law axes evaluated → score is null (all-unknown per doctrine)
    assert!(vec["score"].is_null(), "Initial score should be null (no axes evaluated)");

    // Add a diagnostic — score should become a number
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

    // Clear via RPC
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

    // After clearing: 0 diagnostics → no axes evaluated → score is null again
    let result3 = mesh.dispatch_rpc("LSP_1", "max/conformanceVector", serde_json::json!(null));
    assert!(result3.is_ok());
    assert!(
        result3.unwrap()["score"].is_null(),
        "Conformance score should be null after clearing all diagnostics"
    );
}
