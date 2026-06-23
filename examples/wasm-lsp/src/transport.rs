//! WasmTransport — in-process channel transport for the WASM/WebWorker context
//! Law-status: CANDIDATE — WebWorker bridge receipt OPEN
//!
//! This module provides an mpsc-based in-process transport that mirrors the
//! message-passing model a WebWorker would use.  The native stdio Server wires
//! tokio::io::{stdin,stdout}; for a real WASM target those sinks are replaced
//! by the channel ends exposed through `WasmTransportHandle`.
//!
//! No `wasm-bindgen` or `js-sys` dependencies are introduced here — the goal
//! is a pure-Rust in-process harness that the host side (browser or test) drives
//! by sending raw byte frames through `WasmTransportHandle::send`.

use tokio::sync::mpsc;

/// Maximum number of pending frames the channel will buffer before back-pressure.
const CHANNEL_CAPACITY: usize = 64;

/// Law-status of this transport layer.
pub fn transport_law_status() -> &'static str {
    // CANDIDATE: WebWorker bridge receipt chain is OPEN.
    // Promote to "ADMITTED" only after transcript + negative control + receipt
    // artifact are attached and verified.
    "CANDIDATE"
}

/// The server-side half of the in-process WASM transport.
///
/// Holds the receiving end for frames arriving from the "browser side" and the
/// sending end for frames the LSP server emits back.  For a real wasm32 build
/// these channels would be bridged to `postMessage` / `onmessage`; here they
/// remain pure Rust.
pub struct WasmTransport {
    /// Frames inbound from the browser/host side.
    pub inbound: mpsc::Receiver<Vec<u8>>,
    /// Frames outbound to the browser/host side.
    pub outbound: mpsc::Sender<Vec<u8>>,
}

/// The "browser side" handle — what the host environment holds to drive the
/// transport.  In a real WebWorker integration the JS glue would wrap this.
pub struct WasmTransportHandle {
    /// Send frames into the LSP server.
    pub sender: mpsc::Sender<Vec<u8>>,
    /// Receive frames from the LSP server.
    pub receiver: mpsc::Receiver<Vec<u8>>,
}

impl WasmTransport {
    /// Construct the transport pair.
    ///
    /// Returns `(server_side, browser_side)`.  The caller wires `server_side`
    /// into the lsp-max `Server`; the caller hands `browser_side` to whatever
    /// drives the WebWorker channel.
    pub fn new() -> (Self, WasmTransportHandle) {
        let (browser_tx, server_rx) = mpsc::channel::<Vec<u8>>(CHANNEL_CAPACITY);
        let (server_tx, browser_rx) = mpsc::channel::<Vec<u8>>(CHANNEL_CAPACITY);

        let transport = WasmTransport {
            inbound: server_rx,
            outbound: server_tx,
        };

        let handle = WasmTransportHandle {
            sender: browser_tx,
            receiver: browser_rx,
        };

        (transport, handle)
    }
}

impl WasmTransportHandle {
    /// Push a raw LSP frame (JSON-RPC bytes) toward the server.
    ///
    /// Returns an error if the server-side channel has been dropped.
    pub async fn send(&self, frame: Vec<u8>) -> Result<(), mpsc::error::SendError<Vec<u8>>> {
        self.sender.send(frame).await
    }

    /// Receive the next frame emitted by the LSP server.
    ///
    /// Returns `None` when the server has shut down and the channel is closed.
    pub async fn recv(&mut self) -> Option<Vec<u8>> {
        self.receiver.recv().await
    }
}
