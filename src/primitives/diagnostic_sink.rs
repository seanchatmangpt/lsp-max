//! Last-value deduplicating diagnostic publisher.
//!
//! # Invariant: Context-free rules (zero curvature)
//!
//! Rules emitting diagnostics through this sink MUST be context-free: their
//! output must depend only on the current document content, never on edit
//! history or version sequence.
//!
//! If a rule's output depends on edit history, the hash-based dedup is unsound —
//! identical content at different versions may hash identically while representing
//! semantically different diagnostic states. There is no runtime check for this
//! invariant; it is the caller's obligation.
//!
//! Formally: the diagnostic section is a covariantly constant section of the
//! fiber bundle (URI → `Vec<Diagnostic>`) iff the curvature of the connection
//! (version counter) is zero, which holds iff rules are context-free.

use lsp_max_protocol::{LawAxis, MaxDiagnostic};
use lsp_types_max::{Diagnostic, Uri as Url};
use parking_lot::RwLock;
use rustc_hash::FxHashMap;
use std::sync::Arc;

use crate::service::Client;

/// Wraps `Client::publish_diagnostics` with last-value deduplication.
///
/// Publishing is skipped when the new diagnostic set hashes identically to the
/// last published set, eliminating redundant LSP notifications on unchanged
/// files. The stored state is a `u64` FNV-1a content hash — not the full
/// `Vec<Diagnostic>` — so no clone of the diagnostic vector is required on the
/// fast path (no change detected).
///
/// Clone is O(1): all clones share the same last-hash map via `Arc`.
#[derive(Clone, Debug)]
pub struct DiagnosticSink {
    client: Client,
    /// `(axis_bits, fnv_hash)` — axis_bits is 0 for plain-LSP publishes.
    last: Arc<RwLock<FxHashMap<Url, (u64, u64)>>>,
}

impl DiagnosticSink {
    /// Wraps `client` with deduplication tracking.
    pub fn new(client: Client) -> Self {
        Self {
            client,
            last: Arc::new(RwLock::new(FxHashMap::default())),
        }
    }

    /// Publish `diags` for `uri`.  No-ops if content hash is unchanged.
    ///
    /// The fast path (no change) reads one `u64` and returns — no clone,
    /// no allocation, no LSP notification.
    pub async fn publish(&self, uri: Url, diags: Vec<Diagnostic>) {
        let hash = hash_diagnostics(&diags);
        {
            let last = self.last.read();
            if last.get(&uri).map(|(_, fnv)| *fnv) == Some(hash) {
                return;
            }
        }
        self.last.write().insert(uri.clone(), (0u64, hash));
        self.client.publish_diagnostics(uri, diags, None).await;
    }

    /// Publish a batch of `MaxDiagnostic`s with hybrid axis-bitmask + FNV-1a dedup.
    ///
    /// The fast path checks whether the law-axis bit set AND the FNV-1a hash of
    /// the underlying LSP diagnostics are both unchanged. Axis-set changes (most
    /// common in law-state transitions) are detected by a single `u64` comparison.
    /// Custom-axis diagnostics have no stable bit and fall through to FNV-1a only.
    ///
    /// No wire message is emitted when both checks agree the set is unchanged.
    pub async fn publish_max(&self, uri: Url, diags: Vec<MaxDiagnostic>) {
        let curr_bits = diagnostics_to_axis_bits(&diags);
        let lsp_diags: Vec<Diagnostic> = diags.iter().map(|d| d.lsp.clone()).collect();
        let curr_fnv = hash_diagnostics(&lsp_diags);
        {
            let last = self.last.read();
            if let Some(&(prev_bits, prev_fnv)) = last.get(&uri) {
                if prev_bits == curr_bits && prev_fnv == curr_fnv {
                    return;
                }
            }
        }
        self.last.write().insert(uri.clone(), (curr_bits, curr_fnv));
        self.client.publish_diagnostics(uri, lsp_diags, None).await;
    }

    /// Clear diagnostics for `uri`.  No-ops if already empty.
    pub async fn clear(&self, uri: &Url) {
        let was_present = self.last.write().remove(uri).is_some();
        if was_present {
            self.client
                .publish_diagnostics(uri.clone(), vec![], None)
                .await;
        }
    }

    /// Returns the FNV-1a hash of the last published diagnostic set, or `None`
    /// if the URI has never been published to (or was cleared).
    pub fn last_published(&self, uri: &Url) -> Option<u64> {
        self.last.read().get(uri).map(|(_, fnv)| *fnv)
    }
}

/// Map a `LawAxis` to its bit position (bits 0–10 for named variants; `None` for Custom).
fn law_axis_bit(axis: &LawAxis) -> Option<u64> {
    match axis {
        LawAxis::Protocol => Some(1 << 0),
        LawAxis::Type => Some(1 << 1),
        LawAxis::Fixture => Some(1 << 2),
        LawAxis::Documentation => Some(1 << 3),
        LawAxis::Release => Some(1 << 4),
        LawAxis::Hook => Some(1 << 5),
        LawAxis::Repair => Some(1 << 6),
        LawAxis::Receipt => Some(1 << 7),
        LawAxis::Security => Some(1 << 8),
        LawAxis::Autopoiesis => Some(1 << 9),
        LawAxis::Domain => Some(1 << 10),
        LawAxis::Custom(_) => None,
    }
}

/// Fold all named `LawAxis` bits from a `MaxDiagnostic` slice into a `u64` bitmask.
fn diagnostics_to_axis_bits(diags: &[MaxDiagnostic]) -> u64 {
    diags
        .iter()
        .filter_map(|d| law_axis_bit(&d.law_axis))
        .fold(0u64, |acc, bit| acc | bit)
}

/// Structural FNV-1a hash over a `Vec<Diagnostic>` — allocation-free.
///
/// Hashes the fields that determine diagnostic identity: message, severity,
/// range, and source. Tags and related_information are deliberately excluded
/// as they don't affect the visible diagnostic set in most editors.
///
/// This is the Boolean-algebra projection step: the diagnostic Vec is mapped
/// to an element of ℤ/2^64ℤ. Two sets with identical hash are treated as
/// identical for dedup purposes (negligible collision probability in practice).
fn hash_diagnostics(diags: &[Diagnostic]) -> u64 {
    const OFFSET: u64 = 0xcbf29ce484222325;
    const PRIME: u64 = 0x100000001b3;

    let mut h = OFFSET;
    // Mix in count so that empty != non-empty even if all fields hash to 0.
    h = (h ^ diags.len() as u64).wrapping_mul(PRIME);

    for d in diags {
        // message
        for b in d.message.bytes() {
            h = (h ^ b as u64).wrapping_mul(PRIME);
        }
        // severity
        h = (h ^ d
            .severity
            .map(|s| match s {
                lsp_types_max::DiagnosticSeverity::ERROR => 1u64,
                lsp_types_max::DiagnosticSeverity::WARNING => 2,
                lsp_types_max::DiagnosticSeverity::INFORMATION => 3,
                lsp_types_max::DiagnosticSeverity::HINT => 4,
                _ => 0,
            })
            .unwrap_or(0xff))
        .wrapping_mul(PRIME);
        // range — start
        h = (h ^ d.range.start.line as u64).wrapping_mul(PRIME);
        h = (h ^ d.range.start.character as u64).wrapping_mul(PRIME);
        // range — end
        h = (h ^ d.range.end.line as u64).wrapping_mul(PRIME);
        h = (h ^ d.range.end.character as u64).wrapping_mul(PRIME);
        // source (optional)
        if let Some(src) = &d.source {
            for b in src.bytes() {
                h = (h ^ b as u64).wrapping_mul(PRIME);
            }
        }
        // separator between diagnostics
        h = h.wrapping_mul(PRIME);
    }
    h
}
