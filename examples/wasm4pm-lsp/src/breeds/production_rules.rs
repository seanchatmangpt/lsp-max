//! `mycin` — MYCIN Production Rules breed (Shortliffe 1976).
//!
//! Family: SymbolicAI
//! Paper: `shortliffe1976mycin`
//! Oracle value: 0.693
//!
//! Status: CANDIDATE

use crate::breeds::breed::{BreedInput, CognitiveBreed};
use serde_json::json;

pub struct Mycin;

fn combine_cf(cf1: f64, cf2: f64) -> f64 {
    if cf1 >= 0.0 && cf2 >= 0.0 {
        cf1 + cf2 * (1.0 - cf1)
    } else if cf1 < 0.0 && cf2 < 0.0 {
        cf1 + cf2 * (1.0 + cf1)
    } else {
        let denom = 1.0 - cf1.abs().min(cf2.abs());
        if denom.abs() < f64::EPSILON {
            0.0
        } else {
            (cf1 + cf2) / denom
        }
    }
}

fn default_input() -> serde_json::Value {
    json!({
        "rules": [
            {"hypothesis": "infection", "evidence": "fever",    "cf": 0.6},
            {"hypothesis": "infection", "evidence": "bacteria", "cf": 0.2325}
        ],
        "evidence": ["fever", "bacteria"],
        "query": "infection"
    })
}

impl CognitiveBreed for Mycin {
    fn breed_id(&self) -> &'static str {
        "mycin"
    }

    fn run(&self, input: &BreedInput) -> Option<serde_json::Value> {
        let payload = if input
            .payload
            .as_object()
            .map(|m| m.is_empty())
            .unwrap_or(true)
        {
            default_input()
        } else {
            input.payload.clone()
        };

        let query = payload.get("query")?.as_str()?;
        let rules = payload.get("rules")?.as_array()?;
        let evidence_list: Vec<&str> = payload
            .get("evidence")?
            .as_array()?
            .iter()
            .filter_map(|v| v.as_str())
            .collect();

        let fired_cfs: Vec<f64> = rules
            .iter()
            .filter_map(|r| {
                let hyp = r.get("hypothesis")?.as_str()?;
                let ev = r.get("evidence")?.as_str()?;
                let cf = r.get("cf")?.as_f64()?;
                if hyp == query && evidence_list.contains(&ev) {
                    Some(cf)
                } else {
                    None
                }
            })
            .collect();

        if fired_cfs.is_empty() {
            return None;
        }

        let combined = fired_cfs[1..]
            .iter()
            .fold(fired_cfs[0], |acc, &cf| combine_cf(acc, cf));
        let rounded = (combined * 1000.0).round() / 1000.0;

        Some(json!({"hypothesis": query, "cf": rounded}))
    }
}
