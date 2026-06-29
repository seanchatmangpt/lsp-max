// Routing-safe modules — no lsp_max, wasm4pm, or lsp_max_runtime imports; always compiled.
pub mod andon_snapshot;
pub mod config;
pub mod connections;
pub mod diagnostic_ack;
pub mod diagnostic_buffer;
pub mod dt_context;
pub mod fanout;
pub mod fanout_coordinator;
pub mod gate_cli_compat;
pub mod gate_file;
pub mod health_response;
pub mod merge;
pub mod receipt;
pub mod registry;
pub mod registry_init;
pub mod state_response;

// Heavy modules — require the optional lsp-max* and wasm4pm deps; gated behind `full`.
#[cfg(feature = "full")]
pub mod capability_merge;
#[cfg(feature = "full")]
pub mod child_process;
#[cfg(feature = "full")]
pub mod compositor_client;
#[cfg(feature = "full")]
pub mod declare;
#[cfg(feature = "full")]
pub mod dfg;
#[cfg(feature = "full")]
pub mod flush_coordinator;
#[cfg(feature = "full")]
pub mod mesh;
#[cfg(feature = "full")]
pub mod receipt_chain;
#[cfg(feature = "full")]
pub mod routing;
#[cfg(feature = "full")]
pub mod server;

// Re-exports from routing-safe modules (always available).
pub use andon_snapshot::AndonSnapshot;
pub use config::CompositorConfig;
pub use connections::ChildConnections;
pub use diagnostic_buffer::DiagnosticBuffer;
pub use dt_context::{AndonEvent, DtContext, DtContextStatus, GateContext, RepairAction};
pub use fanout_coordinator::FanoutCoordinator;
pub use gate_file::GateFile;
pub use merge::{MergeContext, MergeResult};
pub use registry::{ChildServer, ChildTier, ExtensionRouter};

// Re-exports from heavy modules (only available with `full` feature).
#[cfg(feature = "full")]
pub use compositor_client::CompositorClient;
#[cfg(feature = "full")]
pub use declare::{ConstraintViolation, DeclareModel};
#[cfg(feature = "full")]
pub use dfg::DirectlyFollowsGraph;
#[cfg(feature = "full")]
pub use flush_coordinator::FlushCoordinator;
#[cfg(feature = "full")]
pub use mesh::{MeshNode, MeshTopology, MeshTransport};
#[cfg(feature = "full")]
pub use routing::{RoutingDecision, RoutingStrategy, RoutingTable, ServerCapabilityDecl};
