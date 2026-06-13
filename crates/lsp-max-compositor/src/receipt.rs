use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

/// Per-flush merge provenance record emitted by FlushCoordinator.
///
/// Encodes what law set governed this flush (via prefixes_fingerprint),
/// how many server inputs contributed, and whether any ANDON block was
/// present at emission time.
///
/// This is NOT a cryptographic receipt — it is a lightweight audit
/// trail for identifying which law set was active at each flush.
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
