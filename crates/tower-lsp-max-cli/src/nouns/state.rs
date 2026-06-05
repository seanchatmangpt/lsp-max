use clap_noun_verb::Result;
use clap_noun_verb_macros::verb;
use serde::Serialize;

// ==============================================================================
// 1. Domain Tier
// ==============================================================================

/// Represents the overall state in the domain.
#[derive(Debug, Clone, Serialize)]
pub struct ServerState {
    pub id: String,
    pub status: StateStatus,
    pub revision: u64,
}

#[derive(Debug, Clone, Serialize)]
pub enum StateStatus {
    Uninitialized,
    Running,
    Stopped,
}

/// Represents a patch payload for modifying state.
#[derive(Debug, Clone)]
pub struct StatePatch {
    pub status: Option<StateStatus>,
}

// ==============================================================================
// 2. Service Tier
// ==============================================================================

/// Service for managing ServerState lifecycle and operations.
pub struct StateService;

impl StateService {
    pub fn new() -> Self {
        Self
    }

    pub fn dump(&self, state_id: &str) -> ServerState {
        // Mock implementation
        ServerState {
            id: state_id.to_string(),
            status: StateStatus::Running,
            revision: 1,
        }
    }

    pub fn restore(&self, state_id: &str, revision: u64) -> ServerState {
        // Mock implementation
        ServerState {
            id: state_id.to_string(),
            status: StateStatus::Uninitialized,
            revision,
        }
    }

    pub fn verify(&self, state_id: &str) -> bool {
        // Mock implementation
        !state_id.is_empty()
    }

    pub fn patch(&self, state_id: &str, _patch: StatePatch) -> ServerState {
        // Mock implementation
        ServerState {
            id: state_id.to_string(),
            status: StateStatus::Running,
            revision: 2,
        }
    }
}

impl Default for StateService {
    fn default() -> Self {
        Self::new()
    }
}

// ==============================================================================
// 3. CLI Tier
// ==============================================================================

#[derive(Serialize)]
pub struct DumpResult {
    pub state: ServerState,
}

#[verb("dump")]
pub fn dump(state_id: String) -> Result<DumpResult> {
    let service = StateService::new();
    let state = service.dump(&state_id);
    Ok(DumpResult { state })
}

#[derive(Serialize)]
pub struct RestoreResult {
    pub state: ServerState,
    pub restored_revision: u64,
}

#[verb("restore")]
pub fn restore(state_id: String, revision: u64) -> Result<RestoreResult> {
    let service = StateService::new();
    let state = service.restore(&state_id, revision);
    Ok(RestoreResult {
        state,
        restored_revision: revision,
    })
}

#[derive(Serialize)]
pub struct VerifyResult {
    pub state_id: String,
    pub is_valid: bool,
}

#[verb("verify")]
pub fn verify(state_id: String) -> Result<VerifyResult> {
    let service = StateService::new();
    let is_valid = service.verify(&state_id);
    Ok(VerifyResult { state_id, is_valid })
}

#[derive(Serialize)]
pub struct PatchResult {
    pub state: ServerState,
}

#[verb("patch")]
pub fn patch(state_id: String, status_override: Option<String>) -> Result<PatchResult> {
    let service = StateService::new();

    let parsed_status = match status_override.as_deref() {
        Some("running") => Some(StateStatus::Running),
        Some("stopped") => Some(StateStatus::Stopped),
        Some("uninitialized") => Some(StateStatus::Uninitialized),
        _ => None,
    };

    let state_patch = StatePatch {
        status: parsed_status,
    };

    let state = service.patch(&state_id, state_patch);
    Ok(PatchResult { state })
}
