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
//
// `from_flush_contribution` (RFC-B wiring path): the compositor itself signs a chain
// link for each child's per-flush contribution. The child's own external receipt file
// is OPEN (not yet published by the child server); `from_flush_contribution` covers
// the compositor-observable side — `admitted_count` + `has_andon` are hashed into
// the `consequence_hash` so the link is attributable to THAT flush, not a generic
// stub. Full crypto (ed25519) is CANDIDATE; signing via compositor Keystore is wired.

use lsp_max_runtime::control_plane::receipts::{
    moniker_object_id, Blake3Hash, CryptographicReceipt, Keystore,
};
use uuid::Uuid;

/// Stable discipline UUID for compositor-originated chain links.
///
/// Chosen as the nil UUID namespace — all compositor-signed links share this id,
/// distinguishing them from child-server-originated receipts that carry a
/// server-specific discipline UUID. A verifier can identify compositor-signed
/// evidence by checking `receipt.discipline_id == compositor_discipline_id()`.
///
/// Using the nil UUID here is intentional: the compositor has not yet established
/// a persistent key identity (OPEN). When a persistent compositor Keystore is
/// introduced, this UUID should be replaced with a stable v5 UUID derived from the
/// compositor's public key.
pub fn compositor_discipline_id() -> Uuid {
    Uuid::nil()
}

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

    /// Build a compositor-signed chain link for a child's per-flush contribution.
    ///
    /// The compositor signs on the child's behalf using its own `Keystore` because
    /// the child has not yet published its own receipt file (that path is OPEN).
    /// `admitted_count` and `has_andon` are folded into the `consequence_hash` via
    /// BLAKE3 so the link is attributable to this specific flush, not a generic stub.
    /// `sequence` must be monotonically increasing per compositor Keystore lifetime.
    pub fn from_flush_contribution(
        server_id: impl Into<String>,
        admitted_count: usize,
        has_andon: bool,
        sequence: u64,
        prev_hash: Blake3Hash,
        keystore: &Keystore,
    ) -> Self {
        // Fold flush-time observables into consequence_hash so this link binds to
        // THAT flush: (server_id bytes) ++ admitted_count ++ has_andon flag.
        let server_id_str = server_id.into();
        let consequence_hash = {
            let mut h = blake3::Hasher::new();
            h.update(server_id_str.as_bytes());
            h.update(&admitted_count.to_le_bytes());
            h.update(&[u8::from(has_andon)]);
            h.update(&sequence.to_le_bytes());
            Blake3Hash(*h.finalize().as_bytes())
        };
        let mut receipt = CryptographicReceipt {
            prev_hash,
            discipline_id: compositor_discipline_id(),
            law_id: Uuid::nil(), // no single law governs the flush aggregate
            consequence_hash,
            sequence,
            signature: [0u8; 64],
        };
        keystore.sign_receipt(&mut receipt);
        Self {
            server_id: server_id_str,
            receipt,
            symbol_object_id: None,
            has_andon_contribution: has_andon,
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

    #[test]
    fn from_flush_contribution_carries_server_id_and_andon_state() {
        let ks = Keystore::from_seed(&[42u8; 32]);
        let prev = Blake3Hash([0u8; 32]);

        let ev =
            ChildEvidence::from_flush_contribution("anti-llm-cheat-lsp", 5, false, 1, prev, &ks);
        assert_eq!(ev.server_id, "anti-llm-cheat-lsp");
        assert!(!ev.has_andon_contribution);
        assert_eq!(ev.receipt.sequence, 1);
        assert_eq!(ev.receipt.discipline_id, compositor_discipline_id());
        // consequence_hash is non-zero — flush observables were folded in.
        assert_ne!(ev.receipt.consequence_hash.0, [0u8; 32]);
    }

    #[test]
    fn from_flush_contribution_andon_flag_changes_consequence_hash() {
        // Two otherwise identical flushes that differ only in `has_andon` must
        // produce distinct consequence_hashes so the chain links are distinguishable.
        let ks = Keystore::from_seed(&[13u8; 32]);
        let prev = Blake3Hash([0u8; 32]);
        let ev_clear =
            ChildEvidence::from_flush_contribution("wasm4pm-lsp", 3, false, 7, prev, &ks);
        let ev_andon =
            ChildEvidence::from_flush_contribution("wasm4pm-lsp", 3, true, 7, prev, &ks);
        assert_ne!(
            ev_clear.receipt.consequence_hash,
            ev_andon.receipt.consequence_hash
        );
    }
}
