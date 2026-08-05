use serde::{Deserialize, Serialize};

use crate::identity::digest_parts;

const GENESIS: &str = "0000000000000000000000000000000000000000000000000000000000000000";

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReceiptKind {
    Admission,
    SemanticRevision,
    Construction,
    Authorization,
    Consequence,
    Refusal,
    Differential,
}

impl ReceiptKind {
    fn tag(self) -> &'static str {
        match self {
            Self::Admission => "admission",
            Self::SemanticRevision => "semantic_revision",
            Self::Construction => "construction",
            Self::Authorization => "authorization",
            Self::Consequence => "consequence",
            Self::Refusal => "refusal",
            Self::Differential => "differential",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Outcome {
    Admitted,
    Constructed,
    Authorized,
    Executed,
    Refused,
    Equivalent,
    Divergent,
}

impl Outcome {
    fn tag(self) -> &'static str {
        match self {
            Self::Admitted => "admitted",
            Self::Constructed => "constructed",
            Self::Authorized => "authorized",
            Self::Executed => "executed",
            Self::Refused => "refused",
            Self::Equivalent => "equivalent",
            Self::Divergent => "divergent",
        }
    }
}

/// Tamper-evident statement binding identity, intent, consequence, and chain order.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Receipt {
    pub sequence: u64,
    pub previous_hash: String,
    pub kind: ReceiptKind,
    pub subject_hash: String,
    pub intent_hash: String,
    pub consequence_hash: String,
    pub outcome: Outcome,
    pub receipt_hash: String,
}

impl Receipt {
    fn calculate_hash(
        sequence: u64,
        previous_hash: &str,
        kind: ReceiptKind,
        subject_hash: &str,
        intent_hash: &str,
        consequence_hash: &str,
        outcome: Outcome,
    ) -> String {
        let sequence_bytes = sequence.to_le_bytes();
        digest_parts([
            b"ra-max/receipt/v1".as_slice(),
            sequence_bytes.as_slice(),
            previous_hash.as_bytes(),
            kind.tag().as_bytes(),
            subject_hash.as_bytes(),
            intent_hash.as_bytes(),
            consequence_hash.as_bytes(),
            outcome.tag().as_bytes(),
        ])
    }

    pub fn verify_hash(&self) -> bool {
        self.receipt_hash
            == Self::calculate_hash(
                self.sequence,
                &self.previous_hash,
                self.kind,
                &self.subject_hash,
                &self.intent_hash,
                &self.consequence_hash,
                self.outcome,
            )
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReceiptChain {
    receipts: Vec<Receipt>,
}

impl ReceiptChain {
    pub fn append(
        &mut self,
        kind: ReceiptKind,
        subject_hash: impl Into<String>,
        intent_hash: impl Into<String>,
        consequence_hash: impl Into<String>,
        outcome: Outcome,
    ) -> Receipt {
        let sequence = self.receipts.len() as u64;
        let previous_hash = self
            .receipts
            .last()
            .map(|receipt| receipt.receipt_hash.clone())
            .unwrap_or_else(|| GENESIS.to_owned());
        let subject_hash = subject_hash.into();
        let intent_hash = intent_hash.into();
        let consequence_hash = consequence_hash.into();
        let receipt_hash = Receipt::calculate_hash(
            sequence,
            &previous_hash,
            kind,
            &subject_hash,
            &intent_hash,
            &consequence_hash,
            outcome,
        );
        let receipt = Receipt {
            sequence,
            previous_hash,
            kind,
            subject_hash,
            intent_hash,
            consequence_hash,
            outcome,
            receipt_hash,
        };
        self.receipts.push(receipt.clone());
        receipt
    }

    pub fn receipts(&self) -> &[Receipt] {
        &self.receipts
    }

    pub fn last(&self) -> Option<&Receipt> {
        self.receipts.last()
    }

    pub fn verify(&self) -> bool {
        let mut expected_previous = GENESIS;
        for (index, receipt) in self.receipts.iter().enumerate() {
            if receipt.sequence != index as u64
                || receipt.previous_hash != expected_previous
                || !receipt.verify_hash()
            {
                return false;
            }
            expected_previous = &receipt.receipt_hash;
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chain_detects_tampering() {
        let mut chain = ReceiptChain::default();
        chain.append(
            ReceiptKind::Admission,
            "subject",
            "observe",
            "admitted",
            Outcome::Admitted,
        );
        chain.append(
            ReceiptKind::SemanticRevision,
            "subject",
            "analyze",
            "revision",
            Outcome::Executed,
        );
        assert!(chain.verify());

        chain.receipts[0].consequence_hash = "tampered".to_owned();
        assert!(!chain.verify());
    }

    #[test]
    fn empty_chain_is_valid() {
        assert!(ReceiptChain::default().verify());
    }
}
