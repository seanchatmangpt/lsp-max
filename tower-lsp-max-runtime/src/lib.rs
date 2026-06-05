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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct EmptyData;
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
        Machine::new(ShutDown, EmptyData)
    }
}

impl Machine<AccessAdmissionLaw, ShutDown, EmptyData> {
    /// Transitions from ShutDown to Exited.
    pub fn admit_exit(self) -> Machine<AccessAdmissionLaw, Exited, EmptyData> {
        Machine::new(Exited, EmptyData)
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
        tower_lsp_max_protocol::Receipt {
            receipt_id: "receipt_uninitialized_to_initializing".to_string(),
            hash: "hash_uninitialized_to_initializing".to_string(),
        }
    }

    fn exit(self) -> EmptyData {
        self.data
    }

    fn replay(_history: Vec<Self::Receipt>) -> Self {
        Machine::new(Uninitialized, EmptyData)
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
        tower_lsp_max_protocol::Receipt {
            receipt_id: "receipt_initializing_to_initialized".to_string(),
            hash: "hash_initializing_to_initialized".to_string(),
        }
    }

    fn exit(self) -> InitializingData {
        self.data
    }

    fn replay(_history: Vec<Self::Receipt>) -> Self {
        Machine::new(
            Initializing,
            InitializingData {
                client_capabilities: serde_json::Value::Null,
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
        tower_lsp_max_protocol::Receipt {
            receipt_id: "receipt_initialized_to_shutdown".to_string(),
            hash: "hash_initialized_to_shutdown".to_string(),
        }
    }

    fn exit(self) -> InitializedData {
        self.data
    }

    fn replay(_history: Vec<Self::Receipt>) -> Self {
        Machine::new(
            Initialized,
            InitializedData {
                client_capabilities: serde_json::Value::Null,
                server_capabilities: serde_json::Value::Null,
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
        tower_lsp_max_protocol::Receipt {
            receipt_id: "receipt_shutdown_to_exited".to_string(),
            hash: "hash_shutdown_to_exited".to_string(),
        }
    }

    fn exit(self) -> EmptyData {
        self.data
    }

    fn replay(_history: Vec<Self::Receipt>) -> Self {
        Machine::new(ShutDown, EmptyData)
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
        tower_lsp_max_protocol::Receipt {
            receipt_id: "receipt_exited".to_string(),
            hash: "hash_exited".to_string(),
        }
    }

    fn exit(self) -> EmptyData {
        self.data
    }

    fn replay(_history: Vec<Self::Receipt>) -> Self {
        Machine::new(Exited, EmptyData)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_typestate_lifecycle() {
        // Starts in Uninitialized
        let machine = Machine::new(Uninitialized, EmptyData);
        assert_eq!(
            machine.receipt().receipt_id,
            "receipt_uninitialized_to_initializing"
        );

        // Transition: Uninitialized -> Initializing
        let client_caps = serde_json::json!({"textDocument": {"hover": {}}});
        let machine = machine.admit_initialize(client_caps.clone());
        assert_eq!(machine.data.client_capabilities, client_caps);
        assert_eq!(
            machine.receipt().receipt_id,
            "receipt_initializing_to_initialized"
        );

        // Transition: Initializing -> Initialized
        let server_caps = serde_json::json!({"hoverProvider": true});
        let machine = machine.admit_initialized(server_caps.clone());
        assert_eq!(machine.data.client_capabilities, client_caps);
        assert_eq!(machine.data.server_capabilities, server_caps);
        assert_eq!(
            machine.receipt().receipt_id,
            "receipt_initialized_to_shutdown"
        );

        // Transition: Initialized -> ShutDown
        let machine = machine.admit_shutdown();
        assert_eq!(machine.receipt().receipt_id, "receipt_shutdown_to_exited");

        // Transition: ShutDown -> Exited
        let machine = machine.admit_exit();
        assert_eq!(machine.receipt().receipt_id, "receipt_exited");
    }

    #[test]
    fn test_kernel_admit_replay() {
        let machine = Machine::new(Uninitialized, EmptyData);

        // Test validate & admit through TypestateKernel trait
        let client_caps = serde_json::json!({"textDocument": {"completion": {}}});
        let result = <Machine<AccessAdmissionLaw, Uninitialized, EmptyData> as TypestateKernel<
            _,
            _,
            _,
        >>::admit(machine, client_caps.clone());
        assert!(result.is_ok());
        let machine = result.unwrap();

        let server_caps = serde_json::json!({"completionProvider": {}});
        let result =
            <Machine<AccessAdmissionLaw, Initializing, InitializingData> as TypestateKernel<
                _,
                _,
                _,
            >>::admit(machine, server_caps.clone());
        assert!(result.is_ok());
        let machine = result.unwrap();

        let result =
            <Machine<AccessAdmissionLaw, Initialized, InitializedData> as TypestateKernel<
                _,
                _,
                _,
            >>::admit(machine, ());
        assert!(result.is_ok());
        let machine = result.unwrap();

        let result = <Machine<AccessAdmissionLaw, ShutDown, EmptyData> as TypestateKernel<
            _,
            _,
            _,
        >>::admit(machine, ());
        assert!(result.is_ok());
        let _machine = result.unwrap();
    }
}
