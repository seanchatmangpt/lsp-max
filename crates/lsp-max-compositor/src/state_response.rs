use serde::{Deserialize, Serialize};

/// Per-URI ANDON state entry returned by max/compositorState.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UriAndonState {
    pub uri: String,
    pub has_andon_block: bool,
    /// Active ANDON codes (severity==1, REFUSED_BY_LAW).
    pub andon_codes: Vec<String>,
    /// Total diagnostic count in the buffer for this URI.
    pub diagnostic_count: usize,
}

/// Response for max/compositorState requests.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompositorStateResponse {
    /// All URIs currently in the diagnostic buffer.
    pub uris: Vec<UriAndonState>,
    /// True if ANY URI has has_andon_block == true.
    pub global_andon_block: bool,
    /// Number of child servers currently in the pool.
    pub child_server_count: usize,
    /// Unix timestamp (milliseconds since epoch) when this response was computed.
    /// CI gates can compare this against their own clock to verify freshness.
    pub query_timestamp_ms: u64,
}
