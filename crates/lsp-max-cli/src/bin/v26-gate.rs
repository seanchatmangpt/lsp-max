use std::fs;
use std::path::Path;
use serde_json::{json, Value};

fn main() {
    let gate_file = "scripts/v26-gate.json";
    
    if !Path::new(gate_file).exists() {
        let out = json!({
            "release": "v26.6.28",
            "q_release": 0,
            "failset_cardinality": 1,
            "counterexamples": ["v26-gate.json missing"],
            "components": {}
        });
        println!("{}", serde_json::to_string_pretty(&out).unwrap());
        std::process::exit(1);
    }

    let data_str = fs::read_to_string(gate_file).unwrap();
    let data: Value = serde_json::from_str(&data_str).unwrap();
    
    let mut missing = Vec::new();
    
    if let Some(detectors) = data.get("detectors").and_then(|d| d.as_array()) {
        for d in detectors {
            if let Some(receipt) = d.get("receipt_file").and_then(|v| v.as_str()) {
                if !Path::new(receipt).exists() {
                    if let Some(name) = d.get("name").and_then(|v| v.as_str()) {
                        missing.push(name.to_string());
                    }
                }
            }
        }
    }

    let q_release = if missing.is_empty() { 1 } else { 0 };
    let failset = missing.len();

    let out = json!({
        "release": "v26.6.28",
        "q_release": q_release,
        "failset_cardinality": failset,
        "counterexamples": missing,
        "components": {}
    });

    println!("{}", serde_json::to_string_pretty(&out).unwrap());
    
    if !missing.is_empty() {
        std::process::exit(1);
    }
}
