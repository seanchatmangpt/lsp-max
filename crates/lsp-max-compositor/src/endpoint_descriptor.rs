// endpoint_descriptor.rs — writes .claude/compositor-endpoint.json at compositor startup.
//
// Claude Code and discover-lsp-chains.sh read this file to substitute the compositor
// as the LSP binding target rather than connecting to child servers directly.

use serde::Serialize;
use std::path::PathBuf;

#[derive(Serialize)]
struct CompositorEndpoint {
    transport: &'static str,
    command: String,
    args: Vec<String>,
    pid: u32,
}

/// Write `.claude/compositor-endpoint.json` to the workspace root.
///
/// The file is best-effort: if the `.claude/` directory does not exist or is not writable,
/// the error is logged at warn level and startup continues.  The compositor is functional
/// regardless of whether the descriptor file was written.
pub fn write_endpoint_descriptor() {
    let descriptor = CompositorEndpoint {
        transport: "stdio",
        command: std::env::current_exe()
            .unwrap_or_else(|_| PathBuf::from("lsp-max-compositor"))
            .to_string_lossy()
            .into_owned(),
        args: vec![],
        pid: std::process::id(),
    };

    // Prefer the workspace root derived from the current directory.
    let target = PathBuf::from(".claude/compositor-endpoint.json");

    match serde_json::to_string_pretty(&descriptor) {
        Ok(json) => {
            if let Err(e) = std::fs::write(&target, &json) {
                tracing::warn!(
                    path = %target.display(),
                    error = %e,
                    "compositor: OPEN — endpoint descriptor write BLOCKED"
                );
            } else {
                tracing::info!(
                    path = %target.display(),
                    pid = descriptor.pid,
                    "compositor: endpoint descriptor CANDIDATE"
                );
            }
        }
        Err(e) => {
            tracing::warn!(error = %e, "compositor: endpoint descriptor serialization BLOCKED");
        }
    }
}
