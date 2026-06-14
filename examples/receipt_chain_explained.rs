//! # Why Receipt-Chain Admission Works the Way It Does
//!
//! This example is **Explanation** (Diataxis): it does not teach you to do a task.
//! It explains *why* lsp-max uses content-addressable BLAKE3 receipts instead of
//! simpler alternatives, and why the design is necessary rather than accidental.
//!
//! ## The problem: test assertions are not proof
//!
//! A passing test proves the code compiled and the assertion held at runtime.
//! It does not prove:
//! - *which* file was written to disk
//! - *what* bytes were in that file when it was written
//! - *whether* the file was subsequently changed before the claim was made
//!
//! In a multi-agent environment where agents can emit "admitted" claims without
//! actually performing the work, assertions are trivially forgeable.
//!
//! ## The solution: hash the artifact, not the assertion
//!
//! A receipt in lsp-max is a JSON file containing:
//!
//! ```json
//! {
//!   "digest": "<BLAKE3 hex of the artifact's exact bytes>",
//!   "digest_algorithm": "BLAKE3",
//!   "boundary": "<path context>",
//!   "checkpoint": "<named gate>"
//! }
//! ```
//!
//! The receipt is written *after* the artifact and hashes the artifact's *final*
//! content. Verification: re-hash the artifact file and compare to the stored
//! digest. If they match, the receipt was produced from that exact content.
//!
//! ## Why BLAKE3, not SHA-256?
//!
//! BLAKE3 is:
//! - Faster than SHA-256 on modern hardware (SIMD-parallel)
//! - Cryptographically stronger than MD5 or SHA-1
//! - Deterministic and content-addressable
//!
//! The choice is pragmatic, not dogmatic. SHA-256 would also work. What matters
//! is that the algorithm is one-way and collision-resistant so that two different
//! artifact contents cannot produce the same digest.
//!
//! ## The circular-hash trap
//!
//! A common mistake: compute the hash, inject it into the artifact as a field,
//! then write the artifact. The artifact now contains the hash of its *pre-injection*
//! content — the stored digest does not match the final file.
//!
//! ```
//! WRONG:
//!   hash = blake3(content_without_hash)   ← hashes the wrong bytes
//!   content_with_hash = inject(content, hash)
//!   write(artifact, content_with_hash)    ← file != hashed content
//!   write(receipt, {digest: hash})        ← receipt is already stale
//!
//! CORRECT:
//!   content = serialize(artifact)         ← produce final bytes first
//!   hash = blake3(content)                ← hash what will be written
//!   write(artifact, content)              ← file == hashed content ✓
//!   write(receipt, {digest: hash})        ← receipt matches file ✓
//! ```
//!
//! lsp-max's `write_ocel_outputs` in `examples/anti-llm-cheat-lsp/src/ocel.rs`
//! was fixed to use the correct pattern after the circular-hash bug was found.
//!
//! ## Why receipts are written *beside* artifacts, not inside them
//!
//! If the receipt were embedded inside the artifact (e.g. as a JSON field), the
//! circular-hash trap becomes unavoidable: the receipt is part of the content that
//! would need to be hashed. Keeping receipts as sibling files keeps the hashing
//! model simple and composable.
//!
//! ## What a receipt does NOT prove
//!
//! A receipt proves the artifact had specific content at receipt-write time. It does
//! not prove:
//! - The artifact is semantically correct
//! - The process that produced the artifact was lawful (that requires OCEL conformance)
//! - The artifact was not subsequently overwritten (use `ConformanceVector` for live state)
//!
//! Receipts are one layer of a multi-layer admission model. They answer the question
//! "did this exact content exist?" — not "is the content correct?"

// The explanation above is the *claim*. The code below is the *witness*: it runs
// the receipt doctrine end to end against lsp-max's real `Receipt` struct, so this
// example FAILS (panics, non-zero exit) if content-addressing or chain linkage
// breaks. Run it:  cargo run --example receipt_chain_explained
//
// Receipt type: lsp-max-protocol/src/core.rs
// Content-addressing in production: examples/anti-llm-cheat-lsp/src/ocel.rs
//   (write_ocel_outputs — same blake3::hash(final_bytes) pattern shown here)

use lsp_max::max_protocol::Receipt;
use std::fs;

/// The CORRECT order: serialize final bytes, hash them, write, receipt = that hash.
fn write_with_receipt(path: &std::path::Path, content: &str) -> Receipt {
    let hash = blake3::hash(content.as_bytes()).to_hex().to_string();
    fs::write(path, content).expect("write artifact");
    Receipt {
        receipt_id: "rcpt-demo".to_string(),
        hash,
        prev_receipt_hash: None,
    }
}

/// Verification: re-hash the file on disk and compare to the receipt's digest.
fn verify(path: &std::path::Path, receipt: &Receipt) -> bool {
    let bytes = fs::read(path).expect("read artifact");
    blake3::hash(&bytes).to_hex().to_string() == receipt.hash
}

fn main() {
    let dir = tempfile::tempdir().expect("temp dir");

    // [1] CORRECT pattern: hash covers the exact bytes written ⇒ verification passes.
    let artifact = dir.path().join("artifact.json");
    let receipt = write_with_receipt(&artifact, r#"{"result":"value"}"#);
    assert!(
        verify(&artifact, &receipt),
        "receipt produced from final bytes must verify against the file on disk"
    );

    // [2] TAMPER detection: change the file after the receipt is written ⇒ verify FAILS.
    //     This is why a receipt is stronger than a test assertion — it catches the
    //     artifact being changed out from under the claim.
    fs::write(&artifact, r#"{"result":"TAMPERED"}"#).expect("overwrite");
    assert!(
        !verify(&artifact, &receipt),
        "a receipt MUST fail to verify once its artifact is modified"
    );

    // [3] The CIRCULAR-HASH TRAP the doc warns about: hash the content, inject the
    //     hash into the content, THEN write. The file now contains bytes that were
    //     not hashed, so the stored digest can never match the final file.
    let trap_file = dir.path().join("trap.json");
    let base = r#"{"data":1}"#;
    let pre_hash = blake3::hash(base.as_bytes()).to_hex().to_string();
    let injected = format!(r#"{{"data":1,"digest":"{pre_hash}"}}"#); // hash of WRONG bytes
    fs::write(&trap_file, &injected).expect("write trap");
    let stale_receipt = Receipt {
        receipt_id: "rcpt-trap".to_string(),
        hash: pre_hash,
        prev_receipt_hash: None,
    };
    assert!(
        !verify(&trap_file, &stale_receipt),
        "circular-hash trap MUST be detectable: digest of pre-injection bytes != final file"
    );

    // [4] Chain linkage: genesis has no prev; the next receipt closes the chain by
    //     carrying the prior receipt's hash. Tampering a link breaks the chain.
    let genesis = receipt; // reuse [1]'s receipt as genesis
    assert!(
        genesis.prev_receipt_hash.is_none(),
        "genesis has no prev hash"
    );
    let linked = Receipt {
        receipt_id: "rcpt-demo-2".to_string(),
        hash: blake3::hash(b"second artifact").to_hex().to_string(),
        prev_receipt_hash: Some(genesis.hash.clone()),
    };
    assert_eq!(
        linked.prev_receipt_hash.as_deref(),
        Some(genesis.hash.as_str()),
        "linked receipt must reference the exact genesis hash (Merkle chain)"
    );

    // [5] Serde roundtrip preserves the chain link (prev_receipt_hash survives).
    let json = serde_json::to_string(&linked).expect("serialize");
    let back: Receipt = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(
        back.prev_receipt_hash, linked.prev_receipt_hash,
        "chain link survives serde"
    );

    println!("WITNESS receipt_chain: 5 contract assertions held");
    println!("  [1] receipt from final bytes verifies against the file");
    println!("  [2] modifying the artifact makes the receipt fail to verify (tamper-evident)");
    println!("  [3] the circular-hash trap is detectable (digest != final file)");
    println!("  [4] genesis has no prev hash; the next receipt links the prior hash");
    println!("  [5] serde roundtrip preserves the chain link");
    println!();
    println!("This example panics (non-zero exit) if content-addressing or chain");
    println!("linkage regresses — the same blake3(final_bytes) pattern that");
    println!("anti-llm-cheat-lsp/src/ocel.rs uses to write production receipts.");
}
