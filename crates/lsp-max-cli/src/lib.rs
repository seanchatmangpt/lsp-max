#[cfg(feature = "gen")]
pub mod gen;

#[cfg(feature = "client")]
pub use lsp_max::client;
