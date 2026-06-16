//! # Transport Utilities: `ExitedError`, `ClientSocket`, and `Loopback`
//!
//! This example is **Reference** (Diataxis): it demonstrates the three minor
//! public transport utilities that are re-exported from the root `lsp_max` crate
//! but not exercised by any other run-to-exit example.
//!
//! ## What each type does
//!
//! ### `ExitedError(i32)`
//!
//! Returned by `LspService::poll_ready` and `Server::serve` once the language
//! server has exited. The inner `i32` is the exit status code: `0` on a lawful
//! shutdown (via `initialize`→`shutdown`→`exit`), `1` on abnormal termination
//! (e.g. stdin EOF before `exit`). The `.code()` accessor reads it without moving
//! the value.
//!
//! ### `ClientSocket`
//!
//! The loopback half returned by `LspService::new()`. It routes server-to-client
//! requests and client-to-server responses back across the bidirectional channel.
//! You pass it to `Server::new(stdin, stdout, socket)` so the transport can route
//! both halves through one struct.
//!
//! ### `Loopback`
//!
//! A trait that `ClientSocket` implements. Any type that can be split into a
//! `(RequestStream, ResponseSink)` pair qualifies. The trait is the abstraction
//! boundary that lets tests swap in a `MockLoopback` (as used in
//! `src/transport.rs` tests) without changing `Server`.
//!
//! ## Sources
//!
//! - `ExitedError` / `ClientSocket`: `src/service.rs`, `src/service/client/socket.rs`
//! - `Loopback` trait + `ClientSocket` impl: `src/transport.rs`
//! - See also: [`examples/transport_utilities_explained.rs`]

// Run: cargo run --example transport_utilities_explained
// Exit 0 on all assertions held; panics (non-zero exit) if any contract breaks.

use lsp_max::lsp_types_max::{InitializeParams, InitializeResult};
use lsp_max::{Client, ExitedError, LanguageServer, Loopback, LspService};
use lsp_max::jsonrpc::Result as RpcResult;

struct MinimalBackend;

#[lsp_max::async_trait]
impl LanguageServer for MinimalBackend {
    async fn initialize(&self, _: InitializeParams) -> RpcResult<InitializeResult> {
        Ok(InitializeResult::default())
    }
    async fn shutdown(&self) -> RpcResult<()> {
        Ok(())
    }
}

fn main() {
    // ------------------------------------------------------------------ [1]
    // ExitedError: exit code 0 differs from exit code 1, and .code() reads
    // the inner i32 without consuming the value.
    let ok_exit = ExitedError(0);
    let err_exit = ExitedError(1);
    assert_ne!(ok_exit, err_exit, "ExitedError(0) != ExitedError(1)");
    assert_eq!(ok_exit.code(), 0, "ok_exit.code() == 0");
    assert_eq!(err_exit.code(), 1, "err_exit.code() == 1");

    // ------------------------------------------------------------------ [2]
    // ExitedError: the inner i32 is accessible via the public field.
    let ExitedError(code) = err_exit.clone();
    assert_eq!(code, 1, "destructuring ExitedError gives the exit code");

    // ------------------------------------------------------------------ [3]
    // ClientSocket: obtained from LspService::new() as the second tuple element.
    // The socket is the loopback half; creating it is a synchronous, allocation-
    // only operation — no reactor required.
    lsp_max::reset_registry_for_tests();
    let (_service, socket) = LspService::new(|_client: Client| MinimalBackend);
    // The socket is Loopback — split() decomposes it into (RequestStream, ResponseSink).
    // Calling split() without polling is a non-panicking, non-blocking operation.
    let (_request_stream, _response_sink) = socket.split();

    // ------------------------------------------------------------------ [4]
    // Loopback trait bound: confirm ClientSocket satisfies Loopback at compile
    // time by passing it through a generic function bounded on Loopback.
    fn assert_loopback<L: Loopback>(_socket: L) {}
    let (_service2, socket2) = LspService::new(|_client: Client| MinimalBackend);
    assert_loopback(socket2);  // compiles only if ClientSocket: Loopback

    println!("WITNESS transport_utilities: 4 assertions held");
    println!("  [1] ExitedError(0) != ExitedError(1); .code() reads inner i32");
    println!("  [2] ExitedError inner i32 accessible via destructuring");
    println!("  [3] ClientSocket obtained from LspService::new(); split() is non-panicking");
    println!("  [4] ClientSocket satisfies the Loopback trait bound (compile-time)");
}
