// Per-URI diagnostic staging area.
// Child servers deposit diagnostics here via deposit().
// flush() calls MergeContext::merge() and returns the MergeResult.

use std::sync::Arc;

use dashmap::DashMap;

use crate::gate_file::GateFile;
use crate::merge::{DiagnosticEntry, MergeContext, MergeResult};
use crate::registry::ChildTier;

/// Per-URI diagnostic staging area.
/// Child servers deposit diagnostics here via deposit().
/// flush() calls MergeContext::merge() and returns the MergeResult.
pub struct DiagnosticBuffer {
    /// Keyed by URI. Value is Vec of (server_id, tier, entries) tuples.
    inner: DashMap<String, Vec<(String, ChildTier, Vec<DiagnosticEntry>)>>,
    ctx: Arc<MergeContext>,
    /// Gate file shared with FlushCoordinator — written eagerly on deposit() when
    /// an incoming entry matches an ANDON prefix, eliminating the debounce staleness window.
    gate: Arc<GateFile>,
}

impl DiagnosticBuffer {
    pub fn new(ctx: Arc<MergeContext>, gate: Arc<GateFile>) -> Self {
        Self {
            inner: DashMap::new(),
            ctx,
            gate,
        }
    }

    /// Record diagnostics from a child server for a URI.
    /// Replaces any previous entries from the same server_id for that URI.
    /// If any incoming entry matches an ANDON prefix (severity == 1 and code has an ANDON
    /// prefix), the gate file is written IMMEDIATELY — before the debounce window expires.
    /// Status: CANDIDATE until FlushCoordinator confirms or clears after a full flush.
    pub fn deposit(
        &self,
        uri: &str,
        server_id: &str,
        tier: ChildTier,
        entries: Vec<DiagnosticEntry>,
    ) {
        // Eager ANDON gate write: check incoming entries before storing them.
        // Uses per-server prefix set (L7 Speciation) — each server's own C_D, not the union.
        // Falls back to workspace union when server has no override in lsp-max.toml.
        let effective_prefixes = self.ctx.prefixes_for_server(server_id);
        let has_incoming_andon = entries.iter().any(|e| {
            e.severity == 1
                && effective_prefixes
                    .iter()
                    .any(|p| e.code.starts_with(p.as_str()))
        });
        if has_incoming_andon {
            tracing::warn!(
                uri = %uri,
                server_id = %server_id,
                "diagnostic-buffer: ANDON prefix matched on deposit — gate BLOCKED (eager write)"
            );
            self.gate.write(true);
        }

        let mut slot = self.inner.entry(uri.to_string()).or_default();
        // Replace previous from same server_id
        slot.retain(|(sid, _, _)| sid != server_id);
        slot.push((server_id.to_string(), tier, entries));
    }

    /// Merge all deposited diagnostics for a URI and return the result.
    /// Does not clear the buffer — call clear_uri() after the result is delivered.
    pub fn flush(&self, uri: &str) -> MergeResult {
        let inputs = match self.inner.get(uri) {
            None => return self.ctx.merge(vec![]),
            Some(slot) => slot
                .iter()
                .map(|(_, tier, entries)| (tier.clone(), entries.clone()))
                .collect(),
        };
        self.ctx.merge(inputs)
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
