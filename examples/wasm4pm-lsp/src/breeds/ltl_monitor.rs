//! `ltl_monitor` — LTL Runtime Monitor breed stub.
//!
//! Family: FormalMethods
//! Paper: `bauer2011ltl`
//! Oracle value: 1.0
//!
//! Status: CANDIDATE

use wasm4pm_compat::{BreedInput, CognitiveBreed};

pub struct LtlMonitor;

impl CognitiveBreed for LtlMonitor {
    fn breed_id(&self) -> &'static str {
        "ltl_monitor"
    }

    fn run(&self, _input: &BreedInput) -> Option<serde_json::Value> {
        None
    }
}
