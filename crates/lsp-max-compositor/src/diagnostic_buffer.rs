// Per-URI diagnostic staging area.
// Child servers deposit diagnostics here via deposit().
// flush() calls MergeContext::merge() and returns the MergeResult.

use std::sync::atomic::{AtomicBool, Ordering};
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
    /// Tracks the last value written to the gate file. A gate write is skipped when
    /// the desired state matches what was last written — eliminating redundant file I/O
    /// on every deposit() when ANDON is already active.
    /// BM-5 showed the gate write costs ~50µs flat regardless of entry count; this
    /// reduces it to an O(1) atomic load on the hot path when state is stable.
    gate_last_written: AtomicBool,
}

impl DiagnosticBuffer {
    pub fn new(ctx: Arc<MergeContext>, gate: Arc<GateFile>) -> Self {
        Self {
            inner: HashMap::new(),
            ctx,
            gate,
            gate_last_written: AtomicBool::new(false),
        }
    }

    /// Record diagnostics from a child server for a URI.
    /// Replaces any previous entries from the same server_id for that URI.
    /// If any incoming entry matches an ANDON prefix (severity == 1 and code has an ANDON
    /// prefix), the gate file is written IMMEDIATELY — before the debounce window expires.
    /// The write is skipped when the gate is already in the target state (state-change only).
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
            // Only write when transitioning from clear → ANDON. Avoids ~50µs file I/O
            // on every deposit when the gate is already blocked (BM-5 finding).
            if !self.gate_last_written.swap(true, Ordering::Release) {
                tracing::warn!(
                    uri = %uri,
                    server_id = %server_id,
                    "diagnostic-buffer: ANDON prefix matched on deposit — gate BLOCKED (eager write)"
                );
                self.gate.write(true);
            }
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

    /// Called by FlushCoordinator after writing the gate file at the end of a flush batch.
    /// Syncs `gate_last_written` so the next deposit() skips redundant writes correctly.
    pub fn sync_gate_written(&self, andon: bool) {
        self.gate_last_written.store(andon, Ordering::Release);
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
