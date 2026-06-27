use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct DtContext {
    pub seq: Option<u64>,
    pub workspace_id: Option<String>,
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AndonEvent {
    pub code: String,
    pub blocking: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RepairAction {
    pub next_lawful_step: String,
    pub required_command: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GateContext {
    pub admission_allowed: bool,
    pub active_andon_codes: Vec<String>,
    pub active_invariant_ids: Vec<String>,
    pub governing_axes: Vec<String>,
    pub available_repairs: Vec<RepairAction>,
    pub required_commands: Vec<String>,
    pub virtual_doc_uris: Vec<String>,
    pub since_seq: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DtContextStatus {
    Valid,
    Stop(String),
    Refused(String),
}

impl DtContext {
    pub fn empty() -> Self {
        Self {
            seq: None,
            workspace_id: None,
            timestamp: String::new(),
            admission_allowed: None,
            active_andon_codes: vec![],
            active_invariant_ids: vec![],
            governing_axes: vec![],
            events: vec![],
            repairs: vec![],
            required_commands: vec![],
            virtual_doc_uris: vec![],
            gate_file: None,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.active_andon_codes.is_empty()
            && self.active_invariant_ids.is_empty()
            && self.events.is_empty()
            && self.repairs.is_empty()
    }

    pub fn validate(&self, is_gate_active: bool, active_andon_seq: Option<u64>) -> DtContextStatus {
        if is_gate_active && self.is_empty() {
            return DtContextStatus::Refused("LSPMAX-DT-CONTEXT-EMPTY-WHILE-BLOCKED".to_string());
        }
        if self.seq.is_none() {
            return DtContextStatus::Stop("LSPMAX-DT-CONTEXT-MISSING".to_string());
        }
        if let (Some(seq), Some(active_seq)) = (self.seq, active_andon_seq) {
            if seq < active_seq {
                return DtContextStatus::Stop("LSPMAX-DT-CONTEXT-STALE".to_string());
            }
        }
        if self.admission_allowed.is_none() {
            return DtContextStatus::Refused("LSPMAX-ADMISSION-GATE-NOT-UPDATED".to_string());
        }

        let has_blocking_events = self.events.iter().any(|e| e.blocking);
        if has_blocking_events && self.repairs.is_empty() {
            return DtContextStatus::Stop("LSPMAX-REPAIR-MISSING".to_string());
        }

        DtContextStatus::Valid
    }

    pub fn render_gate_context(&self) -> Result<String, String> {
        let seq = self.seq.ok_or_else(|| "LSPMAX-GATE-CONTEXT-MISSING".to_string())?;
        let admission_allowed = self.admission_allowed.ok_or_else(|| "LSPMAX-GATE-CHECK-FORMAT-MISSING".to_string())?;

        let gate_ctx = GateContext {
            admission_allowed,
            active_andon_codes: self.active_andon_codes.clone(),
            active_invariant_ids: self.active_invariant_ids.clone(),
            governing_axes: self.governing_axes.clone(),
            available_repairs: self.repairs.clone(),
            required_commands: self.required_commands.clone(),
            virtual_doc_uris: self.virtual_doc_uris.clone(),
            since_seq: seq,
        };

        let json = serde_json::to_string_pretty(&gate_ctx)
            .map_err(|e| format!("Serialization error: {}", e))?;

        Ok(format!("<gate-context>\n{}\n</gate-context>", json))
    }
}
