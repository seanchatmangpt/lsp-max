use crate::runtime::control_plane::admission::{
    CandidateData, GraphAdmissionError, GraphAdmissionLaw, RawData, CANDIDATE, RAW,
};
use crate::runtime::control_plane::receipts::{Blake3Hash, CryptographicReceipt};
use crate::runtime::{ChainError, Machine, TypestateKernel};
use ed25519_dalek::Signer;

use super::bytes_to_hex;
use super::CandidateAdmitInput;

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
    type OutputPhase = crate::runtime::control_plane::admission::ADMITTED;
    type OutputData = crate::runtime::control_plane::admission::AdmittedData;
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
        crate::runtime::control_plane::admission::ADMITTED
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
        crate::runtime::control_plane::receipts::verify_receipt_chain(
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
