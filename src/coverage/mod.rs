//! Protocol coverage matrices for LSP 3.18 and LSIF.
//!
//! This module hosts two sub-modules that independently measure how much of
//! each respective protocol the lsp-max implementation covers.  Both are
//! derived from authoritative specs rather than hand-maintained lists.
pub mod lsp_coverage;
pub use lsp_coverage::{lsp_coverage, CoverageReport, IMPLEMENTED_METHODS};
