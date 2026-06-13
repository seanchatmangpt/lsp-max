// FlushCoordinator — debounced flush-and-publish pipeline.
// Accepts URI signals from CompositorClient after deposit(), batches bursts within a 100ms
// window, then flushes the DiagnosticBuffer and pushes merged diagnostics to the editor client.

use std::collections::HashSet;
use std::str::FromStr;
use std::sync::Arc;

use lsp_max::lsp_types::{Diagnostic, DiagnosticSeverity, NumberOrString, Position, Range};
use tokio::sync::mpsc;

use crate::diagnostic_buffer::DiagnosticBuffer;
use crate::merge::MergeContext;

/// Background coordinator that debounces URI flush signals and pushes merged diagnostics
/// to the editor via `lsp_max::Client::publish_diagnostics`.
pub struct FlushCoordinator {
    sender: mpsc::Sender<String>,
}

impl FlushCoordinator {
    /// Spawn the flush coordinator background task.
    /// Returns a `FlushCoordinator` whose `signal_flush` can be passed to `CompositorClient`.
    pub fn spawn(
        buffer: Arc<DiagnosticBuffer>,
        _ctx: Arc<MergeContext>,
        client: lsp_max::Client,
    ) -> Self {
        let (tx, mut rx) = mpsc::channel::<String>(256);
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
                for uri in &pending {
                    let result = buffer.flush(uri);

                    if result.has_andon_block {
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
                }
            }
        });

        Self { sender: tx }
    }

    /// Signal that `uri` needs flushing.
    /// Non-blocking — the signal is silently dropped if the channel is full or the receiver
    /// has gone away (e.g. after server shutdown).
    pub fn signal_flush(&self, uri: &str) {
        let _ = self.sender.try_send(uri.to_string());
    }
}
