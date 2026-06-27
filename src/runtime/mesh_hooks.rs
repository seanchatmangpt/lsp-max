use crate::runtime::mesh_types::{
    FailureMode, Hook, HookDescriptor, HookEvent, InstanceId, MaxDiagnostic, MeshAction,
    PolicyState, Receipt,
};

/// Enforces three process-level laws by monitoring the OCEL of the LSP session.
/// Text scanning detects what code looks like. This hook detects what process produced it.
/// PROCESS-001: receipt emitted while unresolved diagnostics are active
/// PROCESS-002: receipt emitted while instance is in ClarificationRequested
/// PROCESS-003: diagnostic cleared with no intervening resolution event (oracle injection signal)
pub struct OcelProcessHook {
    active_diagnostics:
        std::sync::Mutex<std::collections::HashMap<String, std::collections::HashSet<String>>>,
    policy_states: std::sync::Mutex<std::collections::HashMap<String, PolicyState>>,
    diag_emission_baselines: std::sync::Mutex<std::collections::HashMap<String, u64>>,
    resolution_counts: std::sync::Mutex<std::collections::HashMap<String, u64>>,
}

impl Default for OcelProcessHook {
    fn default() -> Self {
        Self::new()
    }
}

impl OcelProcessHook {
    pub fn new() -> Self {
        Self {
            active_diagnostics: std::sync::Mutex::new(std::collections::HashMap::new()),
            policy_states: std::sync::Mutex::new(std::collections::HashMap::new()),
            diag_emission_baselines: std::sync::Mutex::new(std::collections::HashMap::new()),
            resolution_counts: std::sync::Mutex::new(std::collections::HashMap::new()),
        }
    }

    fn make_diag(
        law_id: &str,
        id: &str,
        msg: &str,
        sev: lsp_types_max::DiagnosticSeverity,
    ) -> Box<MaxDiagnostic> {
        Box::new(MaxDiagnostic {
            lsp: lsp_types_max::Diagnostic {
                range: lsp_types_max::Range::default(),
                severity: Some(sev),
                code: Some(lsp_types_max::NumberOrString::String(law_id.to_string())),
                message: msg.to_string(),
                ..Default::default()
            },
            diagnostic_id: id.to_string(),
            law_id: law_id.to_string(),
            violated_invariant: msg.to_string(),
            ..Default::default()
        })
    }
}

impl Hook for OcelProcessHook {
    fn name(&self) -> &str {
        "OcelProcessHook"
    }

    fn trigger(&self, event: &HookEvent) -> Vec<MeshAction> {
        let mut actions = Vec::new();
        match event {
            HookEvent::DiagnosticEmitted {
                instance_id,
                diagnostic,
            } => {
                if let Ok(mut d) = self.active_diagnostics.lock() {
                    d.entry(instance_id.0.clone())
                        .or_default()
                        .insert(diagnostic.diagnostic_id.clone());
                }
                let baseline = self
                    .resolution_counts
                    .lock()
                    .ok()
                    .and_then(|m| m.get(&instance_id.0).copied())
                    .unwrap_or(0);
                if let Ok(mut b) = self.diag_emission_baselines.lock() {
                    b.insert(diagnostic.diagnostic_id.clone(), baseline);
                }
            }
            HookEvent::DiagnosticCleared {
                instance_id,
                diagnostic_id,
            } => {
                let baseline = self
                    .diag_emission_baselines
                    .lock()
                    .ok()
                    .and_then(|m| m.get(diagnostic_id.as_str()).copied());
                let current = self
                    .resolution_counts
                    .lock()
                    .ok()
                    .and_then(|m| m.get(&instance_id.0).copied())
                    .unwrap_or(0);
                if let Some(b) = baseline {
                    if current == b {
                        actions.push(MeshAction::AddDiagnostic {
                            instance_id: instance_id.clone(),
                            diagnostic: Self::make_diag(
                                "PROCESS-003",
                                &format!("process-003-{}", diagnostic_id),
                                &format!("Process violation (PROCESS-003): '{}' cleared without resolution event — oracle injection signal. Text scanning cannot detect this.", diagnostic_id),
                                lsp_types_max::DiagnosticSeverity::WARNING,
                            ),
                        });
                    }
                }
                if let Ok(mut d) = self.active_diagnostics.lock() {
                    if let Some(s) = d.get_mut(&instance_id.0) {
                        s.remove(diagnostic_id.as_str());
                    }
                }
                if let Ok(mut b) = self.diag_emission_baselines.lock() {
                    b.remove(diagnostic_id.as_str());
                }
            }
            HookEvent::ReceiptEmitted {
                instance_id,
                receipt,
            } => {
                let has_active = self
                    .active_diagnostics
                    .lock()
                    .ok()
                    .and_then(|m| m.get(&instance_id.0).map(|s| !s.is_empty()))
                    .unwrap_or(false);
                if has_active {
                    actions.push(MeshAction::AddDiagnostic {
                        instance_id: instance_id.clone(),
                        diagnostic: Self::make_diag(
                            "PROCESS-001",
                            &format!("process-001-{}", receipt.receipt_id),
                            &format!("Process violation (PROCESS-001): receipt '{}' emitted while violations active — stage not lawfully complete. Text scanning cannot detect this.", receipt.receipt_id),
                            lsp_types_max::DiagnosticSeverity::ERROR,
                        ),
                    });
                }
                let in_clarification = self
                    .policy_states
                    .lock()
                    .ok()
                    .and_then(|m| m.get(&instance_id.0).cloned())
                    .map(|s| s == PolicyState::ClarificationRequested)
                    .unwrap_or(false);
                if in_clarification {
                    actions.push(MeshAction::AddDiagnostic {
                        instance_id: instance_id.clone(),
                        diagnostic: Self::make_diag(
                            "PROCESS-002",
                            &format!("process-002-{}", receipt.receipt_id),
                            &format!("Process violation (PROCESS-002): receipt '{}' emitted while ClarificationRequested — pending clarification not resolved. Text scanning cannot detect this.", receipt.receipt_id),
                            lsp_types_max::DiagnosticSeverity::ERROR,
                        ),
                    });
                }
            }
            HookEvent::PolicyStateChanged {
                instance_id,
                to_state,
                ..
            } => {
                if let Ok(mut s) = self.policy_states.lock() {
                    s.insert(instance_id.0.clone(), to_state.clone());
                }
                if let Ok(mut c) = self.resolution_counts.lock() {
                    *c.entry(instance_id.0.clone()).or_insert(0) += 1;
                }
            }
            HookEvent::BoundedActionExecuted { instance_id, .. } => {
                if let Ok(mut c) = self.resolution_counts.lock() {
                    *c.entry(instance_id.0.clone()).or_insert(0) += 1;
                }
            }
            HookEvent::InstanceReset { instance_id } => {
                if let Ok(mut d) = self.active_diagnostics.lock() {
                    d.remove(&instance_id.0);
                }
                if let Ok(mut s) = self.policy_states.lock() {
                    s.remove(&instance_id.0);
                }
                if let Ok(mut c) = self.resolution_counts.lock() {
                    c.remove(&instance_id.0);
                }
            }
            _ => {}
        }
        actions
    }

    fn descriptor(&self) -> HookDescriptor {
        HookDescriptor {
            name: "OcelProcessHook",
            input_type: "HookEvent::DiagnosticEmitted, HookEvent::DiagnosticCleared, HookEvent::ReceiptEmitted, HookEvent::PolicyStateChanged, HookEvent::BoundedActionExecuted, HookEvent::InstanceReset",
            output_type: "MeshAction::AddDiagnostic",
            trigger_law: "PROCESS-001, PROCESS-002, PROCESS-003",
            failure_mode: FailureMode::EmitDiagnostic,
        }
    }
}

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

#[cfg(test)]
mod ocel_process_hook_tests {
    use super::*;

    fn diag(id: &str) -> Box<crate::runtime::mesh_types::MaxDiagnostic> {
        Box::new(crate::runtime::mesh_types::MaxDiagnostic {
            diagnostic_id: id.to_string(),
            law_id: "COG-001".to_string(),
            ..Default::default()
        })
    }
    fn rcpt(id: &str) -> Receipt {
        Receipt {
            receipt_id: id.to_string(),
            hash: "h".to_string(),
            prev_receipt_hash: None,
        }
    }
    fn inst(id: &str) -> InstanceId {
        InstanceId::from(id)
    }

    #[test]
    fn process_001_fires_with_active_diagnostic() {
        let h = OcelProcessHook::new();
        h.trigger(&HookEvent::DiagnosticEmitted {
            instance_id: inst("A"),
            diagnostic: diag("d1"),
        });
        let acts = h.trigger(&HookEvent::ReceiptEmitted {
            instance_id: inst("A"),
            receipt: rcpt("r1"),
        });
        assert!(acts
            .iter()
            .any(|a| matches!(a, MeshAction::AddDiagnostic { diagnostic, .. } if diagnostic.law_id == "PROCESS-001")));
    }

    #[test]
    fn process_001_silent_when_no_active_diagnostics() {
        let h = OcelProcessHook::new();
        let acts = h.trigger(&HookEvent::ReceiptEmitted {
            instance_id: inst("A"),
            receipt: rcpt("r1"),
        });
        assert!(acts.is_empty());
    }

    #[test]
    fn process_002_fires_in_clarification_requested_state() {
        let h = OcelProcessHook::new();
        h.trigger(&HookEvent::PolicyStateChanged {
            instance_id: inst("B"),
            from_state: PolicyState::Operational,
            to_state: PolicyState::ClarificationRequested,
        });
        let acts = h.trigger(&HookEvent::ReceiptEmitted {
            instance_id: inst("B"),
            receipt: rcpt("r2"),
        });
        assert!(acts
            .iter()
            .any(|a| matches!(a, MeshAction::AddDiagnostic { diagnostic, .. } if diagnostic.law_id == "PROCESS-002")));
    }

    #[test]
    fn process_003_fires_on_forced_clear() {
        let h = OcelProcessHook::new();
        h.trigger(&HookEvent::DiagnosticEmitted {
            instance_id: inst("A"),
            diagnostic: diag("d-oracle"),
        });
        let acts = h.trigger(&HookEvent::DiagnosticCleared {
            instance_id: inst("A"),
            diagnostic_id: "d-oracle".to_string(),
        });
        assert!(acts
            .iter()
            .any(|a| matches!(a, MeshAction::AddDiagnostic { diagnostic, .. } if diagnostic.law_id == "PROCESS-003")));
    }

    #[test]
    fn process_003_silent_after_resolution_event() {
        let h = OcelProcessHook::new();
        h.trigger(&HookEvent::DiagnosticEmitted {
            instance_id: inst("A"),
            diagnostic: diag("d-legit"),
        });
        h.trigger(&HookEvent::BoundedActionExecuted {
            instance_id: inst("A"),
            action_id: "fix".to_string(),
            description: "Applied repair".to_string(),
        });
        let acts = h.trigger(&HookEvent::DiagnosticCleared {
            instance_id: inst("A"),
            diagnostic_id: "d-legit".to_string(),
        });
        assert!(!acts
            .iter()
            .any(|a| matches!(a, MeshAction::AddDiagnostic { diagnostic, .. } if diagnostic.law_id == "PROCESS-003")));
    }
}
