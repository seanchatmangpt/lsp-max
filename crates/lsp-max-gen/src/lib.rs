pub mod context;
pub mod engine;
pub mod error;
pub mod generator;
pub mod generators;
pub mod manifest;
pub mod registry;

pub use context::GeneratorContext;
pub use engine::GeneratorEngine;
pub use error::GeneratorError;
pub use generator::{GeneratedFile, Generator, WriteMode};
pub use manifest::GenManifest;
pub use registry::GeneratorRegistry;

pub mod ggen_adapter;
pub use ggen_adapter::{GgenAdapter, SyncReport, ValidationReport};
