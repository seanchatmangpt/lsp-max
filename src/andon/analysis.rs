use crate::andon::andon::AndonEvent;
use crate::andon::core::{InvariantRegistry, Severity};

pub struct AnalysisPipeline;

impl AnalysisPipeline {
    pub fn evaluate_registry(registry: &InvariantRegistry) -> Vec<AndonEvent> {
        let mut events = Vec::new();

        if registry.is_empty() {
            events.push(AndonEvent {
                id: "LSPMAX-INVARIANT-EMPTY-REGISTRY".to_string(),
                severity: Severity::Stop,
                code: "LSPMAX-INVARIANT-EMPTY-REGISTRY".to_string(),
                title: "Empty Invariant Registry".to_string(),
                message: "InvariantRegistry.empty() implies ANDON. A project with no invariants is blind.".to_string(),
                invariant_id: None,
                observed_state: Some("invariants_count=0".to_string()),
                expected_state: Some("invariants_count>0".to_string()),
                blocking: true,
                requires_ack: true,
                admission_allowed: false,
                next_lawful_step: Some("define_project_invariants".to_string()),
                required_command: Some("bcinrPddl.openVirtualDocument".to_string()),
                evidence_uri: None,
                virtual_doc_uri: Some("lsp-max://truth/andon".to_string()),
                receipt_required: false,
            });
            return events;
        }

        for inv in registry.get_all() {
            if inv.true_probe.is_none() {
                events.push(Self::build_missing_probe_event(
                    inv.id.clone(),
                    "LSPMAX-INVARIANT-TRUE-CASE-MISSING",
                    "TRUE",
                    "true_probe missing",
                ));
            }
            if inv.false_probe.is_none() {
                events.push(Self::build_missing_probe_event(
                    inv.id.clone(),
                    "LSPMAX-INVARIANT-FALSE-CASE-MISSING",
                    "FALSE",
                    "false_probe missing",
                ));
            }
            if inv.counterfactual_probe.is_none() {
                events.push(Self::build_missing_probe_event(
                    inv.id.clone(),
                    "LSPMAX-INVARIANT-COUNTERFACTUAL-MISSING",
                    "COUNTERFACTUAL",
                    "counterfactual_probe missing",
                ));
            }
            if inv.witness_rule.is_none() {
                events.push(Self::build_missing_probe_event(
                    inv.id.clone(),
                    "LSPMAX-WITNESS-MISSING",
                    "WITNESS",
                    "witness missing",
                ));
            }
            if inv.blocks && inv.repair_rule.is_none() {
                events.push(Self::build_missing_probe_event(
                    inv.id.clone(),
                    "LSPMAX-REPAIR-MISSING",
                    "REPAIR",
                    "repair missing on block",
                ));
            }
        }

        events
    }

    fn build_missing_probe_event(
        invariant_id: String,
        code: &str,
        probe_type: &str,
        message: &str,
    ) -> AndonEvent {
        AndonEvent {
            id: format!("andon-{}-{}", invariant_id, code.to_lowercase()),
            severity: Severity::Stop,
            code: code.to_string(),
            title: format!("Missing {} in {}", probe_type, invariant_id),
            message: message.to_string(),
            invariant_id: Some(invariant_id),
            observed_state: Some(format!("{} missing", probe_type)),
            expected_state: Some(format!("{} defined", probe_type)),
            blocking: true,
            requires_ack: true,
            admission_allowed: false,
            next_lawful_step: Some(format!("define_{}", probe_type.to_lowercase())),
            required_command: None,
            evidence_uri: None,
            virtual_doc_uri: Some("lsp-max://truth/andon".to_string()),
            receipt_required: false,
        }
    }
}
