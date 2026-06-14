// WITNESS (rule 2): "why is this URI in ANDON" must resolve to a SPECIFIC child
// server's Λ_CD^(D) plus that child's evidence — NOT the workspace union.
//
// Co-located with the merge change it witnesses (RFC-B/RFC-C + strict per-server
// C_D isolation). Construction: server "alpha" declares prefix "ALPHA-"; server
// "beta" declares "BETA-". The union therefore contains BOTH. A code "BETA-001"
// emitted by server "alpha" must NOT be ANDON: alpha never declared "BETA-". If
// routing fell back to the union (union-fallback), alpha's diagnostic WOULD be
// ANDON because the union contains "BETA-" — and these assertions would fail. The
// test is thus constructed so that forcing union-fallback breaks it.

use super::*;

fn two_server_ctx() -> MergeContext {
    // The union deliberately contains BOTH prefixes (the workspace-wide superset),
    // exactly the condition under which a union-fallback bug would leak beta's
    // prefix onto alpha's diagnostic.
    let mut ctx = MergeContext::new(vec!["ALPHA-".to_string(), "BETA-".to_string()]);
    ctx.add_server_prefix_override("alpha".to_string(), vec!["ALPHA-".to_string()]);
    ctx.add_server_prefix_override("beta".to_string(), vec!["BETA-".to_string()]);
    ctx
}

#[test]
fn andon_attributes_to_specific_child_not_union() {
    let ctx = two_server_ctx();
    let route = ctx.attribute_andon("ALPHA-001", Some("alpha"));
    assert_eq!(
        route,
        AndonRoute::PerServer {
            server_id: "alpha".to_string()
        },
        "alpha's own prefix must attribute to alpha, not the union"
    );
}

#[test]
fn cross_server_prefix_does_not_leak_via_union() {
    let ctx = two_server_ctx();
    // beta's prefix carried by an alpha-sourced diagnostic. Under strict isolation
    // this is NOT ANDON. Under union-fallback it WOULD be (the union contains
    // "BETA-"), so this assertion fails if union-fallback is forced.
    let route = ctx.attribute_andon("BETA-001", Some("alpha"));
    assert_eq!(
        route,
        AndonRoute::NotAndon,
        "a prefix declared only by beta must not put alpha's diagnostic in ANDON"
    );
    assert!(!ctx.is_andon_for_server("BETA-001", Some("alpha")));
}

#[test]
fn missing_server_id_is_explicit_union_last_resort() {
    let ctx = two_server_ctx();
    // No server_id → the union is the ONLY available C_D, marked Union so the
    // attribution stays honest about which last-resort path classified it.
    let route = ctx.attribute_andon("BETA-001", None);
    assert_eq!(
        route,
        AndonRoute::Union,
        "no server_id must route through the explicit union last resort"
    );
}

#[test]
fn merged_verdict_traces_to_per_child_evidence() {
    use crate::receipt::CompositorReceipt;
    use crate::receipt_chain::ChildEvidence;
    use lsp_max_runtime::control_plane::receipts::{Blake3Hash, CryptographicReceipt, Keystore};
    use uuid::Uuid;

    let ctx = two_server_ctx();
    let inputs = vec![
        (
            ChildTier::Primary,
            vec![DiagnosticEntry {
                uri: "file:///x.rs".into(),
                line: 1,
                character: 0,
                severity: 1,
                code: "ALPHA-001".into(),
                message: "alpha law".into(),
                source_tier: ChildTier::Primary,
                server_id: Some("alpha".into()),
            }],
        ),
        (
            ChildTier::Secondary,
            vec![DiagnosticEntry {
                uri: "file:///x.rs".into(),
                line: 2,
                character: 0,
                severity: 1,
                code: "BETA-001".into(),
                message: "beta law".into(),
                source_tier: ChildTier::Secondary,
                server_id: Some("beta".into()),
            }],
        ),
    ];
    let result = ctx.merge(inputs);
    assert!(result.has_andon_block, "both servers' own laws are ANDON");

    // Per-child crypto evidence (reusing CryptographicReceipt, not forked).
    let ks = Keystore::from_seed(&[9u8; 32]);
    let mk = |sid: &str, seq: u64| {
        let mut r = CryptographicReceipt {
            prev_hash: Blake3Hash([0u8; 32]),
            discipline_id: Uuid::nil(),
            law_id: Uuid::nil(),
            consequence_hash: Blake3Hash([seq as u8; 32]),
            sequence: seq,
            signature: [0u8; 64],
        };
        ks.sign_receipt(&mut r);
        ChildEvidence::new(sid.to_string(), r, true)
    };
    let receipt = CompositorReceipt::new(
        "file:///x.rs".into(),
        &result,
        &["ALPHA-".to_string(), "BETA-".to_string()],
    )
    .with_child_evidence(vec![mk("alpha", 1), mk("beta", 1)]);

    let traced: Vec<&str> = receipt
        .child_evidence
        .iter()
        .map(|e| e.server_id.as_str())
        .collect();
    assert!(traced.contains(&"alpha"));
    assert!(traced.contains(&"beta"));

    // RFC-C: the flush is minable as an OCEL event whose relationships carry each
    // child's chain id.
    let event = receipt.to_ocel_event("flush-1", "2026-06-13T00:00:00Z");
    let rels = event["relationships"].as_array().unwrap();
    let obj_ids: Vec<&str> = rels
        .iter()
        .map(|r| r["objectId"].as_str().unwrap())
        .collect();
    assert!(obj_ids.contains(&"child_chain_alpha"));
    assert!(obj_ids.contains(&"child_chain_beta"));
}
