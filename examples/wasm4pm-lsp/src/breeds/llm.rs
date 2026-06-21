//! `llm` — Large Language Model Reasoning breed (Brown et al. 2020, GPT-3).
//!
//! Family: NeuralAI
//! Paper: `brown2020language`
//! Oracle value: 0.0 (non-deterministic output; conformance checks response non-empty)
//!
//! Status: CANDIDATE
//!
//! Calls the Anthropic Messages API via a blocking HTTP request.
//! Gracefully returns None if ANTHROPIC_API_KEY is absent.

use crate::breeds::breed::{BreedInput, CognitiveBreed};
use serde_json::{json, Value};

pub struct Llm;

const DEFAULT_MODEL: &str = "claude-haiku-4-5-20251001";
const API_URL: &str = "https://api.anthropic.com/v1/messages";
const ANTHROPIC_VERSION: &str = "2023-06-01";

fn call_api(prompt: String, model: String, max_tokens: u64, api_key: String) -> Option<Value> {
    // Spawn a dedicated thread so blocking I/O is safe inside an async tokio runtime.
    let handle = std::thread::spawn(move || -> Option<Value> {
        let body = json!({
            "model": model,
            "max_tokens": max_tokens,
            "messages": [{"role": "user", "content": prompt}]
        });

        let client = reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .ok()?;

        let resp = client
            .post(API_URL)
            .header("x-api-key", &api_key)
            .header("anthropic-version", ANTHROPIC_VERSION)
            .header("content-type", "application/json")
            .json(&body)
            .send()
            .ok()?;

        if !resp.status().is_success() {
            return None;
        }

        let payload: Value = resp.json().ok()?;

        let response_text = payload
            .get("content")?
            .as_array()?
            .first()?
            .get("text")?
            .as_str()?
            .to_string();

        let input_tokens = payload
            .get("usage")
            .and_then(|u| u.get("input_tokens"))
            .and_then(|v| v.as_u64())
            .unwrap_or(0);

        let output_tokens = payload
            .get("usage")
            .and_then(|u| u.get("output_tokens"))
            .and_then(|v| v.as_u64())
            .unwrap_or(0);

        let model_used = payload
            .get("model")
            .and_then(|v| v.as_str())
            .unwrap_or(DEFAULT_MODEL)
            .to_string();

        Some(json!({
            "response":      response_text,
            "model":         model_used,
            "input_tokens":  input_tokens,
            "output_tokens": output_tokens
        }))
    });

    handle.join().ok().flatten()
}

impl CognitiveBreed for Llm {
    fn breed_id(&self) -> &'static str {
        "llm"
    }

    fn run(&self, input: &BreedInput) -> Option<Value> {
        let api_key = std::env::var("ANTHROPIC_API_KEY").ok()?;

        let prompt = input
            .get("prompt")
            .and_then(|v| v.as_str())
            .unwrap_or("What is 2 + 2? Reply with just the number.")
            .to_string();

        let model = input
            .get("model")
            .and_then(|v| v.as_str())
            .unwrap_or(DEFAULT_MODEL)
            .to_string();

        let max_tokens = input
            .get("max_tokens")
            .and_then(|v| v.as_u64())
            .unwrap_or(256);

        call_api(prompt, model, max_tokens, api_key)
    }
}
