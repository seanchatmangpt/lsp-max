use serde::{Deserialize, Serialize};

/// Sent from FlushCoordinator → child server after a successful
/// publishDiagnostics push. Tells the child which of its diagnostics
/// reached the editor and whether any were suppressed by a higher-tier
/// server.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiagnosticAck {
    /// URI that was flushed.
    pub uri: String,
    /// Total diagnostics from this server_id that survived the merge
    /// and reached the editor.
    pub admitted_count: usize,
    /// Total diagnostics from this server_id that were deposited but
    /// did NOT survive the merge (suppressed by higher tier or dedup).
    pub suppressed_count: usize,
    /// True if any of this server's diagnostics in the merge result
    /// were REFUSED_BY_LAW errors.
    pub has_andon_contribution: bool,
}
