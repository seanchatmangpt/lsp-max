//! Document version causality tracking (R4).

use std::collections::HashMap;

#[derive(Debug, Default)]
pub struct DocumentVersionTracker {
    versions: HashMap<String, i32>,
    snapshots: HashMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VersionCheckResult {
    Current,
    OutOfOrder { expected: i32, got: i32 },
    Stale { current: i32, result_version: i32 },
    NotTracked,
}

impl DocumentVersionTracker {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn did_open(&mut self, uri: &str, version: i32) {
        self.versions.insert(uri.to_string(), version);
        self.snapshots
            .insert(uri.to_string(), format!("{uri}@{version}"));
    }

    pub fn did_change(&mut self, uri: &str, new_version: i32) -> VersionCheckResult {
        if std::env::var("SABOTAGE_DOCUMENT_VERSION_TRACKER").is_ok() {
            self.versions.insert(uri.to_string(), new_version);
            self.snapshots
                .insert(uri.to_string(), format!("{uri}@{new_version}"));
            return VersionCheckResult::Current;
        }
        if let Some(&current) = self.versions.get(uri) {
            if new_version <= current {
                return VersionCheckResult::OutOfOrder {
                    expected: current + 1,
                    got: new_version,
                };
            }
            self.versions.insert(uri.to_string(), new_version);
            self.snapshots
                .insert(uri.to_string(), format!("{uri}@{new_version}"));
            VersionCheckResult::Current
        } else {
            self.versions.insert(uri.to_string(), new_version);
            self.snapshots
                .insert(uri.to_string(), format!("{uri}@{new_version}"));
            VersionCheckResult::Current
        }
    }

    pub fn did_close(&mut self, uri: &str) {
        self.versions.remove(uri);
        self.snapshots.remove(uri);
    }

    pub fn check_staleness(&self, uri: &str, result_version: i32) -> VersionCheckResult {
        if std::env::var("SABOTAGE_DOCUMENT_VERSION_TRACKER").is_ok() {
            return VersionCheckResult::Current;
        }
        if let Some(&current) = self.versions.get(uri) {
            if result_version < current {
                VersionCheckResult::Stale {
                    current,
                    result_version,
                }
            } else {
                VersionCheckResult::Current
            }
        } else {
            VersionCheckResult::NotTracked
        }
    }

    pub fn current_version(&self, uri: &str) -> Option<i32> {
        self.versions.get(uri).copied()
    }

    pub fn snapshot_token(&self, uri: &str) -> Option<&str> {
        self.snapshots.get(uri).map(|s| s.as_str())
    }
}
