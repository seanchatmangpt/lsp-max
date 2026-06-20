pub mod context;
pub mod engine;
pub mod error;
pub mod generator;
pub mod generators;
pub mod manifest;
pub mod receipt_generator;
pub mod registry;
pub mod testmatrix_generator;

pub use context::GeneratorContext;
pub use engine::GeneratorEngine;
pub use error::GeneratorError;
pub use generator::{GeneratedFile, Generator, WriteMode};
pub use manifest::GenManifest;
pub use receipt_generator::ReceiptGenerator;
pub use registry::GeneratorRegistry;
pub use testmatrix_generator::TestMatrixGenerator;

pub mod ggen_adapter;
pub use ggen_adapter::{GgenAdapter, SyncReport, ValidationReport};
