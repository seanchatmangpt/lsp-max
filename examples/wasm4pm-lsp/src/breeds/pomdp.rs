//! `pomdp` — POMDP Solver breed stub.
//!
//! Family: ReinforcementLearning
//! Paper: `kaelbling1998planning`
//! Oracle value: 0.969
//!
//! Status: CANDIDATE

use wasm4pm_compat::{BreedInput, CognitiveBreed};

pub struct Pomdp;

impl CognitiveBreed for Pomdp {
    fn breed_id(&self) -> &'static str {
        "pomdp"
    }

    fn run(&self, _input: &BreedInput) -> Option<serde_json::Value> {
        None
    }
}
