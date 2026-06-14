//! # Cross-product: receipt verification drives the conformance gate
//!
//! This example is **composition** — it shows two capabilities agreeing, which no
//! single-capability example demonstrates:
//!
//! - `Receipt` (content-addressed BLAKE3 digest) — see
//!   `examples/receipt_chain_explained.rs`
//! - `ConformanceVector` (three-valued Admitted/Refused/Unknown gate) — see
//!   `examples/conformance_vector_explained.rs`
//!
//! The admission model composes like this: the `Receipt` law axis starts in
//! `unknown` (no evidence yet). Verifying a receipt against its artifact is the
//! *evidence* that moves that axis — a passing check admits it, a failed check
//! refuses it. The gate (`admits_release`) then reflects the outcome. The point is
//! coherence: the receipt capability *feeds* the conformance capability, and a
//! broken artifact propagates all the way to a blocked release.
//!
//! ```text
//! artifact bytes ──blake3──> Receipt.hash
//!                               │  verify(file, receipt)
//!        ┌──────────────────────┼───────────────────────┐
//!     verified=true          (unknown)              verified=false
//!        │                       │                       │
//!   Receipt → admitted     Receipt stays unknown   Receipt → refused
//!        │                       │                       │
//!  admits_release()=true   admits_release()=false  admits_release()=false
//! ```
//!
//! Run it:  cargo run --example admission_pipeline
//!
//! This example FAILS (panics, non-zero exit) if a tampered artifact ever admits
//! release, or if an unverified (unknown) receipt admits release under strict mode.

use lsp_max::max_protocol::{ConformanceVector, LawAxis, Receipt};
use std::fs;
use std::path::Path;

/// Write the artifact and produce a receipt whose digest covers the final bytes.
fn write_with_receipt(path: &Path, content: &str) -> Receipt {
    let hash = blake3::hash(content.as_bytes()).to_hex().to_string();
    fs::write(path, content).expect("write artifact");
    Receipt {
        receipt_id: "rcpt-admission".to_string(),
        hash,
        prev_receipt_hash: None,
    }
}

/// Re-hash the file on disk and compare to the receipt digest.
fn verify(path: &Path, receipt: &Receipt) -> bool {
    let bytes = fs::read(path).expect("read artifact");
    blake3::hash(&bytes).to_hex().to_string() == receipt.hash
}

/// Build the gate state. `Protocol`/`Type` are already admitted; the `Receipt`
/// axis is resolved out of `unknown` by the verification outcome (None = not yet
/// checked, so it stays unknown).
fn gate(receipt_verified: Option<bool>) -> ConformanceVector {
    let mut admitted = vec![LawAxis::Protocol, LawAxis::Type];
    let mut refused = vec![];
    let mut unknown = vec![];
    match receipt_verified {
        Some(true) => admitted.push(LawAxis::Receipt),
        Some(false) => refused.push(LawAxis::Receipt),
        None => unknown.push(LawAxis::Receipt),
    }
    let mut cv = ConformanceVector {
        admitted,
        refused,
        unknown,
        strict_mode: true,
        ..Default::default()
    };
    cv.sync_bits_from_vecs();
    cv
}

fn main() {
    let dir = tempfile::tempdir().expect("temp dir");
    let artifact = dir.path().join("release.json");
    let receipt = write_with_receipt(&artifact, r#"{"release":"26.6.9"}"#);

    // [A] Not yet checked: the Receipt axis is unknown. Under strict mode the gate
    //     refuses to admit release — unknown is not optimistically treated as passed.
    let before = gate(None);
    assert!(
        !before.all_admitted(),
        "unknown Receipt axis ⇒ not fully admitted"
    );
    assert!(
        !before.admits_release(),
        "strict gate must BLOCK release while the receipt is unverified (unknown)"
    );

    // [B] Verify the intact artifact ⇒ evidence admits the Receipt axis ⇒ the gate
    //     now admits release. This is the receipt capability feeding the gate.
    let verified = verify(&artifact, &receipt);
    assert!(verified, "intact artifact must verify");
    let admitted_gate = gate(Some(verified));
    assert!(admitted_gate.all_admitted(), "all axes resolved-admitted");
    assert!(
        admitted_gate.admits_release(),
        "verified receipt ⇒ gate admits release"
    );

    // [C] Tamper the artifact after the receipt was written ⇒ verification fails ⇒
    //     the Receipt axis is refused ⇒ the gate blocks release. The broken artifact
    //     propagates end-to-end: a fake admission cannot pass this composition.
    fs::write(&artifact, r#"{"release":"TAMPERED"}"#).expect("overwrite");
    let reverified = verify(&artifact, &receipt);
    assert!(!reverified, "tampered artifact must fail verification");
    let refused_gate = gate(Some(reverified));
    assert!(
        !refused_gate.admits_release(),
        "tampered receipt ⇒ refused axis ⇒ gate BLOCKS release"
    );

    println!("WITNESS admission_pipeline: receipt verification drives the gate");
    println!("  [A] unverified receipt (unknown)  → admits_release = false (strict blocks)");
    println!("  [B] verified intact receipt       → admits_release = true");
    println!("  [C] tampered receipt (refused)    → admits_release = false");
    println!();
    println!("Coherence shown: the Receipt capability feeds the ConformanceVector gate,");
    println!("so a tampered artifact cannot launder its way to an admitted release.");
}
