//! wasm-lsp server entrypoint — CANDIDATE
//!
//! This binary wires the standard stdio Server used by native hosts (editors,
//! CI, agents).  For a WebAssembly target the transport is replaced by
//! `WasmTransport` (see transport.rs); the binary itself is native-only.
//!
//! WASM cross-compilation note:
//!   cargo build --target wasm32-unknown-unknown
//!
//!   This crate's lib.rs (backend, transport, capabilities) is target-agnostic
//!   and compiles for wasm32.  Only *this* file (main.rs) is native-only
//!   because tokio::io::stdin / tokio::io::stdout are not available on wasm32.
//!   In a real WebWorker integration, the JS host would instantiate the WASM
//!   module and bridge messages through WasmTransportHandle instead.

use lsp_max::{LspService, Server};
use wasm_lsp::Backend;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt().init();

    let (service, socket) = LspService::new(Backend::new);

    // tokio::io::stdin / stdout require the "io-std" feature, declared in
    // Cargo.toml per CLAUDE.md guidance for crates that depend on lsp-max.
    let _ = Server::new(tokio::io::stdin(), tokio::io::stdout(), socket)
        .serve(service)
        .await;
}
