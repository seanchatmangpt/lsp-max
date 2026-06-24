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
use lsp_max_runtime::control_plane::receipts::{Blake3Hash, Keystore};
use tokio::time::{Duration, Instant};

use crate::child_process::ChildProcessPool;
use crate::declare::{extract_traces, DeclareModel};
use crate::dfg::DirectlyFollowsGraph;
use crate::diagnostic_buffer::DiagnosticBuffer;
use crate::gate_file::GateFile;
use crate::merge::MergeContext;
use crate::receipt::CompositorReceipt;
use crate::receipt_chain::ChildEvidence;

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
    /// RFC C: accumulated OCEL 2.0 events derived from `CompositorReceipt` flushes.
    ocel_events: Arc<std::sync::Mutex<Vec<serde_json::Value>>>,
    /// Monotonic counter used as the OCEL event-id source.
    event_counter: Arc<AtomicU64>,
    /// RFC B: monotonic sequence counter for per-child `ChildEvidence` chain links.
    /// Each per-server flush contribution consumes one sequence slot, ensuring the
    /// `consequence_hash` in each `CryptographicReceipt` is unique across the
    /// coordinator lifetime. Shared with the background task via `Arc<AtomicU64>`.
    receipt_seq: Arc<AtomicU64>,
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
        let ocel_events: Arc<std::sync::Mutex<Vec<serde_json::Value>>> =
            Arc::new(std::sync::Mutex::new(Vec::new()));
        let ocel_events_bg = Arc::clone(&ocel_events);
        let event_counter = Arc::new(AtomicU64::new(0));
        let event_counter_bg = Arc::clone(&event_counter);
        // RFC B: compositor-level Keystore for signing per-child chain links.
        // Generated fresh at spawn time — not persisted. A persistent Keystore backed
        // by a stable seed is OPEN (requires key-management infrastructure).
        let compositor_keystore = Keystore::generate();
        let receipt_seq = Arc::new(AtomicU64::new(0));
        let receipt_seq_bg = Arc::clone(&receipt_seq);

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

                let mut uri_deposited_servers: HashMap<String, HashSet<String>> = HashMap::new();
                for uri in &ready {
                    if let Some(state) = per_uri.remove(uri) {
                        uri_deposited_servers.insert(uri.clone(), state.deposited);
                    }
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

                    // RFC B: aggregate per-server flush contributions before building
                    // the receipt so child evidence can be attached in one pass.
                    // `per_server`: server_id → (admitted_count, has_andon_contribution)
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

                    // RFC B: build one `ChildEvidence` per contributing server and attach
                    // to the receipt. The compositor signs each link using its ephemeral
                    // `Keystore`; the child's own receipt file is OPEN (not yet published).
                    // `prev_hash` uses the zero hash as the chain genesis — a persistent
                    // prev_hash from the previous flush is OPEN (requires chain head state).
                    let child_evidence: Vec<ChildEvidence> = per_server
                        .iter()
                        .map(|(sid, &(admitted_count, has_andon))| {
                            let seq = receipt_seq_bg.fetch_add(1, Ordering::Relaxed);
                            let ev = ChildEvidence::from_flush_contribution(
                                sid.as_str(),
                                admitted_count,
                                has_andon,
                                seq,
                                Blake3Hash([0u8; 32]),
                                &compositor_keystore,
                            );
                            tracing::debug!(
                                server_id = %ev.server_id,
                                compositor_signed = true,
                                has_andon = %ev.has_andon_contribution,
                                admitted_count,
                                seq,
                                "rfc-b: child evidence chain link — status CANDIDATE"
                            );
                            ev
                        })
                        .collect();

                    let receipt =
                        CompositorReceipt::new(uri.clone(), &result, ctx.andon_prefixes())
                            .with_child_evidence(child_evidence);
                    match receipt.status() {
                        crate::receipt::ReceiptStatus::Blocked => {
                            tracing::error!(
                                uri = %receipt.uri,
                                andon_codes = ?receipt.andon_codes,
                                prefixes_fingerprint = receipt.prefixes_fingerprint,
                                child_evidence_count = receipt.child_evidence.len(),
                                status = %receipt.status(),
                                "compositor-receipt: ANDON block — status BLOCKED"
                            );
                        }
                        crate::receipt::ReceiptStatus::Admitted => {
                            tracing::debug!(
                                uri = %receipt.uri,
                                diagnostic_count = receipt.diagnostic_count,
                                prefixes_fingerprint = receipt.prefixes_fingerprint,
                                child_evidence_count = receipt.child_evidence.len(),
                                status = %receipt.status(),
                                "compositor-receipt: flush ADMITTED"
                            );
                        }
                    }

                    let eid = event_counter_bg.fetch_add(1, Ordering::Relaxed);
                    let event_id = format!("cf-{eid}");
                    let timestamp = chrono::Utc::now().to_rfc3339();
                    let compositor_event = receipt.to_ocel_event(&event_id, &timestamp);
                    let mut events_to_log = vec![compositor_event.clone()];
                    for (idx, ev) in receipt.child_evidence.iter().enumerate() {
                        let child_event_id = format!("{}-child-{}", event_id, idx);
                        let child_event = ev.to_ocel_event(
                            &child_event_id,
                            &timestamp,
                            &receipt.merge_object_id(),
                        );
                        events_to_log.push(child_event);
                    }

                    if let Ok(mut guard) = ocel_events_bg.lock() {
                        guard.push(compositor_event);

                        // RFC C + Van der Aalst: run Declare conformance + DFG on the
                        // accumulated log after every flush so violations are surfaced
                        // continuously rather than only at log-export time.
                        let traces = extract_traces(&guard);
                        let model = DeclareModel::compositor();
                        let violations = model.check(&traces);
                        if !violations.is_empty() {
                            tracing::warn!(
                                violations = violations.len(),
                                uri = %uri,
                                "declare-conformance: compositor process model violated"
                            );
                            for v in &violations {
                                tracing::warn!(
                                    constraint = %v.constraint,
                                    case_id = %v.case_id,
                                    detail = %v.detail,
                                    "declare-violation"
                                );
                            }
                        }
                        let dfg = DirectlyFollowsGraph::from_traces(&traces);
                        let normative_arcs = [
                            (
                                "CompositorFlush".to_string(),
                                "CompositorFlushAdmitted".to_string(),
                            ),
                            (
                                "CompositorFlush".to_string(),
                                "CompositorFlushBlocked".to_string(),
                            ),
                            (
                                "CompositorFlushBlocked".to_string(),
                                "AndonCodePresent".to_string(),
                            ),
                        ];
                        if let Some(fitness) = dfg.fitness_against_model(&normative_arcs) {
                            tracing::debug!(
                                fitness,
                                nodes = dfg.node_count(),
                                edges = dfg.edge_count(),
                                transitions = dfg.total_transitions(),
                                "dfg-fitness: compositor process model"
                            );
                        }
                    }

                    let log_path = gate.path().with_extension("ocel.jsonl");
                    if let Ok(mut file) = std::fs::OpenOptions::new()
                        .create(true)
                        .append(true)
                        .open(&log_path)
                    {
                        use std::io::Write;
                        for event_val in events_to_log {
                            if let Ok(json_str) = serde_json::to_string(&event_val) {
                                if let Err(e) = writeln!(file, "{}", json_str) {
                                    tracing::warn!(error = %e, "flush-coordinator: failed to write OCEL event to log file");
                                }
                            }
                        }
                    } else {
                        tracing::warn!(path = %log_path.display(), "flush-coordinator: failed to open OCEL log file");
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
            ocel_events,
            event_counter,
            receipt_seq,
        }
    }

    /// RFC C: drain the accumulated OCEL 2.0 events and return them.
    ///
    /// Each element is a `CompositorFlush` OCEL event produced by
    /// `CompositorReceipt::to_ocel_event`.  The internal buffer is cleared on
    /// each call, so callers own the drained slice.
    pub fn take_ocel_events(&self) -> Vec<serde_json::Value> {
        self.ocel_events
            .lock()
            .map(|mut g| std::mem::take(&mut *g))
            .unwrap_or_default()
    }

    /// RFC C: snapshot the accumulated OCEL log without draining it.
    pub fn ocel_event_count(&self) -> usize {
        self.ocel_events.lock().map(|g| g.len()).unwrap_or(0)
    }

    /// Monotonic count of OCEL event ids assigned over this coordinator's
    /// lifetime. Unlike `ocel_event_count`, this is not reset by `take_*`.
    pub fn event_id_count(&self) -> u64 {
        self.event_counter.load(Ordering::Relaxed)
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

    /// RFC B: monotonic count of `ChildEvidence` chain links signed by the
    /// compositor's ephemeral Keystore over this coordinator lifetime.
    /// Each per-server flush contribution consumes one sequence slot.
    pub fn receipt_seq_count(&self) -> u64 {
        self.receipt_seq.load(Ordering::Relaxed)
    }
}

fn decode_hex(s: &str) -> Result<[u8; 32], String> {
    if s.len() != 64 {
        return Err(format!("Invalid hex string length: {}", s.len()));
    }
    let mut bytes = [0u8; 32];
    for i in 0..32 {
        let hex_slice = &s[i * 2..i * 2 + 2];
        bytes[i] = u8::from_str_radix(hex_slice, 16)
            .map_err(|e| format!("Failed to parse hex slice {}: {}", hex_slice, e))?;
    }
    Ok(bytes)
}
