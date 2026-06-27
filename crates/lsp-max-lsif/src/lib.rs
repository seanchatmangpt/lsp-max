/// LSIF 0.6.0 protocol coverage matrix for `LsifBuilder`.
pub mod coverage;
pub mod db;
pub mod lsif;
pub mod lsif_builder;
pub mod lsif_indexer;
pub mod lsif_reader;
pub mod lsif_types;

pub use coverage::{lsif_coverage, LsifCoverageReport};

pub mod linker;

#[cfg(feature = "rust")]
pub mod indexer_rust;

#[cfg(feature = "typescript")]
pub mod indexer_typescript;
