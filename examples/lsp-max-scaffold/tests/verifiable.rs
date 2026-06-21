//! Replay-Verifiable Diagnostics — threat model coverage.
//!
//! These tests prove the four tamper vectors are all REFUSED. They use a
//! neutral ruleset (a synthetic token) so the test corpus contains none of the
//! sensitive tokens the production analyzer hunts. They are NOT receipts —
//! test stdout is not evidence of admission.

use lsp_max_scaffold::analyzer::{DefaultAnalyzer, ReplayableAnalyzer, Rule};
use lsp_max_scaffold::law::AxisState;
use lsp_max_scaffold::verifiable::{
    genesis_head, verify_chain, verify_receipt, ChainVerdict, VerifiableEngine,
};

fn neutral() -> DefaultAnalyzer {
    DefaultAnalyzer::with_rules(
        "test-v1",
        vec![Rule::new(
            "TEST-001",
            vec!["BANANA".to_string()],
            "synthetic token",
        )],
    )
}

#[test]
fn honest_diagnostic_is_admitted() {
    let analyzer = neutral();
    let mut engine = VerifiableEngine::new(&analyzer);
    let diags = engine.extend("a BANANA here and BANANA there");
    assert_eq!(diags.len(), 2);
    for d in &diags {
        assert_eq!(
            verify_receipt(&d.receipt, &d.witness, &analyzer),
            AxisState::Admitted
        );
    }
}

#[test]
fn tampered_output_is_refused() {
    let analyzer = neutral();
    let mut engine = VerifiableEngine::new(&analyzer);
    let mut diags = engine.extend("x BANANA y");
    // Vector 2: alter the certified output digest.
    diags[0].receipt.output_digest = "deadbeef".to_string();
    assert_eq!(
        verify_receipt(&diags[0].receipt, &diags[0].witness, &analyzer),
        AxisState::Refused
    );
}

#[test]
fn tampered_witness_is_refused() {
    let analyzer = neutral();
    let mut engine = VerifiableEngine::new(&analyzer);
    let mut diags = engine.extend("x BANANA y");
    // Vector 1: flip the witness so it no longer matches the input digest.
    let mut bytes = diags[0].witness.snippet_hex.clone().into_bytes();
    let last = bytes.len() - 1;
    bytes[last] = if bytes[last] == b'0' { b'1' } else { b'0' };
    diags[0].witness.snippet_hex = String::from_utf8(bytes).unwrap();
    assert_eq!(
        verify_receipt(&diags[0].receipt, &diags[0].witness, &analyzer),
        AxisState::Refused
    );
}

#[test]
fn forged_finding_does_not_replay() {
    let analyzer = neutral();
    let mut engine = VerifiableEngine::new(&analyzer);
    let mut diags = engine.extend("x BANANA y");
    // Vector 3: keep the receipt but swap the witness for input that does not
    // actually trigger the rule. Replay finds nothing → REFUSED.
    use lsp_max_scaffold::verifiable::Witness;
    let clean = "harmless";
    let forged = Witness {
        doc_span: (0, clean.len()),
        snippet_hex: to_hex(clean.as_bytes()),
    };
    // Recompute the input digest so vector 1 is not what trips it — the failure
    // must come from replay non-reproduction, not the input-digest check.
    diags[0].receipt.input_digest = input_digest_for(&analyzer, clean);
    assert_eq!(
        verify_receipt(&diags[0].receipt, &forged, &analyzer),
        AxisState::Refused
    );
}

#[test]
fn intact_chain_is_admitted() {
    let analyzer = neutral();
    let mut engine = VerifiableEngine::new(&analyzer);
    let diags = engine.extend("BANANA BANANA BANANA");
    let receipts: Vec<_> = diags.iter().map(|d| d.receipt.clone()).collect();
    let verdict = verify_chain(&receipts);
    assert!(verdict.is_intact());
    if let ChainVerdict::Intact { head, len } = verdict {
        assert_eq!(len, 3);
        assert_eq!(head, engine.head());
    }
}

#[test]
fn reordered_chain_is_broken() {
    let analyzer = neutral();
    let mut engine = VerifiableEngine::new(&analyzer);
    let diags = engine.extend("BANANA BANANA BANANA");
    let mut receipts: Vec<_> = diags.iter().map(|d| d.receipt.clone()).collect();
    // Vector 4: reorder receipts; the hash linkage no longer holds.
    receipts.swap(0, 2);
    assert!(!verify_chain(&receipts).is_intact());
}

#[test]
fn dropped_receipt_breaks_chain() {
    let analyzer = neutral();
    let mut engine = VerifiableEngine::new(&analyzer);
    let diags = engine.extend("BANANA BANANA BANANA");
    let mut receipts: Vec<_> = diags.iter().map(|d| d.receipt.clone()).collect();
    receipts.remove(1);
    assert!(matches!(
        verify_chain(&receipts),
        ChainVerdict::BrokenLink { .. }
    ));
}

#[test]
fn analysis_is_deterministic() {
    let analyzer = neutral();
    let mut e1 = VerifiableEngine::new(&analyzer);
    let mut e2 = VerifiableEngine::new(&analyzer);
    let d1 = e1.extend("BANANA x BANANA");
    let d2 = e2.extend("BANANA x BANANA");
    assert_eq!(d1, d2, "identical input must yield identical proofs");
    assert_eq!(e1.head(), e2.head());
}

#[test]
fn empty_source_yields_genesis_head() {
    let analyzer = neutral();
    let mut engine = VerifiableEngine::new(&analyzer);
    let diags = engine.extend("nothing to see");
    assert!(diags.is_empty());
    assert_eq!(engine.head(), genesis_head());
}

#[test]
fn production_analyzer_detects_cardinal_laws() {
    // Build sensitive tokens at runtime so this test's source stays law-clean.
    let victory: String = "enod".chars().rev().collect();
    let fork = format!("{}-lsp", "tower");
    let source = format!("// {victory}\nuse {fork};\n");

    let analyzer = DefaultAnalyzer::new();
    let findings = analyzer.analyze(&source);
    let codes: Vec<_> = findings.iter().map(|f| f.code.as_str()).collect();
    assert!(codes.contains(&"RVD-VICTORY-001"));
    assert!(codes.contains(&"RVD-FORK-001"));
}

// --- helpers ---------------------------------------------------------------

fn to_hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut s = String::with_capacity(bytes.len() * 2);
    for &b in bytes {
        s.push(HEX[(b >> 4) as usize] as char);
        s.push(HEX[(b & 0x0f) as usize] as char);
    }
    s
}

/// Re-derive an input digest the way the engine does, for the forged-finding
/// test. Mirrors `verifiable::input_digest` (which is private).
fn input_digest_for(analyzer: &DefaultAnalyzer, snippet: &str) -> String {
    let mut h = blake3::Hasher::new();
    h.update(b"lsp-max-rvd/input/v1\n");
    h.update(analyzer.version().as_bytes());
    h.update(b"\n");
    h.update(analyzer.ruleset_digest().as_bytes());
    h.update(b"\n");
    h.update(snippet.as_bytes());
    h.finalize().to_hex().to_string()
}
