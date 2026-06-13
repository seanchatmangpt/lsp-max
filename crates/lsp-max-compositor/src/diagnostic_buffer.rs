// Per-URI diagnostic staging area.
// Child servers deposit diagnostics here via deposit().
// flush() calls MergeContext::merge() and returns the MergeResult.

use std::sync::{Arc, Mutex};

use papaya::HashMap;

use crate::gate_file::GateFile;
use crate::merge::{DiagnosticEntry, MergeContext, MergeResult};
use crate::registry::ChildTier;

type Slot = Mutex<Vec<(String, ChildTier, Vec<DiagnosticEntry>)>>;

/// Per-URI diagnostic staging area.
/// Child servers deposit diagnostics here via deposit().
/// flush() calls MergeContext::merge() and returns the MergeResult.
///
/// `inner` is a papaya::HashMap — hazard-pointer-based, optimistic reads.
/// The flush() read path never blocks concurrent writers (CANDIDATE for high-N scenarios).
/// Interior mutability for each slot is provided by Mutex<Vec<...>>.
pub struct DiagnosticBuffer {
    /// Keyed by URI. Value is a Mutex-guarded Vec of (server_id, tier, entries) tuples.
    inner: HashMap<String, Arc<Slot>>,
    ctx: Arc<MergeContext>,
    /// Gate file shared with FlushCoordinator — written eagerly on deposit() when
    /// an incoming entry matches an ANDON prefix, eliminating the debounce staleness window.
    gate: Arc<GateFile>,
}

impl DiagnosticBuffer {
    pub fn new(ctx: Arc<MergeContext>, gate: Arc<GateFile>) -> Self {
        Self {
            inner: HashMap::new(),
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
        // Uses per-server daachorse automaton (L7 Speciation) — O(|code|) vs former O(|P|×|code|).
        // Falls back to workspace union automaton when server has no override in lsp-max.toml.
        let has_incoming_andon = entries
            .iter()
            .any(|e| e.severity == 1 && self.ctx.is_andon_for_server(&e.code, Some(server_id)));
        if has_incoming_andon {
            tracing::warn!(
                uri = %uri,
                server_id = %server_id,
                "diagnostic-buffer: ANDON prefix matched on deposit — gate BLOCKED (eager write)"
            );
            self.gate.write(true);
        }

        // get_or_insert_with registers the epoch guard internally via pin().
        let guard = self.inner.pin();
        let slot = guard.get_or_insert_with(uri.to_string(), || Arc::new(Mutex::new(Vec::new())));
        let mut vec = slot.lock().expect("diagnostic-buffer slot lock: OPEN");
        // Replace previous entries from same server_id.
        vec.retain(|(sid, _, _)| sid != server_id);
        vec.push((server_id.to_string(), tier, entries));
    }

    /// Merge all deposited diagnostics for a URI and return the result.
    /// Does not clear the buffer — call clear_uri() after the result is delivered.
    pub fn flush(&self, uri: &str) -> MergeResult {
        let guard = self.inner.pin();
        let inputs = match guard.get(uri) {
            None => return self.ctx.merge(vec![]),
            Some(slot) => {
                let vec = slot.lock().expect("diagnostic-buffer slot lock: OPEN");
                vec.iter()
                    .map(|(_, tier, entries)| (tier.clone(), entries.clone()))
                    .collect()
            }
        };
        self.ctx.merge(inputs)
    }

    /// Clear all deposited diagnostics for a URI (call after successful delivery to editor).
    pub fn clear_uri(&self, uri: &str) {
        let guard = self.inner.pin();
        guard.remove(uri);
    }

    /// Number of URIs currently buffered.
    pub fn buffered_uri_count(&self) -> usize {
        self.inner.len()
    }

    /// List all URIs that currently have buffered diagnostics.
    pub fn buffered_uris(&self) -> Vec<String> {
        let guard = self.inner.pin();
        guard.iter().map(|(k, _)| k.clone()).collect()
    }
}
