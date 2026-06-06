use crate::control_plane::admission::{
    AdmittedData, CandidateData, GraphAdmissionError, GraphAdmissionLaw, QuarantinedData, RawData,
    RefusedData, ReplayedData, SupersededData, ADMITTED, CANDIDATE, QUARANTINED, RAW, REFUSED,
    REPLAYED, SUPERSEDED,
};
use crate::control_plane::receipts::{Blake3Hash, CryptographicReceipt};
use crate::{ChainError, Machine, TypestateKernel};
use ed25519_dalek::Signer;

/// Helper function to convert bytes to hex string (zero-dependency).
fn bytes_to_hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}

// ==========================================
// Input payload for CANDIDATE -> ADMITTED
// ==========================================
pub struct CandidateAdmitInput {
    pub store: oxigraph::store::Store,
    pub graph_name: oxigraph::model::GraphName,
    pub receipt: CryptographicReceipt,
}

// ==========================================
// TypestateKernel for RAW
// ==========================================
impl TypestateKernel<GraphAdmissionLaw, RAW, RawData> for Machine<GraphAdmissionLaw, RAW, RawData> {
    type Input = String; // representing snapshot_id
    type OutputPhase = CANDIDATE;
    type OutputData = CandidateData;
    type Receipt = CryptographicReceipt;

    fn validate(&self, _input: &Self::Input) -> Result<(), GraphAdmissionError> {
        Ok(())
    }

    fn select(&self, _input: &Self::Input) -> Self::OutputPhase {
        CANDIDATE
    }

    fn admit(
        self,
        input: Self::Input,
    ) -> Result<Machine<GraphAdmissionLaw, Self::OutputPhase, Self::OutputData>, GraphAdmissionError>
    {
        self.validate(&input)?;
        self.admit_candidate(&input)
    }

    fn receipt(&self) -> Self::Receipt {
        let consequence_hash = Blake3Hash([0u8; 32]);
        let mut receipt = CryptographicReceipt {
            prev_hash: Blake3Hash([0u8; 32]),
            discipline_id: uuid::Uuid::nil(),
            law_id: uuid::Uuid::nil(),
            consequence_hash,
            sequence: 0,
            signature: [0u8; 64],
        };
        let payload_hash = receipt.compute_payload_hash();
        let signing_key = ed25519_dalek::SigningKey::from_bytes(&[0u8; 32]);
        receipt.signature = signing_key.sign(&payload_hash.0).to_bytes();
        receipt
    }

    fn exit(self) -> RawData {
        self.data
    }

    fn replay(history: Vec<Self::Receipt>) -> Result<Self, ChainError> {
        if history.is_empty() {
            return Err(ChainError::EmptyHistory);
        }
        if history[0].sequence != 0 {
            return Err(ChainError::ReceiptIdMismatch {
                index: 0,
                detail: "Expected genesis receipt with sequence 0".to_string(),
            });
        }
        Ok(Machine::new(
            RAW,
            RawData {
                elements: Vec::new(),
            },
        ))
    }
}

// ==========================================
// TypestateKernel for CANDIDATE
// ==========================================
impl TypestateKernel<GraphAdmissionLaw, CANDIDATE, CandidateData>
    for Machine<GraphAdmissionLaw, CANDIDATE, CandidateData>
{
    type Input = CandidateAdmitInput;
    type OutputPhase = ADMITTED;
    type OutputData = AdmittedData;
    type Receipt = CryptographicReceipt;

    fn validate(&self, input: &Self::Input) -> Result<(), GraphAdmissionError> {
        if input.receipt.sequence == 0 {
            return Err(GraphAdmissionError::ParsingFailed(
                "Receipt sequence cannot be 0".to_string(),
            ));
        }
        Ok(())
    }

    fn select(&self, _input: &Self::Input) -> Self::OutputPhase {
        ADMITTED
    }

    fn admit(
        self,
        input: Self::Input,
    ) -> Result<Machine<GraphAdmissionLaw, Self::OutputPhase, Self::OutputData>, GraphAdmissionError>
    {
        self.validate(&input)?;
        self.admit_admitted(&input.store, input.graph_name, input.receipt)
    }

    fn receipt(&self) -> Self::Receipt {
        let mut hasher = blake3::Hasher::new();
        hasher.update(self.data.graph_hash.as_bytes());
        let consequence_hash = Blake3Hash(*hasher.finalize().as_bytes());

        let mut receipt = CryptographicReceipt {
            prev_hash: Blake3Hash([0u8; 32]),
            discipline_id: uuid::Uuid::nil(),
            law_id: uuid::Uuid::nil(),
            consequence_hash,
            sequence: 1,
            signature: [0u8; 64],
        };
        let payload_hash = receipt.compute_payload_hash();
        let signing_key = ed25519_dalek::SigningKey::from_bytes(&[0u8; 32]);
        receipt.signature = signing_key.sign(&payload_hash.0).to_bytes();
        receipt
    }

    fn exit(self) -> CandidateData {
        self.data
    }

    fn replay(history: Vec<Self::Receipt>) -> Result<Self, ChainError> {
        if history.len() < 2 {
            return Err(ChainError::InsufficientHistory {
                required: 2,
                got: history.len(),
            });
        }
        let signing_key = ed25519_dalek::SigningKey::from_bytes(&[0u8; 32]);
        let verifying_key = signing_key.verifying_key();
        crate::control_plane::receipts::verify_receipt_chain(
            &history,
            &verifying_key,
            &history[0].prev_hash,
        )
        .map_err(|e| ChainError::HashMismatch {
            index: 1,
            expected: "Valid signature".to_string(),
            got: e.to_string(),
        })?;

        let graph_hash = bytes_to_hex(&history[1].consequence_hash.0);
        Ok(Machine::new(
            CANDIDATE,
            CandidateData {
                elements: Vec::new(),
                quads: Vec::new(),
                graph_hash,
            },
        ))
    }
}

// ==========================================
// TypestateKernel for ADMITTED
// ==========================================
impl TypestateKernel<GraphAdmissionLaw, ADMITTED, AdmittedData>
    for Machine<GraphAdmissionLaw, ADMITTED, AdmittedData>
{
    type Input = oxigraph::model::GraphName;
    type OutputPhase = SUPERSEDED;
    type OutputData = SupersededData;
    type Receipt = CryptographicReceipt;

    fn validate(&self, _input: &Self::Input) -> Result<(), GraphAdmissionError> {
        Ok(())
    }

    fn select(&self, _input: &Self::Input) -> Self::OutputPhase {
        SUPERSEDED
    }

    fn admit(
        self,
        input: Self::Input,
    ) -> Result<Machine<GraphAdmissionLaw, Self::OutputPhase, Self::OutputData>, GraphAdmissionError>
    {
        self.validate(&input)?;
        Ok(self.admit_supersede(input))
    }

    fn receipt(&self) -> Self::Receipt {
        self.data.receipt.clone()
    }

    fn exit(self) -> AdmittedData {
        self.data
    }

    fn replay(history: Vec<Self::Receipt>) -> Result<Self, ChainError> {
        if history.len() < 3 {
            return Err(ChainError::InsufficientHistory {
                required: 3,
                got: history.len(),
            });
        }
        let signing_key = ed25519_dalek::SigningKey::from_bytes(&[0u8; 32]);
        let verifying_key = signing_key.verifying_key();
        crate::control_plane::receipts::verify_receipt_chain(
            &history,
            &verifying_key,
            &history[0].prev_hash,
        )
        .map_err(|e| ChainError::HashMismatch {
            index: 2,
            expected: "Valid signature".to_string(),
            got: e.to_string(),
        })?;

        Ok(Machine::new(
            ADMITTED,
            AdmittedData {
                graph_name: oxigraph::model::GraphName::DefaultGraph,
                quad_count: 0,
                receipt: history[2].clone(),
            },
        ))
    }
}

// ==========================================
// TypestateKernel for SUPERSEDED
// ==========================================
impl TypestateKernel<GraphAdmissionLaw, SUPERSEDED, SupersededData>
    for Machine<GraphAdmissionLaw, SUPERSEDED, SupersededData>
{
    type Input = ();
    type OutputPhase = SUPERSEDED;
    type OutputData = SupersededData;
    type Receipt = CryptographicReceipt;

    fn validate(&self, _input: &Self::Input) -> Result<(), GraphAdmissionError> {
        Err(GraphAdmissionError::ParsingFailed(
            "Already superseded".to_string(),
        ))
    }

    fn select(&self, _input: &Self::Input) -> Self::OutputPhase {
        SUPERSEDED
    }

    fn admit(
        self,
        input: Self::Input,
    ) -> Result<Machine<GraphAdmissionLaw, Self::OutputPhase, Self::OutputData>, GraphAdmissionError>
    {
        self.validate(&input)?;
        Ok(self)
    }

    fn receipt(&self) -> Self::Receipt {
        let mut receipt = CryptographicReceipt {
            prev_hash: Blake3Hash([0u8; 32]),
            discipline_id: uuid::Uuid::nil(),
            law_id: uuid::Uuid::nil(),
            consequence_hash: Blake3Hash([0u8; 32]),
            sequence: 3,
            signature: [0u8; 64],
        };
        let payload_hash = receipt.compute_payload_hash();
        let signing_key = ed25519_dalek::SigningKey::from_bytes(&[0u8; 32]);
        receipt.signature = signing_key.sign(&payload_hash.0).to_bytes();
        receipt
    }

    fn exit(self) -> SupersededData {
        self.data
    }

    fn replay(history: Vec<Self::Receipt>) -> Result<Self, ChainError> {
        if history.len() < 4 {
            return Err(ChainError::InsufficientHistory {
                required: 4,
                got: history.len(),
            });
        }
        let signing_key = ed25519_dalek::SigningKey::from_bytes(&[0u8; 32]);
        let verifying_key = signing_key.verifying_key();
        crate::control_plane::receipts::verify_receipt_chain(
            &history,
            &verifying_key,
            &history[0].prev_hash,
        )
        .map_err(|e| ChainError::HashMismatch {
            index: 3,
            expected: "Valid signature".to_string(),
            got: e.to_string(),
        })?;

        Ok(Machine::new(
            SUPERSEDED,
            SupersededData {
                graph_name: oxigraph::model::GraphName::DefaultGraph,
                superseded_by: oxigraph::model::GraphName::DefaultGraph,
            },
        ))
    }
}

// ==========================================
// TypestateKernel for REFUSED
// ==========================================
impl TypestateKernel<GraphAdmissionLaw, REFUSED, RefusedData>
    for Machine<GraphAdmissionLaw, REFUSED, RefusedData>
{
    type Input = ();
    type OutputPhase = REFUSED;
    type OutputData = RefusedData;
    type Receipt = CryptographicReceipt;

    fn validate(&self, _input: &Self::Input) -> Result<(), GraphAdmissionError> {
        Err(GraphAdmissionError::ParsingFailed(
            "Already refused".to_string(),
        ))
    }

    fn select(&self, _input: &Self::Input) -> Self::OutputPhase {
        REFUSED
    }

    fn admit(
        self,
        input: Self::Input,
    ) -> Result<Machine<GraphAdmissionLaw, Self::OutputPhase, Self::OutputData>, GraphAdmissionError>
    {
        self.validate(&input)?;
        Ok(self)
    }

    fn receipt(&self) -> Self::Receipt {
        let mut receipt = CryptographicReceipt {
            prev_hash: Blake3Hash([0u8; 32]),
            discipline_id: uuid::Uuid::nil(),
            law_id: uuid::Uuid::nil(),
            consequence_hash: Blake3Hash([0u8; 32]),
            sequence: 2,
            signature: [0u8; 64],
        };
        let payload_hash = receipt.compute_payload_hash();
        let signing_key = ed25519_dalek::SigningKey::from_bytes(&[0u8; 32]);
        receipt.signature = signing_key.sign(&payload_hash.0).to_bytes();
        receipt
    }

    fn exit(self) -> RefusedData {
        self.data
    }

    fn replay(history: Vec<Self::Receipt>) -> Result<Self, ChainError> {
        if history.len() < 3 {
            return Err(ChainError::InsufficientHistory {
                required: 3,
                got: history.len(),
            });
        }
        let signing_key = ed25519_dalek::SigningKey::from_bytes(&[0u8; 32]);
        let verifying_key = signing_key.verifying_key();
        crate::control_plane::receipts::verify_receipt_chain(
            &history,
            &verifying_key,
            &history[0].prev_hash,
        )
        .map_err(|e| ChainError::HashMismatch {
            index: 2,
            expected: "Valid signature".to_string(),
            got: e.to_string(),
        })?;

        Ok(Machine::new(
            REFUSED,
            RefusedData {
                report: crate::control_plane::invariants::VerificationReport {
                    is_success: false,
                    diagnostics: Vec::new(),
                    execution_time_ms: 0,
                },
            },
        ))
    }
}

// ==========================================
// TypestateKernel for QUARANTINED
// ==========================================
impl TypestateKernel<GraphAdmissionLaw, QUARANTINED, QuarantinedData>
    for Machine<GraphAdmissionLaw, QUARANTINED, QuarantinedData>
{
    type Input = String; // representing graph_hash
    type OutputPhase = CANDIDATE;
    type OutputData = CandidateData;
    type Receipt = CryptographicReceipt;

    fn validate(&self, _input: &Self::Input) -> Result<(), GraphAdmissionError> {
        Ok(())
    }

    fn select(&self, _input: &Self::Input) -> Self::OutputPhase {
        CANDIDATE
    }

    fn admit(
        self,
        input: Self::Input,
    ) -> Result<Machine<GraphAdmissionLaw, Self::OutputPhase, Self::OutputData>, GraphAdmissionError>
    {
        self.validate(&input)?;
        Ok(self.into_candidate(input))
    }

    fn receipt(&self) -> Self::Receipt {
        let mut receipt = CryptographicReceipt {
            prev_hash: Blake3Hash([0u8; 32]),
            discipline_id: uuid::Uuid::nil(),
            law_id: uuid::Uuid::nil(),
            consequence_hash: Blake3Hash([0u8; 32]),
            sequence: 2,
            signature: [0u8; 64],
        };
        let payload_hash = receipt.compute_payload_hash();
        let signing_key = ed25519_dalek::SigningKey::from_bytes(&[0u8; 32]);
        receipt.signature = signing_key.sign(&payload_hash.0).to_bytes();
        receipt
    }

    fn exit(self) -> QuarantinedData {
        self.data
    }

    fn replay(history: Vec<Self::Receipt>) -> Result<Self, ChainError> {
        if history.len() < 3 {
            return Err(ChainError::InsufficientHistory {
                required: 3,
                got: history.len(),
            });
        }
        let signing_key = ed25519_dalek::SigningKey::from_bytes(&[0u8; 32]);
        let verifying_key = signing_key.verifying_key();
        crate::control_plane::receipts::verify_receipt_chain(
            &history,
            &verifying_key,
            &history[0].prev_hash,
        )
        .map_err(|e| ChainError::HashMismatch {
            index: 2,
            expected: "Valid signature".to_string(),
            got: e.to_string(),
        })?;

        Ok(Machine::new(
            QUARANTINED,
            QuarantinedData {
                elements: Vec::new(),
                quads: Vec::new(),
                missing_dependencies: Vec::new(),
            },
        ))
    }
}

// ==========================================
// TypestateKernel for REPLAYED
// ==========================================
impl TypestateKernel<GraphAdmissionLaw, REPLAYED, ReplayedData>
    for Machine<GraphAdmissionLaw, REPLAYED, ReplayedData>
{
    type Input = ();
    type OutputPhase = REPLAYED;
    type OutputData = ReplayedData;
    type Receipt = CryptographicReceipt;

    fn validate(&self, _input: &Self::Input) -> Result<(), GraphAdmissionError> {
        Err(GraphAdmissionError::ParsingFailed(
            "Already replayed".to_string(),
        ))
    }

    fn select(&self, _input: &Self::Input) -> Self::OutputPhase {
        REPLAYED
    }

    fn admit(
        self,
        input: Self::Input,
    ) -> Result<Machine<GraphAdmissionLaw, Self::OutputPhase, Self::OutputData>, GraphAdmissionError>
    {
        self.validate(&input)?;
        Ok(self)
    }

    fn receipt(&self) -> Self::Receipt {
        self.data.receipt.clone()
    }

    fn exit(self) -> ReplayedData {
        self.data
    }

    fn replay(history: Vec<Self::Receipt>) -> Result<Self, ChainError> {
        if history.len() < 3 {
            return Err(ChainError::InsufficientHistory {
                required: 3,
                got: history.len(),
            });
        }
        let signing_key = ed25519_dalek::SigningKey::from_bytes(&[0u8; 32]);
        let verifying_key = signing_key.verifying_key();
        crate::control_plane::receipts::verify_receipt_chain(
            &history,
            &verifying_key,
            &history[0].prev_hash,
        )
        .map_err(|e| ChainError::HashMismatch {
            index: 2,
            expected: "Valid signature".to_string(),
            got: e.to_string(),
        })?;

        Ok(Machine::new(
            REPLAYED,
            ReplayedData {
                graph_name: oxigraph::model::GraphName::DefaultGraph,
                receipt: history[2].clone(),
            },
        ))
    }
}

// ==========================================
// Unit Tests
// ==========================================
#[cfg(test)]
mod tests;
