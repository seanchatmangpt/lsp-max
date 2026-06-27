use crate::andon::andon::{AdmissionGate, AdmissionStatus, AndonBus, AndonEvent};
use crate::andon::core::TruthTable;
use crate::andon::lsp::VirtualDocRegistry;
use lsp_types_max::Diagnostic;

pub struct LspMaxHarness {
    pub bus: AndonBus,
    pub gate: AdmissionGate,
    pub diagnostics: Vec<Diagnostic>,
    pub registry: VirtualDocRegistry,
    pub truth_table: TruthTable,
}

impl LspMaxHarness {
    pub fn new() -> Self {
        Self {
            bus: AndonBus::new(),
            gate: AdmissionGate::new(),
            diagnostics: Vec::new(),
            registry: VirtualDocRegistry::new(),
            truth_table: TruthTable { rows: Vec::new() },
        }
    }

    pub fn push_event(&mut self, event: AndonEvent) {
        self.bus.push(event.clone());
        let diag = crate::andon::lsp::LspPushAdapter::event_to_diagnostic(&event);
        self.diagnostics.push(diag);
        self.gate.evaluate(self.bus.get_events());
    }

    pub fn assert_diagnostic(&self, code: &str) {
        assert!(
            self.diagnostics.iter().any(|d| {
                if let Some(lsp_types_max::NumberOrString::String(c)) = &d.code {
                    c == code
                } else {
                    false
                }
            }),
            "Expected diagnostic {} not found",
            code
        );
    }

    pub fn assert_andon(&self, code: &str) {
        assert!(
            self.bus.get_events().iter().any(|e| e.code == code),
            "Expected ANDON event {} not found",
            code
        );
    }

    pub fn assert_admission_disabled(&self) {
        assert!(
            self.gate.status == AdmissionStatus::Blocked
                || self.gate.status == AdmissionStatus::Stopped
                || self.gate.status == AdmissionStatus::Refused,
            "Admission should be disabled, but is {:?}",
            self.gate.status
        );
    }

    pub fn assert_next_lawful_step(&self, step: &str) {
        assert!(
            self.bus.get_events().iter().any(|e| e.next_lawful_step.as_deref() == Some(step)),
            "Expected next lawful step {} not found in any event",
            step
        );
    }

    pub fn assert_virtual_doc_contains(&self, uri: &str, text: &str) {
        let doc = self.registry.get_doc(uri).unwrap_or_default();
        assert!(
            doc.contains(text),
            "Virtual document {} does not contain text: {}",
            uri,
            text
        );
    }

    pub fn assert_truth_table_row(&self, invariant_id: &str) {
        assert!(
            self.truth_table.rows.iter().any(|r| r.invariant_id == invariant_id),
            "Truth table row for {} not found",
            invariant_id
        );
    }

    pub fn assert_counterfactual_failed(&self, invariant_id: &str) {
        let row = self.truth_table.rows.iter().find(|r| r.invariant_id == invariant_id).unwrap();
        assert_eq!(
            row.counterfactual_case,
            crate::andon::core::ProbeResult::Fail,
            "Counterfactual for {} did not fail",
            invariant_id
        );
    }

    pub fn assert_witness_present(&self, invariant_id: &str) {
        let row = self.truth_table.rows.iter().find(|r| r.invariant_id == invariant_id).unwrap();
        assert!(row.witness.is_some(), "Witness for {} is missing", invariant_id);
    }

    pub fn assert_repair_present(&self, invariant_id: &str) {
        let row = self.truth_table.rows.iter().find(|r| r.invariant_id == invariant_id).unwrap();
        assert!(row.repair.is_some(), "Repair for {} is missing", invariant_id);
    }

    pub fn assert_no_vacuous_green(&self) {
        for row in &self.truth_table.rows {
            if row.verdict == crate::andon::core::ProbeResult::Pass {
                assert!(row.witness.is_some(), "Vacuous green: passed without witness");
                assert_eq!(
                    row.counterfactual_case,
                    crate::andon::core::ProbeResult::Fail,
                    "Vacuous green: counterfactual did not fail"
                );
            }
        }
    }
}
