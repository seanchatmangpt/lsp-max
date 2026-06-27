//! `pomdp` — POMDP Solver breed via value iteration (Kaelbling et al. 1998).
//!
//! Family: ReinforcementLearning
//! Paper: `kaelbling1998planning`
//! Oracle value: 0.969
//!
//! Status: CANDIDATE
//!
//! Hardwired to the Tiger Problem (canonical benchmark):
//! S = {tiger-left, tiger-right}
//! A = {listen, open-left, open-right}
//! O = {hear-left, hear-right}

use crate::breeds::breed::{BreedInput, CognitiveBreed};

pub struct Pomdp;

// Tiger Problem parameters — Kaelbling 1998
const GAMMA: f64 = 0.95;
const STATES: usize = 2; // 0 = tiger-left, 1 = tiger-right
const ACTIONS: usize = 3; // 0 = listen, 1 = open-left, 2 = open-right

// R(s, a): reward matrix
fn reward(s: usize, a: usize) -> f64 {
    match (s, a) {
        (_, 0) => -1.0,   // listen always costs 1
        (0, 1) => -100.0, // open-left when tiger is left: found tiger
        (1, 1) => 10.0,   // open-left when tiger is right: escaped
        (0, 2) => 10.0,   // open-right when tiger is left: escaped
        (1, 2) => -100.0, // open-right when tiger is right: found tiger
        _ => 0.0,
    }
}

// T(s'|s, a): transition probability
fn transition(s: usize, a: usize, s_next: usize) -> f64 {
    match a {
        0 => {
            // listen: state unchanged
            if s_next == s {
                1.0
            } else {
                0.0
            }
        }
        _ => {
            // open-left or open-right: reset to uniform
            0.5
        }
    }
}

/// One step of value iteration; returns updated V[s] for all states.
fn value_iteration_step(v_prev: &[f64; STATES]) -> [f64; STATES] {
    let mut v_next = [0.0f64; STATES];
    for (s, cell) in v_next.iter_mut().enumerate() {
        let mut best_q = f64::NEG_INFINITY;
        for a in 0..ACTIONS {
            let r = reward(s, a);
            let mut future = 0.0;
            for (s_next, &prev) in v_prev.iter().enumerate() {
                future += transition(s, a, s_next) * prev;
            }
            let q = r + GAMMA * future;
            if q > best_q {
                best_q = q;
            }
        }
        *cell = best_q;
    }
    v_next
}

/// V(b) = sum_s b(s) * V(s) — linear value at belief point.
fn belief_value(belief: &[f64; STATES], v: &[f64; STATES]) -> f64 {
    belief.iter().zip(v.iter()).map(|(b, vi)| b * vi).sum()
}

/// Greedy action at belief b under value vector V.
fn best_action(belief: &[f64; STATES], v: &[f64; STATES]) -> &'static str {
    const ACTION_NAMES: [&str; ACTIONS] = ["listen", "open-left", "open-right"];
    let mut best_a = 0;
    let mut best_q = f64::NEG_INFINITY;

    for a in 0..ACTIONS {
        let r_b: f64 = belief
            .iter()
            .enumerate()
            .map(|(s, &b)| b * reward(s, a))
            .sum();
        let mut future = 0.0;
        for (s, &b_s) in belief.iter().enumerate() {
            for (s_next, &vi) in v.iter().enumerate() {
                future += b_s * transition(s, a, s_next) * vi;
            }
        }
        let q = r_b + GAMMA * future;
        if q > best_q {
            best_q = q;
            best_a = a;
        }
    }
    ACTION_NAMES[best_a]
}

fn run_tiger(iterations: usize) -> serde_json::Value {
    let belief = [0.5f64, 0.5f64];
    let mut v = [0.0f64; STATES];

    for _ in 0..iterations {
        v = value_iteration_step(&v);
    }

    let value = belief_value(&belief, &v);
    let action = best_action(&belief, &v);

    serde_json::json!({
        "problem": "tiger",
        "belief": [belief[0], belief[1]],
        "value": (value * 1000.0).round() / 1000.0,
        "iterations": iterations,
        "optimal_action": action
    })
}

impl CognitiveBreed for Pomdp {
    fn breed_id(&self) -> &'static str {
        "pomdp"
    }

    fn run(&self, input: &BreedInput) -> Option<serde_json::Value> {
        let iterations = input
            .get("iterations")
            .and_then(|v| v.as_u64())
            .unwrap_or(15) as usize;

        // Tiger problem is the hardwired canonical benchmark; custom problems not yet traced.
        Some(run_tiger(iterations))
    }
}
