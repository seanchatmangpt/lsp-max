#![doc = include_str!("../README.md")]

pub mod config;
pub mod diagnostics;
pub mod engine;
pub mod observations;
pub mod parsers;
pub mod rules;

pub use diagnostics::AntiLlmDiagnostic;
pub use engine::{
    evaluate_diagnostics, evaluate_diagnostics_with_config, observations_to_ocel, scan_file,
};
pub use observations::Observation;
