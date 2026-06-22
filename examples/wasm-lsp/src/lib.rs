//! wasm-lsp — WebAssembly/WebWorker transport example for lsp-max
//! Law-status: CANDIDATE — receipt chain OPEN
//! Source of truth: schema/domain.ttl + gen.toml
//!
//! Feature gates:
//!   (none currently) — all modules compile for both native and wasm32 targets
//!   except main.rs, which is native-only (stdio wiring).

pub mod backend;
pub mod capabilities;
pub mod transport;

pub use backend::WasmLspBackend as Backend;
