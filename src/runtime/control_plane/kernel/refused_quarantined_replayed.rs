use crate::runtime::control_plane::admission::{
    CandidateData, GraphAdmissionError, GraphAdmissionLaw, QuarantinedData, RefusedData,
    ReplayedData, CANDIDATE, QUARANTINED, REFUSED, REPLAYED,
};
use crate::runtime::control_plane::receipts::{Blake3Hash, CryptographicReceipt};
use crate::runtime::{ChainError, Machine, TypestateKernel};
use ed25519_dalek::Signer;

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
        crate::runtime::control_plane::receipts::verify_receipt_chain(
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
                report: crate::runtime::control_plane::invariants::VerificationReport {
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
        crate::runtime::control_plane::receipts::verify_receipt_chain(
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
        crate::runtime::control_plane::receipts::verify_receipt_chain(
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
