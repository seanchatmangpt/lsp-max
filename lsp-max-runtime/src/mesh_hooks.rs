use crate::mesh_types::{
    FailureMode, Hook, HookDescriptor, HookEvent, InstanceId, MeshAction, PolicyState, Receipt,
};

pub struct IntakeDiagnosticHook;

impl Hook for IntakeDiagnosticHook {
    fn name(&self) -> &str {
        "IntakeDiagnosticHook"
    }

    fn trigger(&self, event: &HookEvent) -> Vec<MeshAction> {
        match event {
            HookEvent::DiagnosticEmitted {
                instance_id,
                diagnostic,
            } => {
                if instance_id.0 == "LSP_1" && diagnostic.law_id == "law-intake-validation" {
                    vec![MeshAction::TransitionPolicyState {
                        instance_id: InstanceId::from("LSP_2"),
                        new_state: PolicyState::ClarificationRequested,
                    }]
                } else {
                    vec![]
                }
            }
            _ => vec![],
        }
    }

    fn descriptor(&self) -> HookDescriptor {
        HookDescriptor {
            name: "IntakeDiagnosticHook",
            input_type: "HookEvent::DiagnosticEmitted",
            output_type: "MeshAction::TransitionPolicyState",
            trigger_law: "LAW-INTAKE-001",
            failure_mode: FailureMode::EmitDiagnostic,
        }
    }
}

pub struct IntakeClearHook;

impl Hook for IntakeClearHook {
    fn name(&self) -> &str {
        "IntakeClearHook"
    }

    fn trigger(&self, event: &HookEvent) -> Vec<MeshAction> {
        match event {
            HookEvent::DiagnosticCleared {
                instance_id,
                diagnostic_id,
            } => {
                if instance_id.0 == "LSP_1" && diagnostic_id == "diag-invalid-input" {
                    vec![
                        MeshAction::EmitReceipt {
                            instance_id: InstanceId::from("LSP_1"),
                            receipt: Receipt {
                                receipt_id: "rcpt-intake-validated".to_string(),
                                hash: "hash-intake-validated-mock".to_string(),
                                prev_receipt_hash: None,
                            },
                        },
                        MeshAction::TransitionPolicyState {
                            instance_id: InstanceId::from("LSP_2"),
                            new_state: PolicyState::RefundAuthorized,
                        },
                        MeshAction::ExecuteBoundedAction {
                            instance_id: InstanceId::from("LSP_2"),
                            action_id: "act-create-refund-receipt".to_string(),
                            description: "Creating refund receipt file for policy execution"
                                .to_string(),
                        },
                        MeshAction::EmitReceipt {
                            instance_id: InstanceId::from("LSP_2"),
                            receipt: Receipt {
                                receipt_id: "rcpt-refund-executed".to_string(),
                                hash: "hash-refund-executed-mock".to_string(),
                                prev_receipt_hash: None,
                            },
                        },
                    ]
                } else {
                    vec![]
                }
            }
            _ => vec![],
        }
    }

    fn descriptor(&self) -> HookDescriptor {
        HookDescriptor {
            name: "IntakeClearHook",
            input_type: "HookEvent::DiagnosticCleared",
            output_type: "MeshAction::EmitReceipt, MeshAction::TransitionPolicyState, MeshAction::ExecuteBoundedAction",
            trigger_law: "LAW-INTAKE-002",
            failure_mode: FailureMode::EmitDiagnostic,
        }
    }
}

pub struct CustomerRequestClassifierHook {
    proof_received: std::sync::Mutex<std::collections::HashSet<String>>,
    policy_states: std::sync::Mutex<std::collections::HashMap<String, PolicyState>>,
}

impl Default for CustomerRequestClassifierHook {
    fn default() -> Self {
        Self::new()
    }
}

impl CustomerRequestClassifierHook {
    pub fn new() -> Self {
        Self {
            proof_received: std::sync::Mutex::new(std::collections::HashSet::new()),
            policy_states: std::sync::Mutex::new(std::collections::HashMap::new()),
        }
    }
}

impl Hook for CustomerRequestClassifierHook {
    fn name(&self) -> &str {
        "CustomerRequestClassifierHook"
    }

    fn trigger(&self, event: &HookEvent) -> Vec<MeshAction> {
        let mut actions = Vec::new();
        match event {
            HookEvent::ReceiptEmitted {
                instance_id,
                receipt,
            } if receipt.receipt_id.contains("proof")
                || receipt.receipt_id.contains("customer-proof") =>
            {
                if let Ok(mut proof) = self.proof_received.lock() {
                    proof.insert(instance_id.0.clone());
                }
            }
            HookEvent::PolicyStateChanged {
                instance_id,
                from_state: _,
                to_state,
            } => {
                if let Ok(mut states) = self.policy_states.lock() {
                    states.insert(instance_id.0.clone(), to_state.clone());
                }
            }
            HookEvent::DiagnosticEmitted {
                instance_id,
                diagnostic,
            } => {
                let diag_id = &diagnostic.diagnostic_id;
                let message = diagnostic.lsp.message.to_lowercase();
                let is_proof_issue = diag_id == "missing-proof"
                    || diag_id == "damaged-proof"
                    || message.contains("proof is missing")
                    || message.contains("proof is damaged")
                    || message.contains("damaged proof")
                    || message.contains("missing proof");
                if is_proof_issue {
                    let should_transition = if let Ok(states) = self.policy_states.lock() {
                        !matches!(
                            states.get(instance_id.0.as_str()),
                            Some(PolicyState::ClarificationRequested)
                                | Some(PolicyState::RefundAuthorized)
                        )
                    } else {
                        true
                    };
                    if should_transition {
                        actions.push(MeshAction::TransitionPolicyState {
                            instance_id: instance_id.clone(),
                            new_state: PolicyState::ClarificationRequested,
                        });
                    }
                }
            }
            HookEvent::StateTransition {
                instance_id,
                from_phase: _,
                to_phase,
            } if to_phase == "Initialized" => {
                let is_missing = if let Ok(proof) = self.proof_received.lock() {
                    !proof.contains(instance_id.0.as_str())
                } else {
                    true
                };
                if is_missing {
                    let should_transition = if let Ok(states) = self.policy_states.lock() {
                        !matches!(
                            states.get(instance_id.0.as_str()),
                            Some(PolicyState::ClarificationRequested)
                                | Some(PolicyState::RefundAuthorized)
                        )
                    } else {
                        true
                    };
                    if should_transition {
                        actions.push(MeshAction::TransitionPolicyState {
                            instance_id: instance_id.clone(),
                            new_state: PolicyState::ClarificationRequested,
                        });
                    }
                }
            }
            HookEvent::BoundedActionExecuted {
                instance_id,
                action_id,
                description,
            } => {
                if let Ok(mut proof) = self.proof_received.lock() {
                    proof.insert(instance_id.0.clone());
                }
                actions.push(MeshAction::EmitReceipt {
                    instance_id: instance_id.clone(),
                    receipt: Receipt {
                        receipt_id: format!("bounded-action-executed-{}", action_id),
                        hash: format!("sha256:bounded:{}:{}", action_id, description.len()),
                        prev_receipt_hash: None,
                    },
                });
            }
            HookEvent::InstanceReset { instance_id } => {
                if let Ok(mut proof) = self.proof_received.lock() {
                    proof.remove(&instance_id.0);
                }
                if let Ok(mut states) = self.policy_states.lock() {
                    states.remove(&instance_id.0);
                }
            }
            _ => {}
        }
        actions
    }

    fn descriptor(&self) -> HookDescriptor {
        HookDescriptor {
            name: "CustomerRequestClassifierHook",
            input_type: "HookEvent::ReceiptEmitted, HookEvent::PolicyStateChanged, HookEvent::DiagnosticEmitted, HookEvent::StateTransition, HookEvent::BoundedActionExecuted, HookEvent::InstanceReset",
            output_type: "MeshAction::TransitionPolicyState, MeshAction::EmitReceipt",
            trigger_law: "LAW-CLASSIFY-001",
            failure_mode: FailureMode::RefuseEvent,
        }
    }
}

pub struct PolicyEvaluationHook {
    policy_states: std::sync::Mutex<std::collections::HashMap<String, PolicyState>>,
}

impl Default for PolicyEvaluationHook {
    fn default() -> Self {
        Self::new()
    }
}

impl PolicyEvaluationHook {
    pub fn new() -> Self {
        Self {
            policy_states: std::sync::Mutex::new(std::collections::HashMap::new()),
        }
    }
}

impl Hook for PolicyEvaluationHook {
    fn name(&self) -> &str {
        "PolicyEvaluationHook"
    }

    fn trigger(&self, event: &HookEvent) -> Vec<MeshAction> {
        let mut actions = Vec::new();
        match event {
            HookEvent::ReceiptEmitted {
                instance_id,
                receipt,
            } if receipt.receipt_id.contains("proof")
                || receipt.receipt_id.contains("customer-proof") =>
            {
                let is_clarification_requested = if let Ok(states) = self.policy_states.lock() {
                    states.get(&instance_id.0) == Some(&PolicyState::ClarificationRequested)
                } else {
                    false
                };
                if is_clarification_requested {
                    actions.push(MeshAction::TransitionPolicyState {
                        instance_id: instance_id.clone(),
                        new_state: PolicyState::RefundAuthorized,
                    });
                }
            }
            HookEvent::PolicyStateChanged {
                instance_id,
                from_state,
                to_state,
            } => {
                if let Ok(mut states) = self.policy_states.lock() {
                    states.insert(instance_id.0.clone(), to_state.clone());
                }
                if from_state == &PolicyState::ClarificationRequested
                    && to_state == &PolicyState::RefundAuthorized
                {
                    actions.push(MeshAction::ExecuteBoundedAction {
                        instance_id: instance_id.clone(),
                        action_id: "act-create-refund-receipt".to_string(),
                        description: "Arrival of proof validated, creating refund receipt"
                            .to_string(),
                    });
                }
            }
            HookEvent::BoundedActionExecuted {
                instance_id,
                action_id,
                ..
            } if action_id == "act-create-refund-receipt" => {
                actions.push(MeshAction::EmitReceipt {
                    instance_id: instance_id.clone(),
                    receipt: Receipt {
                        receipt_id: "refund-action-completion-receipt".to_string(),
                        hash: format!("sha256:completion:{}", action_id),
                        prev_receipt_hash: None,
                    },
                });
            }
            HookEvent::InstanceReset { instance_id } => {
                if let Ok(mut states) = self.policy_states.lock() {
                    states.remove(&instance_id.0);
                }
            }
            _ => {}
        }
        actions
    }

    fn descriptor(&self) -> HookDescriptor {
        HookDescriptor {
            name: "PolicyEvaluationHook",
            input_type: "HookEvent::ReceiptEmitted, HookEvent::PolicyStateChanged, HookEvent::BoundedActionExecuted, HookEvent::InstanceReset",
            output_type: "MeshAction::TransitionPolicyState, MeshAction::ExecuteBoundedAction, MeshAction::EmitReceipt",
            trigger_law: "LAW-POLICY-001",
            failure_mode: FailureMode::Halt,
        }
    }
}

pub struct ReceiptRoutingHook {
    active_diagnostics:
        std::sync::Mutex<std::collections::HashMap<String, std::collections::HashSet<String>>>,
}

impl Default for ReceiptRoutingHook {
    fn default() -> Self {
        Self::new()
    }
}

impl ReceiptRoutingHook {
    pub fn new() -> Self {
        Self {
            active_diagnostics: std::sync::Mutex::new(std::collections::HashMap::new()),
        }
    }
}

impl Hook for ReceiptRoutingHook {
    fn name(&self) -> &str {
        "ReceiptRoutingHook"
    }

    fn trigger(&self, event: &HookEvent) -> Vec<MeshAction> {
        let mut actions = Vec::new();
        match event {
            HookEvent::DiagnosticEmitted {
                instance_id,
                diagnostic,
            } => {
                if let Ok(mut diags) = self.active_diagnostics.lock() {
                    diags
                        .entry(instance_id.0.clone())
                        .or_default()
                        .insert(diagnostic.diagnostic_id.clone());
                }
            }
            HookEvent::DiagnosticCleared {
                instance_id,
                diagnostic_id,
            } => {
                if let Ok(mut diags) = self.active_diagnostics.lock() {
                    if let Some(set) = diags.get_mut(&instance_id.0) {
                        set.remove(diagnostic_id);
                    }
                }
            }
            HookEvent::ReceiptEmitted {
                instance_id,
                receipt: _,
            } => {
                let target_instance = if instance_id.0 == "LSP_2" {
                    Some("LSP_1".to_string())
                } else if instance_id.0.contains("LSP_2") {
                    Some(instance_id.0.replace("LSP_2", "LSP_1"))
                } else if instance_id.0.contains("lsp_2") {
                    Some(instance_id.0.replace("lsp_2", "lsp_1"))
                } else {
                    None
                };

                if let Some(target) = target_instance {
                    if let Ok(diags) = self.active_diagnostics.lock() {
                        if let Some(set) = diags.get(&target) {
                            for diag_id in set {
                                actions.push(MeshAction::ClearDiagnostic {
                                    instance_id: InstanceId::from(target.clone()),
                                    diagnostic_id: diag_id.clone(),
                                });
                            }
                        }
                    }
                }
            }
            _ => {}
        }
        actions
    }

    fn descriptor(&self) -> HookDescriptor {
        HookDescriptor {
            name: "ReceiptRoutingHook",
            input_type: "HookEvent::DiagnosticEmitted, HookEvent::DiagnosticCleared, HookEvent::ReceiptEmitted",
            output_type: "MeshAction::ClearDiagnostic",
            trigger_law: "LAW-ROUTING-001",
            failure_mode: FailureMode::EmitDiagnostic,
        }
    }
}
