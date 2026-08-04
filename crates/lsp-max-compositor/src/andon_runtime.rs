//! Live bridge from the ANDON invariant court into compositor state.
//!
//! The compositor previously constructed a registry, bus, and snapshot but never
//! connected them.  This module is the single refresh boundary: it evaluates the
//! registry, incorporates currently observed diagnostic law violations, replaces
//! the live bus contents, and commits the corresponding D_t snapshot.

use lsp_max::max_andon::analysis::AnalysisPipeline;
use lsp_max::max_andon::andon::{AndonBus, AndonEvent as LawEvent};
use lsp_max::max_andon::core::{InvariantRegistry, Severity};

use crate::andon_snapshot::AndonSnapshot;
use crate::dt_context::{AndonEvent, RepairAction};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AndonRefresh {
    pub event_count: usize,
    pub blocking: bool,
    pub admission_allowed: bool,
    pub sequence: u64,
}

/// Re-evaluate the live ANDON court and atomically publish its observable state.
///
/// `diagnostic_codes` must contain only currently observed, error-severity law
/// violations.  Passing historical or advisory diagnostics would manufacture a
/// block and is therefore outside this boundary's admission contract.
pub fn refresh(
    registry: &InvariantRegistry,
    bus: &mut AndonBus,
    snapshot: &AndonSnapshot,
    diagnostic_codes: impl IntoIterator<Item = String>,
) -> AndonRefresh {
    let mut events = AnalysisPipeline::evaluate_registry(registry);
    events.extend(diagnostic_codes.into_iter().map(diagnostic_event));

    bus.clear();
    for event in &events {
        bus.push(event.clone());
    }

    let active_codes = events
        .iter()
        .filter(|event| event.blocking)
        .map(|event| event.code.clone())
        .collect();
    let governing_axes = events
        .iter()
        .filter_map(|event| event.invariant_id.clone())
        .collect();
    let snapshot_events = events
        .iter()
        .map(|event| AndonEvent {
            code: event.code.clone(),
            blocking: event.blocking,
        })
        .collect();
    let repairs = events
        .iter()
        .filter(|event| event.blocking)
        .filter_map(|event| {
            event.next_lawful_step.as_ref().map(|step| RepairAction {
                next_lawful_step: step.clone(),
                required_command: event.required_command.clone().unwrap_or_default(),
            })
        })
        .collect();

    snapshot.commit_new_state(active_codes, governing_axes, snapshot_events, repairs);
    let context = snapshot.get_context();
    let blocking = events.iter().any(|event| event.blocking);

    AndonRefresh {
        event_count: events.len(),
        blocking,
        admission_allowed: !blocking,
        sequence: context.seq.unwrap_or(0),
    }
}

fn diagnostic_event(code: String) -> LawEvent {
    LawEvent {
        id: format!("compositor-diagnostic-{code}"),
        severity: Severity::Stop,
        title: format!("Compositor law violation {code}"),
        message: "A child diagnostic matched its admitted ANDON law-collapse function.".to_string(),
        invariant_id: Some(code.clone()),
        observed_state: Some("error-severity law diagnostic active".to_string()),
        expected_state: Some("no active law diagnostic".to_string()),
        code,
        blocking: true,
        requires_ack: true,
        admission_allowed: false,
        next_lawful_step: Some("repair_originating_child_violation".to_string()),
        required_command: None,
        evidence_uri: None,
        virtual_doc_uri: Some("lsp-max://truth/andon".to_string()),
        receipt_required: true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lsp_max::max_andon::patterns::build_receipt_required;

    #[test]
    fn empty_registry_stops_and_updates_bus_and_snapshot() {
        let registry = InvariantRegistry::new();
        let mut bus = AndonBus::new();
        let snapshot = AndonSnapshot::new();

        let state = refresh(&registry, &mut bus, &snapshot, []);

        assert!(state.blocking);
        assert!(!state.admission_allowed);
        assert_eq!(state.event_count, 1);
        assert_eq!(bus.get_events().len(), 1);
        assert_eq!(snapshot.get_context().admission_allowed, Some(false));
    }

    #[test]
    fn complete_registry_is_clear_until_observed_diagnostic_blocks() {
        let mut registry = InvariantRegistry::new();
        registry.register(build_receipt_required());
        let mut bus = AndonBus::new();
        let snapshot = AndonSnapshot::new();

        let clear = refresh(&registry, &mut bus, &snapshot, []);
        assert_eq!(clear.event_count, 0);
        assert!(clear.admission_allowed);
        assert!(bus.get_events().is_empty());

        let blocked = refresh(
            &registry,
            &mut bus,
            &snapshot,
            ["WASM4PM-CROWN-RECEIPT-MISSING".to_string()],
        );
        assert!(blocked.blocking);
        assert_eq!(blocked.sequence, clear.sequence + 1);
        assert_eq!(bus.get_events()[0].code, "WASM4PM-CROWN-RECEIPT-MISSING");
        assert_eq!(
            snapshot.get_context().active_andon_codes,
            vec!["WASM4PM-CROWN-RECEIPT-MISSING"]
        );
    }
}
