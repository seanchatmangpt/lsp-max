// RFC-B — per-server speciation receipt chains (attributable diagnostic lineage).
//
// Each child server in the compositor emits its OWN C_D receipt chain. The crypto
// is NOT forked: every per-child receipt is a `CryptographicReceipt` (BLAKE3 +
// Ed25519 link, sequence progression) from lsp-max-runtime. This module only adds
// the speciation envelope — the binding of (server_id, optional moniker join key)
// to a child's chain link — so that a merged compositor verdict is traceable to
// per-child evidence rather than to an opaque aggregate.
//
// Join key convention (Phase A): when a diagnostic concerns a code symbol, the
// child's evidence carries the moniker content identity `moniker:{scheme}:{id}`,
// the SAME id an LSIF consumer resolves. Receipt provenance and "go to definition"
// then share one OCEL object id.

use lsp_max_runtime::control_plane::receipts::{moniker_object_id, CryptographicReceipt};

/// One child server's contribution to a merged compositor verdict.
///
/// Binds a child's `CryptographicReceipt` chain link to the server identity that
/// produced it, plus the optional moniker join key when a code symbol is involved.
/// `consequence_hash` / `sequence` are read off the wrapped receipt so a verifier
/// can locate this exact link in the child's chain.
#[derive(Debug, Clone)]
pub struct ChildEvidence {
    /// Originating child server (the Λ_CD^(D) that classified this contribution).
    pub server_id: String,
    /// The child's cryptographic chain link attesting this contribution.
    pub receipt: CryptographicReceipt,
    /// Moniker join key `moniker:{scheme}:{identifier}` when a code symbol is
    /// involved; `None` for whole-file / non-symbol diagnostics.
    pub symbol_object_id: Option<String>,
    /// Whether this child contributed an ANDON (REFUSED_BY_LAW Error) diagnostic.
    pub has_andon_contribution: bool,
}

impl ChildEvidence {
    /// Build evidence for a child contribution NOT tied to a specific code symbol.
    pub fn new(
        server_id: impl Into<String>,
        receipt: CryptographicReceipt,
        has_andon_contribution: bool,
    ) -> Self {
        Self {
            server_id: server_id.into(),
            receipt,
            symbol_object_id: None,
            has_andon_contribution,
        }
    }

    /// Build evidence tied to a code symbol by its moniker content identity.
    /// `symbol_object_id` becomes the Phase-A join key `moniker:{scheme}:{id}`.
    pub fn for_symbol(
        server_id: impl Into<String>,
        receipt: CryptographicReceipt,
        scheme: &str,
        identifier: &str,
        has_andon_contribution: bool,
    ) -> Self {
        Self {
            server_id: server_id.into(),
            receipt,
            symbol_object_id: Some(moniker_object_id(scheme, identifier)),
            has_andon_contribution,
        }
    }

    /// Stable per-child OCEL object id for this child's chain.
    pub fn chain_object_id(&self) -> String {
        format!("child_chain_{}", self.server_id)
    }

    /// Export this child contribution as an OCEL 2.0 event that reuses the child's
    /// `CryptographicReceipt` provenance and relates to the merged verdict and (when
    /// present) the produced code symbol. Reusing the runtime exporter keeps a single
    /// authority for the receipt → OCEL projection.
    pub fn to_ocel_event(
        &self,
        event_id: &str,
        timestamp: &str,
        merge_object_id: &str,
    ) -> serde_json::Value {
        // Reuse the runtime exporter for the base event; the symbol relationship is
        // attached below from the stored moniker join key (no duplicate scheme/id fields).
        let mut event = self.receipt.to_ocel_event(event_id, timestamp);
        if let Some(rels) = event
            .get_mut("relationships")
            .and_then(|r| r.as_array_mut())
        {
            rels.push(serde_json::json!({
                "objectId": self.chain_object_id(),
                "qualifier": "speciated_chain"
            }));
            rels.push(serde_json::json!({
                "objectId": merge_object_id,
                "qualifier": "contributes_to_merge"
            }));
            if let Some(sym) = &self.symbol_object_id {
                rels.push(serde_json::json!({
                    "objectId": sym,
                    "qualifier": "produced_symbol"
                }));
            }
        }
        event
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lsp_max_runtime::control_plane::receipts::{Blake3Hash, Keystore};
    use uuid::Uuid;

    fn sample_receipt(seq: u64) -> CryptographicReceipt {
        let ks = Keystore::from_seed(&[7u8; 32]);
        let mut r = CryptographicReceipt {
            prev_hash: Blake3Hash([0u8; 32]),
            discipline_id: Uuid::nil(),
            law_id: Uuid::nil(),
            consequence_hash: Blake3Hash([1u8; 32]),
            sequence: seq,
            signature: [0u8; 64],
        };
        ks.sign_receipt(&mut r);
        r
    }

    #[test]
    fn child_evidence_carries_server_identity() {
        let ev = ChildEvidence::new("wasm4pm-lsp", sample_receipt(3), true);
        assert_eq!(ev.server_id, "wasm4pm-lsp");
        assert!(ev.has_andon_contribution);
        assert_eq!(ev.chain_object_id(), "child_chain_wasm4pm-lsp");
    }

    #[test]
    fn for_symbol_uses_moniker_join_key() {
        let ev = ChildEvidence::for_symbol(
            "ggen-lsp",
            sample_receipt(1),
            "rust-analyzer",
            "crate::merge::MergeContext",
            false,
        );
        assert_eq!(
            ev.symbol_object_id.as_deref(),
            Some("moniker:rust-analyzer:crate::merge::MergeContext")
        );
    }

    #[test]
    fn ocel_event_relates_to_merge_and_symbol() {
        let ev =
            ChildEvidence::for_symbol("ggen-lsp", sample_receipt(1), "rust-analyzer", "sym", true);
        let event = ev.to_ocel_event("evt-1", "2026-06-13T00:00:00Z", "merge_file_x");
        let rels = event["relationships"].as_array().unwrap();
        let quals: Vec<&str> = rels
            .iter()
            .map(|r| r["qualifier"].as_str().unwrap())
            .collect();
        assert!(quals.contains(&"speciated_chain"));
        assert!(quals.contains(&"contributes_to_merge"));
        assert!(quals.contains(&"produced_symbol"));
    }
}
