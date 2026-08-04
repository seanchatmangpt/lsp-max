//! Real LSP→MCP pull-bridge read path, extracted from `bin/lsp-max-mcp.rs` so it is a
//! directly callable library function instead of only a private `fn` inside a `bin/`
//! crate. The bin's `read_fitness_status()` now delegates here — this is the exact code
//! path exercised by every `lsp_route`/`lsp_violations` MCP tool call, not a re-implementation.
//!
//! The compositor's `FlushCoordinator` writes `<workspace_root>/.claude/lsp-max-fitness.json`
//! on every flush (see `lsp-max-compositor/src/flush_coordinator.rs`, "Write fitness
//! snapshot so MCP bridge and gate-check.sh can read it"). This function is the read half
//! of that bridge.

use serde_json::{json, Value};
use std::path::Path;

/// Read the compositor's fitness snapshot from `<workspace_root>/.claude/lsp-max-fitness.json`.
///
/// Returns `{"law_status": "UNKNOWN"}` when the file is missing or unparsable — the same
/// fallback the bin used inline before this extraction.
pub fn read_fitness_status_at(workspace_root: &Path) -> Value {
    let path = workspace_root.join(".claude/lsp-max-fitness.json");
    std::fs::read_to_string(&path)
        .ok()
        .and_then(|s| serde_json::from_str::<Value>(&s).ok())
        .unwrap_or_else(|| json!({"law_status": "UNKNOWN"}))
}

/// Cwd-resolving wrapper — identical behavior to the bin's original `read_fitness_status()`.
pub fn read_fitness_status() -> Value {
    let workspace = std::env::current_dir().unwrap_or_default();
    read_fitness_status_at(&workspace)
}
