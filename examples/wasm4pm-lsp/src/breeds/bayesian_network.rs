//! `bayesian_network` — Pearl alarm/burglary network (1988), variable elimination.
//!
//! Family: ProbabilisticAI
//! Paper: `pearl1988probabilistic`
//! Oracle value: 0.284
//!
//! Status: CANDIDATE

use crate::breeds::breed::{BreedInput, CognitiveBreed};

pub struct BayesianNetwork;

// P(B=T)
const P_BURGLARY: f64 = 0.001;
// P(E=T)
const P_EARTHQUAKE: f64 = 0.002;

fn p_alarm(b: bool, e: bool) -> f64 {
    match (b, e) {
        (true,  true)  => 0.95,
        (true,  false) => 0.94,
        (false, true)  => 0.29,
        (false, false) => 0.001,
    }
}

fn p_john_calls(a: bool) -> f64 {
    if a { 0.90 } else { 0.05 }
}

fn p_mary_calls(a: bool) -> f64 {
    if a { 0.70 } else { 0.01 }
}

fn joint_b_j_m(b: bool) -> f64 {
    let p_b = if b { P_BURGLARY } else { 1.0 - P_BURGLARY };
    let mut total = 0.0;
    for &e in &[true, false] {
        let p_e = if e { P_EARTHQUAKE } else { 1.0 - P_EARTHQUAKE };
        for &a in &[true, false] {
            let p_a = if a { p_alarm(b, e) } else { 1.0 - p_alarm(b, e) };
            total += p_b * p_e * p_a * p_john_calls(a) * p_mary_calls(a);
        }
    }
    total
}

impl CognitiveBreed for BayesianNetwork {
    fn breed_id(&self) -> &'static str {
        "bayesian_network"
    }

    fn run(&self, _input: &BreedInput) -> Option<serde_json::Value> {
        let p_true_jm  = joint_b_j_m(true);
        let p_false_jm = joint_b_j_m(false);
        let normaliser = p_true_jm + p_false_jm;
        let p_true  = p_true_jm  / normaliser;
        let p_false = p_false_jm / normaliser;
        Some(serde_json::json!({
            "query":   "Burglary",
            "true":    p_true,
            "false":   p_false,
            "network": "alarm"
        }))
    }
}
