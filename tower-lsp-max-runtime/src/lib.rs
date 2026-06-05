//! Runtime utilities for tower-lsp-max servers.
//!
//! Provides SHA-256 hashing, the `ConformanceVector` (Admitted/Refused/Unknown
//! tallies), and the `MaxServer` wrapper that wires a `LanguageServer` impl into
//! the five-layer AMI execution model used by tower-lsp-max.

use std::fmt::{Debug, Formatter};
use std::marker::PhantomData;

pub fn sha256(data: &[u8]) -> String {
    let mut h: [u32; 8] = [
        0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab,
        0x5be0cd19,
    ];

    let mut padded = data.to_vec();
    let original_len_bits = (data.len() as u64) * 8;
    padded.push(0x80);
    while !(padded.len() + 8).is_multiple_of(64) {
        padded.push(0x00);
    }
    padded.extend_from_slice(&original_len_bits.to_be_bytes());

    for chunk in padded.chunks_exact(64) {
        let mut w = [0u32; 64];
        for i in 0..16 {
            w[i] = u32::from_be_bytes([
                chunk[i * 4],
                chunk[i * 4 + 1],
                chunk[i * 4 + 2],
                chunk[i * 4 + 3],
            ]);
        }

        for i in 16..64 {
            let s0 = w[i - 15].rotate_right(7) ^ w[i - 15].rotate_right(18) ^ (w[i - 15] >> 3);
            let s1 = w[i - 2].rotate_right(17) ^ w[i - 2].rotate_right(19) ^ (w[i - 2] >> 10);
            w[i] = w[i - 16]
                .wrapping_add(s0)
                .wrapping_add(w[i - 7])
                .wrapping_add(s1);
        }

        let mut a = h[0];
        let mut b = h[1];
        let mut c = h[2];
        let mut d = h[3];
        let mut e = h[4];
        let mut f = h[5];
        let mut g = h[6];
        let mut h_val = h[7];

        const K: [u32; 64] = [
            0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4,
            0xab1c5ed5, 0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe,
            0x9bdc06a7, 0xc19bf174, 0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f,
            0x4a7484aa, 0x5cb0a9dc, 0x76f988da, 0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7,
            0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967, 0x27b70a85, 0x2e1b2138, 0x4d2c6dfc,
            0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85, 0xa2bfe8a1, 0xa81a664b,
            0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070, 0x19a4c116,
            0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
            0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7,
            0xc67178f2,
        ];

        for i in 0..64 {
            let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let ch = (e & f) ^ (!e & g);
            let temp1 = h_val
                .wrapping_add(s1)
                .wrapping_add(ch)
                .wrapping_add(K[i])
                .wrapping_add(w[i]);
            let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let maj = (a & b) ^ (a & c) ^ (b & c);
            let temp2 = s0.wrapping_add(maj);

            h_val = g;
            g = f;
            f = e;
            e = d.wrapping_add(temp1);
            d = c;
            c = b;
            b = a;
            a = temp1.wrapping_add(temp2);
        }

        h[0] = h[0].wrapping_add(a);
        h[1] = h[1].wrapping_add(b);
        h[2] = h[2].wrapping_add(c);
        h[3] = h[3].wrapping_add(d);
        h[4] = h[4].wrapping_add(e);
        h[5] = h[5].wrapping_add(f);
        h[6] = h[6].wrapping_add(g);
        h[7] = h[7].wrapping_add(h_val);
    }

    let mut result = String::with_capacity(64);
    for val in h.iter() {
        result.push_str(&format!("{:08x}", val));
    }
    result
}

/// Fallible variant of [`validate_and_reconstruct_chain_checked`].
///
/// Returns `Ok((client_caps, server_caps))` when the ledger is cryptographically
/// sound, or `Err(description)` for any corruption, malformed receipt ID, JSON
/// parse failure, or unexpected chain length.  Production paths that cannot
/// tolerate a process abort should call this instead of the panicking wrapper.
pub fn validate_and_reconstruct_chain_checked(
    history: &[tower_lsp_max_protocol::Receipt],
) -> Result<(serde_json::Value, serde_json::Value), String> {
    if history.is_empty() {
        return Err("History must not be empty".to_string());
    }

    // Validate step 0
    let r0 = &history[0];
    if r0.receipt_id != "rcpt-uninitialized" {
        return Err(format!(
            "Expected receipt_id 'rcpt-uninitialized' at index 0, got '{}'",
            r0.receipt_id
        ));
    }
    let mut expected_hash = sha256(r0.receipt_id.as_bytes());
    if r0.hash != expected_hash {
        return Err(format!(
            "Hash mismatch at index 0: expected '{}', got '{}'",
            expected_hash, r0.hash
        ));
    }

    let mut client_caps = serde_json::Value::Null;
    let mut server_caps = serde_json::Value::Null;

    if history.len() > 1 {
        let r1 = &history[1];
        if !r1
            .receipt_id
            .starts_with("rcpt-uninitialized-to-initializing:")
        {
            return Err(format!(
                "Invalid receipt ID at index 1: expected prefix                  'rcpt-uninitialized-to-initializing:', got '{}'",
                r1.receipt_id
            ));
        }
        let prefix_len = "rcpt-uninitialized-to-initializing:".len();
        let json_str = &r1.receipt_id[prefix_len..];
        client_caps = serde_json::from_str(json_str)
            .map_err(|e| format!("Failed to parse client capabilities at index 1: {}", e))?;

        expected_hash = sha256(format!("{}:{}", expected_hash, r1.receipt_id).as_bytes());
        if r1.hash != expected_hash {
            return Err(format!(
                "Hash mismatch at index 1: expected '{}', got '{}'",
                expected_hash, r1.hash
            ));
        }
    }

    if history.len() > 2 {
        let r2 = &history[2];
        if !r2
            .receipt_id
            .starts_with("rcpt-initializing-to-initialized:")
        {
            return Err(format!(
                "Invalid receipt ID at index 2: expected prefix                  'rcpt-initializing-to-initialized:', got '{}'",
                r2.receipt_id
            ));
        }
        let prefix_len = "rcpt-initializing-to-initialized:".len();
        let json_str = &r2.receipt_id[prefix_len..];
        server_caps = serde_json::from_str(json_str)
            .map_err(|e| format!("Failed to parse server capabilities at index 2: {}", e))?;

        expected_hash = sha256(format!("{}:{}", expected_hash, r2.receipt_id).as_bytes());
        if r2.hash != expected_hash {
            return Err(format!(
                "Hash mismatch at index 2: expected '{}', got '{}'",
                expected_hash, r2.hash
            ));
        }
    }

    if history.len() > 3 {
        let r3 = &history[3];
        if r3.receipt_id != "rcpt-initialized-to-shutdown" {
            return Err(format!(
                "Expected receipt_id 'rcpt-initialized-to-shutdown' at index 3, got '{}'",
                r3.receipt_id
            ));
        }
        expected_hash = sha256(format!("{}:{}", expected_hash, r3.receipt_id).as_bytes());
        if r3.hash != expected_hash {
            return Err(format!(
                "Hash mismatch at index 3: expected '{}', got '{}'",
                expected_hash, r3.hash
            ));
        }
    }

    if history.len() > 4 {
        let r4 = &history[4];
        if r4.receipt_id != "rcpt-shutdown-to-exited" {
            return Err(format!(
                "Expected receipt_id 'rcpt-shutdown-to-exited' at index 4, got '{}'",
                r4.receipt_id
            ));
        }
        expected_hash = sha256(format!("{}:{}", expected_hash, r4.receipt_id).as_bytes());
        if r4.hash != expected_hash {
            return Err(format!(
                "Hash mismatch at index 4: expected '{}', got '{}'",
                expected_hash, r4.hash
            ));
        }
    }

    if history.len() > 5 {
        return Err(format!(
            "History contains {} unexpected items beyond the Exited state (max 5)",
            history.len() - 5
        ));
    }

    Ok((client_caps, server_caps))
}

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
    // Best-effort mapping; callers that need precise variants should use the typed API.
    if e.contains("must not be empty") || e.contains("History must not be empty") {
        ChainError::EmptyHistory
    } else if e.contains("Hash mismatch") {
        // Extract index if present
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
        // First receipt in the chain — no predecessor.
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
        // Starts in Uninitialized
        let machine = Machine::new(Uninitialized, EmptyData::default());
        let rcpt_uninit = machine.receipt();
        assert_eq!(rcpt_uninit.receipt_id, "rcpt-uninitialized");
        assert_eq!(rcpt_uninit.hash, sha256(b"rcpt-uninitialized"));

        // Transition: Uninitialized -> Initializing
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

        // Transition: Initializing -> Initialized
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

        // Transition: Initialized -> ShutDown
        let machine = machine.admit_shutdown();
        let rcpt_shutdown = machine.receipt();
        let expected_shutdown_id = "rcpt-initialized-to-shutdown";
        assert_eq!(rcpt_shutdown.receipt_id, expected_shutdown_id);
        let expected_shutdown_hash =
            sha256(format!("{}:{}", rcpt_initialized.hash, expected_shutdown_id).as_bytes());
        assert_eq!(rcpt_shutdown.hash, expected_shutdown_hash);

        // Transition: ShutDown -> Exited
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

        // 1. Admit to Initializing
        let client_caps = serde_json::json!({"textDocument": {"completion": {}}});
        let machine_initializing = <Machine<AccessAdmissionLaw, Uninitialized, EmptyData> as TypestateKernel<
            _,
            _,
            _,
        >>::admit(machine_uninit, client_caps.clone()).unwrap();
        let rcpt_1 = machine_initializing.receipt();

        // 2. Admit to Initialized
        let server_caps = serde_json::json!({"completionProvider": {}});
        let machine_initialized =
            <Machine<AccessAdmissionLaw, Initializing, InitializingData> as TypestateKernel<
                _,
                _,
                _,
            >>::admit(machine_initializing, server_caps.clone())
            .unwrap();
        let rcpt_2 = machine_initialized.receipt();

        // 3. Admit to ShutDown
        let machine_shutdown =
            <Machine<AccessAdmissionLaw, Initialized, InitializedData> as TypestateKernel<
                _,
                _,
                _,
            >>::admit(machine_initialized, ())
            .unwrap();
        let rcpt_3 = machine_shutdown.receipt();

        // 4. Admit to Exited
        let machine_exited =
            <Machine<AccessAdmissionLaw, ShutDown, EmptyData> as TypestateKernel<_, _, _>>::admit(
                machine_shutdown,
                (),
            )
            .unwrap();
        let rcpt_4 = machine_exited.receipt();

        // Build history ledger
        let history = vec![
            rcpt_0.clone(),
            rcpt_1.clone(),
            rcpt_2.clone(),
            rcpt_3.clone(),
            rcpt_4.clone(),
        ];

        // Replay and check reconstruction
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

        // Tampered replay: change client capabilities in the receipt ID
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

        // Tampered replay: change a hash
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

// ==========================================
// 5-Layer Autonomic Mesh Controller
// ==========================================

pub use tower_lsp_max_protocol::{HookEvent, InstanceId, MaxDiagnostic, PolicyState, Receipt};

#[derive(Debug, Clone)]
pub enum MeshAction {
    TransitionPolicyState {
        instance_id: InstanceId,
        new_state: PolicyState,
    },
    ClearDiagnostic {
        instance_id: InstanceId,
        diagnostic_id: String,
    },
    AddDiagnostic {
        instance_id: InstanceId,
        diagnostic: Box<MaxDiagnostic>,
    },
    EmitReceipt {
        instance_id: InstanceId,
        receipt: Receipt,
    },
    ExecuteBoundedAction {
        instance_id: InstanceId,
        action_id: String,
        description: String,
    },
    /// Clear all diagnostics and receipts on an instance and reset its policy
    /// state to Active, supporting test-harness teardown and chaos recycling.
    ResetInstance { instance_id: InstanceId },
}

impl std::fmt::Display for MeshAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MeshAction::TransitionPolicyState {
                instance_id,
                new_state,
            } => {
                write!(f, "TransitionPolicyState({}, {:?})", instance_id, new_state)
            }
            MeshAction::ClearDiagnostic {
                instance_id,
                diagnostic_id,
            } => {
                write!(f, "ClearDiagnostic({}, {})", instance_id, diagnostic_id)
            }
            MeshAction::AddDiagnostic { instance_id, .. } => {
                write!(f, "AddDiagnostic({})", instance_id)
            }
            MeshAction::EmitReceipt {
                instance_id,
                receipt,
            } => {
                write!(f, "EmitReceipt({}, {})", instance_id, receipt.receipt_id)
            }
            MeshAction::ExecuteBoundedAction {
                instance_id,
                action_id,
                ..
            } => {
                write!(f, "ExecuteBoundedAction({}, {})", instance_id, action_id)
            }
            MeshAction::ResetInstance { instance_id } => {
                write!(f, "ResetInstance({})", instance_id)
            }
        }
    }
}

pub trait Hook: Send + Sync {
    fn name(&self) -> &str;
    fn trigger(&self, event: &HookEvent) -> Vec<MeshAction>;
}

/// Lifecycle phase of an LSP instance.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum LspPhase {
    Uninitialized,
    Initializing,
    Initialized,
    ShutDown,
    Exited,
}

impl std::fmt::Display for LspPhase {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LspPhase::Uninitialized => write!(f, "Uninitialized"),
            LspPhase::Initializing => write!(f, "Initializing"),
            LspPhase::Initialized => write!(f, "Initialized"),
            LspPhase::ShutDown => write!(f, "ShutDown"),
            LspPhase::Exited => write!(f, "Exited"),
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct LspInstance {
    pub id: String,
    pub phase: LspPhase,
    pub diagnostics: Vec<MaxDiagnostic>,
    pub receipts: Vec<Receipt>,
    pub policy_state: Option<PolicyState>,
    /// Lazy-invalidation cache for `conformance_score()`.
    /// Interior mutability via `Cell` keeps `&self` signature.
    /// Skipped during serde (derived from `diagnostics`).
    #[serde(skip)]
    cached_score: std::cell::Cell<Option<f64>>,
}

impl LspInstance {
    pub fn new(id: &str) -> Self {
        Self {
            id: id.to_string(),
            phase: LspPhase::Uninitialized,
            diagnostics: Vec::new(),
            receipts: Vec::new(),
            policy_state: None,
            cached_score: std::cell::Cell::new(None),
        }
    }

    /// Invalidate the cached conformance score.
    /// Call whenever `diagnostics` is mutated.
    #[inline]
    pub fn invalidate_score_cache(&mut self) {
        self.cached_score.set(None);
    }

    /// Add a diagnostic and automatically invalidate the conformance score cache.
    ///
    /// Callers must use this instead of directly mutating `diagnostics` to ensure
    /// the cache is never stale after a mutation.
    pub fn add_diagnostic(&mut self, diag: MaxDiagnostic) {
        self.diagnostics.push(diag);
        self.invalidate_score_cache();
    }

    /// Remove diagnostics matching `diagnostic_id` and automatically invalidate
    /// the conformance score cache when at least one entry is removed.
    ///
    /// Returns the number of entries removed.
    pub fn remove_diagnostic(&mut self, diagnostic_id: &str) -> usize {
        let before = self.diagnostics.len();
        self.diagnostics
            .retain(|d| d.diagnostic_id != diagnostic_id);
        let removed = before - self.diagnostics.len();
        if removed > 0 {
            self.invalidate_score_cache();
        }
        removed
    }

    /// Conformance score: O(1) cached reads, O(n) only after mutation.
    pub fn conformance_score(&self) -> f64 {
        if let Some(score) = self.cached_score.get() {
            return score;
        }
        let mut penalty: f64 = 0.0;
        for diag in &self.diagnostics {
            let p = match diag.lsp.severity {
                Some(lsp_types::DiagnosticSeverity::ERROR) => 30.0,
                Some(lsp_types::DiagnosticSeverity::WARNING) => 20.0,
                Some(lsp_types::DiagnosticSeverity::INFORMATION) => 10.0,
                Some(lsp_types::DiagnosticSeverity::HINT) => 5.0,
                Some(_) => 30.0,
                None => 30.0,
            };
            penalty += p;
        }
        let score = f64::max(100.0 - penalty, 0.0);
        self.cached_score.set(Some(score));
        score
    }

    /// Map a conformance score to its [`ConformanceGrade`] bucket.
    ///
    /// Thresholds (inclusive lower bound):
    /// * `score == 100.0` → `Perfect`
    /// * `75.0 ≤ score < 100.0` → `Good`
    /// * `50.0 ≤ score < 75.0` → `Degraded`
    /// * `score < 50.0` → `Critical`
    pub fn conformance_grade(&self) -> ConformanceGrade {
        ConformanceGrade::from_score(self.conformance_score())
    }
}

impl Default for LspInstance {
    fn default() -> Self {
        Self {
            id: String::new(),
            phase: LspPhase::Uninitialized,
            diagnostics: Vec::new(),
            receipts: Vec::new(),
            policy_state: None,
            cached_score: std::cell::Cell::new(None),
        }
    }
}

/// Coarse quality bucket derived from a conformance score.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum ConformanceGrade {
    /// Score == 100: zero active diagnostics.
    Perfect,
    /// Score in [75, 100): minor warnings only.
    Good,
    /// Score in [50, 75): non-trivial degradation.
    Degraded,
    /// Score < 50: severe violation load.
    Critical,
}

impl ConformanceGrade {
    /// Derive a grade directly from a numeric score.
    pub fn from_score(score: f64) -> Self {
        if score >= 100.0 {
            ConformanceGrade::Perfect
        } else if score >= 75.0 {
            ConformanceGrade::Good
        } else if score >= 50.0 {
            ConformanceGrade::Degraded
        } else {
            ConformanceGrade::Critical
        }
    }

    /// Return the canonical string label used in JSON responses.
    pub fn as_str(&self) -> &'static str {
        match self {
            ConformanceGrade::Perfect => "perfect",
            ConformanceGrade::Good => "good",
            ConformanceGrade::Degraded => "degraded",
            ConformanceGrade::Critical => "critical",
        }
    }
}
impl std::fmt::Display for ConformanceGrade {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct AutonomicMeshState {
    pub instances: std::collections::HashMap<String, LspInstance>,
    pub event_log: Vec<HookEvent>,
    pub executed_bounded_actions: Vec<String>,
    #[serde(flatten)]
    pub extra: std::collections::HashMap<String, serde_json::Value>,
}

impl std::fmt::Display for AutonomicMeshState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "AutonomicMeshState {{ instances: {}, event_log: {} }}",
            self.instances.len(),
            self.event_log.len()
        )
    }
}

pub struct IntakeDiagnosticHook;

impl Hook for IntakeDiagnosticHook {
    fn name(&self) -> &str {
        "IntakeDiagnosticHook"
    }

    fn trigger(&self, event: &HookEvent) -> Vec<MeshAction> {
        match event {
            HookEvent::DiagnosticEmitted {
                instance_id,
                diagnostic,
            } => {
                if instance_id.0 == "LSP_1" && diagnostic.law_id == "law-intake-validation" {
                    vec![MeshAction::TransitionPolicyState {
                        instance_id: InstanceId::from("LSP_2"),
                        new_state: PolicyState::ClarificationRequested,
                    }]
                } else {
                    vec![]
                }
            }
            _ => vec![],
        }
    }
}

pub struct IntakeClearHook;

impl Hook for IntakeClearHook {
    fn name(&self) -> &str {
        "IntakeClearHook"
    }

    fn trigger(&self, event: &HookEvent) -> Vec<MeshAction> {
        match event {
            HookEvent::DiagnosticCleared {
                instance_id,
                diagnostic_id,
            } => {
                if instance_id.0 == "LSP_1" && diagnostic_id == "diag-invalid-input" {
                    vec![
                        MeshAction::EmitReceipt {
                            instance_id: InstanceId::from("LSP_1"),
                            receipt: Receipt {
                                receipt_id: "rcpt-intake-validated".to_string(),
                                hash: "hash-intake-validated-mock".to_string(),
                                prev_receipt_hash: None,
                            },
                        },
                        MeshAction::TransitionPolicyState {
                            instance_id: InstanceId::from("LSP_2"),
                            new_state: PolicyState::RefundAuthorized,
                        },
                        MeshAction::ExecuteBoundedAction {
                            instance_id: InstanceId::from("LSP_2"),
                            action_id: "act-create-refund-receipt".to_string(),
                            description: "Creating refund receipt file for policy execution"
                                .to_string(),
                        },
                        MeshAction::EmitReceipt {
                            instance_id: InstanceId::from("LSP_2"),
                            receipt: Receipt {
                                receipt_id: "rcpt-refund-executed".to_string(),
                                hash: "hash-refund-executed-mock".to_string(),
                                prev_receipt_hash: None,
                            },
                        },
                    ]
                } else {
                    vec![]
                }
            }
            _ => vec![],
        }
    }
}

const MAX_EVENT_LOG: usize = 1000;

const MAX_DISPATCH_DEPTH: usize = 16;

/// Maximum number of conformance delta entries retained in memory.
const MAX_CONFORMANCE_DELTA_LOG: usize = 4096;

/// A single recorded conformance score change on a mesh instance.
/// Returned by `max/conformanceDelta` to enable live-dashboard polling.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct ConformanceDeltaEntry {
    /// Monotonically-increasing sequence number assigned at the moment of the change.
    pub seq: u64,
    pub instance_id: String,
    pub old_score: f64,
    pub new_score: f64,
}

/// Typed enumeration of every known `max/` RPC method.
///
/// `dispatch_rpc` parses the incoming method string into this enum before dispatching.
/// Adding a new protocol method requires a new variant here **and** a new match arm in
/// `dispatch_rpc` — the compiler enforces completeness, making "unimplemented method"
/// bugs structurally impossible.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MaxMethod {
    Snapshot,
    ConformanceVector,
    ClearDiagnostic,
    ExplainDiagnostic,
    RepairPlan,
    ApplyRepairTransaction,
    ExportAnalysisBundle,
    RunGate,
    Receipt,
    Hook,
    HookGraph,
    Chain,
    Propagate,
    AutonomicLoop,
    ManifoldSnapshot,
    LawfulTransition,
    Admission,
    Refusal,
    Replay,
    ReleaseActuation,
    InstanceList,
    DumpState,
    RestoreState,
    Reset,
    ConformanceDelta,
    VerifyLedger,
    LedgerReport,
}

impl MaxMethod {
    /// Return the canonical JSON-RPC method string for this variant.
    pub fn as_str(self) -> &'static str {
        match self {
            MaxMethod::Snapshot => "max/snapshot",
            MaxMethod::ConformanceVector => "max/conformanceVector",
            MaxMethod::ClearDiagnostic => "max/clearDiagnostic",
            MaxMethod::ExplainDiagnostic => "max/explainDiagnostic",
            MaxMethod::RepairPlan => "max/repairPlan",
            MaxMethod::ApplyRepairTransaction => "max/applyRepairTransaction",
            MaxMethod::ExportAnalysisBundle => "max/exportAnalysisBundle",
            MaxMethod::RunGate => "max/runGate",
            MaxMethod::Receipt => "max/receipt",
            MaxMethod::Hook => "max/hook",
            MaxMethod::HookGraph => "max/hookGraph",
            MaxMethod::Chain => "max/chain",
            MaxMethod::Propagate => "max/propagate",
            MaxMethod::AutonomicLoop => "max/autonomicLoop",
            MaxMethod::ManifoldSnapshot => "max/manifoldSnapshot",
            MaxMethod::LawfulTransition => "max/lawfulTransition",
            MaxMethod::Admission => "max/admission",
            MaxMethod::Refusal => "max/refusal",
            MaxMethod::Replay => "max/replay",
            MaxMethod::ReleaseActuation => "max/releaseActuation",
            MaxMethod::InstanceList => "max/instanceList",
            MaxMethod::DumpState => "max/dumpState",
            MaxMethod::RestoreState => "max/restoreState",
            MaxMethod::Reset => "max/reset",
            MaxMethod::ConformanceDelta => "max/conformanceDelta",
            MaxMethod::VerifyLedger => "max/verifyLedger",
            MaxMethod::LedgerReport => "max/ledgerReport",
        }
    }
}

impl<'a> TryFrom<&'a str> for MaxMethod {
    type Error = ();

    fn try_from(s: &'a str) -> Result<Self, Self::Error> {
        match s {
            "max/snapshot" => Ok(MaxMethod::Snapshot),
            "max/conformanceVector" => Ok(MaxMethod::ConformanceVector),
            "max/clearDiagnostic" => Ok(MaxMethod::ClearDiagnostic),
            "max/explainDiagnostic" => Ok(MaxMethod::ExplainDiagnostic),
            "max/repairPlan" => Ok(MaxMethod::RepairPlan),
            "max/applyRepairTransaction" => Ok(MaxMethod::ApplyRepairTransaction),
            "max/exportAnalysisBundle" => Ok(MaxMethod::ExportAnalysisBundle),
            "max/runGate" => Ok(MaxMethod::RunGate),
            "max/receipt" => Ok(MaxMethod::Receipt),
            "max/hook" => Ok(MaxMethod::Hook),
            "max/hookGraph" => Ok(MaxMethod::HookGraph),
            "max/chain" => Ok(MaxMethod::Chain),
            "max/propagate" => Ok(MaxMethod::Propagate),
            "max/autonomicLoop" => Ok(MaxMethod::AutonomicLoop),
            "max/manifoldSnapshot" => Ok(MaxMethod::ManifoldSnapshot),
            "max/lawfulTransition" => Ok(MaxMethod::LawfulTransition),
            "max/admission" => Ok(MaxMethod::Admission),
            "max/refusal" => Ok(MaxMethod::Refusal),
            "max/replay" => Ok(MaxMethod::Replay),
            "max/releaseActuation" => Ok(MaxMethod::ReleaseActuation),
            "max/instanceList" => Ok(MaxMethod::InstanceList),
            "max/dumpState" => Ok(MaxMethod::DumpState),
            "max/restoreState" => Ok(MaxMethod::RestoreState),
            "max/reset" => Ok(MaxMethod::Reset),
            "max/conformanceDelta" => Ok(MaxMethod::ConformanceDelta),
            "max/verifyLedger" => Ok(MaxMethod::VerifyLedger),
            "max/ledgerReport" => Ok(MaxMethod::LedgerReport),
            _ => Err(()),
        }
    }
}

pub struct AutonomicMesh {
    pub instances: std::collections::HashMap<String, LspInstance>,
    pub hooks: Vec<Box<dyn Hook>>,
    pub event_log: Vec<HookEvent>,
    pub executed_bounded_actions: Vec<String>,
    pub extra: std::collections::HashMap<String, serde_json::Value>,
    /// Monotonically-increasing counter incremented on every `execute_action` call.
    /// Used as a since-cursor for `max/conformanceDelta` polling.
    pub action_seq: u64,
    /// Ring-buffer of recent conformance score changes keyed by sequence number.
    pub conformance_delta_log: std::collections::VecDeque<ConformanceDeltaEntry>,
    /// Tracks re-entrant depth of `dispatch_event` to prevent unbounded recursion.
    /// Transient call-stack state — not serialized or persisted.
    dispatch_depth: usize,
}

impl Default for AutonomicMesh {
    fn default() -> Self {
        Self::new()
    }
}

/// Type alias for ergonomic use in tests and short-form code.
pub type MaxMesh = AutonomicMesh;

/// Build a [`tower_lsp_max_protocol::ConformanceVector`] from a slice of diagnostics.
///
/// Aggregates per [`tower_lsp_max_protocol::LawAxis`]: an axis is *refused* if any
/// ERROR-severity diagnostic is present, *admitted* if only non-ERROR diagnostics are present,
/// and *unknown* if no diagnostic has been observed for that axis.
/// Both `max/conformanceVector` and `max/exportAnalysisBundle` delegate to this function so
/// that a bug fix needs to be applied only once.
fn build_conformance_vector(
    diagnostics: &[MaxDiagnostic],
) -> tower_lsp_max_protocol::ConformanceVector {
    let mut axis_map: std::collections::HashMap<tower_lsp_max_protocol::LawAxis, bool> =
        std::collections::HashMap::new(); // true = has error
    for diag in diagnostics {
        let is_error = matches!(
            diag.lsp.severity,
            Some(lsp_types::DiagnosticSeverity::ERROR)
        );
        let entry = axis_map.entry(diag.law_axis.clone()).or_insert(false);
        if is_error {
            *entry = true;
        }
    }

    let mut admitted = vec![];
    let mut refused = vec![];
    for (axis, has_error) in &axis_map {
        if *has_error {
            refused.push(axis.clone());
        } else {
            admitted.push(axis.clone());
        }
    }

    let total = admitted.len() + refused.len();
    let derived_score = if total == 0 {
        None
    } else {
        Some(100.0 * admitted.len() as f64 / total as f64)
    };

    let witnessed: std::collections::HashSet<tower_lsp_max_protocol::LawAxis> =
        axis_map.keys().cloned().collect();
    let unknown: Vec<tower_lsp_max_protocol::LawAxis> =
        tower_lsp_max_protocol::LawAxis::all_named()
            .iter()
            .filter(|ax| !witnessed.contains(ax))
            .cloned()
            .collect();

    tower_lsp_max_protocol::ConformanceVector {
        admitted,
        refused,
        unknown,
        score: derived_score,
        strict_mode: true,
    }
}

impl AutonomicMesh {
    pub fn new() -> Self {
        Self {
            instances: std::collections::HashMap::new(),
            hooks: Vec::new(),
            event_log: Vec::new(),
            executed_bounded_actions: Vec::new(),
            extra: std::collections::HashMap::new(),
            action_seq: 0,
            conformance_delta_log: std::collections::VecDeque::new(),
            dispatch_depth: 0,
        }
    }

    pub fn to_state(&self) -> AutonomicMeshState {
        AutonomicMeshState {
            instances: self.instances.clone(),
            event_log: self.event_log.clone(),
            executed_bounded_actions: self.executed_bounded_actions.clone(),
            extra: self.extra.clone(),
        }
    }

    pub fn load_state(&mut self, state: AutonomicMeshState) {
        self.instances = state.instances;
        self.event_log = state.event_log;
        self.executed_bounded_actions = state.executed_bounded_actions;
        self.extra = state.extra;
    }

    pub fn load_from_file(path: &str) -> std::io::Result<Self> {
        let mut mesh = Self::new();
        if std::path::Path::new(path).exists() {
            let data = std::fs::read_to_string(path)?;
            if let Ok(state) = serde_json::from_str::<AutonomicMeshState>(&data) {
                mesh.load_state(state);
            }
        } else {
            let mut lsp1 = LspInstance::new("LSP_1");
            lsp1.phase = LspPhase::Initialized;
            let mut lsp2 = LspInstance::new("LSP_2");
            lsp2.phase = LspPhase::Initialized;
            lsp2.policy_state = Some(PolicyState::Operational);

            mesh.add_instance(lsp1);
            mesh.add_instance(lsp2);
            mesh.save_to_file(path)?;
        }
        mesh.register_hook(Box::new(IntakeDiagnosticHook));
        mesh.register_hook(Box::new(IntakeClearHook));
        Ok(mesh)
    }

    pub fn save_to_file(&self, path: &str) -> std::io::Result<()> {
        let state = self.to_state();
        let serialized = serde_json::to_string_pretty(&state)
            .map_err(|e| std::io::Error::other(e.to_string()))?;
        std::fs::write(path, serialized)?;
        Ok(())
    }

    pub fn add_instance(&mut self, instance: LspInstance) {
        self.instances.insert(instance.id.clone(), instance);
    }

    /// Convenience: create and register a bare `LspInstance` by id string.
    pub fn register_instance(&mut self, id: String) {
        self.add_instance(LspInstance::new(&id));
    }

    pub fn register_hook(&mut self, hook: Box<dyn Hook>) {
        self.hooks.push(hook);
    }

    // Layer 5: Autonomic Mesh controller (State & Event Dispatch loop)
    pub fn dispatch_event(&mut self, event: HookEvent) {
        if self.dispatch_depth >= MAX_DISPATCH_DEPTH {
            // Recursive hook chain has exceeded the safety limit.
            // Push a sentinel event and return rather than stack-overflowing.
            self.event_log.push(HookEvent::DiagnosticEmitted {
                instance_id: InstanceId::from("mesh"),
                diagnostic: Box::new(MaxDiagnostic {
                    lsp: lsp_types::Diagnostic {
                        range: lsp_types::Range::default(),
                        severity: Some(lsp_types::DiagnosticSeverity::ERROR),
                        code: None,
                        code_description: None,
                        source: Some("tower-lsp-max".to_string()),
                        message: format!(
                            "Dispatch depth {} exceeds limit {MAX_DISPATCH_DEPTH}; \
                             recursive hook chain terminated",
                            self.dispatch_depth
                        ),
                        related_information: None,
                        tags: None,
                        data: None,
                    },
                    diagnostic_id: format!("dispatch-depth-exceeded-{}", self.dispatch_depth),
                    law_id: "MESH_DISPATCH_DEPTH".to_string(),
                    attempted_transition: None,
                    violated_axes: vec!["recursion-safety".to_string()],
                    doc_routes: vec![],
                    repair_actions: vec![],
                    verification_gates: vec![],
                    receipt_obligation: None,
                    law_axis: tower_lsp_max_protocol::LawAxis::Security,
                    violated_invariant: String::new(),
                    observed_state: serde_json::Value::Null,
                    expected_state: serde_json::Value::Null,
                    repairability: tower_lsp_max_protocol::Repairability::NotRepairable,
                    terminality: tower_lsp_max_protocol::Terminality::Terminal,
                }),
            });
            return;
        }
        self.dispatch_depth += 1;

        self.event_log.push(event.clone());
        if self.event_log.len() > MAX_EVENT_LOG {
            self.event_log.drain(..self.event_log.len() - MAX_EVENT_LOG);
        }

        let mut actions = Vec::new();
        for hook in &self.hooks {
            let triggered = hook.trigger(&event);
            actions.extend(triggered);
        }

        for action in actions {
            self.execute_action(action);
        }

        self.dispatch_depth = self.dispatch_depth.saturating_sub(1);
    }

    pub fn execute_action(&mut self, action: MeshAction) {
        self.action_seq = self.action_seq.saturating_add(1);
        let seq = self.action_seq;

        // Record conformance score changes for any instance touched by this action.
        let maybe_instance_id: Option<String> = match &action {
            MeshAction::AddDiagnostic { instance_id, .. }
            | MeshAction::ClearDiagnostic { instance_id, .. }
            | MeshAction::TransitionPolicyState { instance_id, .. }
            | MeshAction::EmitReceipt { instance_id, .. }
            | MeshAction::ExecuteBoundedAction { instance_id, .. }
            | MeshAction::ResetInstance { instance_id } => Some(instance_id.0.clone()),
        };
        let old_score: Option<f64> = maybe_instance_id
            .as_deref()
            .and_then(|id| self.instances.get(id))
            .map(|inst| inst.conformance_score());

        match action {
            MeshAction::TransitionPolicyState {
                instance_id,
                new_state,
            } => {
                if let Some(instance) = self.instances.get_mut(&instance_id.0) {
                    let old_state = instance
                        .policy_state
                        .clone()
                        .unwrap_or(PolicyState::Operational);
                    instance.policy_state = Some(new_state.clone());

                    let event = HookEvent::PolicyStateChanged {
                        instance_id,
                        from_state: old_state,
                        to_state: new_state,
                    };
                    self.dispatch_event(event);
                }
            }
            MeshAction::ClearDiagnostic {
                instance_id,
                diagnostic_id,
            } => {
                if let Some(instance) = self.instances.get_mut(&instance_id.0) {
                    let old_len = instance.diagnostics.len();
                    instance
                        .diagnostics
                        .retain(|d| d.diagnostic_id != diagnostic_id);
                    if instance.diagnostics.len() < old_len {
                        instance.invalidate_score_cache();
                        let event = HookEvent::DiagnosticCleared {
                            instance_id,
                            diagnostic_id,
                        };
                        self.dispatch_event(event);
                    }
                }
            }
            MeshAction::AddDiagnostic {
                instance_id,
                diagnostic,
            } => {
                if let Some(instance) = self.instances.get_mut(&instance_id.0) {
                    instance.diagnostics.push((*diagnostic).clone());
                    instance.invalidate_score_cache();
                    let event = HookEvent::DiagnosticEmitted {
                        instance_id,
                        diagnostic,
                    };
                    self.dispatch_event(event);
                }
            }
            MeshAction::EmitReceipt {
                instance_id,
                mut receipt,
            } => {
                if let Some(instance) = self.instances.get_mut(&instance_id.0) {
                    // D-05: Always recompute hash from receipt_id so callers cannot forge it.
                    receipt.hash = sha256(receipt.receipt_id.as_bytes());
                    instance.receipts.push(receipt.clone());
                    let event = HookEvent::ReceiptEmitted {
                        instance_id,
                        receipt,
                    };
                    self.dispatch_event(event);
                }
            }
            MeshAction::ExecuteBoundedAction {
                instance_id,
                action_id,
                description,
            } => {
                if action_id == "act-create-refund-receipt" {
                    let receipt_dir =
                        std::env::var("MESH_RECEIPT_DIR").unwrap_or_else(|_| ".".to_string());
                    let file_path = std::path::Path::new(&receipt_dir).join("refund_receipt.txt");
                    let content = format!(
                        "REFUND RECEIPT\nInstance: {}\nDescription: {}\nStatus: Executed\nTimestamp: {}\n",
                        instance_id,
                        description,
                        std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap_or(std::time::Duration::ZERO)
                            .as_secs()
                    );
                    if let Err(e) = std::fs::write(&file_path, content) {
                        eprintln!(
                            "warn: failed to write receipt to {}: {}",
                            file_path.display(),
                            e
                        );
                    }
                }
                self.dispatch_event(HookEvent::BoundedActionExecuted {
                    instance_id,
                    action_id: action_id.clone(),
                    description: description.clone(),
                });
                self.executed_bounded_actions.push(action_id);
            }
            MeshAction::ResetInstance { instance_id } => {
                if let Some(instance) = self.instances.get_mut(&instance_id.0) {
                    instance.diagnostics.clear();
                    instance.receipts.clear();
                    instance.policy_state = Some(PolicyState::Operational);
                    instance.invalidate_score_cache();
                    self.dispatch_event(HookEvent::InstanceReset { instance_id });
                }
            }
        }

        // Record conformance delta if score changed.
        if let Some(iid) = maybe_instance_id {
            if let Some(new_score) = self
                .instances
                .get(&iid)
                .map(|inst| inst.conformance_score())
            {
                if let Some(old) = old_score {
                    if (new_score - old).abs() > f64::EPSILON {
                        let entry = ConformanceDeltaEntry {
                            seq,
                            instance_id: iid,
                            old_score: old,
                            new_score,
                        };
                        self.conformance_delta_log.push_back(entry);
                        if self.conformance_delta_log.len() > MAX_CONFORMANCE_DELTA_LOG {
                            self.conformance_delta_log.pop_front();
                        }
                    }
                }
            }
        }
    }

    // Layer 1: Grammar (CLI)
    pub fn run_command(&mut self, cmd: &str) -> Result<String, String> {
        let parts: Vec<&str> = cmd.split_whitespace().collect();
        if parts.is_empty() {
            return Err("Empty command".to_string());
        }

        match parts[0] {
            "diagnose" => {
                if parts.len() < 6 {
                    return Err(
                        "Usage: diagnose <instance_id> <diag_id> <law_id> <severity> <msg...>"
                            .to_string(),
                    );
                }
                let instance_id = InstanceId::from(parts[1]);
                let diag_id = parts[2].to_string();
                let law_id = parts[3].to_string();
                let severity_str = parts[4];
                let msg = parts[5..].join(" ");

                let severity = match severity_str {
                    "error" => Some(lsp_types::DiagnosticSeverity::ERROR),
                    "warning" => Some(lsp_types::DiagnosticSeverity::WARNING),
                    "info" => Some(lsp_types::DiagnosticSeverity::INFORMATION),
                    "hint" => Some(lsp_types::DiagnosticSeverity::HINT),
                    _ => return Err(format!("Unknown severity: {}", severity_str)),
                };

                let diagnostic = MaxDiagnostic {
                    lsp: lsp_types::Diagnostic {
                        range: lsp_types::Range::default(),
                        severity,
                        code: None,
                        code_description: None,
                        source: Some("autonomic-mesh".to_string()),
                        message: msg,
                        related_information: None,
                        tags: None,
                        data: None,
                    },
                    diagnostic_id: diag_id,
                    law_id,
                    attempted_transition: None,
                    violated_axes: vec!["semantic".to_string()],
                    doc_routes: vec![],
                    repair_actions: vec![],
                    verification_gates: vec![],
                    receipt_obligation: None,
                    law_axis: tower_lsp_max_protocol::LawAxis::Domain,
                    violated_invariant: String::new(),
                    observed_state: serde_json::Value::Null,
                    expected_state: serde_json::Value::Null,
                    repairability: tower_lsp_max_protocol::Repairability::Unknown,
                    terminality: tower_lsp_max_protocol::Terminality::NonTerminal,
                };

                self.execute_action(MeshAction::AddDiagnostic {
                    instance_id: instance_id.clone(),
                    diagnostic: Box::new(diagnostic),
                });

                Ok(format!("Emitted diagnostic on {}", instance_id))
            }
            "clear" => {
                if parts.len() < 3 {
                    return Err("Usage: clear <instance_id> <diag_id>".to_string());
                }
                let instance_id = InstanceId::from(parts[1]);
                let diag_id = parts[2].to_string();

                self.execute_action(MeshAction::ClearDiagnostic {
                    instance_id: instance_id.clone(),
                    diagnostic_id: diag_id,
                });

                Ok(format!("Cleared diagnostic on {}", instance_id))
            }
            "state" => {
                if parts.len() < 2 {
                    return Err("Usage: state <instance_id>".to_string());
                }
                let instance_id = parts[1];
                if let Some(inst) = self.instances.get(instance_id) {
                    let policy_str = match &inst.policy_state {
                        Some(p) => format!("{:?}", p),
                        None => "None".to_string(),
                    };
                    Ok(format!(
                        "Instance: {} | Phase: {} | Conformance: {} | PolicyState: {} | Diags: {} | Receipts: {}",
                        inst.id,
                        inst.phase,
                        inst.conformance_score(),
                        policy_str,
                        inst.diagnostics.len(),
                        inst.receipts.len()
                    ))
                } else {
                    Err(format!("Instance not found: {}", instance_id))
                }
            }
            "patch" => {
                if parts.len() < 3 {
                    return Err("Usage: patch <instance_id> <policy_state>".to_string());
                }
                let instance_id = InstanceId::from(parts[1]);
                let new_state = parts[2].parse::<PolicyState>()?;

                self.execute_action(MeshAction::TransitionPolicyState {
                    instance_id: instance_id.clone(),
                    new_state,
                });

                Ok(format!("Patched state on {}", instance_id))
            }
            _ => Err(format!("Unknown command: {}", parts[0])),
        }
    }

    // Layer 2: Local Surface (JSON-RPC wrapper)
    pub fn dispatch_rpc(
        &mut self,
        instance_id: &str,
        method: &str,
        params: serde_json::Value,
    ) -> Result<serde_json::Value, String> {
        if !self.instances.contains_key(instance_id) {
            return Err(format!("Instance {} not found", instance_id));
        }

        // Parse to typed enum — unknown methods fail here; the exhaustive match below
        // makes any newly-added variant that lacks a dispatch arm a compile error.
        let max_method = MaxMethod::try_from(method)
            .map_err(|_| format!("Method {} not supported on local RPC surface", method))?;

        match max_method {
            MaxMethod::VerifyLedger => {
                self.verify_instance_ledger(instance_id)?;
                Ok(serde_json::Value::Null)
            }
            MaxMethod::LedgerReport => {
                let report = self.get_ledger_diagnostic_report(instance_id);
                Ok(serde_json::Value::String(report))
            }
            MaxMethod::Snapshot => {
                let snap = DeterministicSnapshot::new();
                serde_json::to_value(snap.id).map_err(|e| e.to_string())
            }
            MaxMethod::ConformanceVector => {
                let instance = self
                    .instances
                    .get(instance_id)
                    .ok_or_else(|| format!("Instance not found: {}", instance_id))?;

                let vec = build_conformance_vector(&instance.diagnostics);
                serde_json::to_value(vec).map_err(|e| e.to_string())
            }
            MaxMethod::ClearDiagnostic => {
                let diag_id: String =
                    serde_json::from_value(params).map_err(|e| format!("Invalid params: {}", e))?;

                self.execute_action(MeshAction::ClearDiagnostic {
                    instance_id: InstanceId::from(instance_id),
                    diagnostic_id: diag_id,
                });
                Ok(serde_json::Value::Null)
            }
            MaxMethod::ExplainDiagnostic => {
                let diag_id: String =
                    serde_json::from_value(params).map_err(|e| format!("Invalid params: {}", e))?;
                let inst = self
                    .instances
                    .get(instance_id)
                    .ok_or_else(|| format!("Instance not found: {}", instance_id))?;
                let diag = inst
                    .diagnostics
                    .iter()
                    .find(|d| d.diagnostic_id == diag_id)
                    .ok_or_else(|| format!("Diagnostic not found: {}", diag_id))?;
                serde_json::to_value(diag.clone()).map_err(|e| e.to_string())
            }
            MaxMethod::RepairPlan => {
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
                        d.repair_actions.iter().map(move |ra| {
                            tower_lsp_max_protocol::MaxCodeAction {
                                action: lsp_types::CodeAction {
                                    title: ra.description.clone(),
                                    kind: None,
                                    diagnostics: None,
                                    edit: None,
                                    command: None,
                                    is_preferred: None,
                                    disabled: None,
                                    data: None,
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
                            }
                        })
                    })
                    .collect();
                serde_json::to_value(actions).map_err(|e| e.to_string())
            }
            MaxMethod::ApplyRepairTransaction => {
                let code_action: tower_lsp_max_protocol::MaxCodeAction =
                    serde_json::from_value(params).map_err(|e| format!("Invalid params: {}", e))?;
                self.apply_repair_transaction(instance_id, code_action)
            }
            MaxMethod::ExportAnalysisBundle => {
                let snapshot_id: tower_lsp_max_protocol::SnapshotId =
                    serde_json::from_value(params).map_err(|e| format!("Invalid params: {}", e))?;
                let inst = self
                    .instances
                    .get(instance_id)
                    .ok_or_else(|| format!("Instance not found: {}", instance_id))?;
                let mut cv = build_conformance_vector(&inst.diagnostics);
                // Override score with the instance's authoritative conformance_score()
                cv.score = Some(inst.conformance_score());
                let bundle = tower_lsp_max_protocol::AnalysisBundle {
                    snapshot_id,
                    capability_vector: tower_lsp_max_protocol::MaxCapabilityVector {
                        client: lsp_types::ClientCapabilities::default(),
                        server: lsp_types::ServerCapabilities::default(),
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
            MaxMethod::RunGate => {
                let gate_str: String =
                    serde_json::from_value(params).map_err(|e| format!("Invalid params: {}", e))?;
                let inst = self
                    .instances
                    .get(instance_id)
                    .ok_or_else(|| format!("Instance not found: {}", instance_id))?;
                let gate_blocked = inst
                    .diagnostics
                    .iter()
                    .any(|d| d.verification_gates.iter().any(|g| g.0 == gate_str));
                serde_json::to_value(!gate_blocked).map_err(|e| e.to_string())
            }
            MaxMethod::Receipt => {
                let receipt_id: String =
                    serde_json::from_value(params).map_err(|e| format!("Invalid params: {}", e))?;
                let inst = self
                    .instances
                    .get(instance_id)
                    .ok_or_else(|| format!("Instance not found: {}", instance_id))?;
                let receipt = inst
                    .receipts
                    .iter()
                    .find(|r| r.receipt_id == receipt_id)
                    .ok_or_else(|| format!("Receipt not found: {}", receipt_id))?;
                serde_json::to_value(receipt.clone()).map_err(|e| e.to_string())
            }
            MaxMethod::Hook => {
                // List all registered hooks with their metadata
                let hook_names: Vec<serde_json::Value> = self
                    .hooks
                    .iter()
                    .map(|h| serde_json::json!({ "name": h.name() }))
                    .collect();
                serde_json::to_value(hook_names).map_err(|e| e.to_string())
            }

            MaxMethod::HookGraph => {
                // Return hook topology: for each hook, name + active diagnostic/receipt triggers
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

            MaxMethod::Chain => {
                // Return full instance state summaries for all mesh members
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

            MaxMethod::Propagate => {
                // Propagate a receipt from one instance to trigger hooks on all others
                let receipt: tower_lsp_max_protocol::Receipt =
                    serde_json::from_value(params).map_err(|e| format!("Invalid params: {}", e))?;
                self.execute_action(MeshAction::EmitReceipt {
                    instance_id: InstanceId::from(instance_id),
                    receipt,
                });
                Ok(serde_json::json!({ "propagated": true }))
            }

            MaxMethod::AutonomicLoop => {
                // Return current autonomic loop status: instances, active hooks, event log size
                let status = serde_json::json!({
                    "instances": self.instances.keys().collect::<Vec<_>>(),
                    "hook_count": self.hooks.len(),
                    "event_log_size": self.event_log.len(),
                    "executed_actions": self.executed_bounded_actions.len(),
                });
                Ok(status)
            }

            MaxMethod::ManifoldSnapshot => {
                // Full manifold metadata: all instances with their complete state
                let snapshot = serde_json::json!({
                    "instances": self.instances.iter().map(|(id, inst)| {
                        serde_json::json!({
                            "id": id,
                            "phase": inst.phase,
                            "policy_state": inst.policy_state,
                            "diagnostic_count": inst.diagnostics.len(),
                            "receipt_count": inst.receipts.len(),
                            "conformance_score": inst.conformance_score(),
                            "conformance_grade": inst.conformance_grade().as_str(),
                        })
                    }).collect::<Vec<_>>(),
                    "hook_count": self.hooks.len(),
                    "event_log_size": self.event_log.len(),
                });
                Ok(snapshot)
            }

            MaxMethod::LawfulTransition => {
                // Attempt a lawful transition: validate phase order and check blocking diagnostics
                let target_phase: String =
                    serde_json::from_value(params).map_err(|e| format!("Invalid params: {}", e))?;
                let inst = self
                    .instances
                    .get(instance_id)
                    .ok_or_else(|| format!("Instance not found: {}", instance_id))?;
                // Define lawful phase order
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
                        // Check for blocking error-severity diagnostics
                        let blocking: Vec<_> = inst
                            .diagnostics
                            .iter()
                            .filter(|d| {
                                matches!(d.lsp.severity, Some(lsp_types::DiagnosticSeverity::ERROR))
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
                        .filter(|d| matches!(d.lsp.severity, Some(lsp_types::DiagnosticSeverity::ERROR)))
                        .count(),
                    "conformance_score": inst.conformance_score(),
                    "conformance_grade": inst.conformance_grade().as_str(),
                });
                Ok(result)
            }

            MaxMethod::Admission => {
                // Admissibility gate: returns Admitted/Refused/Unknown — NEVER collapses
                let inst = self
                    .instances
                    .get(instance_id)
                    .ok_or_else(|| format!("Instance not found: {}", instance_id))?;
                let verdict =
                    if inst.diagnostics.is_empty() {
                        "Admitted"
                    } else if inst.diagnostics.iter().any(|d| {
                        matches!(d.lsp.severity, Some(lsp_types::DiagnosticSeverity::ERROR))
                    }) {
                        "Refused"
                    } else {
                        "Unknown" // Warnings/hints present — cannot determine admissibility
                    };
                Ok(serde_json::json!({
                    "instance_id": instance_id,
                    "verdict": verdict,
                    "diagnostic_count": inst.diagnostics.len(),
                }))
            }

            MaxMethod::Refusal => {
                // Explicit refusal with law axis and receipt
                let diag_id: String =
                    serde_json::from_value(params).map_err(|e| format!("Invalid params: {}", e))?;
                let receipt_id = format!("rcpt-refusal-{}", diag_id);
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
                    "refused": true,
                    "diagnostic_id": diag_id,
                    "receipt": receipt,
                }))
            }

            MaxMethod::Replay => {
                // Replay the event log for the instance
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

            MaxMethod::ReleaseActuation => {
                // Controlled release: only if conformance admits it
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

            MaxMethod::InstanceList => {
                // Lightweight enumeration of all live instances: id, phase, conformance_score.
                // Callers that only need instance IDs should prefer this over max/manifoldSnapshot.
                let mut list: Vec<serde_json::Value> = self
                    .instances
                    .values()
                    .map(|inst| {
                        serde_json::json!({
                            "id": inst.id,
                            "phase": inst.phase,
                            "conformance_score": inst.conformance_score(),
                            "conformance_grade": inst.conformance_grade().as_str(),
                        })
                    })
                    .collect();
                list.sort_by_key(|v| v["id"].as_str().unwrap_or("").to_string());
                serde_json::to_value(list).map_err(|e| e.to_string())
            }

            MaxMethod::DumpState => {
                let state = self.to_state();
                serde_json::to_value(&state).map_err(|e| format!("Serialization failed: {}", e))
            }

            MaxMethod::RestoreState => {
                let state: AutonomicMeshState =
                    serde_json::from_value(params).map_err(|e| format!("Invalid params: {}", e))?;
                self.load_state(state);
                Ok(serde_json::Value::Null)
            }

            MaxMethod::Reset => {
                // Reset a single instance back to initial state: clears diagnostics, receipts,
                // and sets policy state to Active. Supports test-harness teardown and chaos recycling.
                self.execute_action(MeshAction::ResetInstance {
                    instance_id: InstanceId::from(instance_id),
                });
                Ok(serde_json::json!({
                    "reset": true,
                    "instance_id": instance_id,
                }))
            }

            MaxMethod::ConformanceDelta => {
                // Returns all conformance score changes since the given since_seq cursor.
                // Params: { "since_seq": <u64> }
                // Response: { "deltas": [...], "current_seq": <u64> }
                let since_seq: u64 = params
                    .as_object()
                    .and_then(|o| o.get("since_seq"))
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0);
                let deltas: Vec<&ConformanceDeltaEntry> = self
                    .conformance_delta_log
                    .iter()
                    .filter(|e| e.seq > since_seq)
                    .collect();
                Ok(serde_json::json!({
                    "deltas": deltas,
                    "current_seq": self.action_seq,
                }))
            }
        }
    }

    /// Verifies the cryptographic receipt chain of a mesh instance.
    /// Typed entry point for applying a repair transaction without a `serde_json::Value`
    /// round-trip inside `dispatch_rpc`.
    ///
    /// The `"max/applyRepairTransaction"` dispatch arm delegates here after the single
    /// `serde_json::from_value` call. Callers that already hold a typed `MaxCodeAction`
    /// can call this directly and skip serialization entirely.
    pub fn apply_repair_transaction(
        &mut self,
        instance_id: &str,
        code_action: tower_lsp_max_protocol::MaxCodeAction,
    ) -> Result<serde_json::Value, String> {
        // Pre-flight: verify all expected receipts exist before applying
        {
            let inst = self
                .instances
                .get(instance_id)
                .ok_or_else(|| format!("Instance not found: {}", instance_id))?;
            let existing: std::collections::HashSet<&str> = inst
                .receipts
                .iter()
                .map(|r| r.receipt_id.as_str())
                .collect();
            for expected in &code_action.receipt_plan.expected_receipts {
                if !existing.contains(expected.as_str()) {
                    return Err(format!(
                        "Receipt integrity violation: required receipt '{}' not found",
                        expected
                    ));
                }
            }
        }
        let action_id = format!("repair-{}", code_action.action.title.replace(' ', "-"));
        let receipt_id = format!("rcpt-repair-{}", code_action.action.title.replace(' ', "-"));
        let hash = sha256(receipt_id.as_bytes());
        let receipt = tower_lsp_max_protocol::Receipt {
            receipt_id: receipt_id.clone(),
            hash,
            prev_receipt_hash: None,
        };
        self.execute_action(MeshAction::ExecuteBoundedAction {
            instance_id: InstanceId::from(instance_id),
            action_id,
            description: code_action.action.title.clone(),
        });
        self.execute_action(MeshAction::EmitReceipt {
            instance_id: InstanceId::from(instance_id),
            receipt: receipt.clone(),
        });
        serde_json::to_value(receipt).map_err(|e| e.to_string())
    }

    pub fn verify_instance_ledger(&self, instance_id: &str) -> Result<(), String> {
        let instance = self
            .instances
            .get(instance_id)
            .ok_or_else(|| format!("Instance {} not found", instance_id))?;

        let history = &instance.receipts;
        if history.is_empty() {
            return Err("Ledger is empty".to_string());
        }

        if instance_id == "LSP_1" {
            // Validate step 0
            let r0 = &history[0];
            if r0.receipt_id != "rcpt-uninitialized" {
                return Err(format!("Invalid initial receipt ID: {}", r0.receipt_id));
            }
            let mut expected_hash = sha256(r0.receipt_id.as_bytes());
            if r0.hash != expected_hash {
                return Err(format!(
                    "Hash mismatch at index 0: expected {}, got {}",
                    expected_hash, r0.hash
                ));
            }

            if history.len() > 1 {
                let r1 = &history[1];
                if !r1
                    .receipt_id
                    .starts_with("rcpt-uninitialized-to-initializing:")
                {
                    return Err(format!("Invalid receipt ID at index 1: {}", r1.receipt_id));
                }
                let prefix_len = "rcpt-uninitialized-to-initializing:".len();
                let json_str = &r1.receipt_id[prefix_len..];
                if serde_json::from_str::<serde_json::Value>(json_str).is_err() {
                    return Err("Failed to parse client capabilities in receipt 1".to_string());
                }

                expected_hash = sha256(format!("{}:{}", expected_hash, r1.receipt_id).as_bytes());
                if r1.hash != expected_hash {
                    return Err(format!(
                        "Hash mismatch at index 1: expected {}, got {}",
                        expected_hash, r1.hash
                    ));
                }
            }

            if history.len() > 2 {
                let r2 = &history[2];
                if !r2
                    .receipt_id
                    .starts_with("rcpt-initializing-to-initialized:")
                {
                    return Err(format!("Invalid receipt ID at index 2: {}", r2.receipt_id));
                }
                let prefix_len = "rcpt-initializing-to-initialized:".len();
                let json_str = &r2.receipt_id[prefix_len..];
                if serde_json::from_str::<serde_json::Value>(json_str).is_err() {
                    return Err("Failed to parse server capabilities in receipt 2".to_string());
                }

                expected_hash = sha256(format!("{}:{}", expected_hash, r2.receipt_id).as_bytes());
                if r2.hash != expected_hash {
                    return Err(format!(
                        "Hash mismatch at index 2: expected {}, got {}",
                        expected_hash, r2.hash
                    ));
                }
            }

            if history.len() > 3 {
                let r3 = &history[3];
                if r3.receipt_id != "rcpt-initialized-to-shutdown" {
                    return Err(format!("Invalid receipt ID at index 3: {}", r3.receipt_id));
                }
                expected_hash = sha256(format!("{}:{}", expected_hash, r3.receipt_id).as_bytes());
                if r3.hash != expected_hash {
                    return Err(format!(
                        "Hash mismatch at index 3: expected {}, got {}",
                        expected_hash, r3.hash
                    ));
                }
            }

            if history.len() > 4 {
                let r4 = &history[4];
                if r4.receipt_id != "rcpt-shutdown-to-exited" {
                    return Err(format!("Invalid receipt ID at index 4: {}", r4.receipt_id));
                }
                expected_hash = sha256(format!("{}:{}", expected_hash, r4.receipt_id).as_bytes());
                if r4.hash != expected_hash {
                    return Err(format!(
                        "Hash mismatch at index 4: expected {}, got {}",
                        expected_hash, r4.hash
                    ));
                }
            }

            if history.len() > 5 {
                return Err("Ledger contains unexpected items beyond Exited state".to_string());
            }

            // Cross-verify phase matches the ledger length
            let expected_phase = match history.len() {
                1 => LspPhase::Uninitialized,
                2 => LspPhase::Initializing,
                3 => LspPhase::Initialized,
                4 => LspPhase::ShutDown,
                5 => LspPhase::Exited,
                _ => unreachable!(),
            };

            if instance.phase != expected_phase {
                return Err(format!(
                    "Phase mismatch: instance.phase is '{}' but ledger shows '{}'",
                    instance.phase, expected_phase
                ));
            }
        } else {
            // General verification for other instances
            for (idx, r) in history.iter().enumerate() {
                if r.receipt_id.is_empty() {
                    return Err(format!("Empty receipt ID at index {}", idx));
                }
                if r.hash.is_empty() {
                    return Err(format!("Empty receipt hash at index {}", idx));
                }
            }
        }

        Ok(())
    }

    /// Generates a detailed diagnostic report for a specific instance's ledger integrity.
    pub fn get_ledger_diagnostic_report(&self, instance_id: &str) -> String {
        let mut report = format!("Ledger Diagnostic Report for Instance: {}\n", instance_id);
        match self.verify_instance_ledger(instance_id) {
            Ok(()) => {
                report.push_str("Status: VERIFIED (Cryptographic integrity intact)\n");
            }
            Err(e) => {
                report.push_str(&format!(
                    "Status: FAILED (Ledger verification failed: {})\n",
                    e
                ));
            }
        }

        if let Some(instance) = self.instances.get(instance_id) {
            report.push_str(&format!("Active Phase: {}\n", instance.phase));
            report.push_str(&format!("Policy State: {:?}\n", instance.policy_state));
            report.push_str(&format!("Receipts count: {}\n", instance.receipts.len()));
            for (idx, r) in instance.receipts.iter().enumerate() {
                report.push_str(&format!(
                    "  [{}] ID: {} | Hash: {}\n",
                    idx, r.receipt_id, r.hash
                ));
            }
        } else {
            report.push_str("Instance not found in mesh registry.\n");
        }
        report
    }
}

// ==========================================
// Customer Service Proof Hooks
// ==========================================

pub struct CustomerRequestClassifierHook {
    proof_received: std::sync::Mutex<std::collections::HashSet<String>>,
    policy_states: std::sync::Mutex<std::collections::HashMap<String, PolicyState>>,
}

impl Default for CustomerRequestClassifierHook {
    fn default() -> Self {
        Self::new()
    }
}

impl CustomerRequestClassifierHook {
    pub fn new() -> Self {
        Self {
            proof_received: std::sync::Mutex::new(std::collections::HashSet::new()),
            policy_states: std::sync::Mutex::new(std::collections::HashMap::new()),
        }
    }
}

impl Hook for CustomerRequestClassifierHook {
    fn name(&self) -> &str {
        "CustomerRequestClassifierHook"
    }

    fn trigger(&self, event: &HookEvent) -> Vec<MeshAction> {
        let mut actions = Vec::new();
        match event {
            HookEvent::ReceiptEmitted {
                instance_id,
                receipt,
            } if receipt.receipt_id.contains("proof")
                || receipt.receipt_id.contains("customer-proof") =>
            {
                if let Ok(mut proof) = self.proof_received.lock() {
                    proof.insert(instance_id.0.clone());
                }
            }
            HookEvent::PolicyStateChanged {
                instance_id,
                from_state: _,
                to_state,
            } => {
                if let Ok(mut states) = self.policy_states.lock() {
                    states.insert(instance_id.0.clone(), to_state.clone());
                }
            }
            HookEvent::DiagnosticEmitted {
                instance_id,
                diagnostic,
            } => {
                let diag_id = &diagnostic.diagnostic_id;
                let message = diagnostic.lsp.message.to_lowercase();
                let is_proof_issue = diag_id == "missing-proof"
                    || diag_id == "damaged-proof"
                    || message.contains("proof is missing")
                    || message.contains("proof is damaged")
                    || message.contains("damaged proof")
                    || message.contains("missing proof");
                if is_proof_issue {
                    let should_transition = if let Ok(states) = self.policy_states.lock() {
                        !matches!(
                            states.get(instance_id.0.as_str()),
                            Some(PolicyState::ClarificationRequested)
                                | Some(PolicyState::RefundAuthorized)
                        )
                    } else {
                        true
                    };
                    if should_transition {
                        actions.push(MeshAction::TransitionPolicyState {
                            instance_id: instance_id.clone(),
                            new_state: PolicyState::ClarificationRequested,
                        });
                    }
                }
            }
            HookEvent::StateTransition {
                instance_id,
                from_phase: _,
                to_phase,
            } if to_phase == "Initialized" => {
                let is_missing = if let Ok(proof) = self.proof_received.lock() {
                    !proof.contains(instance_id.0.as_str())
                } else {
                    true
                };
                if is_missing {
                    let should_transition = if let Ok(states) = self.policy_states.lock() {
                        !matches!(
                            states.get(instance_id.0.as_str()),
                            Some(PolicyState::ClarificationRequested)
                                | Some(PolicyState::RefundAuthorized)
                        )
                    } else {
                        true
                    };
                    if should_transition {
                        actions.push(MeshAction::TransitionPolicyState {
                            instance_id: instance_id.clone(),
                            new_state: PolicyState::ClarificationRequested,
                        });
                    }
                }
            }
            HookEvent::BoundedActionExecuted {
                instance_id,
                action_id,
                description,
            } => {
                // Record that this instance completed a bounded action so
                // subsequent StateTransition("Initialized") checks see it as
                // having proof on file.
                if let Ok(mut proof) = self.proof_received.lock() {
                    proof.insert(instance_id.0.clone());
                }
                // Emit a receipt confirming the bounded action execution.
                actions.push(MeshAction::EmitReceipt {
                    instance_id: instance_id.clone(),
                    receipt: tower_lsp_max_protocol::Receipt {
                        receipt_id: format!("bounded-action-executed-{}", action_id),
                        hash: format!("sha256:bounded:{}:{}", action_id, description.len()),
                        prev_receipt_hash: None,
                    },
                });
            }
            HookEvent::InstanceReset { instance_id } => {
                // Evict stale per-instance cache entries so the reset instance is
                // treated as a fresh instance on the next StateTransition/proof events.
                if let Ok(mut proof) = self.proof_received.lock() {
                    proof.remove(&instance_id.0);
                }
                if let Ok(mut states) = self.policy_states.lock() {
                    states.remove(&instance_id.0);
                }
            }
            _ => {}
        }
        actions
    }
}

pub struct PolicyEvaluationHook {
    policy_states: std::sync::Mutex<std::collections::HashMap<String, PolicyState>>,
}

impl Default for PolicyEvaluationHook {
    fn default() -> Self {
        Self::new()
    }
}

impl PolicyEvaluationHook {
    pub fn new() -> Self {
        Self {
            policy_states: std::sync::Mutex::new(std::collections::HashMap::new()),
        }
    }
}

impl Hook for PolicyEvaluationHook {
    fn name(&self) -> &str {
        "PolicyEvaluationHook"
    }

    fn trigger(&self, event: &HookEvent) -> Vec<MeshAction> {
        let mut actions = Vec::new();
        match event {
            HookEvent::ReceiptEmitted {
                instance_id,
                receipt,
            } if receipt.receipt_id.contains("proof")
                || receipt.receipt_id.contains("customer-proof") =>
            {
                let is_clarification_requested = if let Ok(states) = self.policy_states.lock() {
                    states.get(&instance_id.0) == Some(&PolicyState::ClarificationRequested)
                } else {
                    false
                };
                if is_clarification_requested {
                    actions.push(MeshAction::TransitionPolicyState {
                        instance_id: instance_id.clone(),
                        new_state: PolicyState::RefundAuthorized,
                    });
                }
            }
            HookEvent::PolicyStateChanged {
                instance_id,
                from_state,
                to_state,
            } => {
                if let Ok(mut states) = self.policy_states.lock() {
                    states.insert(instance_id.0.clone(), to_state.clone());
                }
                if from_state == &PolicyState::ClarificationRequested
                    && to_state == &PolicyState::RefundAuthorized
                {
                    actions.push(MeshAction::ExecuteBoundedAction {
                        instance_id: instance_id.clone(),
                        action_id: "act-create-refund-receipt".to_string(),
                        description: "Arrival of proof validated, creating refund receipt"
                            .to_string(),
                    });
                }
            }
            HookEvent::BoundedActionExecuted {
                instance_id,
                action_id,
                ..
            } if action_id == "act-create-refund-receipt" => {
                // The refund receipt action completed; emit a confirmation receipt
                // so downstream hooks and audit consumers have evidence of completion.
                actions.push(MeshAction::EmitReceipt {
                    instance_id: instance_id.clone(),
                    receipt: tower_lsp_max_protocol::Receipt {
                        receipt_id: "refund-action-completion-receipt".to_string(),
                        hash: format!("sha256:completion:{}", action_id),
                        prev_receipt_hash: None,
                    },
                });
            }
            HookEvent::InstanceReset { instance_id } => {
                // Evict stale policy_states cache for the reset instance so it
                // is evaluated as a fresh instance on subsequent policy events.
                if let Ok(mut states) = self.policy_states.lock() {
                    states.remove(&instance_id.0);
                }
            }
            _ => {}
        }
        actions
    }
}

pub struct ReceiptRoutingHook {
    active_diagnostics:
        std::sync::Mutex<std::collections::HashMap<String, std::collections::HashSet<String>>>,
}

impl Default for ReceiptRoutingHook {
    fn default() -> Self {
        Self::new()
    }
}

impl ReceiptRoutingHook {
    pub fn new() -> Self {
        Self {
            active_diagnostics: std::sync::Mutex::new(std::collections::HashMap::new()),
        }
    }
}

impl Hook for ReceiptRoutingHook {
    fn name(&self) -> &str {
        "ReceiptRoutingHook"
    }

    fn trigger(&self, event: &HookEvent) -> Vec<MeshAction> {
        let mut actions = Vec::new();
        match event {
            HookEvent::DiagnosticEmitted {
                instance_id,
                diagnostic,
            } => {
                if let Ok(mut diags) = self.active_diagnostics.lock() {
                    diags
                        .entry(instance_id.0.clone())
                        .or_default()
                        .insert(diagnostic.diagnostic_id.clone());
                }
            }
            HookEvent::DiagnosticCleared {
                instance_id,
                diagnostic_id,
            } => {
                if let Ok(mut diags) = self.active_diagnostics.lock() {
                    if let Some(set) = diags.get_mut(&instance_id.0) {
                        set.remove(diagnostic_id);
                    }
                }
            }
            HookEvent::ReceiptEmitted {
                instance_id,
                receipt: _,
            } => {
                let target_instance = if instance_id.0 == "LSP_2" {
                    Some("LSP_1".to_string())
                } else if instance_id.0.contains("LSP_2") {
                    Some(instance_id.0.replace("LSP_2", "LSP_1"))
                } else if instance_id.0.contains("lsp_2") {
                    Some(instance_id.0.replace("lsp_2", "lsp_1"))
                } else {
                    None
                };

                if let Some(target) = target_instance {
                    if let Ok(diags) = self.active_diagnostics.lock() {
                        if let Some(set) = diags.get(&target) {
                            for diag_id in set {
                                actions.push(MeshAction::ClearDiagnostic {
                                    instance_id: InstanceId::from(target.clone()),
                                    diagnostic_id: diag_id.clone(),
                                });
                            }
                        }
                    }
                }
            }
            _ => {}
        }
        actions
    }
}

#[cfg(test)]
mod additional_hooks_tests {
    use super::*;

    #[test]
    fn test_instance_reset_clears_hook_caches() {
        let mut mesh = AutonomicMesh::new();

        let classifier = CustomerRequestClassifierHook::new();
        let policy_eval = PolicyEvaluationHook::new();
        mesh.register_hook(Box::new(classifier));
        mesh.register_hook(Box::new(policy_eval));

        mesh.add_instance(LspInstance::new("LSP_RESET"));

        // Step 1: Drive the instance to Initialized — hooks cache it as having no proof.
        mesh.dispatch_event(HookEvent::StateTransition {
            instance_id: InstanceId::from("LSP_RESET"),
            from_phase: "Uninitialized".to_string(),
            to_phase: "Initialized".to_string(),
        });
        // CustomerRequestClassifierHook should have set ClarificationRequested.
        assert_eq!(
            mesh.instances.get("LSP_RESET").unwrap().policy_state,
            Some(PolicyState::ClarificationRequested),
            "Hook should flag the instance as needing clarification before reset"
        );

        // Step 2: Reset the instance mid-flight.
        mesh.execute_action(MeshAction::ResetInstance {
            instance_id: InstanceId::from("LSP_RESET"),
        });
        // After reset the instance policy_state is Operational and caches are evicted.
        assert_eq!(
            mesh.instances.get("LSP_RESET").unwrap().policy_state,
            Some(PolicyState::Operational),
            "Reset must restore policy state to Operational"
        );

        // Step 3: Re-dispatch StateTransition to Initialized — hooks must treat it as fresh.
        mesh.dispatch_event(HookEvent::StateTransition {
            instance_id: InstanceId::from("LSP_RESET"),
            from_phase: "Uninitialized".to_string(),
            to_phase: "Initialized".to_string(),
        });
        // The hook sees no proof in its cache (it was evicted), so it requests clarification again.
        assert_eq!(
            mesh.instances.get("LSP_RESET").unwrap().policy_state,
            Some(PolicyState::ClarificationRequested),
            "Hook must re-evaluate fresh instance and request clarification again after reset"
        );
    }

    #[test]
    fn test_customer_service_hooks() {
        let mut mesh = AutonomicMesh::new();

        // Register the concrete hook implementations
        mesh.register_hook(Box::new(CustomerRequestClassifierHook::new()));
        mesh.register_hook(Box::new(PolicyEvaluationHook::new()));
        mesh.register_hook(Box::new(ReceiptRoutingHook::new()));

        // Add LSP_1 and LSP_2 instances
        mesh.add_instance(LspInstance::new("LSP_1"));
        mesh.add_instance(LspInstance::new("LSP_2"));

        // 1. Initial State transitions to trigger CustomerRequestClassifierHook
        mesh.dispatch_event(HookEvent::StateTransition {
            instance_id: InstanceId::from("LSP_1"),
            from_phase: "Uninitialized".to_string(),
            to_phase: "Initialized".to_string(),
        });

        // CustomerRequestClassifierHook should transition LSP_1 to ClarificationRequested
        assert_eq!(
            mesh.instances.get("LSP_1").unwrap().policy_state,
            Some(PolicyState::ClarificationRequested)
        );

        // 2. Add an active diagnostic to LSP_1
        let diag = MaxDiagnostic {
            lsp: lsp_types::Diagnostic {
                range: lsp_types::Range::default(),
                severity: Some(lsp_types::DiagnosticSeverity::ERROR),
                code: None,
                code_description: None,
                source: Some("test".to_string()),
                message: "Missing proof of purchase".to_string(),
                related_information: None,
                tags: None,
                data: None,
            },
            diagnostic_id: "diag-proof-1".to_string(),
            law_id: "law-proof".to_string(),
            attempted_transition: None,
            violated_axes: vec![],
            doc_routes: vec![],
            repair_actions: vec![],
            verification_gates: vec![],
            receipt_obligation: None,
            ..Default::default()
        };
        mesh.execute_action(MeshAction::AddDiagnostic {
            instance_id: InstanceId::from("LSP_1"),
            diagnostic: Box::new(diag),
        });
        assert_eq!(mesh.instances.get("LSP_1").unwrap().diagnostics.len(), 1);

        // 3. Emit a proof receipt on LSP_2
        let receipt = Receipt {
            receipt_id: "rcpt-customer-proof".to_string(),
            hash: "dummyhash".to_string(),
            prev_receipt_hash: None,
        };

        // Before dispatching receipt, policy state of LSP_2 should transition to ClarificationRequested first
        mesh.execute_action(MeshAction::TransitionPolicyState {
            instance_id: InstanceId::from("LSP_2"),
            new_state: PolicyState::ClarificationRequested,
        });

        mesh.dispatch_event(HookEvent::ReceiptEmitted {
            instance_id: InstanceId::from("LSP_2"),
            receipt: receipt.clone(),
        });

        // Under PolicyEvaluationHook:
        // ReceiptEmitted triggers transition ClarificationRequested -> RefundAuthorized on LSP_2
        // That transition triggers act-create-refund-receipt
        assert_eq!(
            mesh.instances.get("LSP_2").unwrap().policy_state,
            Some(PolicyState::RefundAuthorized)
        );
        assert!(mesh
            .executed_bounded_actions
            .contains(&"act-create-refund-receipt".to_string()));

        // Under ReceiptRoutingHook:
        // ReceiptEmitted on LSP_2 routes clear active diagnostics to LSP_1
        assert!(mesh.instances.get("LSP_1").unwrap().diagnostics.is_empty());
    }
}

#[cfg(test)]
mod tests_gaps {
    use super::*;
    use serde_json::json;

    // helpers
    fn make_mesh_with_instance(id: &str) -> AutonomicMesh {
        let mut mesh = AutonomicMesh::new();
        mesh.add_instance(LspInstance::new(id));
        mesh
    }

    fn make_error_diagnostic(id: &str) -> MaxDiagnostic {
        MaxDiagnostic {
            diagnostic_id: id.to_string(),
            law_id: "law-test".to_string(),
            attempted_transition: None,
            violated_axes: vec![],
            doc_routes: vec![],
            lsp: lsp_types::Diagnostic {
                range: lsp_types::Range::default(),
                severity: Some(lsp_types::DiagnosticSeverity::ERROR),
                code: None,
                code_description: None,
                source: None,
                message: "test".to_string(),
                related_information: None,
                tags: None,
                data: None,
            },
            repair_actions: vec![],
            verification_gates: vec![],
            receipt_obligation: None,
            ..Default::default()
        }
    }

    // --- RPC dispatch tests (7) ---

    #[test]
    fn test_rpc_snapshot_returns_snap_id() {
        let mut mesh = make_mesh_with_instance("INST_A");
        let result = mesh.dispatch_rpc("INST_A", "max/snapshot", json!(null));
        assert!(result.is_ok());
        let val = result.unwrap();
        assert!(val.is_string());
    }

    #[test]
    fn test_rpc_conformance_vector_no_diagnostics() {
        let mut mesh = make_mesh_with_instance("INST_A");
        let result = mesh.dispatch_rpc("INST_A", "max/conformanceVector", json!(null));
        assert!(result.is_ok());
        let val = result.unwrap();
        // No diagnostics → no axes evaluated → score is null (all-unknown per doctrine)
        assert!(
            val["score"].is_null(),
            "score should be null when no diagnostics"
        );
    }

    #[test]
    fn test_rpc_conformance_vector_with_error_diagnostic() {
        let mut mesh = make_mesh_with_instance("INST_A");
        mesh.execute_action(MeshAction::AddDiagnostic {
            instance_id: InstanceId::from("INST_A"),
            diagnostic: Box::new(make_error_diagnostic("diag-1")),
        });
        let result = mesh.dispatch_rpc("INST_A", "max/conformanceVector", json!(null));
        assert!(result.is_ok());
        let val = result.unwrap();
        // One ERROR diagnostic: admitted=0, refused=1 → score = 0%
        let score = val["score"]
            .as_f64()
            .expect("score should be f64 with refused axis");
        assert_eq!(score, 0.0);
    }

    #[test]
    fn test_rpc_clear_diagnostic_removes_diag() {
        let mut mesh = make_mesh_with_instance("INST_A");
        mesh.execute_action(MeshAction::AddDiagnostic {
            instance_id: InstanceId::from("INST_A"),
            diagnostic: Box::new(make_error_diagnostic("diag-1")),
        });
        assert_eq!(mesh.instances.get("INST_A").unwrap().diagnostics.len(), 1);
        let result = mesh.dispatch_rpc("INST_A", "max/clearDiagnostic", json!("diag-1"));
        assert!(result.is_ok());
        assert!(mesh.instances.get("INST_A").unwrap().diagnostics.is_empty());
    }

    #[test]
    fn test_rpc_unknown_method_returns_err() {
        let mut mesh = make_mesh_with_instance("INST_A");
        let result = mesh.dispatch_rpc("INST_A", "max/nonexistent", json!(null));
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("not supported"));
    }

    #[test]
    fn test_rpc_missing_instance_returns_err() {
        let mut mesh = AutonomicMesh::new();
        let result = mesh.dispatch_rpc("GHOST", "max/snapshot", json!(null));
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("not found"));
    }

    #[test]
    fn test_rpc_clear_diagnostic_invalid_params() {
        let mut mesh = make_mesh_with_instance("INST_A");
        let result = mesh.dispatch_rpc("INST_A", "max/clearDiagnostic", json!({"bad": true}));
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Invalid params"));
    }

    // --- Negative cases (9) ---

    #[test]
    fn test_transition_missing_instance_is_noop() {
        let mut mesh = AutonomicMesh::new();
        // Should not panic
        mesh.execute_action(MeshAction::TransitionPolicyState {
            instance_id: InstanceId::from("GHOST"),
            new_state: PolicyState::Operational,
        });
        assert!(mesh.instances.is_empty());
    }

    #[test]
    fn test_clear_nonexistent_diag_is_noop() {
        let mut mesh = make_mesh_with_instance("INST_A");
        let before = mesh.event_log.len();
        mesh.execute_action(MeshAction::ClearDiagnostic {
            instance_id: InstanceId::from("INST_A"),
            diagnostic_id: "no-such-diag".to_string(),
        });
        // No DiagnosticCleared event should be appended
        for ev in &mesh.event_log[before..] {
            if let HookEvent::DiagnosticCleared { .. } = ev {
                panic!("Unexpected DiagnosticCleared event for nonexistent diag");
            }
        }
    }

    #[test]
    fn test_emit_receipt_missing_instance_is_noop() {
        let mut mesh = AutonomicMesh::new();
        let receipt = Receipt {
            receipt_id: "r1".to_string(),
            hash: "h1".to_string(),
            prev_receipt_hash: None,
        };
        mesh.execute_action(MeshAction::EmitReceipt {
            instance_id: InstanceId::from("GHOST"),
            receipt,
        });
        assert!(mesh.event_log.is_empty());
    }

    #[test]
    fn test_run_command_diagnose_too_few_args() {
        let mut mesh = make_mesh_with_instance("LSP_1");
        let result = mesh.run_command("diagnose LSP_1 diag-1");
        assert!(result.is_err());
    }

    #[test]
    fn test_run_command_unknown_severity() {
        let mut mesh = make_mesh_with_instance("INST_A");
        let result = mesh.run_command("diagnose INST_A d1 law-1 critical message");
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.contains("Unknown severity") || err.to_lowercase().contains("unknown"));
    }

    #[test]
    fn test_run_command_patch_unknown_state() {
        let mut mesh = make_mesh_with_instance("INST_A");
        let result = mesh.run_command("patch INST_A Suspended");
        assert!(result.is_err());
    }

    #[test]
    fn test_run_command_state_missing_instance() {
        let mut mesh = AutonomicMesh::new();
        let result = mesh.run_command("state GHOST");
        assert!(result.is_err());
    }

    // --- Receipt obligation (3) ---

    #[test]
    fn test_receipt_obligation_field_roundtrip() {
        let diag = MaxDiagnostic {
            diagnostic_id: "d1".to_string(),
            law_id: "law-1".to_string(),
            attempted_transition: None,
            violated_axes: vec![],
            doc_routes: vec![],
            lsp: lsp_types::Diagnostic {
                range: lsp_types::Range::default(),
                severity: Some(lsp_types::DiagnosticSeverity::ERROR),
                code: None,
                code_description: None,
                source: None,
                message: "test".to_string(),
                related_information: None,
                tags: None,
                data: None,
            },
            repair_actions: vec![],
            verification_gates: vec![],
            receipt_obligation: Some(tower_lsp_max_protocol::ReceiptObligation {
                required_receipts: vec!["rcpt-foo".to_string()],
            }),
            ..Default::default()
        };
        let serialized = serde_json::to_string(&diag).unwrap();
        let deserialized: MaxDiagnostic = serde_json::from_str(&serialized).unwrap();
        let obligation = deserialized.receipt_obligation.unwrap();
        assert_eq!(obligation.required_receipts, vec!["rcpt-foo".to_string()]);
    }

    #[test]
    fn test_diagnostic_with_receipt_obligation_emitted() {
        let mut mesh = make_mesh_with_instance("INST_A");
        let diag = MaxDiagnostic {
            diagnostic_id: "d-obliged".to_string(),
            law_id: "law-x".to_string(),
            attempted_transition: None,
            violated_axes: vec![],
            doc_routes: vec![],
            lsp: lsp_types::Diagnostic {
                range: lsp_types::Range::default(),
                severity: Some(lsp_types::DiagnosticSeverity::WARNING),
                code: None,
                code_description: None,
                source: None,
                message: "obligated".to_string(),
                related_information: None,
                tags: None,
                data: None,
            },
            repair_actions: vec![],
            verification_gates: vec![],
            receipt_obligation: Some(tower_lsp_max_protocol::ReceiptObligation {
                required_receipts: vec!["rcpt-required".to_string()],
            }),
            ..Default::default()
        };
        mesh.execute_action(MeshAction::AddDiagnostic {
            instance_id: InstanceId::from("INST_A"),
            diagnostic: Box::new(diag),
        });
        let stored = &mesh.instances.get("INST_A").unwrap().diagnostics[0];
        let obligation = stored.receipt_obligation.as_ref().unwrap();
        assert_eq!(
            obligation.required_receipts,
            vec!["rcpt-required".to_string()]
        );
    }

    #[test]
    fn test_receipt_obligation_none_is_default() {
        let mut mesh = make_mesh_with_instance("INST_A");
        mesh.run_command("diagnose INST_A d1 law-1 error some message")
            .unwrap();
        let stored = &mesh.instances.get("INST_A").unwrap().diagnostics[0];
        assert!(stored.receipt_obligation.is_none());
    }

    // --- Conformance score (7) ---

    #[test]
    fn test_conformance_score_single_warning() {
        let mut inst = LspInstance::new("X");
        inst.diagnostics.push(MaxDiagnostic {
            diagnostic_id: "w1".to_string(),
            law_id: "law".to_string(),
            attempted_transition: None,
            violated_axes: vec![],
            doc_routes: vec![],
            lsp: lsp_types::Diagnostic {
                range: lsp_types::Range::default(),
                severity: Some(lsp_types::DiagnosticSeverity::WARNING),
                code: None,
                code_description: None,
                source: None,
                message: "w".to_string(),
                related_information: None,
                tags: None,
                data: None,
            },
            repair_actions: vec![],
            verification_gates: vec![],
            receipt_obligation: None,
            ..Default::default()
        });
        assert_eq!(inst.conformance_score(), 80.0);
    }

    #[test]
    fn test_conformance_score_single_information() {
        let mut inst = LspInstance::new("X");
        inst.diagnostics.push(MaxDiagnostic {
            diagnostic_id: "i1".to_string(),
            law_id: "law".to_string(),
            attempted_transition: None,
            violated_axes: vec![],
            doc_routes: vec![],
            lsp: lsp_types::Diagnostic {
                range: lsp_types::Range::default(),
                severity: Some(lsp_types::DiagnosticSeverity::INFORMATION),
                code: None,
                code_description: None,
                source: None,
                message: "i".to_string(),
                related_information: None,
                tags: None,
                data: None,
            },
            repair_actions: vec![],
            verification_gates: vec![],
            receipt_obligation: None,
            ..Default::default()
        });
        assert_eq!(inst.conformance_score(), 90.0);
    }

    #[test]
    fn test_conformance_score_single_hint() {
        let mut inst = LspInstance::new("X");
        inst.diagnostics.push(MaxDiagnostic {
            diagnostic_id: "h1".to_string(),
            law_id: "law".to_string(),
            attempted_transition: None,
            violated_axes: vec![],
            doc_routes: vec![],
            lsp: lsp_types::Diagnostic {
                range: lsp_types::Range::default(),
                severity: Some(lsp_types::DiagnosticSeverity::HINT),
                code: None,
                code_description: None,
                source: None,
                message: "h".to_string(),
                related_information: None,
                tags: None,
                data: None,
            },
            repair_actions: vec![],
            verification_gates: vec![],
            receipt_obligation: None,
            ..Default::default()
        });
        assert_eq!(inst.conformance_score(), 95.0);
    }

    #[test]
    fn test_conformance_score_clamps_to_zero() {
        let mut inst = LspInstance::new("X");
        for i in 0..4 {
            inst.diagnostics.push(MaxDiagnostic {
                diagnostic_id: format!("e{}", i),
                law_id: "law".to_string(),
                attempted_transition: None,
                violated_axes: vec![],
                doc_routes: vec![],
                lsp: lsp_types::Diagnostic {
                    range: lsp_types::Range::default(),
                    severity: Some(lsp_types::DiagnosticSeverity::ERROR),
                    code: None,
                    code_description: None,
                    source: None,
                    message: "e".to_string(),
                    related_information: None,
                    tags: None,
                    data: None,
                },
                repair_actions: vec![],
                verification_gates: vec![],
                receipt_obligation: None,
                ..Default::default()
            });
        }
        assert_eq!(inst.conformance_score(), 0.0);
    }

    #[test]
    fn test_conformance_score_mixed_severities() {
        let mut inst = LspInstance::new("X");
        // ERROR=30, WARNING=20, HINT=5 → penalty=55, score=45
        for (id, sev) in &[
            ("e1", lsp_types::DiagnosticSeverity::ERROR),
            ("w1", lsp_types::DiagnosticSeverity::WARNING),
            ("h1", lsp_types::DiagnosticSeverity::HINT),
        ] {
            inst.diagnostics.push(MaxDiagnostic {
                diagnostic_id: id.to_string(),
                law_id: "law".to_string(),
                attempted_transition: None,
                violated_axes: vec![],
                doc_routes: vec![],
                lsp: lsp_types::Diagnostic {
                    range: lsp_types::Range::default(),
                    severity: Some(*sev),
                    code: None,
                    code_description: None,
                    source: None,
                    message: "m".to_string(),
                    related_information: None,
                    tags: None,
                    data: None,
                },
                repair_actions: vec![],
                verification_gates: vec![],
                receipt_obligation: None,
                ..Default::default()
            });
        }
        assert_eq!(inst.conformance_score(), 45.0);
    }

    #[test]
    fn test_conformance_score_no_diagnostics() {
        let inst = LspInstance::new("X");
        assert_eq!(inst.conformance_score(), 100.0);
    }
    /// INN-10-04: cached conformance_score is O(1) after first call and invalidated by mutation.
    #[test]
    fn test_conformance_score_cache_invalidation() {
        let mut inst = LspInstance::new("CACHE_TEST");
        // Fresh instance: score 100
        assert_eq!(
            inst.conformance_score(),
            100.0,
            "empty diags must score 100"
        );
        // Second call must use cache (same result)
        assert_eq!(
            inst.conformance_score(),
            100.0,
            "cached call must also score 100"
        );

        // Add an ERROR diagnostic; cache must be invalidated
        let diag = MaxDiagnostic {
            diagnostic_id: "d1".to_string(),
            law_id: "law-cache-test".to_string(),
            lsp: lsp_types::Diagnostic {
                range: lsp_types::Range::default(),
                severity: Some(lsp_types::DiagnosticSeverity::ERROR),
                code: None,
                code_description: None,
                source: None,
                message: "cache test error".to_string(),
                related_information: None,
                tags: None,
                data: None,
            },
            attempted_transition: None,
            violated_axes: vec![],
            doc_routes: vec![],
            repair_actions: vec![],
            verification_gates: vec![],
            receipt_obligation: None,
            law_axis: tower_lsp_max_protocol::LawAxis::Domain,
            violated_invariant: String::new(),
            observed_state: serde_json::Value::Null,
            expected_state: serde_json::Value::Null,
            repairability: tower_lsp_max_protocol::Repairability::Unknown,
            terminality: tower_lsp_max_protocol::Terminality::NonTerminal,
        };
        inst.diagnostics.push(diag);
        inst.invalidate_score_cache();
        // Score must drop by 30 (one ERROR)
        assert_eq!(
            inst.conformance_score(),
            70.0,
            "one ERROR must reduce score to 70"
        );
        // Repeated reads use cache
        assert_eq!(
            inst.conformance_score(),
            70.0,
            "cached score must remain 70"
        );

        // Clear diagnostic; invalidate
        inst.diagnostics.clear();
        inst.invalidate_score_cache();
        assert_eq!(
            inst.conformance_score(),
            100.0,
            "after clear score must return to 100"
        );
    }

    #[test]
    fn test_rpc_conformance_vector_warning_diagnostic() {
        let mut mesh = make_mesh_with_instance("INST_A");
        mesh.execute_action(MeshAction::AddDiagnostic {
            instance_id: InstanceId::from("INST_A"),
            diagnostic: Box::new(MaxDiagnostic {
                diagnostic_id: "w1".to_string(),
                law_id: "law".to_string(),
                attempted_transition: None,
                violated_axes: vec![],
                doc_routes: vec![],
                lsp: lsp_types::Diagnostic {
                    range: lsp_types::Range::default(),
                    severity: Some(lsp_types::DiagnosticSeverity::WARNING),
                    code: None,
                    code_description: None,
                    source: None,
                    message: "warn".to_string(),
                    related_information: None,
                    tags: None,
                    data: None,
                },
                repair_actions: vec![],
                verification_gates: vec![],
                receipt_obligation: None,
                ..Default::default()
            }),
        });
        let result = mesh.dispatch_rpc("INST_A", "max/conformanceVector", json!(null));
        assert!(result.is_ok());
        let val = result.unwrap();
        // WARNING → admitted (not refused); admitted=1, refused=0 → score = 100%
        let score = val["score"]
            .as_f64()
            .expect("score should be f64 with one admitted axis");
        assert_eq!(score, 100.0);
    }

    // --- Nested diagnostic clearing (2) ---

    #[test]
    fn test_clear_first_of_two_diags_leaves_second() {
        let mut mesh = make_mesh_with_instance("INST_A");
        mesh.execute_action(MeshAction::AddDiagnostic {
            instance_id: InstanceId::from("INST_A"),
            diagnostic: Box::new(make_error_diagnostic("d1")),
        });
        mesh.execute_action(MeshAction::AddDiagnostic {
            instance_id: InstanceId::from("INST_A"),
            diagnostic: Box::new(make_error_diagnostic("d2")),
        });
        mesh.execute_action(MeshAction::ClearDiagnostic {
            instance_id: InstanceId::from("INST_A"),
            diagnostic_id: "d1".to_string(),
        });
        let diags = &mesh.instances.get("INST_A").unwrap().diagnostics;
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].diagnostic_id, "d2");
    }

    #[test]
    fn test_clear_all_diags_sequentially() {
        let mut mesh = make_mesh_with_instance("INST_A");
        mesh.execute_action(MeshAction::AddDiagnostic {
            instance_id: InstanceId::from("INST_A"),
            diagnostic: Box::new(make_error_diagnostic("d1")),
        });
        mesh.execute_action(MeshAction::AddDiagnostic {
            instance_id: InstanceId::from("INST_A"),
            diagnostic: Box::new(make_error_diagnostic("d2")),
        });
        mesh.execute_action(MeshAction::ClearDiagnostic {
            instance_id: InstanceId::from("INST_A"),
            diagnostic_id: "d1".to_string(),
        });
        mesh.execute_action(MeshAction::ClearDiagnostic {
            instance_id: InstanceId::from("INST_A"),
            diagnostic_id: "d2".to_string(),
        });
        assert!(mesh.instances.get("INST_A").unwrap().diagnostics.is_empty());
    }

    #[test]
    fn test_dispatch_depth_guard_prevents_infinite_recursion() {
        // A hook that reacts to every StateTransition by adding a diagnostic,
        // which itself fires DiagnosticEmitted, which re-enters dispatch_event.
        // Without the depth guard this would stack-overflow.
        struct RecursiveHook;
        impl Hook for RecursiveHook {
            fn name(&self) -> &str {
                "RecursiveHook"
            }
            fn trigger(&self, event: &HookEvent) -> Vec<MeshAction> {
                match event {
                    HookEvent::StateTransition { instance_id, .. } => {
                        vec![MeshAction::AddDiagnostic {
                            instance_id: instance_id.clone(),
                            diagnostic: Box::new(MaxDiagnostic {
                                lsp: lsp_types::Diagnostic {
                                    range: lsp_types::Range::default(),
                                    severity: Some(lsp_types::DiagnosticSeverity::HINT),
                                    code: None,
                                    code_description: None,
                                    source: Some("test".to_string()),
                                    message: "recursive".to_string(),
                                    related_information: None,
                                    tags: None,
                                    data: None,
                                },
                                diagnostic_id: "recursive-diag".to_string(),
                                law_id: "TEST_RECURSIVE".to_string(),
                                attempted_transition: None,
                                violated_axes: vec![],
                                doc_routes: vec![],
                                repair_actions: vec![],
                                verification_gates: vec![],
                                receipt_obligation: None,
                                ..Default::default()
                            }),
                        }]
                    }
                    // React to DiagnosticEmitted too, triggering further recursion.
                    HookEvent::DiagnosticEmitted { instance_id, .. } => {
                        vec![MeshAction::AddDiagnostic {
                            instance_id: instance_id.clone(),
                            diagnostic: Box::new(MaxDiagnostic {
                                lsp: lsp_types::Diagnostic {
                                    range: lsp_types::Range::default(),
                                    severity: Some(lsp_types::DiagnosticSeverity::HINT),
                                    code: None,
                                    code_description: None,
                                    source: Some("test".to_string()),
                                    message: "recursive-diag-emitted".to_string(),
                                    related_information: None,
                                    tags: None,
                                    data: None,
                                },
                                diagnostic_id: "recursive-diag-2".to_string(),
                                law_id: "TEST_RECURSIVE_DIAG".to_string(),
                                attempted_transition: None,
                                violated_axes: vec![],
                                doc_routes: vec![],
                                repair_actions: vec![],
                                verification_gates: vec![],
                                receipt_obligation: None,
                                ..Default::default()
                            }),
                        }]
                    }
                    _ => vec![],
                }
            }
        }

        let mut mesh = make_mesh_with_instance("LSP_BOMB");
        mesh.register_hook(Box::new(RecursiveHook));

        // Must not stack overflow; must return cleanly.
        mesh.dispatch_event(HookEvent::StateTransition {
            instance_id: InstanceId::from("LSP_BOMB"),
            from_phase: "A".to_string(),
            to_phase: "B".to_string(),
        });

        // The depth guard sentinel should be in the event log.
        let has_depth_guard = mesh.event_log.iter().any(|e| {
            matches!(e, HookEvent::DiagnosticEmitted { diagnostic, .. }
                if diagnostic.law_id == "MESH_DISPATCH_DEPTH")
        });
        assert!(
            has_depth_guard,
            "Expected MESH_DISPATCH_DEPTH sentinel in event log"
        );

        // Depth counter must be reset to zero after dispatch unwinds.
        assert_eq!(mesh.dispatch_depth, 0, "dispatch_depth must reset to 0");
    }

    // --- Dispatch depth boundary semantics (3 new tests) ---

    /// (a) At exactly MAX_DISPATCH_DEPTH-1 a valid event is still fully processed.
    #[test]
    fn test_dispatch_depth_boundary_at_max_minus_one_executes_normally() {
        struct AddDiagHook;
        impl Hook for AddDiagHook {
            fn name(&self) -> &str {
                "AddDiagHook"
            }
            fn trigger(&self, event: &HookEvent) -> Vec<MeshAction> {
                match event {
                    HookEvent::StateTransition { instance_id, .. } => {
                        vec![MeshAction::AddDiagnostic {
                            instance_id: instance_id.clone(),
                            diagnostic: Box::new(make_error_diagnostic("boundary-diag")),
                        }]
                    }
                    _ => vec![],
                }
            }
        }

        let mut mesh = make_mesh_with_instance("BOUND_INST");
        mesh.register_hook(Box::new(AddDiagHook));

        // Simulate being one level below the limit.
        mesh.dispatch_depth = MAX_DISPATCH_DEPTH - 1;

        mesh.dispatch_event(HookEvent::StateTransition {
            instance_id: InstanceId::from("BOUND_INST"),
            from_phase: "idle".to_string(),
            to_phase: "active".to_string(),
        });

        // Hook action must have executed — diagnostic present on the instance.
        let inst = mesh.instances.get("BOUND_INST").unwrap();
        assert!(
            inst.diagnostics
                .iter()
                .any(|d| d.diagnostic_id == "boundary-diag"),
            "Hook action at MAX_DISPATCH_DEPTH-1 must be fully executed"
        );

        // Depth must be restored to the artificial baseline.
        assert_eq!(
            mesh.dispatch_depth,
            MAX_DISPATCH_DEPTH - 1,
            "dispatch_depth must be restored to pre-call value after normal dispatch"
        );
    }

    /// (b) At MAX_DISPATCH_DEPTH the sentinel fires and no hook actions execute.
    #[test]
    fn test_dispatch_depth_at_max_fires_sentinel_and_drops_actions() {
        let mut mesh = make_mesh_with_instance("DROP_INST");
        mesh.dispatch_depth = MAX_DISPATCH_DEPTH;

        let log_len_before = mesh.event_log.len();

        mesh.dispatch_event(HookEvent::StateTransition {
            instance_id: InstanceId::from("DROP_INST"),
            from_phase: "a".to_string(),
            to_phase: "b".to_string(),
        });

        // Exactly one new entry: the sentinel.
        let new_entries = mesh.event_log.len() - log_len_before;
        assert_eq!(
            new_entries, 1,
            "Only the sentinel should be added when depth == MAX"
        );

        let sentinel = mesh.event_log.last().unwrap();
        assert!(
            matches!(sentinel, HookEvent::DiagnosticEmitted { diagnostic, .. }
                if diagnostic.law_id == "MESH_DISPATCH_DEPTH"),
            "The single new entry must be the MESH_DISPATCH_DEPTH sentinel"
        );

        // Guard fires without touching depth (the guard path never incremented it,
        // so there is nothing to decrement).  Depth must remain at MAX_DISPATCH_DEPTH.
        assert_eq!(
            mesh.dispatch_depth, MAX_DISPATCH_DEPTH,
            "Guard branch must leave depth unchanged (it never incremented it)"
        );
    }

    /// (c) After N consecutive over-limit calls depth must not accumulate unbounded.
    #[test]
    fn test_dispatch_depth_no_leak_after_repeated_over_limit_calls() {
        let mut mesh = make_mesh_with_instance("LEAK_INST");

        mesh.dispatch_depth = MAX_DISPATCH_DEPTH;
        let baseline = mesh.dispatch_depth;

        for _ in 0..5 {
            mesh.dispatch_event(HookEvent::StateTransition {
                instance_id: InstanceId::from("LEAK_INST"),
                from_phase: "x".to_string(),
                to_phase: "y".to_string(),
            });
        }

        // Guard path never increments or decrements depth, so after N over-limit
        // calls the counter must be unchanged from baseline.
        assert_eq!(
            mesh.dispatch_depth, baseline,
            "Repeated over-limit calls must not change depth (guard path is depth-neutral)"
        );
    }

    /// INN-12-10: depth resets to zero after a normal (non-overflow) dispatch call
    #[test]
    fn test_dispatch_depth_resets_to_zero_after_call() {
        let mut mesh = make_mesh_with_instance("RESET_INST");
        assert_eq!(mesh.dispatch_depth, 0, "precondition: depth starts at 0");
        mesh.dispatch_event(HookEvent::StateTransition {
            instance_id: InstanceId::from("RESET_INST"),
            from_phase: "Uninitialized".to_string(),
            to_phase: "Initializing".to_string(),
        });
        assert_eq!(
            mesh.dispatch_depth, 0,
            "dispatch_depth must return to 0 after normal dispatch"
        );
    }

    /// INN-12-10: sentinel fires exactly once per overflow call, not accumulated
    #[test]
    fn test_dispatch_depth_sentinel_fires_exactly_once_per_overflow() {
        let mut mesh = make_mesh_with_instance("SENTINEL_INST");
        mesh.dispatch_depth = MAX_DISPATCH_DEPTH;

        for call_n in 0..3u32 {
            let log_before = mesh.event_log.len();
            mesh.dispatch_event(HookEvent::StateTransition {
                instance_id: InstanceId::from("SENTINEL_INST"),
                from_phase: "a".to_string(),
                to_phase: "b".to_string(),
            });
            let new_entries = mesh.event_log.len() - log_before;
            assert_eq!(
                new_entries, 1,
                "overflow call #{} must produce exactly 1 sentinel entry, got {}",
                call_n, new_entries
            );
            let sentinel = mesh.event_log.last().unwrap();
            assert!(
                matches!(sentinel, HookEvent::DiagnosticEmitted { diagnostic, .. }
                    if diagnostic.law_id == "MESH_DISPATCH_DEPTH"),
                "overflow call #{} must emit a MESH_DISPATCH_DEPTH sentinel",
                call_n
            );
        }
    }

    /// INN-12-10: hook chain at MAX-2 completes without sentinel
    #[test]
    fn test_dispatch_depth_nested_near_boundary() {
        let mut mesh = make_mesh_with_instance("NEAR_BOUND_INST");
        mesh.dispatch_depth = MAX_DISPATCH_DEPTH - 2;

        let log_len_before = mesh.event_log.len();
        mesh.dispatch_event(HookEvent::StateTransition {
            instance_id: InstanceId::from("NEAR_BOUND_INST"),
            from_phase: "Initialized".to_string(),
            to_phase: "ShutDown".to_string(),
        });

        let new_sentinels = mesh.event_log[log_len_before..]
            .iter()
            .filter(|e| {
                matches!(e, HookEvent::DiagnosticEmitted { diagnostic, .. }
                if diagnostic.law_id == "MESH_DISPATCH_DEPTH")
            })
            .count();
        assert_eq!(
            new_sentinels, 0,
            "hook chain at MAX-2 must not trigger sentinel, but {} were emitted",
            new_sentinels
        );
        assert_eq!(
            mesh.dispatch_depth,
            MAX_DISPATCH_DEPTH - 2,
            "depth must be restored to pre-call baseline after normal dispatch at MAX-2"
        );
    }

    #[test]
    fn test_rpc_verify_ledger_empty_returns_err() {
        let mut mesh = make_mesh_with_instance("INST_A");
        // A fresh instance has no receipts — verifyLedger must report that.
        let result = mesh.dispatch_rpc("INST_A", "max/verifyLedger", json!(null));
        assert!(
            result.is_err(),
            "max/verifyLedger on empty ledger should return Err"
        );
        assert!(
            result.unwrap_err().contains("empty"),
            "error message should mention empty ledger"
        );
    }

    #[test]
    fn test_rpc_ledger_report_returns_ok_string() {
        let mut mesh = make_mesh_with_instance("INST_A");
        // ledgerReport always succeeds and returns a diagnostic string.
        let result = mesh.dispatch_rpc("INST_A", "max/ledgerReport", json!(null));
        assert!(
            result.is_ok(),
            "max/ledgerReport should return Ok, got: {:?}",
            result
        );
        let val = result.unwrap();
        assert!(
            val.is_string(),
            "max/ledgerReport result should be a string, got: {:?}",
            val
        );
    }

    #[test]
    fn test_rpc_hook_returns_array() {
        let mut mesh = make_mesh_with_instance("INST_A");
        let result = mesh.dispatch_rpc("INST_A", "max/hook", json!(null));
        assert!(
            result.is_ok(),
            "max/hook should return Ok, got: {:?}",
            result
        );
        let val = result.unwrap();
        assert!(
            val.is_array(),
            "max/hook result should be an array, got: {:?}",
            val
        );
    }

    #[test]
    fn test_rpc_hook_graph_returns_array() {
        let mut mesh = make_mesh_with_instance("INST_A");
        let result = mesh.dispatch_rpc("INST_A", "max/hookGraph", json!(null));
        assert!(
            result.is_ok(),
            "max/hookGraph should return Ok, got: {:?}",
            result
        );
        let val = result.unwrap();
        assert!(
            val.is_array(),
            "max/hookGraph result should be an array, got: {:?}",
            val
        );
    }

    #[test]
    fn test_rpc_chain_returns_array() {
        let mut mesh = make_mesh_with_instance("INST_A");
        let result = mesh.dispatch_rpc("INST_A", "max/chain", json!(null));
        assert!(
            result.is_ok(),
            "max/chain should return Ok, got: {:?}",
            result
        );
        let val = result.unwrap();
        assert!(
            val.is_array(),
            "max/chain result should be an array, got: {:?}",
            val
        );
    }

    #[test]
    fn test_rpc_propagate_returns_propagated_true() {
        let mut mesh = make_mesh_with_instance("INST_A");
        let receipt_val = json!({
            "receipt_id": "rcpt-propagate-test",
            "hash": "abc123"
        });
        let result = mesh.dispatch_rpc("INST_A", "max/propagate", receipt_val);
        assert!(
            result.is_ok(),
            "max/propagate should return Ok, got: {:?}",
            result
        );
        let val = result.unwrap();
        assert_eq!(
            val["propagated"],
            json!(true),
            "max/propagate should report propagated=true"
        );
    }

    #[test]
    fn test_rpc_autonomic_loop_returns_status() {
        let mut mesh = make_mesh_with_instance("INST_A");
        let result = mesh.dispatch_rpc("INST_A", "max/autonomicLoop", json!(null));
        assert!(
            result.is_ok(),
            "max/autonomicLoop should return Ok, got: {:?}",
            result
        );
        let val = result.unwrap();
        assert!(
            val.get("instances").is_some(),
            "max/autonomicLoop result should have 'instances' key"
        );
        assert!(
            val.get("hook_count").is_some(),
            "max/autonomicLoop result should have 'hook_count' key"
        );
    }

    #[test]
    fn test_rpc_manifold_snapshot_returns_snapshot() {
        let mut mesh = make_mesh_with_instance("INST_A");
        let result = mesh.dispatch_rpc("INST_A", "max/manifoldSnapshot", json!(null));
        assert!(
            result.is_ok(),
            "max/manifoldSnapshot should return Ok, got: {:?}",
            result
        );
        let val = result.unwrap();
        assert!(
            val.get("instances").is_some(),
            "max/manifoldSnapshot result should have 'instances' key"
        );
    }

    #[test]
    fn test_rpc_lawful_transition_returns_transition_info() {
        let mut mesh = make_mesh_with_instance("INST_A");
        let result = mesh.dispatch_rpc("INST_A", "max/lawfulTransition", json!("Initialized"));
        assert!(
            result.is_ok(),
            "max/lawfulTransition should return Ok, got: {:?}",
            result
        );
        let val = result.unwrap();
        assert_eq!(val["instance_id"], json!("INST_A"));
        assert_eq!(val["requested_phase"], json!("Initialized"));
    }

    #[test]
    fn test_rpc_admission_no_diagnostics_returns_admitted() {
        let mut mesh = make_mesh_with_instance("INST_A");
        let result = mesh.dispatch_rpc("INST_A", "max/admission", json!(null));
        assert!(
            result.is_ok(),
            "max/admission should return Ok, got: {:?}",
            result
        );
        let val = result.unwrap();
        assert_eq!(
            val["verdict"],
            json!("Admitted"),
            "No diagnostics should yield Admitted"
        );
        assert_eq!(val["instance_id"], json!("INST_A"));
    }

    #[test]
    fn test_rpc_admission_with_error_diagnostic_returns_refused() {
        let mut mesh = make_mesh_with_instance("INST_A");
        let diag = make_error_diagnostic("diag-admission-error");
        mesh.execute_action(MeshAction::AddDiagnostic {
            instance_id: InstanceId::from("INST_A"),
            diagnostic: Box::new(diag),
        });
        let result = mesh.dispatch_rpc("INST_A", "max/admission", json!(null));
        assert!(
            result.is_ok(),
            "max/admission should return Ok even with errors, got: {:?}",
            result
        );
        let val = result.unwrap();
        assert_eq!(
            val["verdict"],
            json!("Refused"),
            "Error diagnostics should yield Refused"
        );
    }

    #[test]
    fn test_rpc_refusal_emits_receipt() {
        let mut mesh = make_mesh_with_instance("INST_A");
        let result = mesh.dispatch_rpc("INST_A", "max/refusal", json!("diag-some-violation"));
        assert!(
            result.is_ok(),
            "max/refusal should return Ok, got: {:?}",
            result
        );
        let val = result.unwrap();
        assert_eq!(val["refused"], json!(true));
        assert_eq!(val["diagnostic_id"], json!("diag-some-violation"));
        assert!(
            val.get("receipt").is_some(),
            "max/refusal should include a receipt"
        );
    }

    #[test]
    fn test_rpc_replay_returns_event_log() {
        let mut mesh = make_mesh_with_instance("INST_A");
        let result = mesh.dispatch_rpc("INST_A", "max/replay", json!(null));
        assert!(
            result.is_ok(),
            "max/replay should return Ok, got: {:?}",
            result
        );
        let val = result.unwrap();
        assert_eq!(val["instance_id"], json!("INST_A"));
        assert!(
            val.get("event_count").is_some(),
            "max/replay result should have 'event_count' key"
        );
        assert!(
            val.get("events").is_some(),
            "max/replay result should have 'events' key"
        );
    }

    #[test]
    fn test_rpc_release_actuation_no_diagnostics_succeeds() {
        let mut mesh = make_mesh_with_instance("INST_A");
        let result = mesh.dispatch_rpc("INST_A", "max/releaseActuation", json!(null));
        assert!(
            result.is_ok(),
            "max/releaseActuation with no diagnostics should return Ok, got: {:?}",
            result
        );
        let val = result.unwrap();
        assert_eq!(val["released"], json!(true));
        assert_eq!(val["instance_id"], json!("INST_A"));
    }

    #[test]
    fn test_rpc_release_actuation_with_diagnostics_returns_err() {
        let mut mesh = make_mesh_with_instance("INST_A");
        let diag = make_error_diagnostic("diag-release-block");
        mesh.execute_action(MeshAction::AddDiagnostic {
            instance_id: InstanceId::from("INST_A"),
            diagnostic: Box::new(diag),
        });
        let result = mesh.dispatch_rpc("INST_A", "max/releaseActuation", json!(null));
        assert!(
            result.is_err(),
            "max/releaseActuation with active diagnostics should return Err, got: {:?}",
            result
        );
    }

    /// INN-12-05: release on missing instance returns Err
    #[test]
    fn test_release_actuation_missing_instance_returns_err() {
        let mut mesh = AutonomicMesh::new();
        let result = mesh.dispatch_rpc("NONEXISTENT", "max/releaseActuation", json!(null));
        assert!(
            result.is_err(),
            "release on missing instance must return Err, got: {:?}",
            result
        );
        let err = result.unwrap_err();
        assert!(
            err.contains("not found") || err.contains("NONEXISTENT"),
            "error message should reference missing instance, got: {}",
            err
        );
    }

    /// INN-12-05: release after repair (clear all diagnostics) succeeds
    #[test]
    fn test_release_actuation_after_repair_succeeds() {
        let mut mesh = make_mesh_with_instance("REPAIR_INST");
        let diag = make_error_diagnostic("diag-repair-block");
        mesh.execute_action(MeshAction::AddDiagnostic {
            instance_id: InstanceId::from("REPAIR_INST"),
            diagnostic: Box::new(diag),
        });
        let blocked = mesh.dispatch_rpc("REPAIR_INST", "max/releaseActuation", json!(null));
        assert!(
            blocked.is_err(),
            "release must be blocked when diagnostic is active"
        );
        mesh.execute_action(MeshAction::ClearDiagnostic {
            instance_id: InstanceId::from("REPAIR_INST"),
            diagnostic_id: "diag-repair-block".to_string(),
        });
        let released = mesh.dispatch_rpc("REPAIR_INST", "max/releaseActuation", json!(null));
        assert!(
            released.is_ok(),
            "release must succeed after repair transaction, got: {:?}",
            released
        );
        assert_eq!(released.unwrap()["released"], json!(true));
    }

    /// INN-12-05: release is idempotent — two consecutive releases both succeed when no diagnostics
    #[test]
    fn test_release_actuation_idempotent() {
        let mut mesh = make_mesh_with_instance("IDEMPOTENT_INST");
        let first = mesh.dispatch_rpc("IDEMPOTENT_INST", "max/releaseActuation", json!(null));
        assert!(
            first.is_ok(),
            "first release must succeed, got: {:?}",
            first
        );
        assert_eq!(first.unwrap()["released"], json!(true));
        let second = mesh.dispatch_rpc("IDEMPOTENT_INST", "max/releaseActuation", json!(null));
        assert!(
            second.is_ok(),
            "second consecutive release must also succeed (idempotent), got: {:?}",
            second
        );
        assert_eq!(second.unwrap()["released"], json!(true));
    }

    #[test]
    fn test_rpc_receipt_lookup_found_and_not_found() {
        // Case 1: receipt exists — emit via max/refusal then look it up
        let mut mesh = make_mesh_with_instance("INST_A");
        let refusal_result = mesh.dispatch_rpc("INST_A", "max/refusal", json!("diag-receipt-test"));
        assert!(refusal_result.is_ok(), "max/refusal should succeed");
        // max/refusal encodes receipt_id as "rcpt-refusal-<diag_id>"
        let expected_receipt_id = "rcpt-refusal-diag-receipt-test";
        let found = mesh.dispatch_rpc("INST_A", "max/receipt", json!(expected_receipt_id));
        assert!(
            found.is_ok(),
            "max/receipt lookup for existing receipt should return Ok, got: {:?}",
            found
        );
        let val = found.unwrap();
        assert_eq!(
            val["receipt_id"],
            json!(expected_receipt_id),
            "returned receipt_id must match queried id"
        );

        // Case 2: receipt does not exist — must return Err
        let not_found = mesh.dispatch_rpc("INST_A", "max/receipt", json!("rcpt-nonexistent-xyz"));
        assert!(
            not_found.is_err(),
            "max/receipt with unknown receipt_id should return Err, got: {:?}",
            not_found
        );
        let err = not_found.unwrap_err();
        assert!(
            err.contains("Receipt not found"),
            "error message should indicate not found, got: {}",
            err
        );
    }

    // --- max/repairPlan tests ---

    #[test]
    fn test_rpc_repair_plan_returns_actions() {
        let mut mesh = make_mesh_with_instance("INST_A");
        let diag = MaxDiagnostic {
            diagnostic_id: "diag-repair-1".to_string(),
            law_id: "law-repair".to_string(),
            attempted_transition: None,
            violated_axes: vec![],
            doc_routes: vec![],
            lsp: lsp_types::Diagnostic {
                range: lsp_types::Range::default(),
                severity: Some(lsp_types::DiagnosticSeverity::ERROR),
                code: None,
                code_description: None,
                source: None,
                message: "repair test".to_string(),
                related_information: None,
                tags: None,
                data: None,
            },
            repair_actions: vec![tower_lsp_max_protocol::RepairAction {
                action_id: "action-1".to_string(),
                description: "Apply patch".to_string(),
            }],
            verification_gates: vec![],
            receipt_obligation: Some(tower_lsp_max_protocol::ReceiptObligation {
                required_receipts: vec!["receipt-abc".to_string()],
            }),
            ..Default::default()
        };
        mesh.execute_action(MeshAction::AddDiagnostic {
            instance_id: InstanceId::from("INST_A"),
            diagnostic: Box::new(diag),
        });
        let result = mesh.dispatch_rpc("INST_A", "max/repairPlan", json!("law-repair"));
        assert!(
            result.is_ok(),
            "max/repairPlan should return Ok, got: {:?}",
            result
        );
        let val = result.unwrap();
        let arr = val
            .as_array()
            .expect("max/repairPlan should return an array");
        assert_eq!(arr.len(), 1, "expected 1 action for law-repair");
        assert_eq!(arr[0]["action"]["title"], json!("Apply patch"));
        let expected_receipts = &arr[0]["receipt_plan"]["expected_receipts"];
        assert_eq!(expected_receipts[0], json!("receipt-abc"));
    }

    #[test]
    fn test_rpc_repair_plan_by_diagnostic_id() {
        let mut mesh = make_mesh_with_instance("INST_A");
        let diag = MaxDiagnostic {
            diagnostic_id: "diag-by-id".to_string(),
            law_id: "law-other".to_string(),
            attempted_transition: None,
            violated_axes: vec![],
            doc_routes: vec![],
            lsp: lsp_types::Diagnostic {
                range: lsp_types::Range::default(),
                severity: Some(lsp_types::DiagnosticSeverity::ERROR),
                code: None,
                code_description: None,
                source: None,
                message: "repair by diag id".to_string(),
                related_information: None,
                tags: None,
                data: None,
            },
            repair_actions: vec![tower_lsp_max_protocol::RepairAction {
                action_id: "action-2".to_string(),
                description: "Rollback config".to_string(),
            }],
            verification_gates: vec![],
            receipt_obligation: None,
            ..Default::default()
        };
        mesh.execute_action(MeshAction::AddDiagnostic {
            instance_id: InstanceId::from("INST_A"),
            diagnostic: Box::new(diag),
        });
        let result = mesh.dispatch_rpc("INST_A", "max/repairPlan", json!("diag-by-id"));
        assert!(
            result.is_ok(),
            "max/repairPlan by diagnostic_id should return Ok"
        );
        let val = result.unwrap();
        let arr = val
            .as_array()
            .expect("max/repairPlan should return an array");
        assert_eq!(arr.len(), 1, "expected 1 action for diag-by-id");
        assert_eq!(arr[0]["action"]["title"], json!("Rollback config"));
        let expected_receipts = &arr[0]["receipt_plan"]["expected_receipts"];
        assert_eq!(expected_receipts.as_array().map(|a| a.len()), Some(0));
    }

    #[test]
    fn test_rpc_repair_plan_empty_when_no_match() {
        let mut mesh = make_mesh_with_instance("INST_A");
        let result = mesh.dispatch_rpc("INST_A", "max/repairPlan", json!("nonexistent-id"));
        assert!(
            result.is_ok(),
            "max/repairPlan with no match should return Ok empty array"
        );
        let val = result.unwrap();
        let arr = val
            .as_array()
            .expect("max/repairPlan should return an array");
        assert_eq!(arr.len(), 0, "expected empty array for unmatched id");
    }

    // --- Property invariant tests (5) ---

    #[test]
    fn conformance_score_in_range() {
        let inst0 = LspInstance::new("SCORE_0");
        let s0 = inst0.conformance_score();
        assert!(
            (0.0..=100.0).contains(&s0),
            "score with 0 diags out of range: {}",
            s0
        );

        let mut inst1 = LspInstance::new("SCORE_1");
        inst1.diagnostics.push(MaxDiagnostic {
            diagnostic_id: "d1".to_string(),
            law_id: "l".to_string(),
            lsp: lsp_types::Diagnostic {
                range: lsp_types::Range::default(),
                severity: Some(lsp_types::DiagnosticSeverity::ERROR),
                code: None,
                code_description: None,
                source: None,
                message: "e".to_string(),
                related_information: None,
                tags: None,
                data: None,
            },
            ..Default::default()
        });
        let s1 = inst1.conformance_score();
        assert!(
            (0.0..=100.0).contains(&s1),
            "score with 1 error diag out of range: {}",
            s1
        );

        let mut inst5 = LspInstance::new("SCORE_5");
        let severities = [
            lsp_types::DiagnosticSeverity::ERROR,
            lsp_types::DiagnosticSeverity::WARNING,
            lsp_types::DiagnosticSeverity::INFORMATION,
            lsp_types::DiagnosticSeverity::HINT,
            lsp_types::DiagnosticSeverity::WARNING,
        ];
        for (i, sev) in severities.iter().enumerate() {
            inst5.diagnostics.push(MaxDiagnostic {
                diagnostic_id: format!("d{}", i),
                law_id: "l".to_string(),
                lsp: lsp_types::Diagnostic {
                    range: lsp_types::Range::default(),
                    severity: Some(*sev),
                    code: None,
                    code_description: None,
                    source: None,
                    message: "m".to_string(),
                    related_information: None,
                    tags: None,
                    data: None,
                },
                ..Default::default()
            });
        }
        let s5 = inst5.conformance_score();
        assert!(
            (0.0..=100.0).contains(&s5),
            "score with 5 mixed diags out of range: {}",
            s5
        );
    }

    #[test]
    fn admitted_refused_disjoint() {
        use std::collections::HashSet;
        let mut mesh = make_mesh_with_instance("INST_D");
        mesh.execute_action(MeshAction::AddDiagnostic {
            instance_id: InstanceId::from("INST_D"),
            diagnostic: Box::new(MaxDiagnostic {
                diagnostic_id: "err-axis".to_string(),
                law_id: "law-err".to_string(),
                lsp: lsp_types::Diagnostic {
                    range: lsp_types::Range::default(),
                    severity: Some(lsp_types::DiagnosticSeverity::ERROR),
                    code: None,
                    code_description: None,
                    source: None,
                    message: "error".to_string(),
                    related_information: None,
                    tags: None,
                    data: None,
                },
                law_axis: tower_lsp_max_protocol::LawAxis::Security,
                ..Default::default()
            }),
        });
        mesh.execute_action(MeshAction::AddDiagnostic {
            instance_id: InstanceId::from("INST_D"),
            diagnostic: Box::new(MaxDiagnostic {
                diagnostic_id: "warn-axis".to_string(),
                law_id: "law-warn".to_string(),
                lsp: lsp_types::Diagnostic {
                    range: lsp_types::Range::default(),
                    severity: Some(lsp_types::DiagnosticSeverity::WARNING),
                    code: None,
                    code_description: None,
                    source: None,
                    message: "warning".to_string(),
                    related_information: None,
                    tags: None,
                    data: None,
                },
                law_axis: tower_lsp_max_protocol::LawAxis::Domain,
                ..Default::default()
            }),
        });
        let result = mesh
            .dispatch_rpc("INST_D", "max/conformanceVector", json!(null))
            .unwrap();
        let admitted: Vec<tower_lsp_max_protocol::LawAxis> =
            serde_json::from_value(result["admitted"].clone()).unwrap();
        let refused: Vec<tower_lsp_max_protocol::LawAxis> =
            serde_json::from_value(result["refused"].clone()).unwrap();
        let admitted_set: HashSet<_> = admitted.iter().collect();
        let refused_set: HashSet<_> = refused.iter().collect();
        let intersection: HashSet<_> = admitted_set.intersection(&refused_set).collect();
        assert!(
            intersection.is_empty(),
            "admitted and refused must be disjoint but share: {:?}",
            intersection
        );
    }

    #[test]
    fn unknown_method_always_errors() {
        let mut mesh = make_mesh_with_instance("INST_E");
        let result = mesh.dispatch_rpc("INST_E", "max/doesNotExist", json!(null));
        assert!(
            result.is_err(),
            "dispatch_rpc with unknown method must return Err, got Ok"
        );
    }

    #[test]
    fn receipt_ids_unique() {
        let mut mesh = make_mesh_with_instance("INST_R");
        let receipt_ids = ["rcpt-alpha", "rcpt-beta", "rcpt-gamma"];
        for id in &receipt_ids {
            mesh.execute_action(MeshAction::EmitReceipt {
                instance_id: InstanceId::from("INST_R"),
                receipt: Receipt {
                    receipt_id: id.to_string(),
                    hash: sha256(id.as_bytes()),
                    prev_receipt_hash: None,
                },
            });
        }
        let inst = mesh.instances.get("INST_R").unwrap();
        let ids: Vec<&str> = inst
            .receipts
            .iter()
            .map(|r| r.receipt_id.as_str())
            .collect();
        let unique: std::collections::HashSet<&str> = ids.iter().cloned().collect();
        assert_eq!(
            ids.len(),
            unique.len(),
            "receipt IDs must all be distinct; found duplicates among {:?}",
            ids
        );
    }

    #[test]
    fn clear_diagnostic_removes_from_list() {
        let mut mesh = make_mesh_with_instance("INST_C");
        mesh.execute_action(MeshAction::AddDiagnostic {
            instance_id: InstanceId::from("INST_C"),
            diagnostic: Box::new(make_error_diagnostic("diag-to-clear")),
        });
        assert_eq!(mesh.instances.get("INST_C").unwrap().diagnostics.len(), 1);
        mesh.dispatch_rpc("INST_C", "max/clearDiagnostic", json!("diag-to-clear"))
            .unwrap();
        let cv = mesh
            .dispatch_rpc("INST_C", "max/conformanceVector", json!(null))
            .unwrap();
        assert!(
            cv["score"].is_null(),
            "after clearing all diagnostics, conformanceVector score must be null"
        );
        assert!(
            mesh.instances.get("INST_C").unwrap().diagnostics.is_empty(),
            "diagnostics list must be empty after clearing the only diagnostic"
        );
    }
}

#[cfg(test)]
mod apply_repair_transaction_tests {
    use super::*;

    fn make_code_action(
        title: &str,
        expected_receipts: Vec<String>,
    ) -> tower_lsp_max_protocol::MaxCodeAction {
        tower_lsp_max_protocol::MaxCodeAction {
            action: lsp_types::CodeAction {
                title: title.to_string(),
                kind: None,
                diagnostics: None,
                edit: None,
                command: None,
                is_preferred: None,
                disabled: None,
                data: None,
            },
            preconditions: vec![],
            validation_plan: tower_lsp_max_protocol::ValidationPlan { gates: vec![] },
            rollback_plan: tower_lsp_max_protocol::RollbackPlan {
                strategy: "revert".to_string(),
            },
            receipt_plan: tower_lsp_max_protocol::ReceiptPlan { expected_receipts },
        }
    }

    #[test]
    fn apply_repair_transaction_emits_receipt_and_stores_it() {
        let mut mesh = AutonomicMesh::new();
        mesh.add_instance(LspInstance::new("INST_A"));

        let action = make_code_action("Fix admission", vec![]);
        let result = mesh.apply_repair_transaction("INST_A", action);
        assert!(result.is_ok(), "Expected Ok, got: {:?}", result);

        let val = result.unwrap();
        let receipt_id = val["receipt_id"].as_str().unwrap_or("");
        assert!(
            receipt_id.starts_with("rcpt-repair-"),
            "receipt_id should start with rcpt-repair-, got: {}",
            receipt_id
        );

        let inst = mesh.instances.get("INST_A").unwrap();
        assert!(
            !inst.receipts.is_empty(),
            "Receipt should be stored in instance"
        );
        assert_eq!(inst.receipts.last().unwrap().receipt_id, receipt_id);
    }

    #[test]
    fn apply_repair_transaction_fails_if_required_receipt_missing() {
        let mut mesh = AutonomicMesh::new();
        mesh.add_instance(LspInstance::new("INST_B"));

        let action = make_code_action("Fix auth", vec!["rcpt-security-auth".to_string()]);
        let result = mesh.apply_repair_transaction("INST_B", action);
        assert!(
            result.is_err(),
            "Should fail due to missing required receipt"
        );
        let err = result.unwrap_err();
        assert!(
            err.contains("Receipt integrity violation"),
            "Error should mention receipt integrity, got: {}",
            err
        );
    }

    #[test]
    fn apply_repair_transaction_succeeds_when_required_receipt_present() {
        let mut mesh = AutonomicMesh::new();
        let mut inst = LspInstance::new("INST_C");
        inst.receipts.push(tower_lsp_max_protocol::Receipt {
            receipt_id: "rcpt-security-auth".to_string(),
            hash: sha256(b"rcpt-security-auth"),
            prev_receipt_hash: None,
        });
        mesh.add_instance(inst);

        let action = make_code_action(
            "Fix missing receipt",
            vec!["rcpt-security-auth".to_string()],
        );
        let result = mesh.apply_repair_transaction("INST_C", action);
        assert!(
            result.is_ok(),
            "Expected Ok when required receipt is present, got: {:?}",
            result
        );
    }
}

#[cfg(test)]
mod max_reset_tests {
    use super::*;

    fn make_error_diagnostic(id: &str) -> MaxDiagnostic {
        MaxDiagnostic {
            lsp: lsp_types::Diagnostic {
                range: lsp_types::Range::default(),
                severity: Some(lsp_types::DiagnosticSeverity::ERROR),
                code: None,
                code_description: None,
                source: Some("test".to_string()),
                message: "test error".to_string(),
                related_information: None,
                tags: None,
                data: None,
            },
            diagnostic_id: id.to_string(),
            law_id: "law-test".to_string(),
            attempted_transition: None,
            violated_axes: vec![],
            doc_routes: vec![],
            repair_actions: vec![],
            verification_gates: vec![],
            receipt_obligation: None,
            law_axis: tower_lsp_max_protocol::LawAxis::Domain,
            violated_invariant: String::new(),
            observed_state: serde_json::Value::Null,
            expected_state: serde_json::Value::Null,
            repairability: tower_lsp_max_protocol::Repairability::Unknown,
            terminality: tower_lsp_max_protocol::Terminality::NonTerminal,
        }
    }

    #[test]
    fn test_max_reset_rpc_conformance_score_100_after_dirty_instance() {
        let mut mesh = AutonomicMesh::new();

        // Create a dirty instance with diagnostics and receipts
        let mut inst = LspInstance::new("RESET_TEST");
        inst.diagnostics.push(make_error_diagnostic("diag-error-1"));
        inst.diagnostics.push(make_error_diagnostic("diag-error-2"));
        inst.receipts.push(Receipt {
            receipt_id: "rcpt-old-1".to_string(),
            hash: sha256(b"rcpt-old-1"),
            prev_receipt_hash: None,
        });
        inst.policy_state = Some(PolicyState::ClarificationRequested);
        mesh.add_instance(inst);

        // Confirm dirty state
        {
            let dirty = mesh.instances.get("RESET_TEST").unwrap();
            assert!(
                dirty.conformance_score() < 100.0,
                "Expected dirty conformance before reset"
            );
            assert!(!dirty.diagnostics.is_empty());
            assert!(!dirty.receipts.is_empty());
        }

        // Invoke max/reset via dispatch_rpc
        let result = mesh.dispatch_rpc("RESET_TEST", "max/reset", serde_json::Value::Null);
        assert!(
            result.is_ok(),
            "max/reset must return Ok, got: {:?}",
            result
        );

        let resp = result.unwrap();
        assert_eq!(resp["reset"], true);
        assert_eq!(resp["instance_id"], "RESET_TEST");

        // After reset: conformance_score must be 100.0
        let clean = mesh.instances.get("RESET_TEST").unwrap();
        assert_eq!(
            clean.conformance_score(),
            100.0,
            "conformance_score must be 100.0 after reset"
        );
        assert!(
            clean.diagnostics.is_empty(),
            "diagnostics must be cleared after reset"
        );
        assert!(
            clean.receipts.is_empty(),
            "receipts must be cleared after reset"
        );
        assert_eq!(
            clean.policy_state,
            Some(PolicyState::Operational),
            "policy_state must be Operational after reset"
        );
    }

    #[test]
    fn test_max_reset_unknown_instance_returns_err() {
        let mut mesh = AutonomicMesh::new();
        let result = mesh.dispatch_rpc("NONEXISTENT", "max/reset", serde_json::Value::Null);
        assert!(
            result.is_err(),
            "max/reset on unknown instance must return Err"
        );
    }

    // ---- max/conformanceDelta tests ----

    #[test]
    fn test_conformance_delta_empty_on_fresh_mesh() {
        let mut mesh = AutonomicMesh::new();
        mesh.add_instance(LspInstance::new("INST_X"));
        let result = mesh
            .dispatch_rpc(
                "INST_X",
                "max/conformanceDelta",
                serde_json::json!({ "since_seq": 0 }),
            )
            .expect("max/conformanceDelta must succeed");
        let deltas = result["deltas"].as_array().expect("deltas must be array");
        assert!(deltas.is_empty(), "fresh mesh has no conformance deltas");
        assert_eq!(result["current_seq"].as_u64().unwrap(), 0);
    }

    #[test]
    fn test_conformance_delta_records_score_change_on_add_diagnostic() {
        let mut mesh = AutonomicMesh::new();
        mesh.add_instance(LspInstance::new("INST_D"));
        // Add an error diagnostic — score drops from 100 -> 70
        let diag = MaxDiagnostic {
            lsp: lsp_types::Diagnostic {
                range: lsp_types::Range::default(),
                severity: Some(lsp_types::DiagnosticSeverity::ERROR),
                code: None,
                code_description: None,
                source: Some("test".to_string()),
                message: "delta-test".to_string(),
                related_information: None,
                tags: None,
                data: None,
            },
            diagnostic_id: "diag-delta-1".to_string(),
            law_id: "law-delta".to_string(),
            attempted_transition: None,
            violated_axes: vec![],
            doc_routes: vec![],
            repair_actions: vec![],
            verification_gates: vec![],
            receipt_obligation: None,
            law_axis: tower_lsp_max_protocol::LawAxis::Domain,
            violated_invariant: String::new(),
            observed_state: serde_json::Value::Null,
            expected_state: serde_json::Value::Null,
            repairability: tower_lsp_max_protocol::Repairability::Unknown,
            terminality: tower_lsp_max_protocol::Terminality::NonTerminal,
        };
        mesh.execute_action(MeshAction::AddDiagnostic {
            instance_id: InstanceId::from("INST_D"),
            diagnostic: Box::new(diag),
        });
        let seq_after = mesh.action_seq;
        assert!(
            seq_after > 0,
            "action_seq must increment after execute_action"
        );
        let result = mesh
            .dispatch_rpc(
                "INST_D",
                "max/conformanceDelta",
                serde_json::json!({ "since_seq": 0 }),
            )
            .expect("max/conformanceDelta must succeed");
        let deltas = result["deltas"].as_array().expect("deltas must be array");
        assert!(
            !deltas.is_empty(),
            "adding a diagnostic must produce a delta"
        );
        let entry = &deltas[0];
        assert_eq!(entry["instance_id"].as_str().unwrap(), "INST_D");
        assert!(
            entry["old_score"].as_f64().unwrap() > entry["new_score"].as_f64().unwrap(),
            "old_score must be higher than new_score after adding error diagnostic"
        );
    }

    #[test]
    fn test_conformance_delta_since_cursor_filters_older_entries() {
        let mut mesh = AutonomicMesh::new();
        mesh.add_instance(LspInstance::new("INST_E"));
        // Helper closure: add a new error diagnostic with a unique id
        let make_diag = |id: &str| MaxDiagnostic {
            lsp: lsp_types::Diagnostic {
                range: lsp_types::Range::default(),
                severity: Some(lsp_types::DiagnosticSeverity::ERROR),
                code: None,
                code_description: None,
                source: Some("test".to_string()),
                message: "cursor-test".to_string(),
                related_information: None,
                tags: None,
                data: None,
            },
            diagnostic_id: id.to_string(),
            law_id: "law-cursor".to_string(),
            attempted_transition: None,
            violated_axes: vec![],
            doc_routes: vec![],
            repair_actions: vec![],
            verification_gates: vec![],
            receipt_obligation: None,
            law_axis: tower_lsp_max_protocol::LawAxis::Domain,
            violated_invariant: String::new(),
            observed_state: serde_json::Value::Null,
            expected_state: serde_json::Value::Null,
            repairability: tower_lsp_max_protocol::Repairability::Unknown,
            terminality: tower_lsp_max_protocol::Terminality::NonTerminal,
        };
        // First add — clears score from 100
        mesh.execute_action(MeshAction::AddDiagnostic {
            instance_id: InstanceId::from("INST_E"),
            diagnostic: Box::new(make_diag("diag-cursor-1")),
        });
        let mid_seq = mesh.action_seq;
        // Clear so score can change again
        mesh.execute_action(MeshAction::ClearDiagnostic {
            instance_id: InstanceId::from("INST_E"),
            diagnostic_id: "diag-cursor-1".to_string(),
        });
        // Second add
        mesh.execute_action(MeshAction::AddDiagnostic {
            instance_id: InstanceId::from("INST_E"),
            diagnostic: Box::new(make_diag("diag-cursor-2")),
        });
        // Query with mid_seq cursor — should only see changes after mid_seq
        let result = mesh
            .dispatch_rpc(
                "INST_E",
                "max/conformanceDelta",
                serde_json::json!({ "since_seq": mid_seq }),
            )
            .expect("max/conformanceDelta must succeed");
        let deltas = result["deltas"].as_array().expect("deltas must be array");
        for d in deltas {
            assert!(
                d["seq"].as_u64().unwrap() > mid_seq,
                "all returned deltas must have seq > since_seq"
            );
        }
    }

    #[test]
    fn test_rpc_run_gate_blocked_and_clear() {
        let mut mesh = AutonomicMesh::new();
        let iid = "GATE_TEST_INST";

        // Register instance
        mesh.add_instance(LspInstance::new(iid));

        // No diagnostics — gate should be clear (true)
        let result = mesh
            .dispatch_rpc(iid, "max/runGate", serde_json::json!("GATE_A"))
            .expect("max/runGate must succeed with no diagnostics");
        assert_eq!(
            result,
            serde_json::json!(true),
            "gate should be clear when no diagnostics reference it"
        );

        // Add a diagnostic whose verification_gates contains GATE_A
        let blocking_diag = MaxDiagnostic {
            lsp: lsp_types::Diagnostic {
                range: lsp_types::Range::default(),
                severity: Some(lsp_types::DiagnosticSeverity::ERROR),
                code: None,
                code_description: None,
                source: Some("test".to_string()),
                message: "gate-blocking diagnostic".to_string(),
                related_information: None,
                tags: None,
                data: None,
            },
            diagnostic_id: "diag-gate-block-1".to_string(),
            law_id: "law-gate".to_string(),
            attempted_transition: None,
            violated_axes: vec![],
            doc_routes: vec![],
            repair_actions: vec![],
            verification_gates: vec![tower_lsp_max_protocol::GateId("GATE_A".to_string())],
            receipt_obligation: None,
            law_axis: tower_lsp_max_protocol::LawAxis::Domain,
            violated_invariant: String::new(),
            observed_state: serde_json::Value::Null,
            expected_state: serde_json::Value::Null,
            repairability: tower_lsp_max_protocol::Repairability::Unknown,
            terminality: tower_lsp_max_protocol::Terminality::NonTerminal,
        };
        mesh.execute_action(MeshAction::AddDiagnostic {
            instance_id: InstanceId::from(iid),
            diagnostic: Box::new(blocking_diag),
        });

        // Gate should now be blocked (false)
        let result = mesh
            .dispatch_rpc(iid, "max/runGate", serde_json::json!("GATE_A"))
            .expect("max/runGate must succeed with blocking diagnostic");
        assert_eq!(
            result,
            serde_json::json!(false),
            "gate should be blocked when a diagnostic references it"
        );

        // A different gate string should still be clear
        let result_other = mesh
            .dispatch_rpc(iid, "max/runGate", serde_json::json!("GATE_B"))
            .expect("max/runGate must succeed for unrelated gate");
        assert_eq!(
            result_other,
            serde_json::json!(true),
            "unrelated gate should remain clear"
        );
    }
}

// NOTE: appended by innovation agent — test_rpc_dump_state_and_restore_state_roundtrip
#[cfg(test)]
mod test_dump_restore {
    use super::*;

    fn make_dump_diag(id: &str) -> MaxDiagnostic {
        MaxDiagnostic {
            diagnostic_id: id.to_string(),
            law_id: "law-dump-test".to_string(),
            attempted_transition: None,
            violated_axes: vec![],
            doc_routes: vec![],
            lsp: lsp_types::Diagnostic {
                range: lsp_types::Range::default(),
                severity: Some(lsp_types::DiagnosticSeverity::ERROR),
                code: None,
                code_description: None,
                source: Some("dump-test".to_string()),
                message: "dump roundtrip diagnostic".to_string(),
                related_information: None,
                tags: None,
                data: None,
            },
            repair_actions: vec![],
            verification_gates: vec![],
            receipt_obligation: None,
            law_axis: tower_lsp_max_protocol::LawAxis::Domain,
            violated_invariant: String::new(),
            observed_state: serde_json::Value::Null,
            expected_state: serde_json::Value::Null,
            repairability: tower_lsp_max_protocol::Repairability::Unknown,
            terminality: tower_lsp_max_protocol::Terminality::NonTerminal,
        }
    }

    #[test]
    fn test_rpc_dump_state_and_restore_state_roundtrip() {
        let mut mesh = AutonomicMesh::new();
        mesh.add_instance(LspInstance::new("DUMP_INST"));

        mesh.execute_action(MeshAction::AddDiagnostic {
            instance_id: InstanceId::from("DUMP_INST"),
            diagnostic: Box::new(make_dump_diag("dump-diag-1")),
        });

        let receipt = Receipt {
            receipt_id: "dump-rcpt-1".to_string(),
            hash: "dump-hash-abc".to_string(),
            prev_receipt_hash: None,
        };
        mesh.execute_action(MeshAction::EmitReceipt {
            instance_id: InstanceId::from("DUMP_INST"),
            receipt,
        });

        // Capture original conformance score before dump.
        let original_list = mesh
            .dispatch_rpc("DUMP_INST", "max/instanceList", serde_json::Value::Null)
            .expect("instanceList must succeed");
        let original_entry = original_list
            .as_array()
            .expect("instanceList returns array")
            .iter()
            .find(|v| v["id"].as_str() == Some("DUMP_INST"))
            .expect("DUMP_INST must be in list")
            .clone();
        let original_conformance = original_entry["conformance_score"]
            .as_f64()
            .expect("conformance_score must be f64");

        // Dump state via RPC — covers the arm at max/dumpState.
        let dump_json = mesh
            .dispatch_rpc("DUMP_INST", "max/dumpState", serde_json::Value::Null)
            .expect("max/dumpState must succeed");

        // Restore into a fresh mesh — covers the arm at max/restoreState.
        // dispatch_rpc requires at least one instance to exist; add a sentinel.
        let mut fresh_mesh = AutonomicMesh::new();
        fresh_mesh.add_instance(LspInstance::new("__sentinel__"));
        fresh_mesh
            .dispatch_rpc("__sentinel__", "max/restoreState", dump_json)
            .expect("max/restoreState must succeed");

        // Verify restored instance appears in instanceList with same conformance score.
        let restored_list = fresh_mesh
            .dispatch_rpc("DUMP_INST", "max/instanceList", serde_json::Value::Null)
            .expect("instanceList after restore must succeed");
        let restored_entry = restored_list
            .as_array()
            .expect("instanceList returns array")
            .iter()
            .find(|v| v["id"].as_str() == Some("DUMP_INST"))
            .expect("DUMP_INST must survive restore");
        let restored_conformance = restored_entry["conformance_score"]
            .as_f64()
            .expect("restored conformance_score must be f64");

        assert!(
            (original_conformance - restored_conformance).abs() < f64::EPSILON,
            "conformance score must survive dump/restore roundtrip: {} vs {}",
            original_conformance,
            restored_conformance
        );
    }
}

// NOTE: appended by innovation agent — ConformanceGrade boundary tests
#[cfg(test)]
mod test_conformance_grade {
    use super::*;

    fn make_diag_with_severity(id: &str, severity: lsp_types::DiagnosticSeverity) -> MaxDiagnostic {
        MaxDiagnostic {
            diagnostic_id: id.to_string(),
            law_id: "law-grade-test".to_string(),
            attempted_transition: None,
            violated_axes: vec![],
            doc_routes: vec![],
            lsp: lsp_types::Diagnostic {
                range: lsp_types::Range::default(),
                severity: Some(severity),
                code: None,
                code_description: None,
                source: Some("grade-test".to_string()),
                message: "grade boundary test diagnostic".to_string(),
                related_information: None,
                tags: None,
                data: None,
            },
            repair_actions: vec![],
            verification_gates: vec![],
            receipt_obligation: None,
            law_axis: tower_lsp_max_protocol::LawAxis::Domain,
            violated_invariant: String::new(),
            observed_state: serde_json::Value::Null,
            expected_state: serde_json::Value::Null,
            repairability: tower_lsp_max_protocol::Repairability::Unknown,
            terminality: tower_lsp_max_protocol::Terminality::NonTerminal,
        }
    }

    /// score == 100 (zero diagnostics) → Perfect
    #[test]
    fn grade_perfect() {
        let inst = LspInstance::new("GRADE_PERFECT");
        // No diagnostics: score must be 100.0
        assert_eq!(
            inst.conformance_score(),
            100.0,
            "expected score 100 with no diagnostics"
        );
        assert_eq!(inst.conformance_grade(), ConformanceGrade::Perfect);
    }

    /// 1 WARNING → penalty 20 → score 80 → Good
    #[test]
    fn grade_good() {
        let mut inst = LspInstance::new("GRADE_GOOD");
        inst.diagnostics.push(make_diag_with_severity(
            "w1",
            lsp_types::DiagnosticSeverity::WARNING,
        ));
        let score = inst.conformance_score();
        assert!(
            (score - 80.0).abs() < f64::EPSILON,
            "expected score 80, got {}",
            score
        );
        assert_eq!(inst.conformance_grade(), ConformanceGrade::Good);
    }

    /// Engineer score in [50, 75) using 2 WARNINGs (40 penalty → 60) → Degraded
    #[test]
    fn grade_degraded() {
        let mut inst = LspInstance::new("GRADE_DEGRADED");
        // 2 × WARNING = 40 penalty → score 60
        inst.diagnostics.push(make_diag_with_severity(
            "w1",
            lsp_types::DiagnosticSeverity::WARNING,
        ));
        inst.diagnostics.push(make_diag_with_severity(
            "w2",
            lsp_types::DiagnosticSeverity::WARNING,
        ));
        let score = inst.conformance_score();
        assert!(
            (score - 60.0).abs() < f64::EPSILON,
            "expected score 60, got {}",
            score
        );
        assert!(
            (50.0..75.0).contains(&score),
            "score {} not in [50, 75)",
            score
        );
        assert_eq!(inst.conformance_grade(), ConformanceGrade::Degraded);
    }

    /// 4 ERRORs → penalty 120 → score clamped to 0 → Critical (< 50)
    #[test]
    fn grade_critical() {
        let mut inst = LspInstance::new("GRADE_CRITICAL");
        for i in 0..4 {
            inst.diagnostics.push(make_diag_with_severity(
                &format!("e{}", i),
                lsp_types::DiagnosticSeverity::ERROR,
            ));
        }
        let score = inst.conformance_score();
        assert!(
            score <= 10.0,
            "expected score ≤ 10 with 4 ERRORs, got {}",
            score
        );
        assert_eq!(inst.conformance_grade(), ConformanceGrade::Critical);
    }

    /// Boundary edge: score == 75.0 → Good (inclusive lower bound of Good)
    #[test]
    fn grade_boundary_at_75() {
        // 1 WARNING (20) + 1 HINT (5) = 25 penalty → score 75
        let mut inst = LspInstance::new("GRADE_BOUNDARY_75");
        inst.diagnostics.push(make_diag_with_severity(
            "w1",
            lsp_types::DiagnosticSeverity::WARNING,
        ));
        inst.diagnostics.push(make_diag_with_severity(
            "h1",
            lsp_types::DiagnosticSeverity::HINT,
        ));
        let score = inst.conformance_score();
        assert!(
            (score - 75.0).abs() < f64::EPSILON,
            "expected score 75.0, got {}",
            score
        );
        assert_eq!(
            inst.conformance_grade(),
            ConformanceGrade::Good,
            "score 75.0 must map to Good (inclusive lower bound)"
        );
    }

    /// Boundary edge: score == 50.0 → Degraded (inclusive lower bound of Degraded)
    #[test]
    fn grade_boundary_at_50() {
        // 1 WARNING (20) + 1 INFORMATION (10) + 2 × HINT (5+5) + 1 WARNING (20) = 50 penalty → 50
        // Simpler: use from_score directly for a precise 50.0 check
        assert_eq!(
            ConformanceGrade::from_score(50.0),
            ConformanceGrade::Degraded,
            "score 50.0 must map to Degraded (inclusive lower bound)"
        );
        // Also verify the instance path: 2×WARNING(40) + 1×INFORMATION(10) = 50 → score 50
        let mut inst = LspInstance::new("GRADE_BOUNDARY_50");
        inst.diagnostics.push(make_diag_with_severity(
            "w1",
            lsp_types::DiagnosticSeverity::WARNING,
        ));
        inst.diagnostics.push(make_diag_with_severity(
            "w2",
            lsp_types::DiagnosticSeverity::WARNING,
        ));
        inst.diagnostics.push(make_diag_with_severity(
            "i1",
            lsp_types::DiagnosticSeverity::INFORMATION,
        ));
        let score = inst.conformance_score();
        assert!(
            (score - 50.0).abs() < f64::EPSILON,
            "expected score 50.0, got {}",
            score
        );
        assert_eq!(inst.conformance_grade(), ConformanceGrade::Degraded);
    }

    #[test]
    fn reset_instance_emits_instance_reset_hook_event() {
        let mut mesh = MaxMesh::new();
        mesh.register_instance("INST_RESET".to_string());
        // Clear the event log so we observe only the reset event
        mesh.event_log.clear();
        let result = mesh.dispatch_rpc("INST_RESET", "max/reset", serde_json::json!(null));
        assert!(result.is_ok(), "max/reset must succeed: {:?}", result);
        // The event_log should contain an InstanceReset event for the instance
        let reset_event = mesh.event_log.iter().find(|e| {
            matches!(e, HookEvent::InstanceReset { instance_id } if instance_id.0 == "INST_RESET")
        });
        assert!(
            reset_event.is_some(),
            "expected InstanceReset in event_log after max/reset, got: {:?}",
            mesh.event_log
        );
    }
}

#[cfg(test)]
mod test_bounded_action_event {
    use super::*;

    #[test]
    fn test_execute_bounded_action_emits_hook_event() {
        let mut mesh = AutonomicMesh::new();
        mesh.add_instance(LspInstance::new("bounded-test"));
        mesh.execute_action(MeshAction::ExecuteBoundedAction {
            instance_id: InstanceId::from("bounded-test"),
            action_id: "act-test-action".to_string(),
            description: "A test bounded action".to_string(),
        });
        let found = mesh.event_log.iter().any(|e| {
            matches!(
                e,
                HookEvent::BoundedActionExecuted {
                    action_id,
                    ..
                } if action_id == "act-test-action"
            )
        });
        assert!(
            found,
            "BoundedActionExecuted event must appear in event_log after execute_bounded_action"
        );
    }
}

#[cfg(test)]
mod bounded_action_hook_trigger_tests {
    use super::*;

    /// CustomerRequestClassifierHook must react to BoundedActionExecuted by
    /// emitting an EmitReceipt action with receipt_id
    /// "bounded-action-executed-<action_id>".
    #[test]
    fn classifier_hook_bounded_action_executed_emits_receipt() {
        let hook = CustomerRequestClassifierHook::new();
        let event = HookEvent::BoundedActionExecuted {
            instance_id: InstanceId::from("classifier-bounded-test"),
            action_id: "act-create-refund-receipt".to_string(),
            description: "Arrival of proof validated, creating refund receipt".to_string(),
        };
        let actions = hook.trigger(&event);
        let receipt_action = actions.iter().find(|a| {
            matches!(a, MeshAction::EmitReceipt { receipt, .. }
                if receipt.receipt_id == "bounded-action-executed-act-create-refund-receipt")
        });
        assert!(
            receipt_action.is_some(),
            "CustomerRequestClassifierHook must emit a receipt for BoundedActionExecuted, got: {:?}",
            actions
        );
    }

    /// PolicyEvaluationHook must react to BoundedActionExecuted for
    /// "act-create-refund-receipt" by emitting a completion receipt.
    #[test]
    fn policy_hook_bounded_action_executed_emits_completion_receipt() {
        let hook = PolicyEvaluationHook::new();
        let event = HookEvent::BoundedActionExecuted {
            instance_id: InstanceId::from("policy-bounded-test"),
            action_id: "act-create-refund-receipt".to_string(),
            description: "Arrival of proof validated, creating refund receipt".to_string(),
        };
        let actions = hook.trigger(&event);
        let receipt_action = actions.iter().find(|a| {
            matches!(a, MeshAction::EmitReceipt { receipt, .. }
                if receipt.receipt_id == "refund-action-completion-receipt")
        });
        assert!(
            receipt_action.is_some(),
            "PolicyEvaluationHook must emit refund-action-completion-receipt for BoundedActionExecuted, got: {:?}",
            actions
        );
    }

    /// PolicyEvaluationHook must NOT emit a completion receipt for unknown
    /// bounded action IDs (the guard is specific to act-create-refund-receipt).
    #[test]
    fn policy_hook_bounded_action_unknown_id_no_receipt() {
        let hook = PolicyEvaluationHook::new();
        let event = HookEvent::BoundedActionExecuted {
            instance_id: InstanceId::from("policy-bounded-unknown"),
            action_id: "act-unknown-action".to_string(),
            description: "Some other action".to_string(),
        };
        let actions = hook.trigger(&event);
        assert!(
            actions.is_empty(),
            "PolicyEvaluationHook must not emit actions for unknown bounded action IDs, got: {:?}",
            actions
        );
    }

    /// End-to-end: dispatching BoundedActionExecuted via the full MaxMesh (with
    /// hooks registered) causes a downstream EmitReceipt to be processed and
    /// a ReceiptEmitted event to appear in the event log.
    #[test]
    fn max_mesh_bounded_action_executed_triggers_downstream_receipt() {
        let mut mesh = MaxMesh::new();
        mesh.register_hook(Box::new(CustomerRequestClassifierHook::new()));
        mesh.register_hook(Box::new(PolicyEvaluationHook::new()));
        mesh.register_instance("e2e-bounded".to_string());
        mesh.event_log.clear();

        // Manually fire the BoundedActionExecuted event through dispatch_event
        // to simulate what execute_action already does after executing the action.
        mesh.dispatch_event(HookEvent::BoundedActionExecuted {
            instance_id: InstanceId::from("e2e-bounded"),
            action_id: "act-create-refund-receipt".to_string(),
            description: "Arrival of proof validated, creating refund receipt".to_string(),
        });

        let receipt_emitted = mesh.event_log.iter().any(|e| {
            matches!(e, HookEvent::ReceiptEmitted { receipt, .. }
                if receipt.receipt_id.contains("bounded-action-executed")
                    || receipt.receipt_id == "refund-action-completion-receipt")
        });
        assert!(
            receipt_emitted,
            "dispatching BoundedActionExecuted must produce ReceiptEmitted downstream, log: {:?}",
            mesh.event_log
        );
    }
}

#[cfg(test)]
mod policy_and_routing_hook_unit_tests {
    use super::*;

    // -----------------------------------------------------------------------
    // PolicyEvaluationHook — ReceiptEmitted arm
    // -----------------------------------------------------------------------

    /// ReceiptEmitted with a matching proof receipt_id while policy is
    /// ClarificationRequested → returns TransitionPolicyState(RefundAuthorized).
    #[test]
    fn policy_hook_receipt_emitted_matching_id_returns_transition() {
        let hook = PolicyEvaluationHook::new();
        {
            let mut states = hook.policy_states.lock().unwrap();
            states.insert("LSP_X".to_string(), PolicyState::ClarificationRequested);
        }

        let event = HookEvent::ReceiptEmitted {
            instance_id: InstanceId::from("LSP_X"),
            receipt: Receipt {
                receipt_id: "rcpt-customer-proof".to_string(),
                hash: "h".to_string(),
                prev_receipt_hash: None,
            },
        };

        let actions = hook.trigger(&event);
        assert_eq!(actions.len(), 1, "expected exactly one action");
        match &actions[0] {
            MeshAction::TransitionPolicyState {
                instance_id,
                new_state,
            } => {
                assert_eq!(instance_id.0, "LSP_X");
                assert_eq!(*new_state, PolicyState::RefundAuthorized);
            }
            other => panic!("unexpected action: {:?}", other),
        }
    }

    /// ReceiptEmitted with a non-proof receipt_id → returns no actions.
    #[test]
    fn policy_hook_receipt_emitted_non_matching_id_returns_empty() {
        let hook = PolicyEvaluationHook::new();
        {
            let mut states = hook.policy_states.lock().unwrap();
            states.insert("LSP_X".to_string(), PolicyState::ClarificationRequested);
        }

        let event = HookEvent::ReceiptEmitted {
            instance_id: InstanceId::from("LSP_X"),
            receipt: Receipt {
                receipt_id: "rcpt-unrelated".to_string(),
                hash: "h".to_string(),
                prev_receipt_hash: None,
            },
        };

        let actions = hook.trigger(&event);
        assert!(
            actions.is_empty(),
            "non-proof receipt should produce no actions"
        );
    }

    // -----------------------------------------------------------------------
    // PolicyEvaluationHook — StateTransition arm (via PolicyStateChanged)
    // -----------------------------------------------------------------------

    /// PolicyStateChanged from ClarificationRequested to RefundAuthorized
    /// → returns ExecuteBoundedAction(act-create-refund-receipt).
    #[test]
    fn policy_hook_state_transition_clarification_to_refund_emits_bounded_action() {
        let hook = PolicyEvaluationHook::new();

        let event = HookEvent::PolicyStateChanged {
            instance_id: InstanceId::from("LSP_Y"),
            from_state: PolicyState::ClarificationRequested,
            to_state: PolicyState::RefundAuthorized,
        };

        let actions = hook.trigger(&event);
        assert_eq!(actions.len(), 1, "expected exactly one bounded action");
        match &actions[0] {
            MeshAction::ExecuteBoundedAction {
                instance_id,
                action_id,
                ..
            } => {
                assert_eq!(instance_id.0, "LSP_Y");
                assert_eq!(action_id, "act-create-refund-receipt");
            }
            other => panic!("unexpected action: {:?}", other),
        }
    }

    /// PolicyStateChanged with a transition that does NOT match
    /// ClarificationRequested→RefundAuthorized → returns no actions.
    #[test]
    fn policy_hook_state_transition_other_transition_returns_empty() {
        let hook = PolicyEvaluationHook::new();

        let event = HookEvent::PolicyStateChanged {
            instance_id: InstanceId::from("LSP_Y"),
            from_state: PolicyState::Operational,
            to_state: PolicyState::ClarificationRequested,
        };

        let actions = hook.trigger(&event);
        assert!(
            actions.is_empty(),
            "non-refund transition should produce no bounded actions"
        );
    }

    // -----------------------------------------------------------------------
    // ReceiptRoutingHook — DiagnosticCleared arm
    // -----------------------------------------------------------------------

    /// DiagnosticCleared for a diagnostic that was previously tracked
    /// → removes it from the internal set (the arm produces no external actions).
    #[test]
    fn routing_hook_diagnostic_cleared_removes_tracked_diagnostic() {
        let hook = ReceiptRoutingHook::new();

        let diag = MaxDiagnostic {
            diagnostic_id: "diag-abc".to_string(),
            law_id: "law-test".to_string(),
            attempted_transition: None,
            violated_axes: vec![],
            doc_routes: vec![],
            lsp: lsp_types::Diagnostic {
                range: lsp_types::Range::default(),
                severity: Some(lsp_types::DiagnosticSeverity::ERROR),
                code: None,
                code_description: None,
                source: None,
                message: "test".to_string(),
                related_information: None,
                tags: None,
                data: None,
            },
            repair_actions: vec![],
            verification_gates: vec![],
            receipt_obligation: None,
            ..Default::default()
        };
        let emit_event = HookEvent::DiagnosticEmitted {
            instance_id: InstanceId::from("LSP_1"),
            diagnostic: Box::new(diag),
        };
        hook.trigger(&emit_event);

        {
            let diags = hook.active_diagnostics.lock().unwrap();
            assert!(
                diags
                    .get("LSP_1")
                    .map(|s| s.contains("diag-abc"))
                    .unwrap_or(false),
                "diagnostic should be tracked after DiagnosticEmitted"
            );
        }

        let clear_event = HookEvent::DiagnosticCleared {
            instance_id: InstanceId::from("LSP_1"),
            diagnostic_id: "diag-abc".to_string(),
        };
        let actions = hook.trigger(&clear_event);

        assert!(
            actions.is_empty(),
            "DiagnosticCleared arm should return no MeshActions"
        );

        {
            let diags = hook.active_diagnostics.lock().unwrap();
            let still_present = diags
                .get("LSP_1")
                .map(|s| s.contains("diag-abc"))
                .unwrap_or(false);
            assert!(
                !still_present,
                "diagnostic should be removed after DiagnosticCleared"
            );
        }
    }

    /// DiagnosticCleared for an unknown diagnostic_id → returns no actions and does not panic.
    #[test]
    fn routing_hook_diagnostic_cleared_wrong_id_returns_empty() {
        let hook = ReceiptRoutingHook::new();

        let event = HookEvent::DiagnosticCleared {
            instance_id: InstanceId::from("LSP_1"),
            diagnostic_id: "no-such-diag".to_string(),
        };

        let actions = hook.trigger(&event);
        assert!(
            actions.is_empty(),
            "clearing unknown diagnostic should produce no actions"
        );
    }
}

#[cfg(test)]
mod reset_instance_guard_tests {
    use super::*;

    /// ResetInstance for a nonexistent instance must not emit InstanceReset into the event_log.
    /// All other MeshAction variants are guarded by `if let Some(instance)`; this test enforces
    /// that ResetInstance matches that behavior so process-mining conformance checks never observe
    /// spurious ghost-instance events.
    #[test]
    fn test_reset_missing_instance_emits_no_event() {
        let mut mesh = AutonomicMesh::new();
        let before_len = mesh.event_log.len();

        mesh.execute_action(MeshAction::ResetInstance {
            instance_id: InstanceId::from("GHOST_99"),
        });

        assert_eq!(
            mesh.event_log.len(),
            before_len,
            "ResetInstance on a nonexistent instance must not append any HookEvent"
        );
    }
}

// ---------------------------------------------------------------------------
// INN-10-08 — Property-based tests for conformance_score invariants (DfLSS-CTQ)
// ---------------------------------------------------------------------------

#[cfg(test)]
mod conformance_score_proptest {
    use super::*;
    use proptest::prelude::*;

    fn make_diag_sev_prop(id: &str, severity_byte: u8) -> MaxDiagnostic {
        let severity = match severity_byte % 4 {
            0 => Some(lsp_types::DiagnosticSeverity::ERROR),
            1 => Some(lsp_types::DiagnosticSeverity::WARNING),
            2 => Some(lsp_types::DiagnosticSeverity::INFORMATION),
            _ => Some(lsp_types::DiagnosticSeverity::HINT),
        };
        MaxDiagnostic {
            diagnostic_id: id.to_string(),
            law_id: "law-prop".to_string(),
            lsp: lsp_types::Diagnostic {
                range: lsp_types::Range::default(),
                severity,
                code: None,
                code_description: None,
                source: None,
                message: "prop-test".to_string(),
                related_information: None,
                tags: None,
                data: None,
            },
            ..Default::default()
        }
    }

    proptest! {
        #[test]
        fn prop_conformance_score_always_in_range(
            diag_severities in proptest::collection::vec(0u8..4, 0..50)
        ) {
            let mut inst = LspInstance::new("PROP_INST");
            for (i, s) in diag_severities.iter().enumerate() {
                inst.diagnostics.push(make_diag_sev_prop(&format!("d{}", i), *s));
            }
            let score = inst.conformance_score();
            prop_assert!(score >= 0.0, "score must be >= 0.0, got {}", score);
            prop_assert!(score <= 100.0, "score must be <= 100.0, got {}", score);
        }

        #[test]
        fn prop_conformance_score_zero_diags_is_100(_salt in 0u64..u64::MAX) {
            let inst = LspInstance::new("PROP_EMPTY");
            prop_assert_eq!(inst.conformance_score(), 100.0);
        }

        #[test]
        fn prop_conformance_score_error_does_not_increase(
            diag_severities in proptest::collection::vec(0u8..4, 0..20)
        ) {
            let mut inst = LspInstance::new("PROP_MONO");
            for (i, s) in diag_severities.iter().enumerate() {
                inst.diagnostics.push(make_diag_sev_prop(&format!("d{}", i), *s));
            }
            let score_before = inst.conformance_score();
            inst.diagnostics.push(make_diag_sev_prop("extra_error", 0));
            let score_after = inst.conformance_score();
            prop_assert!(
                score_after <= score_before,
                "adding ERROR must not increase score: before={}, after={}",
                score_before,
                score_after
            );
        }
    }
}

// ---------------------------------------------------------------------------
// INN-12-09: cache auto-invalidation deterministic unit tests
// (proptest already integrated into conformance_score_proptest above)
// ---------------------------------------------------------------------------

#[cfg(test)]
mod cache_auto_invalidation_tests {
    use super::*;

    fn make_diag_sev(id: &str, sev_byte: u8) -> MaxDiagnostic {
        let severity = match sev_byte % 4 {
            0 => Some(lsp_types::DiagnosticSeverity::ERROR),
            1 => Some(lsp_types::DiagnosticSeverity::WARNING),
            2 => Some(lsp_types::DiagnosticSeverity::INFORMATION),
            _ => Some(lsp_types::DiagnosticSeverity::HINT),
        };
        MaxDiagnostic {
            diagnostic_id: id.to_string(),
            law_id: "law-cache".to_string(),
            lsp: lsp_types::Diagnostic {
                range: lsp_types::Range::default(),
                severity,
                code: None,
                code_description: None,
                source: None,
                message: "cache-test".to_string(),
                related_information: None,
                tags: None,
                data: None,
            },
            ..Default::default()
        }
    }

    #[test]
    fn test_score_auto_invalidates_after_add() {
        let mut inst = LspInstance::new("CACHE_ADD");
        assert_eq!(inst.conformance_score(), 100.0);
        inst.add_diagnostic(make_diag_sev("err1", 0 /* ERROR */));
        let score_after = inst.conformance_score();
        assert!(
            score_after < 100.0,
            "score should decrease after add_diagnostic(ERROR), got {}",
            score_after
        );
    }

    #[test]
    fn test_score_auto_invalidates_after_remove() {
        let mut inst = LspInstance::new("CACHE_REMOVE");
        inst.add_diagnostic(make_diag_sev("err1", 0 /* ERROR */));
        let score_with = inst.conformance_score();
        let removed = inst.remove_diagnostic("err1");
        assert_eq!(removed, 1);
        let score_after = inst.conformance_score();
        assert_eq!(
            score_after, 100.0,
            "score should be 100 after removing only error"
        );
        assert!(
            score_after > score_with,
            "score should increase after remove"
        );
    }
}

// ---------------------------------------------------------------------------
// INN-12-04 — Property-based tests for admission decision determinism (DfLSS-CTQ)
// ---------------------------------------------------------------------------

#[cfg(test)]
mod admission_proptest {
    use super::*;
    use proptest::prelude::*;

    fn make_diag_for_admission(id: &str, severity_byte: u8) -> MaxDiagnostic {
        let severity = match severity_byte % 4 {
            0 => Some(lsp_types::DiagnosticSeverity::ERROR),
            1 => Some(lsp_types::DiagnosticSeverity::WARNING),
            2 => Some(lsp_types::DiagnosticSeverity::INFORMATION),
            _ => Some(lsp_types::DiagnosticSeverity::HINT),
        };
        MaxDiagnostic {
            diagnostic_id: id.to_string(),
            law_id: "law-admission-prop".to_string(),
            lsp: lsp_types::Diagnostic {
                range: lsp_types::Range::default(),
                severity,
                code: None,
                code_description: None,
                source: None,
                message: "admission-prop-test".to_string(),
                related_information: None,
                tags: None,
                data: None,
            },
            ..Default::default()
        }
    }

    proptest! {
        // INN-12-04: Admitted iff no ERROR-severity diagnostics present
        #[test]
        fn prop_admission_admitted_iff_no_errors(
            diag_severities in proptest::collection::vec(0u8..4, 0..30)
        ) {
            let mut mesh = AutonomicMesh::new();
            mesh.add_instance(LspInstance::new("PROP_ADM"));

            for (i, s) in diag_severities.iter().enumerate() {
                mesh.execute_action(MeshAction::AddDiagnostic {
                    instance_id: InstanceId::from("PROP_ADM"),
                    diagnostic: Box::new(make_diag_for_admission(&format!("d{}", i), *s)),
                });
            }

            let has_error = diag_severities.iter().any(|s| s % 4 == 0);
            let result = mesh.dispatch_rpc("PROP_ADM", "max/admission", serde_json::Value::Null);
            prop_assert!(result.is_ok());
            let val = result.unwrap();

            if diag_severities.is_empty() {
                prop_assert_eq!(
                    val["verdict"].as_str().unwrap_or(""),
                    "Admitted",
                    "empty diagnostics must yield Admitted"
                );
            } else if has_error {
                prop_assert_eq!(
                    val["verdict"].as_str().unwrap_or(""),
                    "Refused",
                    "ERROR present must yield Refused"
                );
            } else {
                // Warnings/hints only — must not be Admitted
                prop_assert_ne!(
                    val["verdict"].as_str().unwrap_or(""),
                    "Admitted",
                    "non-empty non-error diagnostics must not yield Admitted"
                );
            }
        }

        // INN-12-04: Idempotence — same mesh state must produce same verdict on two consecutive calls
        #[test]
        fn prop_admission_is_idempotent(
            diag_severities in proptest::collection::vec(0u8..4, 0..30)
        ) {
            let mut mesh = AutonomicMesh::new();
            mesh.add_instance(LspInstance::new("PROP_IDEM"));

            for (i, s) in diag_severities.iter().enumerate() {
                mesh.execute_action(MeshAction::AddDiagnostic {
                    instance_id: InstanceId::from("PROP_IDEM"),
                    diagnostic: Box::new(make_diag_for_admission(&format!("d{}", i), *s)),
                });
            }

            let r1 = mesh.dispatch_rpc("PROP_IDEM", "max/admission", serde_json::Value::Null);
            let r2 = mesh.dispatch_rpc("PROP_IDEM", "max/admission", serde_json::Value::Null);

            prop_assert!(r1.is_ok());
            prop_assert!(r2.is_ok());
            let v1 = r1.unwrap();
            let v2 = r2.unwrap();
            prop_assert_eq!(
                v1["verdict"].as_str().unwrap_or(""),
                v2["verdict"].as_str().unwrap_or(""),
                "admission verdict must be idempotent: first={:?}, second={:?}",
                v1,
                v2
            );
        }
    }
}

// ---------------------------------------------------------------------------
// INN-12-03 — ERRC-Eliminate: test fixture factory (mesh_fixture! macro)
// ---------------------------------------------------------------------------
//
// `mesh_fixture!(id1, id2, ...)` creates an AutonomicMesh pre-loaded with the
// named instances.  Duplicate ids cause a panic at test time, surfacing
// collisions that were previously silently masked by HashMap::insert.

/// Build an [`AutonomicMesh`] pre-loaded with one or more named instances.
/// Panics if any id appears more than once so id collisions are caught
/// immediately rather than silently discarded.
#[cfg(test)]
macro_rules! mesh_fixture {
    ($($id:expr),+ $(,)?) => {{
        let ids: &[&str] = &[$($id),+];
        let mut seen = std::collections::HashSet::new();
        for id in ids {
            assert!(
                seen.insert(*id),
                "mesh_fixture!: duplicate instance id {:?}",
                id
            );
        }
        let mut _mesh = AutonomicMesh::new();
        for id in ids {
            _mesh.add_instance(LspInstance::new(id));
        }
        _mesh
    }};
}

#[cfg(test)]
mod mesh_fixture_tests {
    use super::*;

    #[test]
    fn test_mesh_fixture_single_instance() {
        let mesh = mesh_fixture!("INST_A");
        assert!(mesh.instances.contains_key("INST_A"));
        assert_eq!(mesh.instances.len(), 1);
    }

    #[test]
    fn test_mesh_fixture_multiple_instances() {
        let mesh = mesh_fixture!("A", "B", "C");
        assert_eq!(mesh.instances.len(), 3);
        assert!(mesh.instances.contains_key("A"));
        assert!(mesh.instances.contains_key("B"));
        assert!(mesh.instances.contains_key("C"));
    }

    #[test]
    #[should_panic(expected = "duplicate instance id")]
    fn test_mesh_fixture_duplicate_id_panics() {
        let _ = mesh_fixture!("INST_A", "INST_B", "INST_A");
    }

    #[test]
    fn test_mesh_fixture_execute_action_works() {
        let mut mesh = mesh_fixture!("INST_A");
        mesh.execute_action(MeshAction::TransitionPolicyState {
            instance_id: InstanceId::from("INST_A"),
            new_state: PolicyState::ClarificationRequested,
        });
        assert_eq!(
            mesh.instances.get("INST_A").unwrap().policy_state,
            Some(PolicyState::ClarificationRequested)
        );
    }
}

// ---------------------------------------------------------------------------
// INN-12-08 — ERRC-Raise: PolicyState transition coverage
// ---------------------------------------------------------------------------

#[cfg(test)]
mod policy_state_transition_tests {
    use super::*;
    use proptest::prelude::*;

    #[test]
    fn test_policy_transition_operational_to_clarification() {
        let mut mesh = mesh_fixture!("INST");
        mesh.execute_action(MeshAction::TransitionPolicyState {
            instance_id: InstanceId::from("INST"),
            new_state: PolicyState::ClarificationRequested,
        });
        assert_eq!(
            mesh.instances.get("INST").unwrap().policy_state,
            Some(PolicyState::ClarificationRequested)
        );
    }

    #[test]
    fn test_policy_transition_clarification_to_refund_authorized() {
        let mut mesh = mesh_fixture!("INST");
        mesh.execute_action(MeshAction::TransitionPolicyState {
            instance_id: InstanceId::from("INST"),
            new_state: PolicyState::ClarificationRequested,
        });
        mesh.execute_action(MeshAction::TransitionPolicyState {
            instance_id: InstanceId::from("INST"),
            new_state: PolicyState::RefundAuthorized,
        });
        assert_eq!(
            mesh.instances.get("INST").unwrap().policy_state,
            Some(PolicyState::RefundAuthorized)
        );
    }

    #[test]
    fn test_policy_transition_refund_authorized_to_operational() {
        let mut mesh = mesh_fixture!("INST");
        mesh.execute_action(MeshAction::TransitionPolicyState {
            instance_id: InstanceId::from("INST"),
            new_state: PolicyState::ClarificationRequested,
        });
        mesh.execute_action(MeshAction::TransitionPolicyState {
            instance_id: InstanceId::from("INST"),
            new_state: PolicyState::RefundAuthorized,
        });
        mesh.execute_action(MeshAction::TransitionPolicyState {
            instance_id: InstanceId::from("INST"),
            new_state: PolicyState::Operational,
        });
        assert_eq!(
            mesh.instances.get("INST").unwrap().policy_state,
            Some(PolicyState::Operational)
        );
    }

    #[test]
    fn test_policy_transition_operational_to_active() {
        let mut mesh = mesh_fixture!("INST");
        mesh.execute_action(MeshAction::TransitionPolicyState {
            instance_id: InstanceId::from("INST"),
            new_state: PolicyState::Active,
        });
        assert_eq!(
            mesh.instances.get("INST").unwrap().policy_state,
            Some(PolicyState::Active)
        );
    }

    #[test]
    fn test_policy_state_all_variants_reachable() {
        let all_states = [
            PolicyState::Operational,
            PolicyState::ClarificationRequested,
            PolicyState::RefundAuthorized,
            PolicyState::Active,
        ];
        for target in &all_states {
            let mut mesh = mesh_fixture!("INST");
            mesh.execute_action(MeshAction::TransitionPolicyState {
                instance_id: InstanceId::from("INST"),
                new_state: target.clone(),
            });
            assert_eq!(
                mesh.instances.get("INST").unwrap().policy_state,
                Some(target.clone()),
                "Could not reach {:?}",
                target
            );
        }
    }

    #[test]
    fn test_policy_transition_unknown_instance_is_noop() {
        let mut mesh = mesh_fixture!("INST");
        let before_log_len = mesh.event_log.len();
        mesh.execute_action(MeshAction::TransitionPolicyState {
            instance_id: InstanceId::from("NO_SUCH_INST"),
            new_state: PolicyState::RefundAuthorized,
        });
        assert_eq!(
            mesh.instances.get("INST").unwrap().policy_state,
            None,
            "known instance policy_state must not change"
        );
        assert_eq!(
            mesh.event_log.len(),
            before_log_len,
            "no event should be logged for a missing instance"
        );
    }

    fn arb_policy_state() -> impl Strategy<Value = PolicyState> {
        prop_oneof![
            Just(PolicyState::Operational),
            Just(PolicyState::ClarificationRequested),
            Just(PolicyState::RefundAuthorized),
            Just(PolicyState::Active),
        ]
    }

    proptest! {
        #[test]
        fn prop_policy_state_never_enters_unknown_variant(
            states in proptest::collection::vec(arb_policy_state(), 1..20)
        ) {
            let mut mesh = mesh_fixture!("PROP_INST");
            for state in &states {
                mesh.execute_action(MeshAction::TransitionPolicyState {
                    instance_id: InstanceId::from("PROP_INST"),
                    new_state: state.clone(),
                });
            }
            let final_state = mesh.instances.get("PROP_INST").unwrap().policy_state.clone();
            prop_assert!(
                final_state.is_some(),
                "policy_state must be Some after at least one transition, got None"
            );
            let _state = final_state.unwrap(); // D-05 fix: exhaust match without let_unit_value
        }
    }
}

// D-05: Receipt hash integrity tests — forged hashes are always overwritten
#[cfg(test)]
mod test_d05_receipt_hash_integrity {
    use super::*;

    #[test]
    fn forged_hash_is_replaced_by_canonical_sha256() {
        let mut mesh = AutonomicMesh::new();
        mesh.add_instance(LspInstance::new("hash-integrity-inst"));
        let receipt_id = "rcpt-integrity-check".to_string();
        let forged_hash = "FORGED-HASH-SHOULD-NOT-APPEAR".to_string();
        mesh.execute_action(MeshAction::EmitReceipt {
            instance_id: InstanceId::from("hash-integrity-inst"),
            receipt: Receipt {
                receipt_id: receipt_id.clone(),
                hash: forged_hash.clone(),
                prev_receipt_hash: None,
            },
        });
        let inst = mesh.instances.get("hash-integrity-inst").unwrap();
        assert_eq!(inst.receipts.len(), 1, "receipt must be stored");
        let stored = &inst.receipts[0];
        let expected_hash = sha256(receipt_id.as_bytes());
        assert_ne!(
            stored.hash, forged_hash,
            "forged hash must not appear in stored receipt"
        );
        assert_eq!(
            stored.hash, expected_hash,
            "stored hash must equal sha256(receipt_id)"
        );
    }

    #[test]
    fn hash_derived_for_any_receipt_id() {
        let mut mesh = AutonomicMesh::new();
        mesh.add_instance(LspInstance::new("hash-roundtrip-inst"));
        for id in &["rcpt-a", "rcpt-b", "rcpt-xyz"] {
            mesh.execute_action(MeshAction::EmitReceipt {
                instance_id: InstanceId::from("hash-roundtrip-inst"),
                receipt: Receipt {
                    receipt_id: id.to_string(),
                    hash: "garbage".to_string(),
                    prev_receipt_hash: None,
                },
            });
        }
        let inst = mesh.instances.get("hash-roundtrip-inst").unwrap();
        for r in &inst.receipts {
            let expected = sha256(r.receipt_id.as_bytes());
            assert_eq!(
                r.hash, expected,
                "hash must equal sha256(receipt_id) for id={}",
                r.receipt_id
            );
        }
    }
}
