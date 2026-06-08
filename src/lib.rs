//! Language Server Protocol (LSP) server abstraction for [Tower].
//!
//! [Tower]: https://github.com/tower-rs/tower
//!
//! # Example
//!
//! ```rust,no_run
//! use tower_lsp_max::jsonrpc::Result;
//! use tower_lsp_max::lsp_types_max::*;
//! use tower_lsp_max::{Client, LanguageServer, LspService, Server};
//!
//! #[derive(Debug)]
//! struct Backend {
//!     client: Client,
//! }
//!
//! #[tower_lsp_max::async_trait]
//! impl LanguageServer for Backend {
//!     async fn initialize(&self, _: InitializeParams) -> Result<InitializeResult> {
//!         Ok(InitializeResult {
//!             capabilities: ServerCapabilities {
//!                 hover_provider: Some(HoverProviderCapability::Simple(true)),
//!                 completion_provider: Some(CompletionOptions::default()),
//!                 ..Default::default()
//!             },
//!             ..Default::default()
//!         })
//!     }
//!
//!     async fn initialized(&self, _: InitializedParams) {
//!         self.client
//!             .log_message(MessageType::INFO, "server initialized!")
//!             .await;
//!     }
//!
//!     async fn shutdown(&self) -> Result<()> {
//!         Ok(())
//!     }
//!
//!     async fn completion(&self, _: CompletionParams) -> Result<Option<CompletionResponse>> {
//!         Ok(Some(CompletionResponse::Array(vec![
//!             CompletionItem::new_simple("Hello".to_string(), "Some detail".to_string()),
//!             CompletionItem::new_simple("Bye".to_string(), "More detail".to_string())
//!         ])))
//!     }
//!
//!     async fn hover(&self, _: HoverParams) -> Result<Option<Hover>> {
//!         Ok(Some(Hover {
//!             contents: HoverContents::Scalar(
//!                 MarkedString::String("You're hovering!".to_string())
//!             ),
//!             range: None
//!         }))
//!     }
//! }
//!
//! #[tokio::main]
//! async fn main() {
//! #   tracing_subscriber::fmt().init();
//! #
//! #   #[cfg(all(feature = "runtime-agnostic", not(feature = "runtime-tokio")))]
//! #   use tokio_util::compat::{TokioAsyncReadCompatExt, TokioAsyncWriteCompatExt};
//! #   use std::io::Cursor;
//!     let stdin = tokio::io::stdin();
//!     let stdout = tokio::io::stdout();
//! #   let message = r#"{"jsonrpc":"2.0","method":"exit"}"#;
//! #   let (stdin, stdout) = (Cursor::new(format!("Content-Length: {}\r\n\r\n{}", message.len(), message).into_bytes()), Cursor::new(Vec::new()));
//! #   #[cfg(all(feature = "runtime-agnostic", not(feature = "runtime-tokio")))]
//! #   let (stdin, stdout) = (stdin.compat(), stdout.compat_write());
//!
//!     let (service, socket) = LspService::new(|client| Backend { client });
//!     let _ = Server::new(stdin, stdout, socket).serve(service).await;
//! }
//! ```

#![deny(missing_debug_implementations)]
#![deny(missing_docs)]
#![deny(unsafe_code)]
#![allow(clippy::mutable_key_type)]

pub extern crate lsp_types_max;
pub use lsp_types_max as lsp_types;

pub extern crate tower_lsp_max_agent as max_agent;
pub extern crate tower_lsp_max_protocol as max_protocol;
pub extern crate tower_lsp_max_runtime as max_runtime;
use url::Url;

/// A re-export of [`async-trait`](https://docs.rs/async-trait) for convenience.
pub use async_trait::async_trait;

pub use self::service::progress::{
    Bounded, Cancellable, NotCancellable, OngoingProgress, Progress, Unbounded,
};
pub use self::service::{Client, ClientSocket, ExitedError, LspService, LspServiceBuilder};
pub use self::transport::{Loopback, Server};

use auto_impl::auto_impl;
use lsp_types_max::request::{
    GotoDeclarationParams, GotoDeclarationResponse, GotoImplementationParams,
    GotoImplementationResponse, GotoTypeDefinitionParams, GotoTypeDefinitionResponse,
};
use lsp_types_max::*;
use serde_json::Value;
use std::str::FromStr;
use tower_lsp_max_macros::rpc;
use tracing::{error, warn};

use self::jsonrpc::{Error, Result};

/// Semantic parsing and incremental AST generation bridged from `auto-lsp`.
pub mod auto_lsp {
    pub use auto_lsp_adapter::*;
}

pub mod jsonrpc;

mod codec;
pub mod service;
mod transport;
/// Module containing the `LanguageServer` trait and its macro-generated router.
pub mod language_server;
pub use language_server::LanguageServer;
pub(crate) use language_server::generated;

pub use tower_lsp_max_lsif as lsif;

mod composition;
pub use composition::{ComposedServer, SourceHealth, CompositionState, SharedCompositionState};


/// Module containing validation gate logic.
pub mod gate;
/// Module containing diagnostic update and management functions.
pub mod diagnostics;
/// Module containing helper functions to apply workspace and text edits.
pub mod workspace_edit;

use gate::run_gate_logic;
use workspace_edit::apply_workspace_edit;
pub(crate) use diagnostics::update_diagnostics;

fn _assert_object_safe() {
    fn assert_impl<T: LanguageServer>() {}
    assert_impl::<Box<dyn LanguageServer>>();
}

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

/// Record representing a snapshot of the server state.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[allow(dead_code)]
pub struct SnapshotRecord {
    /// The unique identifier of this snapshot.
    #[allow(dead_code)]
    pub id: max_protocol::SnapshotId,
    /// The capability vector at the time of snapshot.
    pub capability_vector: max_protocol::MaxCapabilityVector,
    /// Diagnostics present during the snapshot.
    pub diagnostics: Vec<max_protocol::MaxDiagnostic>,
    /// Actions generated/available for the snapshot.
    pub actions: Vec<max_protocol::MaxCodeAction>,
    /// The conformance vector of the server.
    pub conformance_vector: max_protocol::ConformanceVector,
    /// Receipts associated with the snapshot.
    pub receipts: Vec<max_protocol::Receipt>,
}

/// Registry storing server diagnostic and capability state.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct ServerRegistry {
    /// Client capabilities negotiated during initialization.
    pub client_capabilities: Option<ClientCapabilities>,
    /// Server capabilities returned during initialization.
    pub server_capabilities: Option<ServerCapabilities>,
    /// Table mapping diagnostic IDs to diagnostics.
    pub diagnostics: HashMap<String, max_protocol::MaxDiagnostic>,
    /// Table mapping file/resource paths to lists of repair code actions.
    pub repair_plans: HashMap<String, Vec<max_protocol::MaxCodeAction>>,
    /// Autonomic capability gates.
    pub gates: HashMap<String, bool>,
    /// Table mapping receipt IDs to receipt data.
    pub receipts: HashMap<String, max_protocol::Receipt>,
    /// Table mapping snapshot IDs to snapshot records.
    pub snapshots: HashMap<String, SnapshotRecord>,
    /// Table of cleared diagnostic IDs.
    #[serde(default)]
    pub cleared_diagnostics: std::collections::HashSet<String>,
    /// Current server lifecycle phase state.
    pub current_state: crate::service::State,
    /// Table mapping document URIs to their current versions.
    pub document_versions: HashMap<url::Url, i32>,
    /// Root path for gate and diagnostic file resolution.
    pub root_path: std::path::PathBuf,
    /// Monotonically-increasing counter incremented on every release actuation.
    /// Serves as a since-cursor for `max/conformanceDelta` polling.
    #[serde(default)]
    pub action_seq: u64,
    /// Ring-buffer of recent conformance score changes keyed by sequence number.
    /// The single authoritative conformance-delta store; replaces the former MESH global.
    #[serde(default)]
    pub conformance_delta_log: std::collections::VecDeque<max_runtime::ConformanceDeltaEntry>,
}

/// Global static instance of the server registry.
pub static REGISTRY: OnceLock<Mutex<ServerRegistry>> = OnceLock::new();

/// Global static instance of the autonomic mesh, used by RPC bridge methods.
pub static MESH: OnceLock<Mutex<max_runtime::AutonomicMesh>> = OnceLock::new();

/// Retrieves a reference to the global server registry.
pub fn get_registry() -> &'static Mutex<ServerRegistry> {
    REGISTRY.get_or_init(|| {
        let diagnostics = HashMap::new();
        let repair_plans = HashMap::new();
        let mut gates = HashMap::new();
        gates.insert("gate-state-check".to_string(), false);

        Mutex::new(ServerRegistry {
            client_capabilities: None,
            server_capabilities: None,
            diagnostics,
            repair_plans,
            gates,
            receipts: HashMap::new(),
            snapshots: HashMap::new(),
            cleared_diagnostics: std::collections::HashSet::new(),
            current_state: crate::service::State::Uninitialized,
            document_versions: HashMap::new(),
            root_path: std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from(".")),
            action_seq: 0,
            conformance_delta_log: std::collections::VecDeque::new(),
        })
    })
}

pub(crate) fn lock_registry() -> Result<std::sync::MutexGuard<'static, ServerRegistry>> {
    get_registry().lock().map_err(|_| Error::internal_error())
}

#[allow(dead_code)]
fn lock_mesh() -> Result<std::sync::MutexGuard<'static, max_runtime::AutonomicMesh>> {
    MESH.get_or_init(|| Mutex::new(max_runtime::AutonomicMesh::new()))
        .lock()
        .map_err(|_| Error::internal_error())
}

/// Reset the global registry to a fresh state.
/// Exposed as a public function for integration tests to prevent shared-state pollution.
pub fn reset_registry_for_tests() {
    if let Ok(mut reg) = get_registry().lock() {
        reg.client_capabilities = None;
        reg.server_capabilities = None;
        reg.diagnostics.clear();
        reg.repair_plans.clear();
        reg.gates.clear();
        reg.gates.insert("gate-state-check".to_string(), false);
        reg.receipts.clear();
        reg.snapshots.clear();
        reg.cleared_diagnostics.clear();
        reg.current_state = crate::service::State::Uninitialized;
        reg.document_versions.clear();
        reg.root_path = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
        reg.action_seq = 0;
        reg.conformance_delta_log.clear();
    }
}

pub(crate) fn sha256(data: &[u8]) -> String {
    let mut h = [
        0x6a09e667u32,
        0xbb67ae85u32,
        0x3c6ef372u32,
        0xa54ff53au32,
        0x510e527fu32,
        0x9b05688cu32,
        0x1f83d9abu32,
        0x5be0cd19u32,
    ];
    let k = [
        0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4,
        0xab1c5ed5, 0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe,
        0x9bdc06a7, 0xc19bf174, 0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f,
        0x4a7484aa, 0x5cb0a9dc, 0x76f988da, 0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7,
        0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967, 0x27b70a85, 0x2e1b2138, 0x4d2c6dfc,
        0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85, 0xa2bfe8a1, 0xa81a664b,
        0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070, 0x19a4c116,
        0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
        0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7,
        0xc67178f2,
    ];

    let mut padded = data.to_vec();
    let bit_len = (data.len() as u64) * 8;
    padded.push(0x80);
    while (padded.len() + 8) % 64 != 0 {
        padded.push(0);
    }
    padded.extend_from_slice(&bit_len.to_be_bytes());

    for chunk in padded.chunks_exact(64) {
        let mut w = [0u32; 64];
        for i in 0..16 {
            w[i] = u32::from_be_bytes([
                chunk[i * 4],
                chunk[i * 4 + 1],
                chunk[i * 4 + 2],
                chunk[i * 4 + 3],
            ]);
        }
        for i in 16..64 {
            let s0 = w[i - 15].rotate_right(7) ^ w[i - 15].rotate_right(18) ^ (w[i - 15] >> 3);
            let s1 = w[i - 2].rotate_right(17) ^ w[i - 2].rotate_right(19) ^ (w[i - 2] >> 10);
            w[i] = w[i - 16]
                .wrapping_add(s0)
                .wrapping_add(w[i - 7])
                .wrapping_add(s1);
        }

        let mut a = h[0];
        let mut b = h[1];
        let mut c = h[2];
        let mut d = h[3];
        let mut e = h[4];
        let mut f = h[5];
        let mut g = h[6];
        let mut h_val = h[7];

        for i in 0..64 {
            let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let ch = (e & f) ^ ((!e) & g);
            let temp1 = h_val
                .wrapping_add(s1)
                .wrapping_add(ch)
                .wrapping_add(k[i])
                .wrapping_add(w[i]);
            let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let maj = (a & b) ^ (a & c) ^ (b & c);
            let temp2 = s0.wrapping_add(maj);

            h_val = g;
            g = f;
            f = e;
            e = d.wrapping_add(temp1);
            d = c;
            c = b;
            b = a;
            a = temp1.wrapping_add(temp2);
        }

        h[0] = h[0].wrapping_add(a);
        h[1] = h[1].wrapping_add(b);
        h[2] = h[2].wrapping_add(c);
        h[3] = h[3].wrapping_add(d);
        h[4] = h[4].wrapping_add(e);
        h[5] = h[5].wrapping_add(f);
        h[6] = h[6].wrapping_add(g);
        h[7] = h[7].wrapping_add(h_val);
    }

    let mut result = String::new();
    for &val in &h {
        result.push_str(&format!("{:08x}", val));
    }
    result
}
