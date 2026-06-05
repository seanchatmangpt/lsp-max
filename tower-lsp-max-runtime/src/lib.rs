use std::fmt::{Debug, Formatter};
use std::marker::PhantomData;

fn sha256(data: &[u8]) -> String {
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

fn validate_and_reconstruct_chain(
    history: &[tower_lsp_max_protocol::Receipt],
) -> (serde_json::Value, serde_json::Value) {
    assert!(!history.is_empty(), "History must not be empty");

    // Validate step 0
    let r0 = &history[0];
    assert_eq!(r0.receipt_id, "rcpt-uninitialized");
    let mut expected_hash = sha256(r0.receipt_id.as_bytes());
    assert_eq!(r0.hash, expected_hash, "Hash mismatch at index 0");

    let mut client_caps = serde_json::Value::Null;
    let mut server_caps = serde_json::Value::Null;

    if history.len() > 1 {
        let r1 = &history[1];
        assert!(
            r1.receipt_id
                .starts_with("rcpt-uninitialized-to-initializing:"),
            "Invalid receipt ID at index 1: {}",
            r1.receipt_id
        );
        let prefix_len = "rcpt-uninitialized-to-initializing:".len();
        let json_str = &r1.receipt_id[prefix_len..];
        client_caps = serde_json::from_str(json_str).expect("Failed to parse client capabilities");

        expected_hash = sha256(format!("{}:{}", expected_hash, r1.receipt_id).as_bytes());
        assert_eq!(r1.hash, expected_hash, "Hash mismatch at index 1");
    }

    if history.len() > 2 {
        let r2 = &history[2];
        assert!(
            r2.receipt_id
                .starts_with("rcpt-initializing-to-initialized:"),
            "Invalid receipt ID at index 2: {}",
            r2.receipt_id
        );
        let prefix_len = "rcpt-initializing-to-initialized:".len();
        let json_str = &r2.receipt_id[prefix_len..];
        server_caps = serde_json::from_str(json_str).expect("Failed to parse server capabilities");

        expected_hash = sha256(format!("{}:{}", expected_hash, r2.receipt_id).as_bytes());
        assert_eq!(r2.hash, expected_hash, "Hash mismatch at index 2");
    }

    if history.len() > 3 {
        let r3 = &history[3];
        assert_eq!(r3.receipt_id, "rcpt-initialized-to-shutdown");
        expected_hash = sha256(format!("{}:{}", expected_hash, r3.receipt_id).as_bytes());
        assert_eq!(r3.hash, expected_hash, "Hash mismatch at index 3");
    }

    if history.len() > 4 {
        let r4 = &history[4];
        assert_eq!(r4.receipt_id, "rcpt-shutdown-to-exited");
        expected_hash = sha256(format!("{}:{}", expected_hash, r4.receipt_id).as_bytes());
        assert_eq!(r4.hash, expected_hash, "Hash mismatch at index 4");
    }

    assert!(
        history.len() <= 5,
        "History contains unexpected items beyond the Exited state"
    );

    (client_caps, server_caps)
}

pub struct DeterministicSnapshot {
    pub id: tower_lsp_max_protocol::SnapshotId,
    pub timestamp: u64,
}

impl DeterministicSnapshot {
    pub fn new() -> Self {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
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
    fn replay(history: Vec<Self::Receipt>) -> Self;
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
        tower_lsp_max_protocol::Receipt { receipt_id, hash }
    }

    fn exit(self) -> EmptyData {
        self.data
    }

    fn replay(history: Vec<Self::Receipt>) -> Self {
        validate_and_reconstruct_chain(&history);
        Machine::new(Uninitialized, EmptyData::default())
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
        let client_caps_json = serde_json::to_string(&self.data.client_capabilities).unwrap();
        let receipt_id = format!("rcpt-uninitialized-to-initializing:{}", client_caps_json);
        let hash_uninit = sha256(b"rcpt-uninitialized");
        let hash = sha256(format!("{}:{}", hash_uninit, receipt_id).as_bytes());
        tower_lsp_max_protocol::Receipt { receipt_id, hash }
    }

    fn exit(self) -> InitializingData {
        self.data
    }

    fn replay(history: Vec<Self::Receipt>) -> Self {
        assert!(
            history.len() >= 2,
            "Insufficient history for Initializing state"
        );
        let (client_caps, _) = validate_and_reconstruct_chain(&history);
        Machine::new(
            Initializing,
            InitializingData {
                client_capabilities: client_caps,
            },
        )
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
        let client_caps_json = serde_json::to_string(&self.data.client_capabilities).unwrap();
        let server_caps_json = serde_json::to_string(&self.data.server_capabilities).unwrap();
        let hash_0 = sha256(b"rcpt-uninitialized");
        let rcpt_1 = format!("rcpt-uninitialized-to-initializing:{}", client_caps_json);
        let hash_1 = sha256(format!("{}:{}", hash_0, rcpt_1).as_bytes());
        let receipt_id = format!("rcpt-initializing-to-initialized:{}", server_caps_json);
        let hash = sha256(format!("{}:{}", hash_1, receipt_id).as_bytes());
        tower_lsp_max_protocol::Receipt { receipt_id, hash }
    }

    fn exit(self) -> InitializedData {
        self.data
    }

    fn replay(history: Vec<Self::Receipt>) -> Self {
        assert!(
            history.len() >= 3,
            "Insufficient history for Initialized state"
        );
        let (client_caps, server_caps) = validate_and_reconstruct_chain(&history);
        Machine::new(
            Initialized,
            InitializedData {
                client_capabilities: client_caps,
                server_capabilities: server_caps,
            },
        )
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
            .map(|c| serde_json::to_string(c).unwrap())
            .unwrap_or_else(|| "null".to_string());
        let server_caps_json = self
            .data
            .server_capabilities
            .as_ref()
            .map(|s| serde_json::to_string(s).unwrap())
            .unwrap_or_else(|| "null".to_string());
        let hash_0 = sha256(b"rcpt-uninitialized");
        let rcpt_1 = format!("rcpt-uninitialized-to-initializing:{}", client_caps_json);
        let hash_1 = sha256(format!("{}:{}", hash_0, rcpt_1).as_bytes());
        let rcpt_2 = format!("rcpt-initializing-to-initialized:{}", server_caps_json);
        let hash_2 = sha256(format!("{}:{}", hash_1, rcpt_2).as_bytes());
        let receipt_id = "rcpt-initialized-to-shutdown".to_string();
        let hash = sha256(format!("{}:{}", hash_2, receipt_id).as_bytes());
        tower_lsp_max_protocol::Receipt { receipt_id, hash }
    }

    fn exit(self) -> EmptyData {
        self.data
    }

    fn replay(history: Vec<Self::Receipt>) -> Self {
        assert!(
            history.len() >= 4,
            "Insufficient history for ShutDown state"
        );
        let (client_caps, server_caps) = validate_and_reconstruct_chain(&history);
        Machine::new(
            ShutDown,
            EmptyData {
                client_capabilities: Some(client_caps),
                server_capabilities: Some(server_caps),
            },
        )
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
            .map(|c| serde_json::to_string(c).unwrap())
            .unwrap_or_else(|| "null".to_string());
        let server_caps_json = self
            .data
            .server_capabilities
            .as_ref()
            .map(|s| serde_json::to_string(s).unwrap())
            .unwrap_or_else(|| "null".to_string());
        let hash_0 = sha256(b"rcpt-uninitialized");
        let rcpt_1 = format!("rcpt-uninitialized-to-initializing:{}", client_caps_json);
        let hash_1 = sha256(format!("{}:{}", hash_0, rcpt_1).as_bytes());
        let rcpt_2 = format!("rcpt-initializing-to-initialized:{}", server_caps_json);
        let hash_2 = sha256(format!("{}:{}", hash_1, rcpt_2).as_bytes());
        let rcpt_3 = "rcpt-initialized-to-shutdown".to_string();
        let hash_3 = sha256(format!("{}:{}", hash_2, rcpt_3).as_bytes());
        let receipt_id = "rcpt-shutdown-to-exited".to_string();
        let hash = sha256(format!("{}:{}", hash_3, receipt_id).as_bytes());
        tower_lsp_max_protocol::Receipt { receipt_id, hash }
    }

    fn exit(self) -> EmptyData {
        self.data
    }

    fn replay(history: Vec<Self::Receipt>) -> Self {
        assert!(history.len() >= 5, "Insufficient history for Exited state");
        let (client_caps, server_caps) = validate_and_reconstruct_chain(&history);
        Machine::new(
            Exited,
            EmptyData {
                client_capabilities: Some(client_caps),
                server_capabilities: Some(server_caps),
            },
        )
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
            serde_json::to_string(&client_caps).unwrap()
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
            serde_json::to_string(&server_caps).unwrap()
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
        assert_eq!(
            replayed_exited.data.client_capabilities.as_ref().unwrap(),
            &client_caps
        );
        assert_eq!(
            replayed_exited.data.server_capabilities.as_ref().unwrap(),
            &server_caps
        );

        // Tampered replay: change client capabilities in the receipt ID
        let mut tampered_history = history.clone();
        tampered_history[1].receipt_id = "rcpt-uninitialized-to-initializing:{}".to_string();
        let result = std::panic::catch_unwind(|| {
            <Machine<AccessAdmissionLaw, Exited, EmptyData> as TypestateKernel<_, _, _>>::replay(
                tampered_history,
            );
        });
        assert!(
            result.is_err(),
            "Replaying a tampered receipt history must abort/panic"
        );

        // Tampered replay: change a hash
        let mut tampered_history_hash = history.clone();
        tampered_history_hash[2].hash = "wrong_hash".to_string();
        let result = std::panic::catch_unwind(|| {
            <Machine<AccessAdmissionLaw, Exited, EmptyData> as TypestateKernel<_, _, _>>::replay(
                tampered_history_hash,
            );
        });
        assert!(
            result.is_err(),
            "Replaying a tampered hash must abort/panic"
        );
    }
}
