// Per-URI diagnostic staging area.
// Child servers deposit diagnostics here via deposit().
// flush() calls MergeContext::merge() and returns the MergeResult.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use dashmap::DashMap;

use crate::merge::{DiagnosticEntry, MergeContext, MergeResult};
use crate::registry::ChildTier;

/// Per-URI diagnostic staging area.
/// Child servers deposit diagnostics here via deposit().
/// flush() calls MergeContext::merge() and returns the MergeResult.
pub struct DiagnosticBuffer {
    /// Keyed by URI. Value is Vec of (server_id, tier, entries) tuples.
    inner: DashMap<String, Vec<(String, ChildTier, Vec<DiagnosticEntry>)>>,
    ctx: Arc<MergeContext>,
    /// Advisory flag: set to true when any flush produces has_andon_block == true.
    /// Monotonic — once set, stays true until restart. Use compositor_state() for
    /// authoritative current state.
    last_andon_block: AtomicBool,
}

impl DiagnosticBuffer {
    pub fn new(ctx: Arc<MergeContext>) -> Self {
        Self {
            inner: DashMap::new(),
            ctx,
            last_andon_block: AtomicBool::new(false),
        }
    }

    /// Record diagnostics from a child server for a URI.
    /// Replaces any previous entries from the same server_id for that URI.
    pub fn deposit(
        &self,
        uri: &str,
        server_id: &str,
        tier: ChildTier,
        entries: Vec<DiagnosticEntry>,
    ) {
        let mut slot = self.inner.entry(uri.to_string()).or_default();
        // Replace previous from same server_id
        slot.retain(|(sid, _, _)| sid != server_id);
        slot.push((server_id.to_string(), tier, entries));
    }

    /// Merge all deposited diagnostics for a URI and return the result.
    /// Does not clear the buffer — call clear_uri() after the result is delivered.
    /// Updates the advisory last_andon_block flag if the result has an ANDON block.
    pub fn flush(&self, uri: &str) -> MergeResult {
        let inputs = match self.inner.get(uri) {
            None => return self.ctx.merge(vec![]),
            Some(slot) => slot
                .iter()
                .map(|(_, tier, entries)| (tier.clone(), entries.clone()))
                .collect(),
        };
        let result = self.ctx.merge(inputs);
        if result.has_andon_block {
            self.last_andon_block.store(true, Ordering::Relaxed);
        }
        result
    }

    /// Advisory cached flag: true if any flush since startup produced an ANDON block.
    /// Monotonic — not cleared by clear_uri(). Use compositor_state() for authoritative data.
    pub fn last_andon_block(&self) -> bool {
        self.last_andon_block.load(Ordering::Relaxed)
    }

    /// Clear all deposited diagnostics for a URI (call after successful delivery to editor).
    pub fn clear_uri(&self, uri: &str) {
        self.inner.remove(uri);
    }

    /// Number of URIs currently buffered.
    pub fn buffered_uri_count(&self) -> usize {
        self.inner.len()
    }

    /// List all URIs that currently have buffered diagnostics.
    pub fn buffered_uris(&self) -> Vec<String> {
        self.inner.iter().map(|e| e.key().clone()).collect()
    }
}
