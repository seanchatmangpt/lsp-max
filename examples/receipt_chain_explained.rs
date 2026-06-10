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
//! lsp-max's `write_ocel_outputs` in `examples/anti-llm-lsp/src/ocel.rs`
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

// This file is intentionally a documentation-only example.
// It has no runnable code — the explanation is in the module doc above.
//
// To see a working receipt implementation, read:
//   examples/anti-llm-lsp/src/ocel.rs  (write_ocel_outputs)
//   lsp-max-protocol/src/receipt.rs    (Receipt type)

fn main() {
    println!("Receipt-chain explanation: see module-level doc comment above.");
    println!("For a working implementation: examples/anti-llm-lsp/src/ocel.rs");
}
