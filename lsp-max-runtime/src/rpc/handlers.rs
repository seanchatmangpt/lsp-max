use crate::mesh::{build_conformance_vector, AutonomicMesh};
use crate::mesh_types::{ConformanceGrade, HookEvent, InstanceId, MeshAction};
use crate::sha256::sha256;

impl AutonomicMesh {
    // --- Private RPC Helpers for Modularization ---

    pub(super) fn handle_repair_plan(
        &self,
        instance_id: &str,
        params: serde_json::Value,
    ) -> Result<serde_json::Value, String> {
        let id: String =
            serde_json::from_value(params).map_err(|e| format!("Invalid params: {}", e))?;
        let inst = self
            .instances
            .get(instance_id)
            .ok_or_else(|| format!("Instance not found: {}", instance_id))?;
        let actions: Vec<tower_lsp_max_protocol::MaxCodeAction> = inst
            .diagnostics
            .iter()
            .filter(|d| d.diagnostic_id == id || d.law_id == id)
            .flat_map(|d| {
                d.repair_actions
                    .iter()
                    .map(move |ra| tower_lsp_max_protocol::MaxCodeAction {
                        action: lsp_types_max::CodeAction {
                            title: ra.description.clone(),
                            ..Default::default()
                        },
                        preconditions: vec![],
                        validation_plan: tower_lsp_max_protocol::ValidationPlan {
                            gates: d.verification_gates.clone(),
                        },
                        rollback_plan: tower_lsp_max_protocol::RollbackPlan {
                            strategy: "revert".to_string(),
                        },
                        receipt_plan: tower_lsp_max_protocol::ReceiptPlan {
                            expected_receipts: d
                                .receipt_obligation
                                .as_ref()
                                .map(|ro| ro.required_receipts.clone())
                                .unwrap_or_default(),
                        },
                    })
            })
            .collect();
        serde_json::to_value(actions).map_err(|e| e.to_string())
    }

    pub(super) fn handle_export_bundle(
        &self,
        instance_id: &str,
        params: serde_json::Value,
    ) -> Result<serde_json::Value, String> {
        let snapshot_id: tower_lsp_max_protocol::SnapshotId =
            serde_json::from_value(params).map_err(|e| format!("Invalid params: {}", e))?;
        let inst = self
            .instances
            .get(instance_id)
            .ok_or_else(|| format!("Instance not found: {}", instance_id))?;
        let mut cv = build_conformance_vector(&inst.diagnostics);
        cv.score = Some(inst.conformance_score());
        let bundle = tower_lsp_max_protocol::AnalysisBundle {
            snapshot_id,
            capability_vector: tower_lsp_max_protocol::MaxCapabilityVector {
                client: lsp_types_max::ClientCapabilities::default(),
                server: lsp_types_max::ServerCapabilities::default(),
                negotiated: serde_json::Value::Null,
                experimental: serde_json::Value::Null,
                gaps: vec![],
            },
            diagnostics: inst.diagnostics.clone(),
            actions: vec![],
            conformance_vector: cv,
            receipts: inst.receipts.clone(),
        };
        serde_json::to_value(bundle).map_err(|e| e.to_string())
    }

    pub(super) fn handle_hook_graph(&self, instance_id: &str) -> Result<serde_json::Value, String> {
        let inst = self
            .instances
            .get(instance_id)
            .ok_or_else(|| format!("Instance not found: {}", instance_id))?;
        let diagnostic_ids: Vec<&str> = inst
            .diagnostics
            .iter()
            .map(|d| d.diagnostic_id.as_str())
            .collect();
        let receipt_ids: Vec<&str> = inst
            .receipts
            .iter()
            .map(|r| r.receipt_id.as_str())
            .collect();
        let graph: Vec<serde_json::Value> = self
            .hooks
            .iter()
            .map(|h| {
                serde_json::json!({
                    "hook": h.name(),
                    "instance_id": instance_id,
                    "active_diagnostic_triggers": diagnostic_ids,
                    "active_receipt_triggers": receipt_ids,
                    "pending_diagnostic_count": inst.diagnostics.len(),
                    "pending_receipt_count": inst.receipts.len(),
                })
            })
            .collect();
        serde_json::to_value(graph).map_err(|e| e.to_string())
    }

    pub(super) fn handle_chain(&self) -> Result<serde_json::Value, String> {
        let mut chain: Vec<serde_json::Value> = self
            .instances
            .iter()
            .map(|(id, inst)| {
                serde_json::json!({
                    "id": id,
                    "phase": inst.phase,
                    "policy_state": inst.policy_state,
                    "conformance_score": inst.conformance_score(),
                    "conformance_grade": inst.conformance_grade().as_str(),
                    "diagnostic_count": inst.diagnostics.len(),
                    "receipt_count": inst.receipts.len(),
                    "diagnostics": inst.diagnostics.iter().map(|d| serde_json::json!({
                        "id": d.diagnostic_id,
                        "law_id": d.law_id,
                        "severity": format!("{:?}", d.lsp.severity),
                        "message": d.lsp.message,
                    })).collect::<Vec<_>>(),
                    "receipts": inst.receipts.iter().map(|r| serde_json::json!({
                        "receipt_id": r.receipt_id,
                        "hash": r.hash,
                    })).collect::<Vec<_>>(),
                })
            })
            .collect();
        chain.sort_by_key(|v| v["id"].as_str().unwrap_or("").to_string());
        serde_json::to_value(chain).map_err(|e| e.to_string())
    }

    pub(super) fn handle_lawful_transition(
        &self,
        instance_id: &str,
        params: serde_json::Value,
    ) -> Result<serde_json::Value, String> {
        let target_phase: String =
            serde_json::from_value(params).map_err(|e| format!("Invalid params: {}", e))?;
        let inst = self
            .instances
            .get(instance_id)
            .ok_or_else(|| format!("Instance not found: {}", instance_id))?;
        let phase_order = [
            "Uninitialized",
            "Initializing",
            "Initialized",
            "ShutDown",
            "Exited",
        ];
        let current_idx = phase_order
            .iter()
            .position(|&p| p == inst.phase.to_string().as_str());
        let target_idx = phase_order.iter().position(|&p| p == target_phase.as_str());
        let (admitted, refused_reason) = match (current_idx, target_idx) {
            (Some(ci), Some(ti)) if ti == ci + 1 => {
                let blocking: Vec<_> = inst
                    .diagnostics
                    .iter()
                    .filter(|d| {
                        matches!(
                            d.lsp.severity,
                            Some(lsp_types_max::DiagnosticSeverity::ERROR)
                        )
                    })
                    .map(|d| d.diagnostic_id.clone())
                    .collect();
                if blocking.is_empty() {
                    (true, None)
                } else {
                    (
                        false,
                        Some(format!(
                            "Blocked by {} error diagnostic(s): {:?}",
                            blocking.len(),
                            blocking
                        )),
                    )
                }
            }
            (Some(ci), Some(ti)) if ti <= ci => (
                false,
                Some(format!(
                    "Backward transitions are not lawful: {} -> {}",
                    inst.phase, target_phase
                )),
            ),
            (Some(ci), Some(ti)) if ti > ci + 1 => (
                false,
                Some(format!(
                    "Cannot skip phases: {} -> {} skips {} intermediate phase(s)",
                    inst.phase,
                    target_phase,
                    ti - ci - 1
                )),
            ),
            _ => (
                false,
                Some(format!(
                    "Unknown phase(s): current='{}', target='{}'",
                    inst.phase, target_phase
                )),
            ),
        };
        let result = serde_json::json!({
            "instance_id": instance_id,
            "current_phase": inst.phase,
            "requested_phase": target_phase,
            "admitted": admitted,
            "refused_reason": refused_reason,
            "blocking_diagnostic_count": inst.diagnostics.iter()
                .filter(|d| matches!(d.lsp.severity, Some(lsp_types_max::DiagnosticSeverity::ERROR)))
                .count(),
            "conformance_score": inst.conformance_score(),
            "conformance_grade": inst.conformance_grade().as_str(),
        });
        Ok(result)
    }

    pub(super) fn handle_replay(&self, instance_id: &str) -> Result<serde_json::Value, String> {
        let inst = self
            .instances
            .get(instance_id)
            .ok_or_else(|| format!("Instance not found: {}", instance_id))?;
        let events: Vec<serde_json::Value> = self
            .event_log
            .iter()
            .filter(|e| match e {
                HookEvent::StateTransition {
                    instance_id: id, ..
                }
                | HookEvent::DiagnosticEmitted {
                    instance_id: id, ..
                }
                | HookEvent::DiagnosticCleared {
                    instance_id: id, ..
                }
                | HookEvent::ReceiptEmitted {
                    instance_id: id, ..
                }
                | HookEvent::PolicyStateChanged {
                    instance_id: id, ..
                } => id.0 == instance_id,
                HookEvent::BoundedActionExecuted {
                    instance_id: id, ..
                } => id.0 == instance_id,
                HookEvent::InstanceReset { instance_id: id } => id.0 == instance_id,
            })
            .filter_map(|e| serde_json::to_value(e).ok())
            .collect();
        Ok(serde_json::json!({
            "instance_id": instance_id,
            "receipt_count": inst.receipts.len(),
            "event_count": events.len(),
            "events": events,
        }))
    }

    pub(super) fn handle_release_actuation(
        &mut self,
        instance_id: &str,
    ) -> Result<serde_json::Value, String> {
        let inst = self
            .instances
            .get(instance_id)
            .ok_or_else(|| format!("Instance not found: {}", instance_id))?;
        let score = inst.conformance_score();
        let admitted = inst.diagnostics.is_empty();
        if !admitted {
            return Err(format!(
                "Release refused: {} active diagnostics blocking conformance (score: {})",
                inst.diagnostics.len(),
                score
            ));
        }
        let receipt_id = format!("rcpt-release-{}", instance_id);
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
            "released": true,
            "instance_id": instance_id,
            "conformance_score": score,
            "conformance_grade": ConformanceGrade::from_score(score).as_str(),
            "release_receipt": receipt,
        }))
    }
}
