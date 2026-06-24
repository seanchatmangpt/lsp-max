use crate::breeds::breed::{BreedInput, CognitiveBreed};
use serde_json::{json, Value};

pub struct Eliza;

struct Script {
    rules: Vec<(String, Vec<String>)>,
    fallbacks: Vec<String>,
}

impl Script {
    fn default_script() -> Self {
        Self {
            rules: vec![
                (
                    "I am *".into(),
                    vec![
                        "How long have you been (1)?".into(),
                        "Do you enjoy being (1)?".into(),
                    ],
                ),
                (
                    "I feel *".into(),
                    vec![
                        "Tell me more about feeling (1).".into(),
                        "Why do you feel (1)?".into(),
                    ],
                ),
                (
                    "* mother *".into(),
                    vec![
                        "Tell me more about your family.".into(),
                        "How does your mother make you feel?".into(),
                    ],
                ),
                (
                    "* father *".into(),
                    vec![
                        "Your family seems important to you.".into(),
                        "Tell me more about your father.".into(),
                    ],
                ),
                (
                    "I need *".into(),
                    vec![
                        "Why do you need (1)?".into(),
                        "What would it mean if you had (1)?".into(),
                    ],
                ),
                (
                    "* dream *".into(),
                    vec![
                        "What do you think this dream means?".into(),
                        "Have you dreamed of this before?".into(),
                    ],
                ),
                (
                    "* sorry *".into(),
                    vec![
                        "Please don't apologize.".into(),
                        "What makes you feel the need to apologize?".into(),
                    ],
                ),
                (
                    "hello *".into(),
                    vec!["Hello! How are you feeling today?".into()],
                ),
                (
                    "* computer *".into(),
                    vec![
                        "Do you think computers can help you?".into(),
                        "How do machines make you feel?".into(),
                    ],
                ),
            ],
            fallbacks: vec![
                "Please go on.".into(),
                "I see.".into(),
                "Very interesting.".into(),
            ],
        }
    }

    fn from_value(val: &Value) -> Option<Self> {
        let arr = val.as_array()?;
        let rules = arr
            .iter()
            .filter_map(|entry| {
                let pattern = entry.get("pattern")?.as_str()?.to_string();
                let responses = entry
                    .get("responses")?
                    .as_array()?
                    .iter()
                    .filter_map(|r| r.as_str().map(String::from))
                    .collect::<Vec<_>>();
                if responses.is_empty() {
                    None
                } else {
                    Some((pattern, responses))
                }
            })
            .collect();
        Some(Self {
            rules,
            fallbacks: vec![
                "Please go on.".into(),
                "I see.".into(),
                "Very interesting.".into(),
            ],
        })
    }
}

// Returns the captured groups (original casing) for each `*` in the pattern.
// Matching is case-insensitive. Wildcards are greedy and contiguous.
fn try_match(pattern: &str, text: &str) -> Option<Vec<String>> {
    let parts: Vec<&str> = pattern.split('*').collect();
    let text_lo = text.to_lowercase();

    if parts.len() == 1 {
        // No wildcard: exact match
        return if text_lo == pattern.to_lowercase() {
            Some(vec![])
        } else {
            None
        };
    }

    let first_lo = parts[0].trim().to_lowercase();
    let last_lo = parts[parts.len() - 1].trim().to_lowercase();

    // Verify prefix and suffix match, then extract the span between them
    let inner_start = if first_lo.is_empty() {
        0
    } else {
        if !text_lo.starts_with(&first_lo) {
            return None;
        }
        first_lo.len()
    };

    let inner_end = if last_lo.is_empty() {
        text.len()
    } else {
        if !text_lo.ends_with(&last_lo) {
            return None;
        }
        text_lo.len() - last_lo.len()
    };

    if inner_start > inner_end {
        return None;
    }
    let inner = text[inner_start..inner_end].trim();

    if parts.len() == 2 {
        // Single wildcard
        if inner.is_empty() {
            return None;
        }
        return Some(vec![inner.to_string()]);
    }

    // Multiple wildcards: split the inner span on the fixed middle segments
    let middle_seps: Vec<String> = parts[1..parts.len() - 1]
        .iter()
        .map(|s| s.trim().to_lowercase())
        .collect();

    let mut captures = Vec::with_capacity(middle_seps.len() + 1);
    let mut rest_lo = inner.to_lowercase();
    let mut rest_orig = inner.to_string();

    for sep in &middle_seps {
        if sep.is_empty() {
            captures.push(String::new());
            continue;
        }
        let pos = rest_lo.find(sep.as_str())?;
        let cap = rest_orig[..pos].trim().to_string();
        if cap.is_empty() {
            return None;
        }
        captures.push(cap);
        rest_orig = rest_orig[pos + sep.len()..].to_string();
        rest_lo = rest_lo[pos + sep.len()..].to_string();
    }
    let final_cap = rest_orig.trim().to_string();
    if final_cap.is_empty() {
        return None;
    }
    captures.push(final_cap);

    Some(captures)
}

fn fill_template(template: &str, captures: &[String]) -> String {
    let mut result = template.to_string();
    for (i, cap) in captures.iter().enumerate() {
        result = result.replace(&format!("({})", i + 1), cap);
    }
    result
}

fn eliza_respond(script: &Script, text: &str, call_count: usize) -> (String, Option<String>) {
    for (pattern, responses) in &script.rules {
        if let Some(captures) = try_match(pattern, text) {
            let idx = call_count % responses.len();
            let response = fill_template(&responses[idx], &captures);
            return (response, Some(pattern.clone()));
        }
    }
    let idx = call_count % script.fallbacks.len();
    (script.fallbacks[idx].clone(), None)
}

impl CognitiveBreed for Eliza {
    fn breed_id(&self) -> &'static str {
        "eliza"
    }

    fn run(&self, input: &BreedInput) -> Option<Value> {
        let text = input.get("text")?.as_str()?;
        let call_count = input
            .get("call_count")
            .and_then(|v| v.as_u64())
            .unwrap_or(0) as usize;

        let script = match input.get("script") {
            Some(v) => Script::from_value(v).unwrap_or_else(Script::default_script),
            None => Script::default_script(),
        };

        let (response, pattern_matched) = eliza_respond(&script, text, call_count);

        Some(match pattern_matched {
            Some(p) => json!({"response": response, "pattern_matched": p}),
            None => json!({"response": response}),
        })
    }
}
