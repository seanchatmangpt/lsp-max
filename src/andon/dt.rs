use crate::andon::andon::AndonEvent;
use crate::andon::core::RepairAction;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DtContext {
    pub seq: Option<u64>,
    pub workspace_id: String,
    pub timestamp: String,
    pub admission_allowed: Option<bool>,
    pub active_andon_codes: Vec<String>,
    pub active_invariant_ids: Vec<String>,
    pub governing_axes: Vec<String>,
    pub events: Vec<AndonEvent>,
    pub repairs: Vec<RepairAction>,
    pub required_commands: Vec<String>,
    pub virtual_doc_uris: Vec<String>,
    pub gate_file: Option<String>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum DtStatus {
    Valid,
    Refused,
    Stop,
}

impl DtContext {
    pub fn empty() -> Self {
        Self {
            seq: None,
            workspace_id: String::new(),
            timestamp: String::new(),
            admission_allowed: None,
            active_andon_codes: Vec::new(),
            active_invariant_ids: Vec::new(),
            governing_axes: Vec::new(),
            events: Vec::new(),
            repairs: Vec::new(),
            required_commands: Vec::new(),
            virtual_doc_uris: Vec::new(),
            gate_file: None,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.active_andon_codes.is_empty()
            && self.active_invariant_ids.is_empty()
            && self.events.is_empty()
    }

    pub fn validate(&self, has_active_gate: bool) -> DtStatus {
        if self.is_empty() && has_active_gate {
            return DtStatus::Refused;
        }

        if self.seq.is_none() {
            return DtStatus::Stop;
        }

        if self.admission_allowed.is_none() {
            return DtStatus::Stop;
        }

        let has_blocking_events = self.events.iter().any(|e| e.blocking);
        if has_blocking_events && self.repairs.is_empty() {
            return DtStatus::Stop;
        }

        DtStatus::Valid
    }
}
