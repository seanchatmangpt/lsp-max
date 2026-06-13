// CompositorClient — the compositor's client-side identity when connecting to a child server.
// Receives pushes from child servers and deposits them into the shared DiagnosticBuffer.
// When a FlushCoordinator is wired in, signals it after each deposit so the coordinator
// can debounce and push merged diagnostics to the editor.

use std::sync::Arc;

use async_trait::async_trait;
use lsp_max::lsp_types::{DiagnosticSeverity, NumberOrString, PublishDiagnosticsParams};
use lsp_max_client::LanguageClient;

use crate::diagnostic_buffer::DiagnosticBuffer;
use crate::flush_coordinator::FlushCoordinator;
use crate::merge::DiagnosticEntry;
use crate::registry::ChildTier;

pub struct CompositorClient {
    server_id: String,
    tier: ChildTier,
    buffer: Arc<DiagnosticBuffer>,
    flush_coordinator: Option<Arc<FlushCoordinator>>,
}

impl CompositorClient {
    pub fn new(server_id: String, tier: ChildTier, buffer: Arc<DiagnosticBuffer>) -> Self {
        Self {
            server_id,
            tier,
            buffer,
            flush_coordinator: None,
        }
    }

    /// Wire a FlushCoordinator so that every deposit automatically signals a flush.
    pub fn with_flush_coordinator(mut self, coordinator: Arc<FlushCoordinator>) -> Self {
        self.flush_coordinator = Some(coordinator);
        self
    }
}

#[async_trait]
impl LanguageClient for CompositorClient {
    async fn publish_diagnostics(&self, params: PublishDiagnosticsParams) {
        let uri = params.uri.to_string();
        let entries: Vec<DiagnosticEntry> = params
            .diagnostics
            .iter()
            .map(|d| DiagnosticEntry {
                uri: uri.clone(),
                line: d.range.start.line,
                character: d.range.start.character,
                severity: match d.severity {
                    Some(DiagnosticSeverity::ERROR) => 1,
                    Some(DiagnosticSeverity::WARNING) => 2,
                    Some(DiagnosticSeverity::INFORMATION) => 3,
                    _ => 4,
                },
                code: match &d.code {
                    Some(NumberOrString::String(s)) => s.clone(),
                    Some(NumberOrString::Number(n)) => n.to_string(),
                    None => String::new(),
                },
                message: d.message.clone(),
                source_tier: self.tier.clone(),
                server_id: Some(self.server_id.clone()),
            })
            .collect();

        self.buffer
            .deposit(&uri, &self.server_id, self.tier.clone(), entries);

        if let Some(coord) = &self.flush_coordinator {
            coord.signal_flush(&uri);
        }
    }
}
