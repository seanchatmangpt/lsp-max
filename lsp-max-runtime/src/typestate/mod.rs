//! Typestate machine types for AccessAdmissionLaw.

use crate::sha256::{sha256, validate_and_reconstruct_chain_checked};
use std::fmt::{Debug, Formatter};
use std::marker::PhantomData;

pub struct DeterministicSnapshot {
    pub id: lsp_max_protocol::SnapshotId,
    pub timestamp: u64,
}

impl DeterministicSnapshot {
    pub fn new() -> Self {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or(std::time::Duration::ZERO)
            .as_secs();
        Self {
            id: lsp_max_protocol::SnapshotId(format!("snap-{}", timestamp)),
            timestamp,
        }
    }
}

impl Default for DeterministicSnapshot {
    fn default() -> Self {
        Self::new()
    }
}

/// Represents the compile-time law governing state transitions and protocol behavior.
pub trait Law {
    type Error;
}

/// The specific admission law defined in the OWL ontology.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AccessAdmissionLaw;

impl Law for AccessAdmissionLaw {
    type Error = &'static str;
}

/// Represents a distinct phase in the lifecycle of the machine.
pub trait Phase {}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Uninitialized;
impl Phase for Uninitialized {}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Initializing;
impl Phase for Initializing {}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Initialized;
impl Phase for Initialized {}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ShutDown;
impl Phase for ShutDown {}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Exited;
impl Phase for Exited {}

/// Represents the inner data/state carried during a particular phase.
pub trait Data {}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct EmptyData {
    pub client_capabilities: Option<serde_json::Value>,
    pub server_capabilities: Option<serde_json::Value>,
}
impl Data for EmptyData {}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct InitializingData {
    pub client_capabilities: serde_json::Value,
}
impl Data for InitializingData {}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct InitializedData {
    pub client_capabilities: serde_json::Value,
    pub server_capabilities: serde_json::Value,
}
impl Data for InitializedData {}

/// The zero-cost typestate machine container.
pub struct Machine<L: Law, P: Phase, D: Data> {
    pub _law: PhantomData<L>,
    pub phase: P,
    pub data: D,
}

impl<L: Law, P: Phase + Debug, D: Data + Debug> Debug for Machine<L, P, D> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Machine")
            .field("phase", &self.phase)
            .field("data", &self.data)
            .finish()
    }
}

impl<L: Law, P: Phase + Clone, D: Data + Clone> Clone for Machine<L, P, D> {
    fn clone(&self) -> Self {
        Self {
            _law: PhantomData,
            phase: self.phase.clone(),
            data: self.data.clone(),
        }
    }
}

impl<L: Law, P: Phase, D: Data> Machine<L, P, D> {
    /// Create a new typestate machine.
    pub const fn new(phase: P, data: D) -> Self {
        Self {
            _law: PhantomData,
            phase,
            data,
        }
    }
}

/// Error type for receipt chain validation failures during replay.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChainError {
    /// The chain is empty when it must not be.
    EmptyHistory,
    /// Cryptographic hash mismatch between the declared and computed hash.
    HashMismatch {
        index: usize,
        expected: String,
        got: String,
    },
    /// Receipt ID at a given index does not match the expected value.
    ReceiptIdMismatch { index: usize, detail: String },
    /// Chain is too short to reconstruct the target phase.
    InsufficientHistory { required: usize, got: usize },
    /// History contains more receipts than the protocol allows.
    ExcessHistory { extra: usize },
    /// JSON embedded in a receipt ID could not be parsed.
    ParseError { index: usize, detail: String },
}

impl std::fmt::Display for ChainError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ChainError::EmptyHistory => write!(f, "Receipt chain is empty"),
            ChainError::HashMismatch {
                index,
                expected,
                got,
            } => write!(
                f,
                "Hash mismatch at index {}: expected '{}', got '{}'",
                index, expected, got
            ),
            ChainError::ReceiptIdMismatch { index, detail } => {
                write!(f, "Receipt ID mismatch at index {}: {}", index, detail)
            }
            ChainError::InsufficientHistory { required, got } => write!(
                f,
                "Insufficient history: required {} receipts, got {}",
                required, got
            ),
            ChainError::ExcessHistory { extra } => {
                write!(f, "History contains {} unexpected extra receipts", extra)
            }
            ChainError::ParseError { index, detail } => {
                write!(f, "Parse error at index {}: {}", index, detail)
            }
        }
    }
}

/// Convert a `validate_and_reconstruct_chain_checked` string error into a `ChainError`.
fn chain_err_from_str(e: String) -> ChainError {
    if e.contains("must not be empty") || e.contains("History must not be empty") {
        ChainError::EmptyHistory
    } else if e.contains("Hash mismatch") {
        let index = e
            .split_whitespace()
            .find_map(|w| w.trim_end_matches(':').parse::<usize>().ok())
            .unwrap_or(0);
        ChainError::HashMismatch {
            index,
            expected: String::new(),
            got: e,
        }
    } else if e.contains("Insufficient history") {
        ChainError::InsufficientHistory {
            required: 0,
            got: 0,
        }
    } else if e.contains("unexpected") {
        ChainError::ExcessHistory { extra: 0 }
    } else {
        ChainError::ReceiptIdMismatch {
            index: 0,
            detail: e,
        }
    }
}

/// Enforces the Admit -> Receipt -> Exit -> Replay operational theorem stages.
pub trait TypestateKernel<L: Law, P: Phase, D: Data> {
    type Input;
    type OutputPhase: Phase;
    type OutputData: Data;
    type Receipt;

    /// Validate the input message or action against the Law.
    fn validate(&self, input: &Self::Input) -> Result<(), L::Error>;

    /// Select the next state phase depending on the input.
    fn select(&self, input: &Self::Input) -> Self::OutputPhase;

    /// Admit the input message and transition the machine into the target typestate.
    fn admit(
        self,
        input: Self::Input,
    ) -> Result<Machine<L, Self::OutputPhase, Self::OutputData>, L::Error>;

    /// Produce a deterministic execution receipt containing the transition metadata.
    fn receipt(&self) -> Self::Receipt;

    /// Destroy/Exit the current phase and yield the underlying data.
    fn exit(self) -> D;

    /// Reconstruct the machine state by replaying a ledger of historic receipts.
    /// Returns `Err(ChainError)` when the history is corrupted, malformed, or insufficient.
    fn replay(history: Vec<Self::Receipt>) -> Result<Self, ChainError>
    where
        Self: Sized;
}

// ==========================================
// Transition Definitions (Admit and Consume)
// ==========================================

mod machine;

#[cfg(test)]
mod tests;
