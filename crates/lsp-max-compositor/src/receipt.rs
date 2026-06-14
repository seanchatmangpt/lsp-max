use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

/// Machine-readable emission status of a CompositorReceipt.
///
/// BLOCKED: has_andon_block was true at construction — receipt must not be treated
///          as evidence of admission. ANDON law violations were present.
/// ADMITTED: no ANDON block — diagnostics were merged and published under a clear gate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReceiptStatus {
    Admitted,
    Blocked,
}

impl ReceiptStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            ReceiptStatus::Admitted => "ADMITTED",
            ReceiptStatus::Blocked => "BLOCKED",
        }
    }
}

impl std::fmt::Display for ReceiptStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Per-flush merge provenance record emitted by FlushCoordinator.
///
/// Encodes what law set governed this flush (via prefixes_fingerprint),
/// how many server inputs contributed, and whether any ANDON block was
/// present at emission time.
///
/// This is NOT a cryptographic receipt — it is a lightweight audit
/// trail for identifying which law set was active at each flush.
///
/// CONSTRUCTION GUARD: when `has_andon_block` is true, `status()` returns
/// `ReceiptStatus::Blocked`. Callers MUST check `status()` before treating
/// this receipt as evidence of admission. A BLOCKED receipt is a structural
/// refusal — not a warning.
#[derive(Debug, Clone)]
pub struct CompositorReceipt {
    /// URI that was flushed.
    pub uri: String,
    /// Number of DiagnosticEntry items in the MergeResult.
    pub diagnostic_count: usize,
    /// ANDON block state at emission time.
    pub has_andon_block: bool,
    /// Active ANDON codes (codes that are REFUSED_BY_LAW with severity==1).
    pub andon_codes: Vec<String>,
    /// Fingerprint of the sorted andon_prefixes list used in this merge.
    /// Two receipts with the same fingerprint were governed by the same law set.
    pub prefixes_fingerprint: u64,
    /// RFC-B speciation evidence: per-child receipt-chain links that the merged
    /// verdict is composed from. A merged verdict is traceable to these per-child
    /// links, not just to the aggregate `andon_codes` / fingerprint above.
    pub child_evidence: Vec<crate::receipt_chain::ChildEvidence>,
}

impl CompositorReceipt {
    pub fn new(uri: String, result: &crate::merge::MergeResult, andon_prefixes: &[String]) -> Self {
        let diagnostic_count = result.diagnostics.len();
        let has_andon_block = result.has_andon_block;
        let andon_codes = result.andon_codes().iter().map(|s| s.to_string()).collect();
        let prefixes_fingerprint = fingerprint_prefixes(andon_prefixes);
        Self {
            uri,
            diagnostic_count,
            has_andon_block,
            andon_codes,
            prefixes_fingerprint,
            child_evidence: Vec::new(),
        }
    }

    /// RFC-B: attach per-child speciation evidence to this merged receipt.
    /// Each `ChildEvidence` binds a child server's own `CryptographicReceipt`
    /// chain link to the merged verdict, so the verdict is attributable.
    pub fn with_child_evidence(
        mut self,
        evidence: Vec<crate::receipt_chain::ChildEvidence>,
    ) -> Self {
        self.child_evidence = evidence;
        self
    }

    /// Stable OCEL object id for this merged flush — the join target every child's
    /// `contributes_to_merge` relationship points at.
    pub fn merge_object_id(&self) -> String {
        format!("merge_{}_{:x}", self.uri, self.prefixes_fingerprint)
    }

    /// RFC-C: export the fan-out → merge → admit flush as an OCEL 2.0 event, so the
    /// compositor process is minable as an object-centric event log. Mirrors
    /// `CryptographicReceipt::to_ocel_event`: an event with attributes and
    /// relationships. Each per-child evidence link is related back by its chain id,
    /// making per-server lineage a single graph traversal.
    pub fn to_ocel_event(&self, event_id: &str, timestamp: &str) -> serde_json::Value {
        let mut relationships: Vec<serde_json::Value> = vec![serde_json::json!({
            "objectId": self.merge_object_id(),
            "qualifier": "merged_verdict"
        })];
        for ev in &self.child_evidence {
            relationships.push(serde_json::json!({
                "objectId": ev.chain_object_id(),
                "qualifier": "speciated_evidence"
            }));
        }
        serde_json::json!({
            "id": event_id,
            "type": "CompositorFlush",
            "time": timestamp,
            "attributes": {
                "uri": self.uri,
                "status": self.status().as_str(),
                "diagnostic_count": self.diagnostic_count,
                "has_andon_block": self.has_andon_block,
                "andon_codes": self.andon_codes,
                "prefixes_fingerprint": self.prefixes_fingerprint,
                "child_evidence_count": self.child_evidence.len()
            },
            "relationships": relationships
        })
    }

    /// Returns true when ANDON law violations were present at flush time.
    /// A blocked receipt must not be used as evidence of admission.
    pub fn is_blocked(&self) -> bool {
        self.has_andon_block
    }

    /// Machine-readable emission status.
    ///
    /// Returns `ReceiptStatus::Blocked` when `has_andon_block` is true.
    /// This is the structural guard: a receipt with BLOCKED status must not be
    /// used as evidence of admission. ANDON law violations were present at flush time.
    pub fn status(&self) -> ReceiptStatus {
        if self.has_andon_block {
            ReceiptStatus::Blocked
        } else {
            ReceiptStatus::Admitted
        }
    }
}

fn fingerprint_prefixes(prefixes: &[String]) -> u64 {
    let mut sorted: Vec<&str> = prefixes.iter().map(|s| s.as_str()).collect();
    sorted.sort_unstable();
    let mut hasher = DefaultHasher::new();
    sorted.hash(&mut hasher);
    hasher.finish()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::merge::MergeResult;

    fn make_merge_result(has_andon: bool) -> MergeResult {
        MergeResult {
            diagnostics: vec![],
            has_andon_block: has_andon,
        }
    }

    #[test]
    fn receipt_captures_andon_block_state() {
        let result = make_merge_result(true);
        let prefixes = vec!["WASM4PM-".to_string(), "ANTI-LLM-".to_string()];
        let receipt = CompositorReceipt::new("file:///test.rs".to_string(), &result, &prefixes);
        assert!(receipt.has_andon_block);
        assert_eq!(receipt.uri, "file:///test.rs");
    }

    #[test]
    fn receipt_status_blocked_when_andon_active() {
        let result = make_merge_result(true);
        let prefixes = vec!["WASM4PM-".to_string()];
        let receipt = CompositorReceipt::new("file:///test.rs".to_string(), &result, &prefixes);
        assert_eq!(receipt.status(), ReceiptStatus::Blocked);
        assert_eq!(receipt.status().as_str(), "BLOCKED");
    }

    #[test]
    fn receipt_status_admitted_when_no_andon() {
        let result = make_merge_result(false);
        let prefixes = vec!["WASM4PM-".to_string()];
        let receipt = CompositorReceipt::new("file:///test.rs".to_string(), &result, &prefixes);
        assert_eq!(receipt.status(), ReceiptStatus::Admitted);
        assert_eq!(receipt.status().as_str(), "ADMITTED");
    }

    #[test]
    fn blocked_receipt_is_not_admitted() {
        // Structural guard: a BLOCKED receipt must never compare equal to ADMITTED.
        assert_ne!(ReceiptStatus::Blocked, ReceiptStatus::Admitted);
    }

    #[test]
    fn receipt_prefixes_fingerprint_deterministic() {
        let prefixes_a = vec!["WASM4PM-".to_string(), "GGEN-".to_string()];
        let prefixes_b = vec!["GGEN-".to_string(), "WASM4PM-".to_string()]; // order swapped
        let result = make_merge_result(false);
        let r_a = CompositorReceipt::new("file:///x.ts".to_string(), &result, &prefixes_a);
        let r_b = CompositorReceipt::new("file:///x.ts".to_string(), &result, &prefixes_b);
        assert_eq!(
            r_a.prefixes_fingerprint, r_b.prefixes_fingerprint,
            "fingerprint must be order-independent (sorted before hashing)"
        );
    }

    #[test]
    fn receipt_different_law_sets_have_different_fingerprints() {
        let prefixes_a = vec!["WASM4PM-".to_string()];
        let prefixes_b = vec!["WASM4PM-".to_string(), "GGEN-".to_string()];
        let result = make_merge_result(false);
        let r_a = CompositorReceipt::new("file:///x.ts".to_string(), &result, &prefixes_a);
        let r_b = CompositorReceipt::new("file:///x.ts".to_string(), &result, &prefixes_b);
        assert_ne!(r_a.prefixes_fingerprint, r_b.prefixes_fingerprint);
    }
}
