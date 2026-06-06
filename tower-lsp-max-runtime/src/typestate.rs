use crate::sha256::{sha256, validate_and_reconstruct_chain_checked};
use std::fmt::{Debug, Formatter};
use std::marker::PhantomData;

pub struct DeterministicSnapshot {
    pub id: tower_lsp_max_protocol::SnapshotId,
    pub timestamp: u64,
}

impl DeterministicSnapshot {
    pub fn new() -> Self {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or(std::time::Duration::ZERO)
            .as_secs();
        Self {
            id: tower_lsp_max_protocol::SnapshotId(format!("snap-{}", timestamp)),
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

impl Machine<AccessAdmissionLaw, Uninitialized, EmptyData> {
    /// Transitions from Uninitialized to Initializing using the client parameters.
    pub fn admit_initialize(
        self,
        client_caps: serde_json::Value,
    ) -> Machine<AccessAdmissionLaw, Initializing, InitializingData> {
        Machine::new(
            Initializing,
            InitializingData {
                client_capabilities: client_caps,
            },
        )
    }
}

impl Machine<AccessAdmissionLaw, Initializing, InitializingData> {
    /// Transitions from Initializing to Initialized once server capabilities are registered.
    pub fn admit_initialized(
        self,
        server_caps: serde_json::Value,
    ) -> Machine<AccessAdmissionLaw, Initialized, InitializedData> {
        Machine::new(
            Initialized,
            InitializedData {
                client_capabilities: self.data.client_capabilities,
                server_capabilities: server_caps,
            },
        )
    }
}

impl Machine<AccessAdmissionLaw, Initialized, InitializedData> {
    /// Transitions from Initialized to ShutDown.
    pub fn admit_shutdown(self) -> Machine<AccessAdmissionLaw, ShutDown, EmptyData> {
        Machine::new(
            ShutDown,
            EmptyData {
                client_capabilities: Some(self.data.client_capabilities),
                server_capabilities: Some(self.data.server_capabilities),
            },
        )
    }
}

impl Machine<AccessAdmissionLaw, ShutDown, EmptyData> {
    /// Transitions from ShutDown to Exited.
    pub fn admit_exit(self) -> Machine<AccessAdmissionLaw, Exited, EmptyData> {
        Machine::new(
            Exited,
            EmptyData {
                client_capabilities: self.data.client_capabilities,
                server_capabilities: self.data.server_capabilities,
            },
        )
    }
}

// ==========================================
// TypestateKernel Implementations
// ==========================================

impl TypestateKernel<AccessAdmissionLaw, Uninitialized, EmptyData>
    for Machine<AccessAdmissionLaw, Uninitialized, EmptyData>
{
    type Input = serde_json::Value;
    type OutputPhase = Initializing;
    type OutputData = InitializingData;
    type Receipt = tower_lsp_max_protocol::Receipt;

    fn validate(&self, _input: &Self::Input) -> Result<(), <AccessAdmissionLaw as Law>::Error> {
        Ok(())
    }

    fn select(&self, _input: &Self::Input) -> Self::OutputPhase {
        Initializing
    }

    fn admit(
        self,
        input: Self::Input,
    ) -> Result<
        Machine<AccessAdmissionLaw, Self::OutputPhase, Self::OutputData>,
        <AccessAdmissionLaw as Law>::Error,
    > {
        self.validate(&input)?;
        Ok(self.admit_initialize(input))
    }

    fn receipt(&self) -> Self::Receipt {
        let receipt_id = "rcpt-uninitialized".to_string();
        let hash = sha256(receipt_id.as_bytes());
        tower_lsp_max_protocol::Receipt {
            receipt_id,
            hash,
            prev_receipt_hash: None,
        }
    }

    fn exit(self) -> EmptyData {
        self.data
    }

    fn replay(history: Vec<Self::Receipt>) -> Result<Self, ChainError> {
        validate_and_reconstruct_chain_checked(&history).map_err(chain_err_from_str)?;
        Ok(Machine::new(Uninitialized, EmptyData::default()))
    }
}

impl TypestateKernel<AccessAdmissionLaw, Initializing, InitializingData>
    for Machine<AccessAdmissionLaw, Initializing, InitializingData>
{
    type Input = serde_json::Value;
    type OutputPhase = Initialized;
    type OutputData = InitializedData;
    type Receipt = tower_lsp_max_protocol::Receipt;

    fn validate(&self, _input: &Self::Input) -> Result<(), <AccessAdmissionLaw as Law>::Error> {
        Ok(())
    }

    fn select(&self, _input: &Self::Input) -> Self::OutputPhase {
        Initialized
    }

    fn admit(
        self,
        input: Self::Input,
    ) -> Result<
        Machine<AccessAdmissionLaw, Self::OutputPhase, Self::OutputData>,
        <AccessAdmissionLaw as Law>::Error,
    > {
        self.validate(&input)?;
        Ok(self.admit_initialized(input))
    }

    fn receipt(&self) -> Self::Receipt {
        let client_caps_json = serde_json::to_string(&self.data.client_capabilities)
            .unwrap_or_else(|_| "<unserializable>".to_string());
        let receipt_id = format!("rcpt-uninitialized-to-initializing:{}", client_caps_json);
        let prev_hash = sha256(b"rcpt-uninitialized");
        let hash = sha256(format!("{}:{}", prev_hash, receipt_id).as_bytes());
        tower_lsp_max_protocol::Receipt {
            receipt_id,
            hash,
            prev_receipt_hash: Some(prev_hash),
        }
    }

    fn exit(self) -> InitializingData {
        self.data
    }

    fn replay(history: Vec<Self::Receipt>) -> Result<Self, ChainError> {
        if history.len() < 2 {
            return Err(ChainError::InsufficientHistory {
                required: 2,
                got: history.len(),
            });
        }
        let (client_caps, _) =
            validate_and_reconstruct_chain_checked(&history).map_err(chain_err_from_str)?;
        Ok(Machine::new(
            Initializing,
            InitializingData {
                client_capabilities: client_caps,
            },
        ))
    }
}

impl TypestateKernel<AccessAdmissionLaw, Initialized, InitializedData>
    for Machine<AccessAdmissionLaw, Initialized, InitializedData>
{
    type Input = ();
    type OutputPhase = ShutDown;
    type OutputData = EmptyData;
    type Receipt = tower_lsp_max_protocol::Receipt;

    fn validate(&self, _input: &Self::Input) -> Result<(), <AccessAdmissionLaw as Law>::Error> {
        Ok(())
    }

    fn select(&self, _input: &Self::Input) -> Self::OutputPhase {
        ShutDown
    }

    fn admit(
        self,
        input: Self::Input,
    ) -> Result<
        Machine<AccessAdmissionLaw, Self::OutputPhase, Self::OutputData>,
        <AccessAdmissionLaw as Law>::Error,
    > {
        self.validate(&input)?;
        Ok(self.admit_shutdown())
    }

    fn receipt(&self) -> Self::Receipt {
        let client_caps_json = serde_json::to_string(&self.data.client_capabilities)
            .unwrap_or_else(|_| "<unserializable>".to_string());
        let server_caps_json = serde_json::to_string(&self.data.server_capabilities)
            .unwrap_or_else(|_| "<unserializable>".to_string());
        let hash_0 = sha256(b"rcpt-uninitialized");
        let rcpt_1 = format!("rcpt-uninitialized-to-initializing:{}", client_caps_json);
        let prev_hash = sha256(format!("{}:{}", hash_0, rcpt_1).as_bytes());
        let receipt_id = format!("rcpt-initializing-to-initialized:{}", server_caps_json);
        let hash = sha256(format!("{}:{}", prev_hash, receipt_id).as_bytes());
        tower_lsp_max_protocol::Receipt {
            receipt_id,
            hash,
            prev_receipt_hash: Some(prev_hash),
        }
    }

    fn exit(self) -> InitializedData {
        self.data
    }

    fn replay(history: Vec<Self::Receipt>) -> Result<Self, ChainError> {
        if history.len() < 3 {
            return Err(ChainError::InsufficientHistory {
                required: 3,
                got: history.len(),
            });
        }
        let (client_caps, server_caps) =
            validate_and_reconstruct_chain_checked(&history).map_err(chain_err_from_str)?;
        Ok(Machine::new(
            Initialized,
            InitializedData {
                client_capabilities: client_caps,
                server_capabilities: server_caps,
            },
        ))
    }
}

impl TypestateKernel<AccessAdmissionLaw, ShutDown, EmptyData>
    for Machine<AccessAdmissionLaw, ShutDown, EmptyData>
{
    type Input = ();
    type OutputPhase = Exited;
    type OutputData = EmptyData;
    type Receipt = tower_lsp_max_protocol::Receipt;

    fn validate(&self, _input: &Self::Input) -> Result<(), <AccessAdmissionLaw as Law>::Error> {
        Ok(())
    }

    fn select(&self, _input: &Self::Input) -> Self::OutputPhase {
        Exited
    }

    fn admit(
        self,
        input: Self::Input,
    ) -> Result<
        Machine<AccessAdmissionLaw, Self::OutputPhase, Self::OutputData>,
        <AccessAdmissionLaw as Law>::Error,
    > {
        self.validate(&input)?;
        Ok(self.admit_exit())
    }

    fn receipt(&self) -> Self::Receipt {
        let client_caps_json = self
            .data
            .client_capabilities
            .as_ref()
            .map(|c| serde_json::to_string(c).unwrap_or_else(|_| "<unserializable>".to_string()))
            .unwrap_or_else(|| "null".to_string());
        let server_caps_json = self
            .data
            .server_capabilities
            .as_ref()
            .map(|s| serde_json::to_string(s).unwrap_or_else(|_| "<unserializable>".to_string()))
            .unwrap_or_else(|| "null".to_string());
        let hash_0 = sha256(b"rcpt-uninitialized");
        let rcpt_1 = format!("rcpt-uninitialized-to-initializing:{}", client_caps_json);
        let hash_1 = sha256(format!("{}:{}", hash_0, rcpt_1).as_bytes());
        let rcpt_2 = format!("rcpt-initializing-to-initialized:{}", server_caps_json);
        let prev_hash = sha256(format!("{}:{}", hash_1, rcpt_2).as_bytes());
        let receipt_id = "rcpt-initialized-to-shutdown".to_string();
        let hash = sha256(format!("{}:{}", prev_hash, receipt_id).as_bytes());
        tower_lsp_max_protocol::Receipt {
            receipt_id,
            hash,
            prev_receipt_hash: Some(prev_hash),
        }
    }

    fn exit(self) -> EmptyData {
        self.data
    }

    fn replay(history: Vec<Self::Receipt>) -> Result<Self, ChainError> {
        if history.len() < 4 {
            return Err(ChainError::InsufficientHistory {
                required: 4,
                got: history.len(),
            });
        }
        let (client_caps, server_caps) =
            validate_and_reconstruct_chain_checked(&history).map_err(chain_err_from_str)?;
        Ok(Machine::new(
            ShutDown,
            EmptyData {
                client_capabilities: Some(client_caps),
                server_capabilities: Some(server_caps),
            },
        ))
    }
}

impl TypestateKernel<AccessAdmissionLaw, Exited, EmptyData>
    for Machine<AccessAdmissionLaw, Exited, EmptyData>
{
    type Input = ();
    type OutputPhase = Exited;
    type OutputData = EmptyData;
    type Receipt = tower_lsp_max_protocol::Receipt;

    fn validate(&self, _input: &Self::Input) -> Result<(), <AccessAdmissionLaw as Law>::Error> {
        Err("Already exited")
    }

    fn select(&self, _input: &Self::Input) -> Self::OutputPhase {
        Exited
    }

    fn admit(
        self,
        _input: Self::Input,
    ) -> Result<
        Machine<AccessAdmissionLaw, Self::OutputPhase, Self::OutputData>,
        <AccessAdmissionLaw as Law>::Error,
    > {
        self.validate(&_input)?;
        Ok(self)
    }

    fn receipt(&self) -> Self::Receipt {
        let client_caps_json = self
            .data
            .client_capabilities
            .as_ref()
            .map(|c| serde_json::to_string(c).unwrap_or_else(|_| "<unserializable>".to_string()))
            .unwrap_or_else(|| "null".to_string());
        let server_caps_json = self
            .data
            .server_capabilities
            .as_ref()
            .map(|s| serde_json::to_string(s).unwrap_or_else(|_| "<unserializable>".to_string()))
            .unwrap_or_else(|| "null".to_string());
        let hash_0 = sha256(b"rcpt-uninitialized");
        let rcpt_1 = format!("rcpt-uninitialized-to-initializing:{}", client_caps_json);
        let hash_1 = sha256(format!("{}:{}", hash_0, rcpt_1).as_bytes());
        let rcpt_2 = format!("rcpt-initializing-to-initialized:{}", server_caps_json);
        let hash_2 = sha256(format!("{}:{}", hash_1, rcpt_2).as_bytes());
        let rcpt_3 = "rcpt-initialized-to-shutdown".to_string();
        let prev_hash = sha256(format!("{}:{}", hash_2, rcpt_3).as_bytes());
        let receipt_id = "rcpt-shutdown-to-exited".to_string();
        let hash = sha256(format!("{}:{}", prev_hash, receipt_id).as_bytes());
        tower_lsp_max_protocol::Receipt {
            receipt_id,
            hash,
            prev_receipt_hash: Some(prev_hash),
        }
    }

    fn exit(self) -> EmptyData {
        self.data
    }

    fn replay(history: Vec<Self::Receipt>) -> Result<Self, ChainError> {
        if history.len() < 5 {
            return Err(ChainError::InsufficientHistory {
                required: 5,
                got: history.len(),
            });
        }
        let (client_caps, server_caps) =
            validate_and_reconstruct_chain_checked(&history).map_err(chain_err_from_str)?;
        Ok(Machine::new(
            Exited,
            EmptyData {
                client_capabilities: Some(client_caps),
                server_capabilities: Some(server_caps),
            },
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_typestate_lifecycle() {
        let machine = Machine::new(Uninitialized, EmptyData::default());
        let rcpt_uninit = machine.receipt();
        assert_eq!(rcpt_uninit.receipt_id, "rcpt-uninitialized");
        assert_eq!(rcpt_uninit.hash, sha256(b"rcpt-uninitialized"));

        let client_caps = serde_json::json!({"textDocument": {"hover": {}}});
        let machine = machine.admit_initialize(client_caps.clone());
        assert_eq!(machine.data.client_capabilities, client_caps);
        let rcpt_initializing = machine.receipt();
        let expected_init_id = format!(
            "rcpt-uninitialized-to-initializing:{}",
            serde_json::to_string(&client_caps).unwrap_or_else(|_| "<unserializable>".to_string())
        );
        assert_eq!(rcpt_initializing.receipt_id, expected_init_id);
        let expected_init_hash =
            sha256(format!("{}:{}", rcpt_uninit.hash, expected_init_id).as_bytes());
        assert_eq!(rcpt_initializing.hash, expected_init_hash);

        let server_caps = serde_json::json!({"hoverProvider": true});
        let machine = machine.admit_initialized(server_caps.clone());
        assert_eq!(machine.data.client_capabilities, client_caps);
        assert_eq!(machine.data.server_capabilities, server_caps);
        let rcpt_initialized = machine.receipt();
        let expected_initialized_id = format!(
            "rcpt-initializing-to-initialized:{}",
            serde_json::to_string(&server_caps).unwrap_or_else(|_| "<unserializable>".to_string())
        );
        assert_eq!(rcpt_initialized.receipt_id, expected_initialized_id);
        let expected_initialized_hash =
            sha256(format!("{}:{}", rcpt_initializing.hash, expected_initialized_id).as_bytes());
        assert_eq!(rcpt_initialized.hash, expected_initialized_hash);

        let machine = machine.admit_shutdown();
        let rcpt_shutdown = machine.receipt();
        let expected_shutdown_id = "rcpt-initialized-to-shutdown";
        assert_eq!(rcpt_shutdown.receipt_id, expected_shutdown_id);
        let expected_shutdown_hash =
            sha256(format!("{}:{}", rcpt_initialized.hash, expected_shutdown_id).as_bytes());
        assert_eq!(rcpt_shutdown.hash, expected_shutdown_hash);

        let machine = machine.admit_exit();
        let rcpt_exited = machine.receipt();
        let expected_exited_id = "rcpt-shutdown-to-exited";
        assert_eq!(rcpt_exited.receipt_id, expected_exited_id);
        let expected_exited_hash =
            sha256(format!("{}:{}", rcpt_shutdown.hash, expected_exited_id).as_bytes());
        assert_eq!(rcpt_exited.hash, expected_exited_hash);
    }

    #[test]
    fn test_kernel_admit_replay() {
        let machine_uninit = Machine::new(Uninitialized, EmptyData::default());
        let rcpt_0 = machine_uninit.receipt();

        let client_caps = serde_json::json!({"textDocument": {"completion": {}}});
        let machine_initializing = <Machine<AccessAdmissionLaw, Uninitialized, EmptyData> as TypestateKernel<
            _,
            _,
            _,
        >>::admit(machine_uninit, client_caps.clone()).unwrap();
        let rcpt_1 = machine_initializing.receipt();

        let server_caps = serde_json::json!({"completionProvider": {}});
        let machine_initialized =
            <Machine<AccessAdmissionLaw, Initializing, InitializingData> as TypestateKernel<
                _,
                _,
                _,
            >>::admit(machine_initializing, server_caps.clone())
            .unwrap();
        let rcpt_2 = machine_initialized.receipt();

        let machine_shutdown =
            <Machine<AccessAdmissionLaw, Initialized, InitializedData> as TypestateKernel<
                _,
                _,
                _,
            >>::admit(machine_initialized, ())
            .unwrap();
        let rcpt_3 = machine_shutdown.receipt();

        let machine_exited =
            <Machine<AccessAdmissionLaw, ShutDown, EmptyData> as TypestateKernel<_, _, _>>::admit(
                machine_shutdown,
                (),
            )
            .unwrap();
        let rcpt_4 = machine_exited.receipt();

        let history = vec![
            rcpt_0.clone(),
            rcpt_1.clone(),
            rcpt_2.clone(),
            rcpt_3.clone(),
            rcpt_4.clone(),
        ];

        let replayed_exited =
            <Machine<AccessAdmissionLaw, Exited, EmptyData> as TypestateKernel<_, _, _>>::replay(
                history.clone(),
            );
        let replayed_exited_ok = replayed_exited.expect("replay must succeed");
        assert_eq!(
            replayed_exited_ok
                .data
                .client_capabilities
                .as_ref()
                .unwrap(),
            &client_caps
        );
        assert_eq!(
            replayed_exited_ok
                .data
                .server_capabilities
                .as_ref()
                .unwrap(),
            &server_caps
        );

        let mut tampered_history = history.clone();
        tampered_history[1].receipt_id = "rcpt-uninitialized-to-initializing:{}".to_string();
        let tampered_result =
            <Machine<AccessAdmissionLaw, Exited, EmptyData> as TypestateKernel<_, _, _>>::replay(
                tampered_history,
            );
        assert!(
            tampered_result.is_err(),
            "Replaying a tampered receipt history must return Err"
        );

        let mut tampered_history_hash = history.clone();
        tampered_history_hash[2].hash = "wrong_hash".to_string();
        let tampered_hash_result =
            <Machine<AccessAdmissionLaw, Exited, EmptyData> as TypestateKernel<_, _, _>>::replay(
                tampered_history_hash,
            );
        assert!(
            tampered_hash_result.is_err(),
            "Replaying a tampered hash must return Err"
        );
    }
}
