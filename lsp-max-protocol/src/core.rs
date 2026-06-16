use crate::{ConformanceVector, MaxCodeAction, MaxDiagnostic, PolicyState, SnapshotId};
use lsp_types_max::{ClientCapabilities, ServerCapabilities};
use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// InstanceId — newtype for LSP instance identifiers
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct InstanceId(pub String);

impl From<String> for InstanceId {
    fn from(s: String) -> Self {
        InstanceId(s)
    }
}

impl From<&str> for InstanceId {
    fn from(s: &str) -> Self {
        InstanceId(s.to_string())
    }
}

impl PartialEq<str> for InstanceId {
    fn eq(&self, other: &str) -> bool {
        self.0 == other
    }
}
impl PartialEq<InstanceId> for str {
    fn eq(&self, other: &InstanceId) -> bool {
        self == other.0
    }
}

impl std::fmt::Display for InstanceId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

// ---------------------------------------------------------------------------
// GateId / ReceiptObligation
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct GateId(pub String);

impl std::fmt::Display for GateId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<String> for GateId {
    fn from(s: String) -> Self {
        Self(s)
    }
}

impl From<&str> for GateId {
    fn from(s: &str) -> Self {
        Self(s.to_owned())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReceiptObligation {
    pub required_receipts: Vec<String>,
}

// ---------------------------------------------------------------------------
// MaxCapabilityVector / CapabilityGap
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MaxCapabilityVector {
    pub client: ClientCapabilities,
    pub server: ServerCapabilities,
    pub negotiated: serde_json::Value,
    pub experimental: serde_json::Value,
    pub gaps: Vec<CapabilityGap>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilityGap {
    pub capability_path: String,
    pub reason: String,
}

// ---------------------------------------------------------------------------
// Receipt
// ---------------------------------------------------------------------------

/// A content-addressed ledger entry: `hash` is the digest of an artifact's exact
/// bytes, and `prev_receipt_hash` closes the Merkle chain (`None` only for
/// genesis). See the runnable witness in `examples/receipt_chain_explained.rs`,
/// which demonstrates content-addressing, tamper detection, the circular-hash
/// trap, and chain linkage — and panics if any of them regress. For receipt
/// verification driving the conformance gate, see `examples/admission_pipeline.rs`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Receipt {
    pub receipt_id: String,
    pub hash: String,
    /// Hash of the immediately preceding receipt in the instance ledger.
    /// `None` for genesis (first) receipts only.  All subsequent receipts
    /// must set this to close the Merkle chain and make `verify_instance_ledger`
    /// meaningful for non-LSP_1 instances.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prev_receipt_hash: Option<String>,
}

// ---------------------------------------------------------------------------
// AnalysisBundle
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisBundle {
    pub snapshot_id: SnapshotId,
    pub capability_vector: MaxCapabilityVector,
    pub diagnostics: Vec<MaxDiagnostic>,
    pub actions: Vec<MaxCodeAction>,
    pub conformance_vector: ConformanceVector,
    pub receipts: Vec<Receipt>,
}

impl Default for AnalysisBundle {
    fn default() -> Self {
        Self {
            snapshot_id: SnapshotId(String::new()),
            capability_vector: MaxCapabilityVector {
                client: lsp_types_max::ClientCapabilities::default(),
                server: lsp_types_max::ServerCapabilities::default(),
                negotiated: serde_json::Value::Null,
                experimental: serde_json::Value::Null,
                gaps: Vec::new(),
            },
            diagnostics: Vec::new(),
            actions: Vec::new(),
            conformance_vector: ConformanceVector::default(),
            receipts: Vec::new(),
        }
    }
}

// ---------------------------------------------------------------------------
// LspStateModel
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LspStateModel {
    pub instance_id: InstanceId,
    pub phase: String, // e.g. "Uninitialized", "Initializing", "Initialized", etc.
    pub diagnostics: Vec<MaxDiagnostic>,
    pub receipts: Vec<Receipt>,
    pub policy_state: Option<PolicyState>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ReceiptPlan;

    #[test]
    fn receipt_genesis_has_no_prev_hash() {
        let r = Receipt {
            receipt_id: "r-0".to_string(),
            hash: "abc123".to_string(),
            prev_receipt_hash: None,
        };
        assert!(
            r.prev_receipt_hash.is_none(),
            "genesis receipt must have no prev_receipt_hash"
        );
    }

    #[test]
    fn receipt_chain_links_prev_hash() {
        let genesis = Receipt {
            receipt_id: "r-0".to_string(),
            hash: "hash0".to_string(),
            prev_receipt_hash: None,
        };
        let next = Receipt {
            receipt_id: "r-1".to_string(),
            hash: "hash1".to_string(),
            prev_receipt_hash: Some(genesis.hash.clone()),
        };
        assert_eq!(
            next.prev_receipt_hash.as_deref(),
            Some("hash0"),
            "second receipt must link to genesis hash"
        );
    }

    #[test]
    fn receipt_plan_is_satisfied_by() {
        let plan = ReceiptPlan {
            expected_receipts: vec!["r-0".to_string(), "r-1".to_string()],
        };
        let receipts = [
            Receipt {
                receipt_id: "r-0".to_string(),
                hash: "h0".to_string(),
                prev_receipt_hash: None,
            },
            Receipt {
                receipt_id: "r-1".to_string(),
                hash: "h1".to_string(),
                prev_receipt_hash: Some("h0".to_string()),
            },
        ];
        let actual_ids: Vec<&str> = receipts.iter().map(|r| r.receipt_id.as_str()).collect();
        for expected in &plan.expected_receipts {
            assert!(
                actual_ids.contains(&expected.as_str()),
                "receipt {} missing",
                expected
            );
        }
    }

    #[test]
    fn analysis_bundle_is_empty() {
        let bundle = AnalysisBundle::default();
        assert!(bundle.diagnostics.is_empty());
        assert!(bundle.actions.is_empty());
        assert!(bundle.receipts.is_empty());
        assert!(bundle.conformance_vector.admitted.is_empty());
    }

    #[test]
    fn instance_id_from_str_and_display() {
        let id = InstanceId::from("test-instance");
        assert_eq!(id.to_string(), "test-instance");
        assert_eq!(id.0, "test-instance");
    }

    #[test]
    fn gate_id_from_str_and_display() {
        let gate: GateId = GateId::from("gate-42");
        assert_eq!(gate.to_string(), "gate-42");
    }

    #[test]
    fn receipt_serde_roundtrip_omits_none_prev_hash() {
        let r = Receipt {
            receipt_id: "r-0".to_string(),
            hash: "deadbeef".to_string(),
            prev_receipt_hash: None,
        };
        let json = serde_json::to_string(&r).expect("serialize");
        assert!(
            !json.contains("prev_receipt_hash"),
            "None must be omitted: {}",
            json
        );
        let r2: Receipt = serde_json::from_str(&json).expect("deserialize");
        assert!(r2.prev_receipt_hash.is_none());
    }

    #[test]
    fn receipt_serde_roundtrip_includes_prev_hash_when_set() {
        let r = Receipt {
            receipt_id: "r-1".to_string(),
            hash: "cafebabe".to_string(),
            prev_receipt_hash: Some("deadbeef".to_string()),
        };
        let json = serde_json::to_string(&r).expect("serialize");
        let r2: Receipt = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(r2.prev_receipt_hash.as_deref(), Some("deadbeef"));
    }

    #[test]
    fn receipt_roundtrip_serde() {
        // Construct a Receipt with all fields set, serialize to JSON, deserialize
        // back, and assert field equality.
        let r = Receipt {
            receipt_id: "r-42".to_string(),
            hash: "sha256:abcdef1234567890".to_string(),
            prev_receipt_hash: Some("sha256:0000000000000000".to_string()),
        };
        let json = serde_json::to_string(&r).expect("serialize");
        let r2: Receipt = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(r2.receipt_id, r.receipt_id);
        assert_eq!(r2.hash, r.hash);
        assert_eq!(r2.prev_receipt_hash, r.prev_receipt_hash);
    }

    #[test]
    fn receipt_with_no_prev_hash_is_genesis() {
        // A Receipt constructed with prev_receipt_hash = None is a genesis entry;
        // the field must remain None after round-trip through serde.
        let r = Receipt {
            receipt_id: "r-genesis".to_string(),
            hash: "sha256:genesis".to_string(),
            prev_receipt_hash: None,
        };
        assert!(r.prev_receipt_hash.is_none());
        let json = serde_json::to_string(&r).expect("serialize");
        let r2: Receipt = serde_json::from_str(&json).expect("deserialize");
        assert!(r2.prev_receipt_hash.is_none(), "genesis prev_receipt_hash must remain None after serde roundtrip");
    }
}
