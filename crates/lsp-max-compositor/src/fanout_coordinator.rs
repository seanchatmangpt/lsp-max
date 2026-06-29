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
    /// CANDIDATE: create a new FanoutCoordinator for the given set of child server IDs.
    pub fn new(server_ids: Vec<String>) -> Self {
        chicago_tdd_tools::scaffold!(
            ticket = "docs/jira/v26.6.30/CC-004-notification-routing.md",
            test   = "tests/chicago/cc_004_fanout.rs",
        )
    }

    /// CANDIDATE: record that didOpen was processed for a URI.
    pub fn record_did_open(&self, uri: &str) {
        chicago_tdd_tools::scaffold!(
            ticket = "docs/jira/v26.6.30/CC-004-notification-routing.md",
            test   = "tests/chicago/cc_004_fanout.rs",
        )
    }

    /// CANDIDATE: record that didClose was processed for a URI.
    pub fn record_did_close(&self, uri: &str) {
        chicago_tdd_tools::scaffold!(
            ticket = "docs/jira/v26.6.30/CC-004-notification-routing.md",
            test   = "tests/chicago/cc_004_fanout.rs",
        )
    }

    /// CANDIDATE: returns true if the URI has been opened (didOpen recorded).
    pub fn is_open(&self, uri: &str) -> bool {
        chicago_tdd_tools::scaffold!(
            ticket = "docs/jira/v26.6.30/CC-004-notification-routing.md",
            test   = "tests/chicago/cc_004_fanout.rs",
        )
    }

    /// CANDIDATE: returns true if a didChange can be dispatched for this URI
    /// (i.e., didOpen has been confirmed for all children).
    pub fn can_did_change(&self, uri: &str) -> bool {
        chicago_tdd_tools::scaffold!(
            ticket = "docs/jira/v26.6.30/CC-004-notification-routing.md",
            test   = "tests/chicago/cc_004_fanout.rs",
        )
    }

    /// CANDIDATE: record the document version seen for a URI.
    pub fn record_version(&self, uri: &str, version: u32) {
        chicago_tdd_tools::scaffold!(
            ticket = "docs/jira/v26.6.30/CC-004-notification-routing.md",
            test   = "tests/chicago/cc_004_fanout.rs",
        )
    }

    /// CANDIDATE: returns true if the incoming version is a regression (lower than recorded).
    pub fn check_version_regression(&self, uri: &str, incoming_version: u32) -> bool {
        chicago_tdd_tools::scaffold!(
            ticket = "docs/jira/v26.6.30/CC-004-notification-routing.md",
            test   = "tests/chicago/cc_004_fanout.rs",
        )
    }
}
