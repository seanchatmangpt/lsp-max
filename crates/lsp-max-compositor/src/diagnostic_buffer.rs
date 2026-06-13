// Per-URI diagnostic staging area.
// Child servers deposit diagnostics here via deposit().
// flush() calls MergeContext::merge() and returns the MergeResult.

use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
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
    ///
    /// Ordering: deposit() uses AcqRel swap (load=Acquire to see sync_gate_written stores,
    /// store=Release to publish the new state). sync_gate_written() uses Release store.
    /// Together: deposit() always observes the latest flushed gate state.
    gate_last_written: AtomicBool,
    /// Count of URIs currently holding at least one Error-severity ANDON diagnostic.
    /// Incremented when a URI transitions from no-ANDON → has-ANDON in deposit().
    /// Decremented in flush() when the merged result has no ANDON block (cleared by flush).
    /// Enables O(1) global gate state check via global_andon_active().
    andon_uri_count: AtomicUsize,
    /// Tracks which URIs currently have at least one ANDON entry (per-URI flag).
    /// Used to detect transitions (no-ANDON → has-ANDON and has-ANDON → no-ANDON)
    /// so andon_uri_count stays accurate without double-counting.
    uri_has_andon: HashMap<String, bool>,
}

impl DiagnosticBuffer {
    pub fn new(ctx: Arc<MergeContext>, gate: Arc<GateFile>) -> Self {
        Self {
            inner: HashMap::new(),
            ctx,
            gate,
            gate_last_written: AtomicBool::new(false),
            andon_uri_count: AtomicUsize::new(0),
            uri_has_andon: HashMap::new(),
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
            if !self.gate_last_written.swap(true, Ordering::AcqRel) {
                tracing::warn!(
                    uri = %uri,
                    server_id = %server_id,
                    "diagnostic-buffer: ANDON prefix matched on deposit — gate BLOCKED (eager write)"
                );
                self.gate.write(true);
            }
            // Track per-URI ANDON state for O(1) global_andon_active().
            // Only increment andon_uri_count on the no-ANDON → has-ANDON transition.
            let andon_guard = self.uri_has_andon.pin();
            if andon_guard.insert(uri.to_string(), true).is_none() {
                self.andon_uri_count.fetch_add(1, Ordering::Release);
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
    /// When the merged result has no ANDON block, the URI's ANDON flag is cleared and
    /// andon_uri_count is decremented (has-ANDON → no-ANDON transition).
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
        let result = self.ctx.merge(inputs);
        // Clear per-URI ANDON flag when this URI's merged result is clean.
        // Decrement andon_uri_count only on the has-ANDON → no-ANDON transition.
        if !result.has_andon_block {
            let andon_guard = self.uri_has_andon.pin();
            if andon_guard.remove(uri).is_some() {
                self.andon_uri_count.fetch_sub(1, Ordering::Release);
            }
        }
        result
    }

    /// Clear all deposited diagnostics for a URI (call after successful delivery to editor).
    /// Also clears the URI's ANDON flag if present, decrementing andon_uri_count.
    pub fn clear_uri(&self, uri: &str) {
        let guard = self.inner.pin();
        guard.remove(uri);
        // Also evict the ANDON flag if present — the URI is gone, no longer contributes.
        let andon_guard = self.uri_has_andon.pin();
        if andon_guard.remove(uri).is_some() {
            self.andon_uri_count.fetch_sub(1, Ordering::Release);
        }
    }

    /// Returns true if any URI currently has at least one Error-severity ANDON diagnostic.
    /// O(1) — reads a single atomic counter maintained by deposit() and flush().
    pub fn global_andon_active(&self) -> bool {
        self.andon_uri_count.load(Ordering::Acquire) > 0
    }

    /// Called by FlushCoordinator after writing the gate file at the end of a flush batch.
    /// Syncs `gate_last_written` so the next deposit() skips redundant writes correctly.
    pub fn sync_gate_written(&self, andon: bool) {
        // Release is correct here: ensures all preceding gate.write() operations
        // are visible before the store is observed by concurrent deposit() AcqRel swaps.
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
