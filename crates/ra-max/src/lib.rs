//! Executable first vertical slice of RFC 0006.
//!
//! `ra-max` separates admitted observation, semantic construction, actuation,
//! and receipt/replay evidence. The current semantic implementation is a
//! deliberately bounded Tree-sitter Rust engine with cross-file lexical
//! intelligence. It does not claim parity with rust-analyzer HIR; ambiguous
//! Rust scope and type questions are refused instead of guessed.

pub mod broker;
pub mod demo;
pub mod differential;
pub mod identity;
pub mod intelligence;
pub mod receipt;
pub mod semantic;
pub mod server;

pub use broker::{ActuationBroker, ConstructedEdit, ToolExecution, WorkspaceState};
pub use demo::{run_demo, DemoReport};
pub use differential::{compare_engines, DifferentialReport};
pub use identity::{ProjectAdmission, SemanticSubject};
pub use intelligence::{
    CompletionCandidate, HoverInfo, IntelligenceError, Occurrence, RenamePlan, TextReplacement,
    WorkspaceIndex,
};
pub use receipt::{Outcome, Receipt, ReceiptChain, ReceiptKind};
pub use semantic::{
    SemanticDiagnostic, SemanticEngine, SemanticSnapshot, SourceRange, Symbol,
    TreeSitterRustEngine,
};
pub use server::RaMaxServer;

/// Exact `lsp-max` source identity used when RFC 0006 was manufactured.
pub const LSP_MAX_SOURCE_SHA: &str = "3c3e559347ce261a71435c98f34a0c171f51d592";

/// Exact rust-analyzer semantic reference admitted by RFC 0006.
pub const RUST_ANALYZER_SOURCE_SHA: &str = "5f258f4534e3b4bdaa45a1299b53a66cf014d803";
