//! CC-004: FanoutCoordinator — per-URI serialized notification fan-out.
//! Ticket: docs/jira/v26.6.30/CC-004-notification-routing.md

use dashmap::DashMap;
use std::sync::Arc;

/// Tracks open document state and enforces per-URI ordering of notifications.
/// Ensures didChange is never dispatched before didOpen completes for a URI.
#[derive(Debug, Clone, Default)]
pub struct FanoutCoordinator {
    server_ids: Vec<String>,
    open_uris: Arc<DashMap<String, ()>>,
    doc_versions: Arc<DashMap<String, u32>>,
}

impl FanoutCoordinator {
    pub fn new(server_ids: Vec<String>) -> Self {
        Self {
            server_ids,
            open_uris: Arc::new(DashMap::new()),
            doc_versions: Arc::new(DashMap::new()),
        }
    }

    pub fn record_did_open(&self, uri: &str) {
        self.open_uris.insert(uri.to_string(), ());
    }

    pub fn record_did_close(&self, uri: &str) {
        self.open_uris.remove(uri);
        self.doc_versions.remove(uri);
    }

    pub fn is_open(&self, uri: &str) -> bool {
        self.open_uris.contains_key(uri)
    }

    /// Returns true if didChange can be dispatched: URI must have been opened first.
    pub fn can_did_change(&self, uri: &str) -> bool {
        self.is_open(uri)
    }

    pub fn record_version(&self, uri: &str, version: u32) {
        self.doc_versions.insert(uri.to_string(), version);
    }

    /// Returns true if incoming_version is lower than the last recorded version for this URI.
    pub fn check_version_regression(&self, uri: &str, incoming_version: u32) -> bool {
        self.doc_versions
            .get(uri)
            .map(|v| incoming_version < *v)
            .unwrap_or(false)
    }

    pub fn server_ids(&self) -> &[String] {
        &self.server_ids
    }
}
