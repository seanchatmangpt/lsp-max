use crate::mesh::{build_conformance_vector, AutonomicMesh};
use crate::mesh_types::{InstanceId, MaxMethod, MeshAction};
use crate::sha256::sha256;
use crate::typestate::DeterministicSnapshot;

impl AutonomicMesh {
    pub fn dispatch_rpc(
        &mut self,
        instance_id: &str,
        method: &str,
        params: serde_json::Value,
    ) -> Result<serde_json::Value, String> {
        if !self.instances.contains_key(instance_id) {
            return Err(format!("Instance {} not found", instance_id));
        }

        let max_method = MaxMethod::try_from(method)
            .map_err(|_| format!("Method {} not supported on local RPC surface", method))?;

        match max_method {
            MaxMethod::VerifyLedger => {
                self.verify_instance_ledger(instance_id)?;
                Ok(serde_json::Value::Null)
            }
            MaxMethod::LedgerReport => {
                let report = self.get_ledger_diagnostic_report(instance_id);
                Ok(serde_json::Value::String(report))
            }
            MaxMethod::Snapshot => {
                let snap = DeterministicSnapshot::new();
                serde_json::to_value(snap.id).map_err(|e| e.to_string())
            }
            MaxMethod::ConformanceVector => {
                let instance = self
                    .instances
                    .get(instance_id)
                    .ok_or_else(|| format!("Instance not found: {}", instance_id))?;

                let vec = build_conformance_vector(&instance.diagnostics);
                serde_json::to_value(vec).map_err(|e| e.to_string())
            }
            MaxMethod::ClearDiagnostic => {
                let diag_id: String =
                    serde_json::from_value(params).map_err(|e| format!("Invalid params: {}", e))?;

                self.execute_action(MeshAction::ClearDiagnostic {
                    instance_id: InstanceId::from(instance_id),
                    diagnostic_id: diag_id,
                });
                Ok(serde_json::Value::Null)
            }
            MaxMethod::ExplainDiagnostic => {
                let diag_id: String =
                    serde_json::from_value(params).map_err(|e| format!("Invalid params: {}", e))?;
                let inst = self
                    .instances
                    .get(instance_id)
                    .ok_or_else(|| format!("Instance not found: {}", instance_id))?;
                let diag = inst
                    .diagnostics
                    .iter()
                    .find(|d| d.diagnostic_id == diag_id)
                    .ok_or_else(|| format!("Diagnostic not found: {}", diag_id))?;
                serde_json::to_value(diag.clone()).map_err(|e| e.to_string())
            }
            MaxMethod::RepairPlan => self.handle_repair_plan(instance_id, params),
            MaxMethod::ApplyRepairTransaction => {
                let code_action: tower_lsp_max_protocol::MaxCodeAction =
                    serde_json::from_value(params).map_err(|e| format!("Invalid params: {}", e))?;
                self.apply_repair_transaction(instance_id, code_action)
            }
            MaxMethod::ExportAnalysisBundle => self.handle_export_bundle(instance_id, params),
            MaxMethod::RunGate => {
                let gate_str: String =
                    serde_json::from_value(params).map_err(|e| format!("Invalid params: {}", e))?;
                let inst = self
                    .instances
                    .get(instance_id)
                    .ok_or_else(|| format!("Instance not found: {}", instance_id))?;
                let gate_blocked = inst
                    .diagnostics
                    .iter()
                    .any(|d| d.verification_gates.iter().any(|g| g.0 == gate_str));
                serde_json::to_value(!gate_blocked).map_err(|e| e.to_string())
            }
            MaxMethod::Receipt => {
                let receipt_id: String =
                    serde_json::from_value(params).map_err(|e| format!("Invalid params: {}", e))?;
                let inst = self
                    .instances
                    .get(instance_id)
                    .ok_or_else(|| format!("Instance not found: {}", instance_id))?;
                let receipt = inst
                    .receipts
                    .iter()
                    .find(|r| r.receipt_id == receipt_id)
                    .ok_or_else(|| format!("Receipt not found: {}", receipt_id))?;
                serde_json::to_value(receipt.clone()).map_err(|e| e.to_string())
            }
            MaxMethod::Hook => {
                let hook_names: Vec<serde_json::Value> = self
                    .hooks
                    .iter()
                    .map(|h| serde_json::json!({ "name": h.name() }))
                    .collect();
                serde_json::to_value(hook_names).map_err(|e| e.to_string())
            }
            MaxMethod::HookGraph => self.handle_hook_graph(instance_id),
            MaxMethod::Chain => self.handle_chain(),
            MaxMethod::Propagate => {
                let receipt: tower_lsp_max_protocol::Receipt =
                    serde_json::from_value(params).map_err(|e| format!("Invalid params: {}", e))?;
                self.execute_action(MeshAction::EmitReceipt {
                    instance_id: InstanceId::from(instance_id),
                    receipt,
                });
                Ok(serde_json::json!({ "propagated": true }))
            }
            MaxMethod::AutonomicLoop => {
                let status = serde_json::json!({
                    "instances": self.instances.keys().collect::<Vec<_>>(),
                    "hook_count": self.hooks.len(),
                    "event_log_size": self.event_log.len(),
                    "executed_actions": self.executed_bounded_actions.len(),
                });
                Ok(status)
            }
            MaxMethod::ManifoldSnapshot => {
                let snapshot = serde_json::json!({
                    "instances": self.instances.iter().map(|(id, inst)| {
                        serde_json::json!({
                            "id": id,
                            "phase": inst.phase,
                            "policy_state": inst.policy_state,
                            "diagnostic_count": inst.diagnostics.len(),
                            "receipt_count": inst.receipts.len(),
                            "conformance_score": inst.conformance_score(),
                            "conformance_grade": inst.conformance_grade().as_str(),
                        })
                    }).collect::<Vec<_>>(),
                    "hook_count": self.hooks.len(),
                    "event_log_size": self.event_log.len(),
                });
                Ok(snapshot)
            }
            MaxMethod::LawfulTransition => self.handle_lawful_transition(instance_id, params),
            MaxMethod::Admission => {
                let inst = self
                    .instances
                    .get(instance_id)
                    .ok_or_else(|| format!("Instance not found: {}", instance_id))?;
                let verdict = if inst.diagnostics.is_empty() {
                    "Admitted"
                } else if inst.diagnostics.iter().any(|d| {
                    matches!(
                        d.lsp.severity,
                        Some(lsp_types_max::DiagnosticSeverity::ERROR)
                    )
                }) {
                    "Refused"
                } else {
                    "Unknown"
                };
                Ok(serde_json::json!({
                    "instance_id": instance_id,
                    "verdict": verdict,
                    "diagnostic_count": inst.diagnostics.len(),
                }))
            }
            MaxMethod::Refusal => {
                let diag_id: String =
                    serde_json::from_value(params).map_err(|e| format!("Invalid params: {}", e))?;
                let receipt_id = format!("rcpt-refusal-{}", diag_id);
                let hash = sha256(receipt_id.as_bytes());
                let receipt = tower_lsp_max_protocol::Receipt {
                    receipt_id: receipt_id.clone(),
                    hash,
                    prev_receipt_hash: None,
                };
                self.execute_action(MeshAction::EmitReceipt {
                    instance_id: InstanceId::from(instance_id),
                    receipt: receipt.clone(),
                });
                Ok(serde_json::json!({
                    "refused": true,
                    "diagnostic_id": diag_id,
                    "receipt": receipt,
                }))
            }
            MaxMethod::Replay => self.handle_replay(instance_id),
            MaxMethod::ReleaseActuation => self.handle_release_actuation(instance_id),
            MaxMethod::InstanceList => {
                let mut list: Vec<serde_json::Value> = self
                    .instances
                    .values()
                    .map(|inst| {
                        serde_json::json!({
                            "id": inst.id,
                            "phase": inst.phase,
                            "conformance_score": inst.conformance_score(),
                            "conformance_grade": inst.conformance_grade().as_str(),
                        })
                    })
                    .collect();
                list.sort_by_key(|v| v["id"].as_str().unwrap_or("").to_string());
                serde_json::to_value(list).map_err(|e| e.to_string())
            }
            MaxMethod::DumpState => {
                let state = self.to_state();
                serde_json::to_value(&state).map_err(|e| format!("Serialization failed: {}", e))
            }
            MaxMethod::RestoreState => {
                let state: crate::mesh_types::AutonomicMeshState =
                    serde_json::from_value(params).map_err(|e| format!("Invalid params: {}", e))?;
                self.load_state(state);
                Ok(serde_json::Value::Null)
            }
            MaxMethod::Reset => {
                self.execute_action(MeshAction::ResetInstance {
                    instance_id: InstanceId::from(instance_id),
                });
                Ok(serde_json::json!({
                    "reset": true,
                    "instance_id": instance_id,
                }))
            }
            MaxMethod::ConformanceDelta => {
                let since_seq: u64 = params
                    .as_object()
                    .and_then(|o| o.get("since_seq"))
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0);
                let deltas: Vec<&crate::mesh_types::ConformanceDeltaEntry> = self
                    .conformance_delta_log
                    .iter()
                    .filter(|e| e.seq > since_seq)
                    .collect();
                Ok(serde_json::json!({
                    "deltas": deltas,
                    "current_seq": self.action_seq,
                }))
            }
        }
    }

    pub fn apply_repair_transaction(
        &mut self,
        instance_id: &str,
        code_action: tower_lsp_max_protocol::MaxCodeAction,
    ) -> Result<serde_json::Value, String> {
        {
            let inst = self
                .instances
                .get(instance_id)
                .ok_or_else(|| format!("Instance not found: {}", instance_id))?;
            let existing: std::collections::HashSet<&str> = inst
                .receipts
                .iter()
                .map(|r| r.receipt_id.as_str())
                .collect();
            for expected in &code_action.receipt_plan.expected_receipts {
                if !existing.contains(expected.as_str()) {
                    return Err(format!(
                        "Receipt integrity violation: required receipt '{}' not found",
                        expected
                    ));
                }
            }
        }
        let action_id = format!("repair-{}", code_action.action.title.replace(' ', "-"));
        let receipt_id = format!("rcpt-repair-{}", code_action.action.title.replace(' ', "-"));
        let hash = sha256(receipt_id.as_bytes());
        let receipt = tower_lsp_max_protocol::Receipt {
            receipt_id: receipt_id.clone(),
            hash,
            prev_receipt_hash: None,
        };
        self.execute_action(MeshAction::ExecuteBoundedAction {
            instance_id: InstanceId::from(instance_id),
            action_id,
            description: code_action.action.title.clone(),
        });
        self.execute_action(MeshAction::EmitReceipt {
            instance_id: InstanceId::from(instance_id),
            receipt: receipt.clone(),
        });
        serde_json::to_value(receipt).map_err(|e| e.to_string())
    }
}
