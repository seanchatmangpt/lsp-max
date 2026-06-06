use crate::ServerRegistry;
use lsp_types_max::*;
use std::collections::HashMap;
use std::str::FromStr;

fn get_receipt_uri(root_path: &std::path::Path, filename: &str) -> lsp_types_max::Uri {
    match url::Url::from_file_path(root_path.join(filename)) {
        Ok(url) => match lsp_types_max::Uri::from_str(url.as_str()) {
            Ok(uri) => uri,
            Err(_) => match lsp_types_max::Uri::from_str("file:///") {
                Ok(uri) => uri,
                Err(_) => loop {},
            },
        },
        Err(_) => match lsp_types_max::Uri::from_str("file:///") {
            Ok(uri) => uri,
            Err(_) => loop {},
        },
    }
}

pub(crate) fn update_diagnostics(registry: &mut ServerRegistry) {
    let root_path = registry.root_path.clone();

    // 1. Check for diag-uninitialized-admission
    let diag1_id = "diag-uninitialized-admission".to_string();
    let gate_state_check_active = registry.gates.get("gate-state-check") == Some(&true)
        || registry.current_state != crate::service::State::Uninitialized;
    let diag1_cleared = registry.cleared_diagnostics.contains(&diag1_id);

    if !gate_state_check_active && !diag1_cleared {
        let diag1 = max_protocol::MaxDiagnostic {
            lsp: Diagnostic {
                range: Range::default(),
                severity: Some(DiagnosticSeverity::ERROR),
                code: None,
                code_description: None,
                source: Some("tower-lsp-max".to_string()),
                message: "Server state violates lifecycle machine match: initialize must transition to InitializingState.".to_string(),
                related_information: None,
                tags: None,
                data: None,
            },
            diagnostic_id: diag1_id.clone(),
            law_id: "LAW-001".to_string(),
            attempted_transition: None,
            violated_axes: vec!["LSP State Mapping".to_string()],
            doc_routes: vec![max_protocol::DocRoute { path: "/doc/lifecycle".to_string() }],
            repair_actions: vec![max_protocol::RepairAction {
                action_id: "repair-state-sync".to_string(),
                description: "Synchronize machine state with semantic state".to_string(),
            }],
            verification_gates: vec![max_protocol::GateId("gate-state-check".to_string())],
            receipt_obligation: None,
            law_axis: max_protocol::LawAxis::default(),
            violated_invariant: String::new(),
            observed_state: serde_json::Value::Null,
            expected_state: serde_json::Value::Null,
            repairability: max_protocol::Repairability::default(),
            terminality: max_protocol::Terminality::default(),
        };
        registry.diagnostics.insert(diag1_id.clone(), diag1.clone());

        let uri1 = get_receipt_uri(&root_path, "admission.receipt");
        let mut changes1 = HashMap::new();
        changes1.insert(
            uri1,
            vec![TextEdit {
                range: Range::default(),
                new_text: "rcpt-uninitialized\n".to_string(),
            }],
        );

        let lsp_action1 = CodeAction {
            title: "Synchronize machine state with semantic state".to_string(),
            kind: Some(CodeActionKind::QUICKFIX),
            diagnostics: Some(vec![diag1.lsp.clone()]),
            edit: Some(WorkspaceEdit {
                changes: Some(changes1),
                document_changes: None,
                change_annotations: None,
                ..Default::default()
            }),
            ..Default::default()
        };

        let action1 = max_protocol::MaxCodeAction {
            action: lsp_action1,
            preconditions: vec![max_protocol::Precondition {
                condition: "State is Uninitialized".to_string(),
            }],
            validation_plan: max_protocol::ValidationPlan {
                gates: vec![max_protocol::GateId("gate-state-check".to_string())],
            },
            rollback_plan: max_protocol::RollbackPlan {
                strategy: "Revert state to Uninitialized".to_string(),
            },
            receipt_plan: max_protocol::ReceiptPlan {
                expected_receipts: vec![],
            },
        };
        registry.repair_plans.insert(diag1_id, vec![action1]);
    } else {
        registry.diagnostics.remove(&diag1_id);
        registry.repair_plans.remove(&diag1_id);
    }

    // 2. Check for diag-missing-receipt and diag-auth-generator
    let diag2_id = "diag-missing-receipt".to_string();
    let diag3_id = "diag-auth-generator".to_string();
    let has_security_auth = registry.receipts.contains_key("rcpt-security-auth");

    if !has_security_auth {
        // diag-missing-receipt
        let diag2_cleared = registry.cleared_diagnostics.contains(&diag2_id);
        if !diag2_cleared {
            let diag2 = max_protocol::MaxDiagnostic {
                lsp: Diagnostic {
                    range: Range::default(),
                    severity: Some(DiagnosticSeverity::WARNING),
                    code: None,
                    code_description: None,
                    source: Some("tower-lsp-max".to_string()),
                    message: "Missing validation receipt for secure admission.".to_string(),
                    related_information: None,
                    tags: None,
                    data: None,
                },
                diagnostic_id: diag2_id.clone(),
                law_id: "LAW-003".to_string(),
                attempted_transition: None,
                violated_axes: vec!["Receipt Integrity".to_string()],
                doc_routes: vec![max_protocol::DocRoute {
                    path: "/doc/receipts".to_string(),
                }],
                repair_actions: vec![max_protocol::RepairAction {
                    action_id: "repair-apply-security-patch".to_string(),
                    description: "Apply cryptographic admission repair".to_string(),
                }],
                verification_gates: vec![max_protocol::GateId("gate-receipt-check".to_string())],
                receipt_obligation: Some(max_protocol::ReceiptObligation {
                    required_receipts: vec!["rcpt-security-auth".to_string()],
                }),
                law_axis: max_protocol::LawAxis::default(),
                violated_invariant: String::new(),
                observed_state: serde_json::Value::Null,
                expected_state: serde_json::Value::Null,
                repairability: max_protocol::Repairability::default(),
                terminality: max_protocol::Terminality::default(),
            };
            registry.diagnostics.insert(diag2_id.clone(), diag2.clone());

            let uri2 = get_receipt_uri(&root_path, "security.receipt");
            let mut changes2 = HashMap::new();
            changes2.insert(
                uri2,
                vec![TextEdit {
                    range: Range::default(),
                    new_text: "rcpt-security-auth\n".to_string(),
                }],
            );

            let lsp_action2 = CodeAction {
                title: "Apply cryptographic admission repair".to_string(),
                kind: Some(CodeActionKind::QUICKFIX),
                diagnostics: Some(vec![diag2.lsp.clone()]),
                edit: Some(WorkspaceEdit {
                    changes: Some(changes2),
                    document_changes: None,
                    change_annotations: None,
                    ..Default::default()
                }),
                ..Default::default()
            };

            let action2 = max_protocol::MaxCodeAction {
                action: lsp_action2,
                preconditions: vec![],
                validation_plan: max_protocol::ValidationPlan {
                    gates: vec![max_protocol::GateId("gate-receipt-check".to_string())],
                },
                rollback_plan: max_protocol::RollbackPlan {
                    strategy: "None".to_string(),
                },
                receipt_plan: max_protocol::ReceiptPlan {
                    expected_receipts: vec!["rcpt-security-auth".to_string()],
                },
            };
            registry.repair_plans.insert(diag2_id, vec![action2]);
        } else {
            registry.diagnostics.remove(&diag2_id);
            registry.repair_plans.remove(&diag2_id);
        }

        // diag-auth-generator
        let diag3_cleared = registry.cleared_diagnostics.contains(&diag3_id);
        if !diag3_cleared {
            let diag3 = max_protocol::MaxDiagnostic {
                lsp: Diagnostic {
                    range: Range::default(),
                    severity: Some(DiagnosticSeverity::INFORMATION),
                    code: None,
                    code_description: None,
                    source: Some("tower-lsp-max".to_string()),
                    message: "Generate security authorization receipt.".to_string(),
                    related_information: None,
                    tags: None,
                    data: None,
                },
                diagnostic_id: diag3_id.clone(),
                law_id: "LAW-003".to_string(),
                attempted_transition: None,
                violated_axes: vec!["Receipt Integrity".to_string()],
                doc_routes: vec![],
                repair_actions: vec![max_protocol::RepairAction {
                    action_id: "repair-generate-auth".to_string(),
                    description: "Generate security authorization receipt".to_string(),
                }],
                verification_gates: vec![max_protocol::GateId("gate-auth-check".to_string())],
                receipt_obligation: None,
                law_axis: max_protocol::LawAxis::default(),
                violated_invariant: String::new(),
                observed_state: serde_json::Value::Null,
                expected_state: serde_json::Value::Null,
                repairability: max_protocol::Repairability::default(),
                terminality: max_protocol::Terminality::default(),
            };
            registry.diagnostics.insert(diag3_id.clone(), diag3.clone());

            let uri3 = get_receipt_uri(&root_path, "auth.receipt");
            let mut changes3 = HashMap::new();
            changes3.insert(
                uri3,
                vec![TextEdit {
                    range: Range::default(),
                    new_text: "generated-rcpt-security-auth\n".to_string(),
                }],
            );

            let lsp_action3 = CodeAction {
                title: "Generate security authorization receipt".to_string(),
                kind: Some(CodeActionKind::QUICKFIX),
                diagnostics: Some(vec![diag3.lsp.clone()]),
                edit: Some(WorkspaceEdit {
                    changes: Some(changes3),
                    document_changes: None,
                    change_annotations: None,
                    ..Default::default()
                }),
                ..Default::default()
            };

            let action3 = max_protocol::MaxCodeAction {
                action: lsp_action3,
                preconditions: vec![],
                validation_plan: max_protocol::ValidationPlan {
                    gates: vec![max_protocol::GateId("gate-auth-check".to_string())],
                },
                rollback_plan: max_protocol::RollbackPlan {
                    strategy: "None".to_string(),
                },
                receipt_plan: max_protocol::ReceiptPlan {
                    expected_receipts: vec![],
                },
            };
            registry.repair_plans.insert(diag3_id, vec![action3]);
        } else {
            registry.diagnostics.remove(&diag3_id);
            registry.repair_plans.remove(&diag3_id);
        }
    } else {
        registry.diagnostics.remove(&diag2_id);
        registry.repair_plans.remove(&diag2_id);
        registry.diagnostics.remove(&diag3_id);
        registry.repair_plans.remove(&diag3_id);
    }
}
