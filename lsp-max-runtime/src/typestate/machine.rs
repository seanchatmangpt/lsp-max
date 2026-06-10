//! TypestateKernel implementations for each AccessAdmissionLaw phase transition.

use super::*;
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
