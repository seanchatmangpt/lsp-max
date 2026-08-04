#[cfg(feature = "gen")]
pub mod gen;

#[cfg(feature = "client")]
pub use lsp_max::client;

/// Real LSP→MCP pull-bridge read path (`lsp-max-mcp`'s fitness-file read), extracted so
/// it is directly callable from tests instead of trapped as a private `fn` in `bin/`.
pub mod mcp_bridge;
