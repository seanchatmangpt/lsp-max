pub mod auto_generated;
/// LSIF 0.6.0 protocol coverage matrix for `LsifBuilder`.
pub mod coverage;
pub mod lsif;
pub mod lsif_builder;
pub mod lsif_indexer;
pub mod lsif_reader;
pub mod lsif_types;
pub use coverage::{lsif_coverage, LsifCoverageReport};
