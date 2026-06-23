//! `meta_reasoning` — Meta Reasoner breed (Cox 2005, "Metacognition in Computation").
//!
//! Family: MetaCognition
//! Paper: `cox2005metacognition`
//! Oracle value: 0.0
//!
//! Status: CANDIDATE
//!
//! Implements strategy selection via introspection → evaluation → selection → monitoring.
//! Score formula: score = (utility / sqrt(cost)) * (1 + (1 - confidence))

use crate::breeds::breed::{BreedInput, CognitiveBreed};

pub struct MetaReasoning;

struct Strategy<'a> {
    name: &'a str,
    cost: f64,
    utility: f64,
    requires: Vec<&'a str>,
}


struct CognitiveState {
    confidence: f64,
    time_elapsed: f64,
}

struct Resources {
    time_budget: f64,
    available_knowledge: Vec<String>,
}

fn default_strategies() -> Vec<serde_json::Value> {
    serde_json::json!([
        {"name": "depth_first_search",   "cost": 8,  "utility": 0.9, "requires": []},
        {"name": "breadth_first_search", "cost": 12, "utility": 0.7, "requires": []},
        {"name": "heuristic_search",     "cost": 3,  "utility": 0.6, "requires": ["domain_knowledge"]},
        {"name": "random_restart",       "cost": 5,  "utility": 0.4, "requires": []}
    ])
    .as_array()
    .unwrap()
    .clone()
}

fn default_cognitive_state() -> serde_json::Value {
    serde_json::json!({
        "confidence": 0.3,
        "knowledge_gaps": ["network", "disk"],
        "time_elapsed": 5
    })
}

fn default_resources() -> serde_json::Value {
    serde_json::json!({
        "time_budget": 20,
        "available_knowledge": ["network"]
    })
}

impl CognitiveBreed for MetaReasoning {
    fn breed_id(&self) -> &'static str {
        "meta_reasoning"
    }

    fn run(&self, input: &BreedInput) -> Option<serde_json::Value> {
        // Parse or apply defaults for each field independently.
        let payload = &input.payload;

        let cog_val = payload
            .get("cognitive_state")
            .cloned()
            .unwrap_or_else(default_cognitive_state);

        let res_val = payload
            .get("resources")
            .cloned()
            .unwrap_or_else(default_resources);

        let strat_array: Vec<serde_json::Value> = payload
            .get("strategies")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_else(default_strategies);

        let cog_state = CognitiveState {
            confidence: cog_val
                .get("confidence")
                .and_then(|v| v.as_f64())
                .unwrap_or(0.5),
            time_elapsed: cog_val
                .get("time_elapsed")
                .and_then(|v| v.as_f64())
                .unwrap_or(0.0),
        };

        let resources = Resources {
            time_budget: res_val
                .get("time_budget")
                .and_then(|v| v.as_f64())
                .unwrap_or(f64::INFINITY),
            available_knowledge: res_val
                .get("available_knowledge")
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|s| s.as_str().map(String::from))
                        .collect()
                })
                .unwrap_or_default(),
        };

        let remaining_time = resources.time_budget - cog_state.time_elapsed;

        // Parse strategy descriptors.
        let strategies: Vec<Strategy> = strat_array
            .iter()
            .filter_map(|s| {
                let name = s.get("name")?.as_str()?;
                let cost = s.get("cost")?.as_f64()?;
                let utility = s.get("utility")?.as_f64()?;
                let requires: Vec<&str> = s
                    .get("requires")
                    .and_then(|v| v.as_array())
                    .map(|arr| arr.iter().filter_map(|r| r.as_str()).collect())
                    .unwrap_or_default();
                Some(Strategy { name, cost, utility, requires })
            })
            .collect();

        // Filter into feasible / rejected sets: (strategy, base_score, rank_score).
        let mut feasible: Vec<(&Strategy, f64, f64)> = Vec::new();
        let mut rejection_reasons: std::collections::HashMap<String, String> =
            std::collections::HashMap::new();

        for s in &strategies {
            let missing_knowledge: Vec<&str> = s
                .requires
                .iter()
                .filter(|req| !resources.available_knowledge.iter().any(|k| k == *req))
                .copied()
                .collect();

            if !missing_knowledge.is_empty() {
                let reason = format!(
                    "requires {} not available",
                    missing_knowledge.join(", ")
                );
                rejection_reasons.insert(s.name.to_string(), reason);
                continue;
            }

            if s.cost > remaining_time {
                rejection_reasons.insert(
                    s.name.to_string(),
                    format!("cost {:.0} exceeds remaining time {:.0}", s.cost, remaining_time),
                );
                continue;
            }

            // Ranking score: utility / sqrt(cost), boosted by low confidence (broader exploration).
            // base_score is reported in output; the boost affects selection ranking only.
            let base_score = s.utility / s.cost.sqrt();
            let confidence_boost = 1.0 + (1.0 - cog_state.confidence);
            let rank_score = base_score * confidence_boost;
            feasible.push((s, base_score, rank_score));
        }

        if feasible.is_empty() {
            return Some(serde_json::json!({
                "selected_strategy": null,
                "score": null,
                "rationale": "no feasible strategy within constraints",
                "feasible_strategies": [],
                "rejected_strategies": strategies.iter().map(|s| s.name).collect::<Vec<_>>(),
                "rejection_reasons": rejection_reasons
            }));
        }

        // Selection: highest rank_score (boosted); report base_score in output.
        let (best, best_base, _) = feasible
            .iter()
            .max_by(|a, b| a.2.partial_cmp(&b.2).unwrap())
            .unwrap();

        let feasible_names: Vec<&str> = feasible.iter().map(|(s, _, _)| s.name).collect();
        let rejected_names: Vec<&str> = rejection_reasons.keys().map(|s| s.as_str()).collect();

        Some(serde_json::json!({
            "selected_strategy": best.name,
            "score": ((best_base * 1000.0).round() / 1000.0),
            "rationale": "highest utility/cost ratio within time budget",
            "feasible_strategies": feasible_names,
            "rejected_strategies": rejected_names,
            "rejection_reasons": rejection_reasons
        }))
    }
}
