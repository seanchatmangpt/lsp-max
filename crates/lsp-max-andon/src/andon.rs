use serde::{Deserialize, Serialize};
use crate::core::Severity;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AndonEvent {
    pub id: String,
    pub severity: Severity,
    pub code: String,
    pub title: String,
    pub message: String,

    pub invariant_id: Option<String>,
    pub observed_state: Option<String>,
    pub expected_state: Option<String>,

    pub blocking: bool,
    pub requires_ack: bool,
    pub admission_allowed: bool,

    pub next_lawful_step: Option<String>,
    pub required_command: Option<String>,

    pub evidence_uri: Option<String>,
    pub virtual_doc_uri: Option<String>,
    pub receipt_required: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum AdmissionStatus {
    Open,
    Candidate,
    Blocked,
    Stopped,
    Refused,
    Admitted,
    Published,
    Unknown,
}

pub struct AdmissionGate {
    pub status: AdmissionStatus,
}

impl AdmissionGate {
    pub fn new() -> Self {
        Self {
            status: AdmissionStatus::Unknown,
        }
    }

    pub fn evaluate(&mut self, events: &[AndonEvent]) {
        if events.is_empty() {
            self.status = AdmissionStatus::Candidate;
            return;
        }

        let mut all_allowed = true;
        let mut has_refused = false;
        let mut has_stopped = false;

        for e in events {
            if !e.admission_allowed || e.blocking {
                all_allowed = false;
            }
            if e.severity == Severity::Refuse {
                has_refused = true;
            }
            if e.severity == Severity::Stop {
                has_stopped = true;
            }
        }

        if !all_allowed {
            if has_refused {
                self.status = AdmissionStatus::Refused;
            } else if has_stopped {
                self.status = AdmissionStatus::Stopped;
            } else {
                self.status = AdmissionStatus::Blocked;
            }
        } else {
            self.status = AdmissionStatus::Candidate;
        }
    }
}

pub struct AndonBus {
    events: Vec<AndonEvent>,
}

impl AndonBus {
    pub fn new() -> Self {
        Self { events: Vec::new() }
    }

    pub fn push(&mut self, event: AndonEvent) {
        self.events.push(event);
    }

    pub fn get_events(&self) -> &[AndonEvent] {
        &self.events
    }

    pub fn clear(&mut self) {
        self.events.clear();
    }
}
