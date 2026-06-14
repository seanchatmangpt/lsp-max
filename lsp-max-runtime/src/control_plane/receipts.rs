use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::RwLock;
use uuid::Uuid;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Serialize, Deserialize)]
pub struct Blake3Hash(pub [u8; 32]);

impl AsRef<[u8; 32]> for Blake3Hash {
    fn as_ref(&self) -> &[u8; 32] {
        &self.0
    }
}

impl AsRef<[u8]> for Blake3Hash {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

impl From<[u8; 32]> for Blake3Hash {
    fn from(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }
}

impl From<blake3::Hash> for Blake3Hash {
    fn from(hash: blake3::Hash) -> Self {
        Self(*hash.as_bytes())
    }
}

mod signature_serde {
    use serde::{Deserializer, Serializer};
    pub fn serialize<S>(bytes: &[u8; 64], serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.collect_seq(bytes.iter())
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<[u8; 64], D::Error>
    where
        D: Deserializer<'de>,
    {
        let v: Vec<u8> = serde::Deserialize::deserialize(deserializer)?;
        if v.len() == 64 {
            let mut array = [0u8; 64];
            array.copy_from_slice(&v);
            Ok(array)
        } else {
            Err(serde::de::Error::custom("expected an array of length 64"))
        }
    }
}

#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct CryptographicReceipt {
    pub prev_hash: Blake3Hash,
    pub discipline_id: Uuid,
    pub law_id: Uuid,
    pub consequence_hash: Blake3Hash,
    pub sequence: u64,
    #[serde(with = "signature_serde")]
    pub signature: [u8; 64],
}

impl CryptographicReceipt {
    pub fn compute_payload_hash(&self) -> Blake3Hash {
        let mut hasher = blake3::Hasher::new();
        hasher.update(&self.prev_hash.0);
        hasher.update(self.discipline_id.as_bytes());
        hasher.update(self.law_id.as_bytes());
        hasher.update(&self.consequence_hash.0);
        hasher.update(&self.sequence.to_le_bytes());
        Blake3Hash(*hasher.finalize().as_bytes())
    }

    /// Returns the metadata attributes formatted for OpenTelemetry / tracing.
    pub fn trace_attributes(&self) -> Vec<(&'static str, String)> {
        vec![
            ("ostar.prev_hash", to_hex(&self.prev_hash.0)),
            ("ostar.discipline_id", self.discipline_id.to_string()),
            ("ostar.law_id", self.law_id.to_string()),
            ("ostar.consequence_hash", to_hex(&self.consequence_hash.0)),
            ("ostar.sequence", self.sequence.to_string()),
        ]
    }

    /// Exports the receipt as an OCEL 2.0 event JSON object representation.
    pub fn to_ocel_event(&self, event_id: &str, timestamp: &str) -> serde_json::Value {
        serde_json::json!({
            "id": event_id,
            "type": "TransitionExecution",
            "time": timestamp,
            "attributes": {
                "sequence": self.sequence,
                "consequence_hash": to_hex(&self.consequence_hash.0)
            },
            "relationships": [
                { "objectId": format!("obj_discipline_{}", self.discipline_id), "qualifier": "discipline" },
                { "objectId": format!("obj_law_{}", self.law_id), "qualifier": "governing_law" },
                { "objectId": format!("receipt_{}", self.sequence), "qualifier": "attestation" }
            ]
        })
    }

    /// Exports the receipt as an OCEL 2.0 object JSON object representation.
    pub fn to_ocel_object(&self) -> serde_json::Value {
        serde_json::json!({
            "id": format!("receipt_{}", self.sequence),
            "type": "Receipt",
            "attributes": {
                "prev_hash": to_hex(&self.prev_hash.0),
                "signature": to_hex(&self.signature)
            }
        })
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Moniker ↔ OCEL join (cross-product #1: provenance-carrying code index)
//
// The bridge that makes "go to definition" (an LSIF moniker vertex) and "show
// the receipt chain that produced this symbol" (OCEL events relating to an
// object) the same identity. The join key is the moniker's CONTENT identity
// `(scheme, identifier)` — NOT the LSIF numeric vertex id, which is
// allocation-order dependent and shifts under unrelated edits. Keying on the
// numeric id would pass a re-run determinism check yet silently break the first
// time any source file changed; the content address is stable under unrelated
// edits (witnessed in `moniker_join` tests below).
// ─────────────────────────────────────────────────────────────────────────────

use lsp_max_lsif::lsif_types::{MonikerKind, UniquenessLevel};

/// The single authoritative OCEL `objectId` for a code symbol, derived from its
/// moniker content identity. Both the LSIF moniker vertex and the OCEL
/// `CodeSymbol` object resolve to this string — it is the join key. Defining it
/// in exactly one place keeps a competing authority from minting a second,
/// divergent id format for the same symbol.
pub fn moniker_object_id(scheme: &str, identifier: &str) -> String {
    format!("moniker:{scheme}:{identifier}")
}

/// Export a code symbol (identified by its moniker) as an OCEL 2.0 object.
/// Its `id` is the moniker join key, so any receipt event that produced this
/// symbol can reference it by the same identity an LSIF consumer would resolve.
pub fn moniker_to_ocel_object(
    scheme: &str,
    identifier: &str,
    kind: &MonikerKind,
    unique: &UniquenessLevel,
) -> serde_json::Value {
    serde_json::json!({
        "id": moniker_object_id(scheme, identifier),
        "type": "CodeSymbol",
        "attributes": {
            "scheme": scheme,
            "identifier": identifier,
            "kind": kind,
            "unique": unique,
        }
    })
}

impl CryptographicReceipt {
    /// Export the receipt as an OCEL 2.0 event that additionally relates to the
    /// code symbol it produced, by the moniker join key. This is the load-bearing
    /// half of cross-product #1: the receipt's operation-event and the LSIF
    /// moniker vertex now share one OCEL object id, so navigation and provenance
    /// are a single graph traversal.
    pub fn to_ocel_event_for_symbol(
        &self,
        event_id: &str,
        timestamp: &str,
        scheme: &str,
        identifier: &str,
    ) -> serde_json::Value {
        let mut event = self.to_ocel_event(event_id, timestamp);
        if let Some(rels) = event
            .get_mut("relationships")
            .and_then(|r| r.as_array_mut())
        {
            rels.push(serde_json::json!({
                "objectId": moniker_object_id(scheme, identifier),
                "qualifier": "produced_symbol"
            }));
        }
        event
    }
}

/// Helper function to format bytes to hex string.
pub fn to_hex(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 2);
    for &b in bytes {
        use std::fmt::Write;
        write!(&mut s, "{:02x}", b).unwrap();
    }
    s
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ChainValidationError {
    #[error("Chain is empty")]
    EmptyChain,
    #[error("Sequence mismatch at index {index}: expected {expected}, found {found}")]
    SequenceMismatch {
        index: usize,
        expected: u64,
        found: u64,
    },
    #[error("Hash mismatch at index {index}")]
    HashMismatch { index: usize },
    #[error("Signature verification failed at index {index}")]
    SignatureVerificationFailed { index: usize },
    #[error("Genesis link broken")]
    GenesisLinkBroken,
}

/// Iteratively verifies an array slice of cryptographic receipts.
#[allow(clippy::explicit_counter_loop)]
pub fn verify_receipt_chain(
    chain: &[CryptographicReceipt],
    verifying_key: &VerifyingKey,
    expected_genesis_hash: &Blake3Hash,
) -> Result<(), ChainValidationError> {
    if chain.is_empty() {
        return Err(ChainValidationError::EmptyChain);
    }

    let mut expected_prev_hash = *expected_genesis_hash;
    let mut expected_sequence = chain[0].sequence;

    for (index, receipt) in chain.iter().enumerate() {
        // 1. Verify chronological progression of execution sequence
        if receipt.sequence != expected_sequence {
            return Err(ChainValidationError::SequenceMismatch {
                index,
                expected: expected_sequence,
                found: receipt.sequence,
            });
        }

        // 2. Verify link integrity to previous receipt
        if receipt.prev_hash != expected_prev_hash {
            return Err(ChainValidationError::HashMismatch { index });
        }

        // 3. Compute and verify the payload digest
        let payload_hash = receipt.compute_payload_hash();

        // 4. Verify Ed25519 signature of the payload digest
        let sig = Signature::from_bytes(&receipt.signature);
        if verifying_key.verify(&payload_hash.0, &sig).is_err() {
            return Err(ChainValidationError::SignatureVerificationFailed { index });
        }

        // Prepare context for the next block evaluation
        expected_prev_hash = payload_hash;
        expected_sequence += 1;
    }

    Ok(())
}

#[derive(Debug, thiserror::Error)]
pub enum KeyManagementError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Key parse error: {0}")]
    KeyParse(String),
    #[error("Signature error: {0}")]
    Signature(#[from] ed25519_dalek::SignatureError),
    #[error("Key not found: {0}")]
    KeyNotFound(Uuid),
}

/// A robust Key Management Keystore for Ed25519 signing and verification.
pub struct Keystore {
    primary_key: SigningKey,
    trusted_keys: RwLock<HashMap<Uuid, VerifyingKey>>,
}

impl Keystore {
    /// Generate a fresh random key pair.
    pub fn generate() -> Self {
        use rand_core::OsRng;
        let mut csprng = OsRng;
        let primary_key = SigningKey::generate(&mut csprng);
        Self {
            primary_key,
            trusted_keys: RwLock::new(HashMap::new()),
        }
    }

    /// Create from a raw seed bytes array.
    pub fn from_seed(seed: &[u8; 32]) -> Self {
        let primary_key = SigningKey::from_bytes(seed);
        Self {
            primary_key,
            trusted_keys: RwLock::new(HashMap::new()),
        }
    }

    /// Create from a seed slice.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, KeyManagementError> {
        if bytes.len() != 32 {
            return Err(KeyManagementError::KeyParse(format!(
                "invalid seed length: expected 32 bytes, got {}",
                bytes.len()
            )));
        }
        let mut seed = [0u8; 32];
        seed.copy_from_slice(bytes);
        Ok(Self::from_seed(&seed))
    }

    /// Load primary key from file containing raw 32 seed bytes.
    pub fn load_from_file<P: AsRef<std::path::Path>>(path: P) -> Result<Self, KeyManagementError> {
        let bytes = std::fs::read(path)?;
        Self::from_bytes(&bytes)
    }

    /// Save primary key seed to file.
    pub fn save_to_file<P: AsRef<std::path::Path>>(
        &self,
        path: P,
    ) -> Result<(), KeyManagementError> {
        std::fs::write(path, self.primary_key.to_bytes())?;
        Ok(())
    }

    /// Returns the raw 32 seed bytes of the primary key.
    pub fn to_bytes(&self) -> [u8; 32] {
        self.primary_key.to_bytes()
    }

    /// Returns the VerifyingKey of the primary key pair.
    pub fn verifying_key(&self) -> VerifyingKey {
        self.primary_key.verifying_key()
    }

    /// Sign a payload hash using the primary key.
    pub fn sign_hash(&self, hash: &Blake3Hash) -> [u8; 64] {
        self.primary_key.sign(hash.as_ref()).to_bytes()
    }

    /// Sign a CryptographicReceipt in place using the primary key.
    pub fn sign_receipt(&self, receipt: &mut CryptographicReceipt) {
        let hash = receipt.compute_payload_hash();
        receipt.signature = self.sign_hash(&hash);
    }

    /// Register a trusted verifying public key associated with a Uuid.
    pub fn register_trusted_key(&self, id: Uuid, key: VerifyingKey) {
        let mut keys = self.trusted_keys.write().unwrap();
        keys.insert(id, key);
    }

    /// Retrieve a trusted verifying public key.
    pub fn get_trusted_key(&self, id: &Uuid) -> Option<VerifyingKey> {
        let keys = self.trusted_keys.read().unwrap();
        keys.get(id).copied()
    }

    /// Verify a signature against a payload hash.
    pub fn verify_signature(
        verifying_key: &VerifyingKey,
        hash: &Blake3Hash,
        signature_bytes: &[u8; 64],
    ) -> Result<(), KeyManagementError> {
        let signature = Signature::from_bytes(signature_bytes);
        verifying_key.verify(hash.as_ref(), &signature)?;
        Ok(())
    }

    /// Verify a CryptographicReceipt's signature using the verifying key registered
    /// for its discipline_id, or fallback to the primary verifying key if no specific key is registered.
    pub fn verify_receipt(&self, receipt: &CryptographicReceipt) -> Result<(), KeyManagementError> {
        let payload_hash = receipt.compute_payload_hash();
        let verifying_key = self
            .get_trusted_key(&receipt.discipline_id)
            .unwrap_or_else(|| self.verifying_key());
        Self::verify_signature(&verifying_key, &payload_hash, &receipt.signature)
    }
}

/// A ReplayEngine that implements Section 4 (Deterministic Replay Protocol) of the ARD.
pub struct ReplayEngine {
    expected_genesis_hash: Blake3Hash,
    verifying_key: VerifyingKey,
}

impl ReplayEngine {
    pub fn new(expected_genesis_hash: Blake3Hash, verifying_key: VerifyingKey) -> Self {
        Self {
            expected_genesis_hash,
            verifying_key,
        }
    }

    /// Replays a sequence of transition inputs and asserts they match the receipt chain.
    pub fn replay<F>(
        &self,
        chain: &[CryptographicReceipt],
        mut transition_function: F,
    ) -> Result<(), ChainValidationError>
    where
        F: FnMut(&CryptographicReceipt) -> Blake3Hash,
    {
        // 1. First, verify the chain cryptographically
        verify_receipt_chain(chain, &self.verifying_key, &self.expected_genesis_hash)?;

        // 2. Perform isolation of state and execute deterministic transition conformance checks
        for (index, receipt) in chain.iter().enumerate() {
            let computed_consequence = transition_function(receipt);
            if computed_consequence.0 != receipt.consequence_hash.0 {
                return Err(ChainValidationError::HashMismatch { index });
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests;
