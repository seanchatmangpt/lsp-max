// FlushCoordinator — debounced flush-and-publish pipeline.
// Accepts URI signals from CompositorClient after deposit(), batches bursts within a 100ms
// window, then flushes the DiagnosticBuffer and pushes merged diagnostics to the editor client.

use std::collections::{HashMap, HashSet};
use std::str::FromStr;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use lsp_max::lsp_types::{Diagnostic, DiagnosticSeverity, NumberOrString, Position, Range};
use tokio::sync::mpsc;

use crate::child_process::ChildProcessPool;
use crate::diagnostic_buffer::DiagnosticBuffer;
use crate::gate_file::GateFile;
use crate::merge::MergeContext;
use crate::receipt::CompositorReceipt;

/// Background coordinator that debounces URI flush signals and pushes merged diagnostics
/// to the editor via `lsp_max::Client::publish_diagnostics`.
pub struct FlushCoordinator {
    sender: mpsc::Sender<String>,
    /// Cumulative count of signals dropped due to a full channel.
    /// Incremented on each `try_send` failure; readable via `signal_drop_count()`.
    drop_counter: Arc<AtomicU64>,
}

impl FlushCoordinator {
    /// Spawn the flush coordinator background task.
    /// Returns a `FlushCoordinator` whose `signal_flush` can be passed to `CompositorClient`.
    /// `gate` must be the same `Arc<GateFile>` passed to `DiagnosticBuffer::new()` so both
    /// sites share a single authoritative gate path.
    pub fn spawn(
        buffer: Arc<DiagnosticBuffer>,
        ctx: Arc<MergeContext>,
        client: lsp_max::Client,
        pool: Arc<ChildProcessPool>,
        gate: Arc<GateFile>,
    ) -> Self {
        // Channel capacity 512 — sized to absorb BM-11 requirement of ≥ 500 concurrent
        // server signals within one debounce window without dropping.
        let (tx, mut rx) = mpsc::channel::<String>(512);
        tokio::spawn(async move {
            loop {
                // Wait for at least one URI signal.
                let Some(uri) = rx.recv().await else {
                    break;
                };
                let mut pending: HashSet<String> = HashSet::new();
                pending.insert(uri);

                // Drain any burst within a 100 ms debounce window.
                let deadline =
                    tokio::time::Instant::now() + tokio::time::Duration::from_millis(100);
                while let Ok(Some(u)) = tokio::time::timeout_at(deadline, rx.recv()).await {
                    pending.insert(u);
                }

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
                    if receipt.has_andon_block {
                        tracing::error!(
                            uri = %receipt.uri,
                            andon_codes = ?receipt.andon_codes,
                            prefixes_fingerprint = receipt.prefixes_fingerprint,
                            "compositor-receipt: ANDON block at flush — gate BLOCKED, law violated"
                        );
                    } else {
                        tracing::debug!(
                            uri = %receipt.uri,
                            diagnostic_count = receipt.diagnostic_count,
                            prefixes_fingerprint = receipt.prefixes_fingerprint,
                            "compositor-receipt: flush ADMITTED"
                        );
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
                // Uses batch_has_andon computed from actual flush results this cycle —
                // not the former monotonic flag, so a batch with zero ANDON blocks
                // correctly writes CLEAR (false) to the gate.
                // PreToolUse hooks read this file with a single syscall, no IPC.
                gate.write(batch_has_andon);
            }
        });

        Self {
            sender: tx,
            drop_counter: Arc::new(AtomicU64::new(0)),
        }
    }

    /// Signal that `uri` needs flushing.
    /// Non-blocking — if the channel is full, the signal is dropped, the drop counter is
    /// incremented, and a `tracing::warn` is emitted so the event is observable.
    /// If the receiver has gone away (server shutdown), the signal is silently discarded.
    pub fn signal_flush(&self, uri: &str) {
        if let Err(_e) = self.sender.try_send(uri.to_string()) {
            self.drop_counter.fetch_add(1, Ordering::Relaxed);
            tracing::warn!(
                uri = %uri,
                "flush-coordinator: signal channel full, drop — uri flush deferred until next batch"
            );
        }
    }

    /// Cumulative count of URI flush signals dropped because the channel was full.
    /// A non-zero value indicates backpressure; the compositor state endpoint surfaces this.
    pub fn signal_drop_count(&self) -> u64 {
        self.drop_counter.load(Ordering::Relaxed)
    }
}
