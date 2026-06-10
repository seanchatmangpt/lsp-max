/// Customer service autonomic mesh workflow test.
use tower_lsp_max_runtime::{
    AutonomicMesh, CustomerRequestClassifierHook, FailureMode, Hook, HookDescriptor, HookEvent,
    InstanceId, LspInstance, LspPhase, MeshAction, PolicyEvaluationHook, PolicyState, Receipt,
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
                if instance_id == "LSP_1" && to_state == &PolicyState::ClarificationRequested {
                    actions.push(MeshAction::TransitionPolicyState {
                        instance_id: InstanceId::from("LSP_2"),
                        new_state: PolicyState::ClarificationRequested,
                    });
                }
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

    fn descriptor(&self) -> HookDescriptor {
        HookDescriptor {
            name: "CustomerServiceWorkflowHook",
            input_type: "HookEvent::ReceiptEmitted, HookEvent::PolicyStateChanged",
            output_type: "MeshAction::EmitReceipt, MeshAction::ClearDiagnostic, MeshAction::TransitionPolicyState",
            trigger_law: "LAW-WORKFLOW-001",
            failure_mode: FailureMode::EmitDiagnostic,
        }
    }
}

#[test]
fn test_customer_service_autonomic_mesh_workflow() {
    let mut mesh = AutonomicMesh::new();

    let mut lsp1 = LspInstance::new("LSP_1");
    lsp1.phase = LspPhase::Uninitialized;
    lsp1.policy_state = Some(PolicyState::Operational);

    let mut lsp2 = LspInstance::new("LSP_2");
    lsp2.phase = LspPhase::Initialized;
    lsp2.policy_state = Some(PolicyState::Operational);

    mesh.add_instance(lsp1);
    mesh.add_instance(lsp2);

    mesh.register_hook(Box::new(CustomerRequestClassifierHook::new()));
    mesh.register_hook(Box::new(PolicyEvaluationHook::new()));
    mesh.register_hook(Box::new(ReceiptRoutingHook::new()));
    mesh.register_hook(Box::new(CustomerServiceWorkflowHook));

    let receipt_dir = std::env::var("MESH_RECEIPT_DIR").unwrap_or_else(|_| ".".to_string());
    let receipt_file = std::path::Path::new(&receipt_dir).join("refund_receipt.txt");
    if receipt_file.exists() {
        std::fs::remove_file(&receipt_file).expect("Failed to clean up refund receipt file");
    }

    mesh.dispatch_event(HookEvent::StateTransition {
        instance_id: InstanceId::from("LSP_1"),
        from_phase: "Uninitialized".to_string(),
        to_phase: "Initialized".to_string(),
    });
    assert_eq!(
        mesh.instances.get("LSP_1").unwrap().policy_state,
        Some(PolicyState::ClarificationRequested)
    );
    assert_eq!(
        mesh.instances.get("LSP_2").unwrap().policy_state,
        Some(PolicyState::ClarificationRequested)
    );

    let parse_fail_res = mesh.run_command(
        "diagnose LSP_1 damaged-proof law-damaged-proof-check error Missing state proof: damaged item proof is missing or unreadable"
    );
    assert!(
        parse_fail_res.is_ok(),
        "Failed to run diagnose command: {:?}",
        parse_fail_res
    );

    {
        let lsp1_ref = mesh.instances.get("LSP_1").unwrap();
        assert_eq!(lsp1_ref.diagnostics.len(), 1);
        assert_eq!(lsp1_ref.diagnostics[0].diagnostic_id, "damaged-proof");
    }

    assert_eq!(
        mesh.instances.get("LSP_1").unwrap().policy_state,
        Some(PolicyState::ClarificationRequested)
    );
    assert_eq!(
        mesh.instances.get("LSP_2").unwrap().policy_state,
        Some(PolicyState::ClarificationRequested)
    );

    mesh.execute_action(MeshAction::EmitReceipt {
        instance_id: InstanceId::from("LSP_1"),
        receipt: Receipt {
            receipt_id: "rcpt-customer-proof".to_string(),
            hash: "sha256-cryptographic-customer-proof-of-damaged-item-verification-hash"
                .to_string(),
            prev_receipt_hash: None,
        },
    });

    assert_eq!(
        mesh.instances.get("LSP_1").unwrap().policy_state,
        Some(PolicyState::RefundAuthorized)
    );

    assert_eq!(
        mesh.instances.get("LSP_2").unwrap().policy_state,
        Some(PolicyState::RefundAuthorized)
    );
    assert!(
        mesh.executed_bounded_actions
            .contains(&"act-create-refund-receipt".to_string()),
        "Bounded action act-create-refund-receipt was not executed"
    );

    assert!(receipt_file.exists(), "Refund receipt file was not created");
    let receipt_content =
        std::fs::read_to_string(receipt_file).expect("Failed to read refund receipt file");
    assert!(receipt_content.contains("REFUND RECEIPT"));
    assert!(receipt_content.contains("Instance: LSP_2"));
    assert!(receipt_content.contains("Status: Executed"));

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
