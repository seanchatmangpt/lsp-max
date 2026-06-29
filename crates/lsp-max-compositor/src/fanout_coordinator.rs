//! CANDIDATE: FanoutCoordinator — per-URI serialized notification fan-out.
//! Status: CANDIDATE
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
    /// Create a new FanoutCoordinator for the given set of child server IDs.
    pub fn new(server_ids: Vec<String>) -> Self {
        Self {
            server_ids,
            open_uris: Arc::new(DashMap::new()),
            doc_versions: Arc::new(DashMap::new()),
        }
    }

    /// Record that didOpen was processed for a URI.
    pub fn record_did_open(&self, uri: &str) {
        self.open_uris.insert(uri.to_string(), ());
    }

    /// Record that didClose was processed for a URI.
    pub fn record_did_close(&self, uri: &str) {
        self.open_uris.remove(uri);
        self.doc_versions.remove(uri);
    }

    /// Returns true if the URI has been opened (didOpen recorded).
    pub fn is_open(&self, uri: &str) -> bool {
        self.open_uris.contains_key(uri)
    }

    /// Returns true if a didChange can be dispatched for this URI
    /// (i.e., didOpen has been confirmed for all children).
    pub fn can_did_change(&self, uri: &str) -> bool {
        self.is_open(uri)
    }

    /// Record the document version seen for a URI.
    pub fn record_version(&self, uri: &str, version: u32) {
        self.doc_versions.insert(uri.to_string(), version);
    }

    /// Returns true if the incoming version is a regression (lower than recorded).
    pub fn check_version_regression(&self, uri: &str, incoming_version: u32) -> bool {
        match self.doc_versions.get(uri) {
            None => false,
            Some(recorded) => *recorded > incoming_version,
        }
    }
}
