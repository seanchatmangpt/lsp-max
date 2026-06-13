// Child connection tracking for lsp-max-compositor.
//
// ChildConnections records which child servers have been notified about which
// URIs. Actual IPC / subprocess launching is deferred; this module owns the
// routing book-keeping so the fanout path is wired and testable independently
// of process lifecycle.

use dashmap::DashMap;

/// Tracks which child server ids have been notified about each document URI.
#[derive(Debug, Default)]
pub struct ChildConnections {
    /// server_id → set of URIs that server has been notified about
    by_server: DashMap<String, Vec<String>>,
    /// uri → set of server ids that received the notification
    by_uri: DashMap<String, Vec<String>>,
}

impl ChildConnections {
    pub fn new() -> Self {
        Self::default()
    }

    /// Record that `server_id` was notified about `uri`.
    pub fn record_notification(&self, server_id: &str, uri: &str) {
        self.by_server
            .entry(server_id.to_string())
            .or_default()
            .push(uri.to_string());
        self.by_uri
            .entry(uri.to_string())
            .or_default()
            .push(server_id.to_string());
    }

    /// Return all server ids that have been notified about `uri`.
    pub fn notified_servers(&self, uri: &str) -> Vec<String> {
        self.by_uri.get(uri).map(|v| v.clone()).unwrap_or_default()
    }

    /// Return all URIs that `server_id` has been notified about.
    pub fn uris_for_server(&self, server_id: &str) -> Vec<String> {
        self.by_server
            .get(server_id)
            .map(|v| v.clone())
            .unwrap_or_default()
    }
}
