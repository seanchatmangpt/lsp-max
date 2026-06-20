//! wasm4pm evidence integration — coordinator module.
//!
//! Types and converters live in [`super::evidence_types`].
//! Oxigraph store extractors live in [`super::evidence_extractors`].

pub use super::evidence_extractors::*;
pub use super::evidence_types::*;

// Tests that exercise to_raw_evidence / workspace_to_admitted_evidence /
// range_to_admitted_evidence / diagnostic_to_admitted_evidence are BLOCKED:
// those helpers panic in stub mode because wasm4pm_compat is unavailable.
// The payload type construction paths are covered by evidence_types unit tests
// instead once wasm4pm_compat is present.
