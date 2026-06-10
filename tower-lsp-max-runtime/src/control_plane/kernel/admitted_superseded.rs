use crate::control_plane::admission::{
    AdmittedData, GraphAdmissionError, GraphAdmissionLaw, SupersededData, ADMITTED, SUPERSEDED,
};
use crate::control_plane::receipts::{Blake3Hash, CryptographicReceipt};
use crate::{ChainError, Machine, TypestateKernel};
use ed25519_dalek::Signer;

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
