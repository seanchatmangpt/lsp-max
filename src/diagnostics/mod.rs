pub mod cognition_laws;
mod engine;
pub(crate) mod law_table;

pub use cognition_laws::{audit_breeds, AuditSummary, BreedDiagnostic};
pub(crate) use engine::update_diagnostics;
