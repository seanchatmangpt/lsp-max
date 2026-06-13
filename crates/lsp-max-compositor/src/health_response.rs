use serde::{Deserialize, Serialize};

/// Lightweight health snapshot for monitoring systems.
/// Does not require iterating diagnostic buffer — O(1) to produce.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompositorHealth {
    /// Number of child servers currently registered in the pool.
    pub child_server_count: usize,
    /// Server IDs registered in the pool.
    pub child_server_ids: Vec<String>,
    /// Number of URIs currently in the diagnostic buffer.
    pub buffered_uri_count: usize,
    /// True if any URI in the buffer has had a known ANDON block since last restart.
    /// This is a cached flag — it reflects the last flush result,
    /// not a fresh merge. Use compositor_state() for authoritative data.
    pub has_any_andon_block: bool,
}
