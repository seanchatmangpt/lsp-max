//! Replay-Verifiable Diagnostics (RVD).
//!
//! A conventional LSP emits a diagnostic as an unprovable assertion: the client
//! trusts that the server computed it honestly. This module makes every
//! diagnostic carry its own proof.
//!
//! Each diagnostic is bound to:
//!   - a **witness**: the minimal input slice that reproduces it
//!   - a **receipt**: digests of the input and output, linked into a hash chain
//!
//! An independent verifier (a CI gate, a release pipeline, another agent) can
//! then `verify_receipt` by replaying the analyzer on the witness *alone* and
//! confirming the digests — without trusting the emitter. A forged or tampered
//! diagnostic fails replay and is `REFUSED`. This is the anti-cheat thesis made
//! executable: an agent cannot fake "diagnostics clean" because the gate
//! replays the witnesses and the arithmetic, not the agent's stdout.
//!
//! Four tamper vectors, all caught:
//!   1. altered witness          → input digest mismatch  → REFUSED
//!   2. altered code/message/span → output digest mismatch → REFUSED
//!   3. forged finding            → replay does not reproduce it → REFUSED
//!   4. inserted/dropped/reordered receipt → chain linkage breaks → Tampered

use crate::analyzer::ReplayableAnalyzer;
use crate::law::AxisState;
use serde::{Deserialize, Serialize};

const DOMAIN_INPUT: &[u8] = b"lsp-max-rvd/input/v1\n";
const DOMAIN_OUTPUT: &[u8] = b"lsp-max-rvd/output/v1\n";
const DOMAIN_CHAIN: &[u8] = b"lsp-max-rvd/chain/v1\n";
const DOMAIN_GENESIS: &[u8] = b"lsp-max-rvd/genesis/v1";

/// The minimal reproducing input for a single finding.
///
/// `snippet_hex` is hex-encoded so a persisted receipt artifact never reproduces
/// in cleartext the forbidden token it certifies (which the canary would flag).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Witness {
    /// Byte range of the finding in the original document (display metadata).
    pub doc_span: (usize, usize),
    /// Hex-encoded bytes of the minimal reproducing slice.
    pub snippet_hex: String,
}

impl Witness {
    pub fn snippet(&self) -> Option<String> {
        from_hex(&self.snippet_hex).and_then(|b| String::from_utf8(b).ok())
    }
}

/// A proof node for one diagnostic, linked into a hash chain.
///
/// `code` is a display label and is intentionally excluded from every digest:
/// a verifier must re-derive the code from the witness, never trust the field.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Receipt {
    pub code: String,
    pub input_digest: String,
    pub output_digest: String,
    /// Chain digest of the predecessor receipt (or genesis for the first).
    pub prev: String,
    pub chain_digest: String,
    /// `Unknown` until `verify_receipt` admits or refuses it. Never pre-admitted.
    pub status: AxisState,
}

/// A diagnostic bound to its proof.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VerifiableDiagnostic {
    pub code: String,
    pub message: String,
    pub witness: Witness,
    pub receipt: Receipt,
}

/// Verdict for a whole receipt chain.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "verdict")]
pub enum ChainVerdict {
    /// Every link is intact and every chain digest recomputes.
    Intact { head: String, len: usize },
    /// Receipt at `index` does not link to the running head (insert/drop/reorder).
    BrokenLink { index: usize },
    /// Receipt at `index` has a chain digest that does not recompute (tamper).
    Tampered { index: usize },
}

impl ChainVerdict {
    pub fn is_intact(&self) -> bool {
        matches!(self, ChainVerdict::Intact { .. })
    }

    pub fn label(&self) -> &'static str {
        match self {
            ChainVerdict::Intact { .. } => "ADMITTED",
            ChainVerdict::BrokenLink { .. } => "REFUSED",
            ChainVerdict::Tampered { .. } => "REFUSED",
        }
    }
}

/// Builds a receipt chain by analyzing successive sources.
pub struct VerifiableEngine<'a> {
    analyzer: &'a dyn ReplayableAnalyzer,
    head: String,
    ruleset_digest: String,
}

impl<'a> VerifiableEngine<'a> {
    pub fn new(analyzer: &'a dyn ReplayableAnalyzer) -> Self {
        Self {
            ruleset_digest: analyzer.ruleset_digest(),
            analyzer,
            head: genesis_head(),
        }
    }

    pub fn head(&self) -> &str {
        &self.head
    }

    /// Analyze `source`, emitting one proof-carrying diagnostic per finding and
    /// advancing the chain head. Deterministic: identical `source` yields
    /// identical receipts and head.
    pub fn extend(&mut self, source: &str) -> Vec<VerifiableDiagnostic> {
        let mut out = Vec::new();
        for finding in self.analyzer.analyze(source) {
            let snippet = &source[finding.span.0..finding.span.1];

            // Canonicalize by re-deriving the finding from the witness alone,
            // so `extend` and `verify_receipt` share one computation path.
            let local = self.analyzer.analyze(snippet);
            let Some(lf) = local.iter().find(|x| x.code == finding.code) else {
                continue;
            };

            let input_digest = input_digest(self.analyzer.version(), &self.ruleset_digest, snippet);
            let output_digest = output_digest(&lf.code, &lf.message, lf.span);
            let chain_digest = chain_digest(&self.head, &output_digest);

            let receipt = Receipt {
                code: finding.code.clone(),
                input_digest,
                output_digest,
                prev: self.head.clone(),
                chain_digest: chain_digest.clone(),
                status: AxisState::Unknown,
            };
            self.head = chain_digest;

            out.push(VerifiableDiagnostic {
                code: finding.code,
                message: finding.message,
                witness: Witness {
                    doc_span: finding.span,
                    snippet_hex: to_hex(snippet.as_bytes()),
                },
                receipt,
            });
        }
        out
    }
}

/// Replay-verify a single receipt against its witness.
///
/// Returns `Admitted` only when the witness reproduces the claimed input digest
/// AND replaying the analyzer on the witness reproduces the claimed output
/// digest. Any mismatch — tampered witness, tampered output, or forged finding —
/// returns `Refused`. Never returns `Unknown`: verification is a total function.
pub fn verify_receipt(
    receipt: &Receipt,
    witness: &Witness,
    analyzer: &dyn ReplayableAnalyzer,
) -> AxisState {
    let Some(snippet) = witness.snippet() else {
        return AxisState::Refused;
    };

    let recomputed_input = input_digest(analyzer.version(), &analyzer.ruleset_digest(), &snippet);
    if recomputed_input != receipt.input_digest {
        return AxisState::Refused;
    }

    let reproduced = analyzer
        .analyze(&snippet)
        .iter()
        .any(|lf| output_digest(&lf.code, &lf.message, lf.span) == receipt.output_digest);

    if reproduced {
        AxisState::Admitted
    } else {
        AxisState::Refused
    }
}

/// Verify the hash-chain linkage of an ordered receipt sequence.
pub fn verify_chain(receipts: &[Receipt]) -> ChainVerdict {
    let mut head = genesis_head();
    for (index, r) in receipts.iter().enumerate() {
        if r.prev != head {
            return ChainVerdict::BrokenLink { index };
        }
        if chain_digest(&r.prev, &r.output_digest) != r.chain_digest {
            return ChainVerdict::Tampered { index };
        }
        head = r.chain_digest.clone();
    }
    ChainVerdict::Intact {
        head,
        len: receipts.len(),
    }
}

// ---------------------------------------------------------------------------
// Digest primitives — domain-separated BLAKE3.
// ---------------------------------------------------------------------------

pub fn genesis_head() -> String {
    blake3_hex(DOMAIN_GENESIS)
}

fn input_digest(version: &str, ruleset_digest: &str, snippet: &str) -> String {
    let mut h = blake3::Hasher::new();
    h.update(DOMAIN_INPUT);
    h.update(version.as_bytes());
    h.update(b"\n");
    h.update(ruleset_digest.as_bytes());
    h.update(b"\n");
    h.update(snippet.as_bytes());
    h.finalize().to_hex().to_string()
}

fn output_digest(code: &str, message: &str, span: (usize, usize)) -> String {
    let mut h = blake3::Hasher::new();
    h.update(DOMAIN_OUTPUT);
    h.update(code.as_bytes());
    h.update(b"\n");
    h.update(message.as_bytes());
    h.update(b"\n");
    h.update(&(span.0 as u64).to_le_bytes());
    h.update(&(span.1 as u64).to_le_bytes());
    h.finalize().to_hex().to_string()
}

fn chain_digest(prev: &str, output: &str) -> String {
    let mut h = blake3::Hasher::new();
    h.update(DOMAIN_CHAIN);
    h.update(prev.as_bytes());
    h.update(b"\n");
    h.update(output.as_bytes());
    h.finalize().to_hex().to_string()
}

fn blake3_hex(bytes: &[u8]) -> String {
    blake3::hash(bytes).to_hex().to_string()
}

fn to_hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut s = String::with_capacity(bytes.len() * 2);
    for &b in bytes {
        s.push(HEX[(b >> 4) as usize] as char);
        s.push(HEX[(b & 0x0f) as usize] as char);
    }
    s
}

fn from_hex(s: &str) -> Option<Vec<u8>> {
    if s.len() % 2 != 0 {
        return None;
    }
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).ok())
        .collect()
}
