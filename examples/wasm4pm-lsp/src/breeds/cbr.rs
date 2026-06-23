//! `cbr` — Case-Based Reasoning breed.
//!
//! Implements the Retrieve phase of the CBR cycle from:
//! Kolodner, "Case-Based Reasoning", Morgan Kaufmann 1993.
//!
//! Similarity metric: weighted cosine similarity over numeric feature vectors.
//!
//! Family: SymbolicAI
//! Paper: `kolodner1993cbr`
//! Oracle value: 0.85
//! Status: CANDIDATE

use crate::breeds::breed::{BreedInput, CognitiveBreed};
use serde_json::Value;

pub struct Cbr;

fn feature_vec(features: &Value, keys: &[&str]) -> Vec<f64> {
    keys.iter()
        .map(|k| features.get(k).and_then(|v| v.as_f64()).unwrap_or(0.0))
        .collect()
}

fn weighted_cosine(q: &[f64], c: &[f64], w: &[f64]) -> f64 {
    let dot: f64 = q
        .iter()
        .zip(c.iter())
        .zip(w.iter())
        .map(|((qi, ci), wi)| wi * qi * ci)
        .sum();
    let norm_q: f64 = q
        .iter()
        .zip(w.iter())
        .map(|(qi, wi)| wi * qi * qi)
        .sum::<f64>()
        .sqrt();
    let norm_c: f64 = c
        .iter()
        .zip(w.iter())
        .map(|(ci, wi)| wi * ci * ci)
        .sum::<f64>()
        .sqrt();
    if norm_q == 0.0 || norm_c == 0.0 {
        0.0
    } else {
        (dot / (norm_q * norm_c)).clamp(0.0, 1.0)
    }
}

const DEFAULT_INPUT: &str = r#"{
  "cases": [
    {"features": {"pain": 1, "fever": 1, "cough": 0}, "solution": "flu",      "id": "c1"},
    {"features": {"pain": 0, "fever": 1, "cough": 1}, "solution": "cold",     "id": "c2"},
    {"features": {"pain": 1, "fever": 0, "cough": 0}, "solution": "headache", "id": "c3"},
    {"features": {"pain": 1, "fever": 1, "cough": 1}, "solution": "flu",      "id": "c4"}
  ],
  "query":   {"features": {"pain": 1, "fever": 1, "cough": 0}},
  "weights": {"pain": 1.0, "fever": 1.0, "cough": 0.5}
}"#;

impl CognitiveBreed for Cbr {
    fn breed_id(&self) -> &'static str {
        "cbr"
    }

    fn run(&self, input: &BreedInput) -> Option<serde_json::Value> {
        let effective: Value = if input
            .payload
            .as_object()
            .map(|m| m.is_empty())
            .unwrap_or(true)
        {
            serde_json::from_str(DEFAULT_INPUT).ok()?
        } else {
            input.payload.clone()
        };

        let cases = effective.get("cases")?.as_array()?;
        let query_features = effective.get("query")?.get("features")?;
        let weights_map = effective.get("weights")?;

        // Collect all feature keys in deterministic order from the first case.
        let keys: Vec<&str> = cases
            .first()
            .and_then(|c| c.get("features"))
            .and_then(|f| f.as_object())
            .map(|m| m.keys().map(|k| k.as_str()).collect())
            .unwrap_or_default();

        let weight_vec: Vec<f64> = keys
            .iter()
            .map(|k| weights_map.get(k).and_then(|v| v.as_f64()).unwrap_or(1.0))
            .collect();

        let query_vec = feature_vec(query_features, &keys);

        // Score every case.
        let mut ranked: Vec<(String, String, f64)> = cases
            .iter()
            .filter_map(|case| {
                let id = case.get("id")?.as_str()?.to_string();
                let solution = case.get("solution")?.as_str()?.to_string();
                let feats = case.get("features")?;
                let case_vec = feature_vec(feats, &keys);
                let sim = weighted_cosine(&query_vec, &case_vec, &weight_vec);
                Some((id, solution, sim))
            })
            .collect();

        ranked.sort_by(|a, b| b.2.partial_cmp(&a.2).unwrap_or(std::cmp::Ordering::Equal));

        let best = ranked.first()?;
        let ranked_json: Vec<Value> = ranked
            .iter()
            .map(|(id, sol, sim)| {
                serde_json::json!({"case_id": id, "solution": sol, "similarity": sim})
            })
            .collect();

        Some(serde_json::json!({
            "retrieved_case": best.0,
            "solution":       best.1,
            "similarity":     best.2,
            "ranked":         ranked_json
        }))
    }
}
