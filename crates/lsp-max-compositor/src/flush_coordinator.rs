// FlushCoordinator — adaptive quorum-based flush-and-publish pipeline.
//
// Replaces the fixed 100ms debounce with a dynamic debounce that fires as soon as all
// expected servers have deposited for a URI (quorum), or after an adaptive timeout based
// on observed inter-arrival spread (2× spread, clamped to [1ms, 30ms]).
//
// The goal is minimum user-perceived lag: if all 500 servers respond in 2ms, the editor
// sees the merged result in 2ms — not 100ms.

use std::collections::{HashMap, HashSet};
use std::str::FromStr;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use lsp_max::lsp_types::{Diagnostic, DiagnosticSeverity, NumberOrString, Position, Range};
use tokio::time::{Duration, Instant};

use crate::child_process::ChildProcessPool;
use crate::diagnostic_buffer::DiagnosticBuffer;
use crate::gate_file::GateFile;
use crate::merge::MergeContext;
use crate::receipt::CompositorReceipt;

const MIN_WAIT: Duration = Duration::from_millis(1);
const MAX_WAIT: Duration = Duration::from_millis(30);

/// Signal carrying both URI and originating server identity.
/// The coordinator uses server_id to track quorum per URI.
#[derive(Debug)]
pub struct FlushSignal {
    pub uri: String,
    pub server_id: String,
}

/// Per-URI state tracked during the collection window.
struct UriFlushState {
    deposited: HashSet<String>,
    first_at: Instant,
    last_at: Instant,
}

impl UriFlushState {
    fn new(server_id: String, now: Instant) -> Self {
        let mut deposited = HashSet::new();
        deposited.insert(server_id);
        Self {
            deposited,
            first_at: now,
            last_at: now,
        }
    }

    /// Adaptive flush deadline for this URI.
    /// Returns `first_at` (i.e., fire immediately) when quorum is reached.
    /// Otherwise: last_at + clamp(2 × spread, MIN_WAIT, MAX_WAIT).
    fn deadline(&self, expected: usize) -> Instant {
        if self.deposited.len() >= expected {
            self.first_at // quorum — already past, fires immediately on next select!
        } else {
            let spread = self.last_at.saturating_duration_since(self.first_at);
            self.last_at + (spread * 2).clamp(MIN_WAIT, MAX_WAIT)
        }
    }
}

/// Background coordinator that debounces URI flush signals and pushes merged diagnostics
/// to the editor via `lsp_max::Client::publish_diagnostics`.
pub struct FlushCoordinator {
    sender: kanal::AsyncSender<FlushSignal>,
    /// Cumulative count of signals dropped due to a full channel.
    /// Incremented on each `try_send` failure; readable via `signal_drop_count()`.
    drop_counter: Arc<AtomicU64>,
}

impl FlushCoordinator {
    /// Spawn the flush coordinator background task.
    /// `expected_server_count` is the number of registered child servers — when all have
    /// deposited for a URI, the flush fires immediately (zero additional wait).
    /// `gate` must be the same `Arc<GateFile>` passed to `DiagnosticBuffer::new()`.
    pub fn spawn(
        buffer: Arc<DiagnosticBuffer>,
        ctx: Arc<MergeContext>,
        client: lsp_max::Client,
        pool: Arc<ChildProcessPool>,
        gate: Arc<GateFile>,
        expected_server_count: usize,
    ) -> Self {
        // Capacity ≥ expected_server_count × URIs per window — 512 handles N=500 at 1 URI.
        let (tx, rx) = kanal::bounded_async::<FlushSignal>(512);
        let drop_counter = Arc::new(AtomicU64::new(0));
        let _drop_counter_bg = Arc::clone(&drop_counter);

        tokio::spawn(async move {
            // per_uri: tracks which servers have deposited for each URI in the current window.
            let mut per_uri: HashMap<String, UriFlushState> = HashMap::new();

            loop {
                // Compute the earliest deadline across all pending URIs.
                let next_deadline = per_uri
                    .values()
                    .map(|s| s.deadline(expected_server_count))
                    .min();

                // Select: either a new signal arrives or the next deadline fires.
                // kanal::AsyncReceiver::recv() returns Result<T, ReceiveError>;
                // Err(ReceiveError::Closed) means all senders dropped — shutdown.
                let timed_out = if let Some(dl) = next_deadline {
                    tokio::select! {
                        res = rx.recv() => {
                            match res {
                                Err(_) => break, // channel closed — shutdown
                                Ok(s) => {
                                    let now = Instant::now();
                                    per_uri
                                        .entry(s.uri.clone())
                                        .and_modify(|state| {
                                            state.deposited.insert(s.server_id.clone());
                                            state.last_at = now;
                                        })
                                        .or_insert_with(|| UriFlushState::new(s.server_id, now));
                                    false
                                }
                            }
                        }
                        _ = tokio::time::sleep_until(dl) => true,
                    }
                } else {
                    // No pending URIs — block until the first signal arrives.
                    match rx.recv().await {
                        Err(_) => break, // channel closed — shutdown
                        Ok(s) => {
                            let now = Instant::now();
                            per_uri.insert(s.uri, UriFlushState::new(s.server_id, now));
                            false
                        }
                    }
                };

                // Collect URIs whose deadline has passed (quorum or adaptive timeout).
                let now = Instant::now();
                let ready: Vec<String> = per_uri
                    .iter()
                    .filter(|(_, state)| timed_out || state.deadline(expected_server_count) <= now)
                    .map(|(uri, _)| uri.clone())
                    .collect();

                if ready.is_empty() {
                    continue;
                }

                for uri in &ready {
                    per_uri.remove(uri);
                }

                let pending: HashSet<String> = ready.into_iter().collect();

                // Flush each pending URI and push to the editor.
                // Track batch-level ANDON state for the gate write below.
                // Computed fresh per flush — not monotonic.
                let mut batch_has_andon = false;
                for uri in &pending {
                    let result = buffer.flush(uri);
                    if result.has_andon_block {
                        batch_has_andon = true;
                        tracing::warn!(
                            uri = %uri,
                            codes = ?result.andon_codes(),
                            "flush-coordinator: ANDON block — law violations present"
                        );
                    }

                    let lsp_diags: Vec<Diagnostic> = result
                        .diagnostics
                        .iter()
                        .map(|d| Diagnostic {
                            range: Range {
                                start: Position {
                                    line: d.line,
                                    character: d.character,
                                },
                                end: Position {
                                    line: d.line,
                                    character: d.character,
                                },
                            },
                            severity: Some(match d.severity {
                                1 => DiagnosticSeverity::ERROR,
                                2 => DiagnosticSeverity::WARNING,
                                3 => DiagnosticSeverity::INFORMATION,
                                _ => DiagnosticSeverity::HINT,
                            }),
                            code: if d.code.is_empty() {
                                None
                            } else {
                                Some(NumberOrString::String(d.code.clone()))
                            },
                            source: Some(match &d.server_id {
                                Some(sid) => {
                                    format!("compositor/{}:{}", d.source_tier.as_str(), sid)
                                }
                                None => format!("compositor/{}", d.source_tier.as_str()),
                            }),
                            message: d.message.clone(),
                            ..Default::default()
                        })
                        .collect();

                    if let Ok(parsed_uri) = lsp_max::lsp_types::Uri::from_str(uri) {
                        client
                            .publish_diagnostics(parsed_uri, lsp_diags, None)
                            .await;
                    }

                    let receipt =
                        CompositorReceipt::new(uri.clone(), &result, ctx.andon_prefixes());
                    match receipt.status() {
                        crate::receipt::ReceiptStatus::Blocked => {
                            tracing::error!(
                                uri = %receipt.uri,
                                andon_codes = ?receipt.andon_codes,
                                prefixes_fingerprint = receipt.prefixes_fingerprint,
                                status = %receipt.status(),
                                "compositor-receipt: ANDON block — status BLOCKED"
                            );
                        }
                        crate::receipt::ReceiptStatus::Admitted => {
                            tracing::debug!(
                                uri = %receipt.uri,
                                diagnostic_count = receipt.diagnostic_count,
                                prefixes_fingerprint = receipt.prefixes_fingerprint,
                                status = %receipt.status(),
                                "compositor-receipt: flush ADMITTED"
                            );
                        }
                    }

                    // Compute per-server acks from the merge result and notify child servers.
                    let mut per_server: HashMap<String, (usize, bool)> = HashMap::new();
                    for d in &result.diagnostics {
                        if let Some(sid) = &d.server_id {
                            let entry = per_server.entry(sid.clone()).or_insert((0, false));
                            entry.0 += 1;
                            if d.severity == 1 && crate::merge::is_refused_by_law(&d.code) {
                                entry.1 = true;
                            }
                        }
                    }

                    // Collect (server_id, handle) while DashMap ref is held briefly,
                    // then drop all refs before awaiting to avoid holding shard locks.
                    let mut ack_targets: Vec<(String, lsp_max_client::ServerHandle)> =
                        Vec::with_capacity(per_server.len());
                    for sid in per_server.keys() {
                        if let Some(proc_ref) = pool.get(sid) {
                            ack_targets.push((sid.clone(), proc_ref.handle.clone()));
                        }
                    }

                    for (sid, handle) in ack_targets {
                        if let Some(&(admitted_count, has_andon)) = per_server.get(&sid) {
                            let ack = crate::diagnostic_ack::DiagnosticAck {
                                uri: uri.clone(),
                                admitted_count,
                                suppressed_count: 0, // pre-merge counts not yet tracked
                                has_andon_contribution: has_andon,
                            };
                            if let Ok(ack_json) = serde_json::to_value(&ack) {
                                handle.notify("max/diagnosticAck", ack_json).await;
                            }
                        }
                    }
                }

                // Materialize global ANDON state to the gate file after each batch.
                // One write per debounce window regardless of URI count — O(1).
                // PreToolUse hooks read this file with a single syscall, no IPC.
                //
                // Correctness: global_andon_active() reads an AtomicUsize counter maintained
                // by DiagnosticBuffer::deposit() and flush() — O(1) regardless of URI count.
                // The batch flush above already called buffer.flush() for each pending URI,
                // which updates the counter before we read it here.
                let effective_andon = batch_has_andon || buffer.global_andon_active();
                gate.write(effective_andon);
                // Sync buffer's last-written flag so deposit() skips redundant writes
                // correctly on the next round (especially important for ANDON → clear transitions).
                buffer.sync_gate_written(effective_andon);
            }
        });

        Self {
            sender: tx,
            drop_counter,
        }
    }

    /// Signal that `uri` received a deposit from `server_id`.
    /// Non-blocking — if the channel is full, the signal is dropped and the drop counter
    /// is incremented. A `tracing::warn` makes the event observable.
    pub fn signal_flush(&self, uri: &str, server_id: &str) {
        let sig = FlushSignal {
            uri: uri.to_string(),
            server_id: server_id.to_string(),
        };
        // kanal try_send: returns Err(SendError) on full or closed channel — same drop semantics.
        if let Err(_e) = self.sender.try_send(sig) {
            self.drop_counter.fetch_add(1, Ordering::Relaxed);
            tracing::warn!(
                uri = %uri,
                server_id = %server_id,
                "flush-coordinator: signal channel full, drop — flush deferred"
            );
        }
    }

    /// Cumulative count of URI flush signals dropped because the channel was full.
    /// A non-zero value indicates backpressure; the compositor state endpoint surfaces this.
    pub fn signal_drop_count(&self) -> u64 {
        self.drop_counter.load(Ordering::Relaxed)
    }
}
