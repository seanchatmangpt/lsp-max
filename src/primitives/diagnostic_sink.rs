use lsp_types_max::{Diagnostic, Url};
use parking_lot::RwLock;
use rustc_hash::FxHashMap;
use std::sync::Arc;

use crate::service::Client;

/// Wraps `Client::publish_diagnostics` with last-value deduplication.
///
/// Publishing is skipped when the new diagnostic set is identical to the last
/// published set, eliminating redundant LSP notifications on unchanged files.
///
/// Clone is O(1): all clones share the same last-published map via `Arc`.
#[derive(Clone, Debug)]
pub struct DiagnosticSink {
    client: Client,
    last: Arc<RwLock<FxHashMap<Url, Vec<Diagnostic>>>>,
}

impl DiagnosticSink {
    /// Wraps `client` with deduplication tracking.
    pub fn new(client: Client) -> Self {
        Self {
            client,
            last: Arc::new(RwLock::new(FxHashMap::default())),
        }
    }

    /// Publish `diags` for `uri`.  No-ops if identical to last publish.
    pub async fn publish(&self, uri: Url, diags: Vec<Diagnostic>) {
        {
            let last = self.last.read();
            if last.get(&uri).is_some_and(|prev| prev == &diags) {
                return;
            }
        }
        self.last.write().insert(uri.clone(), diags.clone());
        self.client.publish_diagnostics(uri, diags, None).await;
    }

    /// Clear diagnostics for `uri`.  No-ops if already empty.
    pub async fn clear(&self, uri: &Url) {
        let was_present = self.last.write().remove(uri).is_some();
        if was_present {
            self.client
                .publish_diagnostics(uri.clone(), vec![], None)
                .await;
        }
    }

    /// Returns the last published diagnostic count for `uri`, or `None` if
    /// the URI has never been published to (or was cleared).
    pub fn last_count(&self, uri: &Url) -> Option<usize> {
        self.last.read().get(uri).map(|v| v.len())
    }
}
