//! Source attribution and merge helpers (R4).

use std::collections::HashSet;

use serde_json::{json, Value};

use super::edit_gate::extract_text_from_hover;

// ── Source Attribution (R4) ────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct AttributedObservation {
    pub source_id: String,
    pub uri: String,
    pub data: Value,
}

pub fn merge_attributed(observations: Vec<AttributedObservation>) -> Value {
    let items: Vec<Value> = observations
        .into_iter()
        .map(|obs| json!({ "source": obs.source_id, "uri": obs.uri, "data": obs.data }))
        .collect();
    Value::Array(items)
}

pub fn merge_deduped_locations(observations: Vec<AttributedObservation>) -> Vec<Value> {
    let mut seen: HashSet<String> = HashSet::new();
    let mut result = Vec::new();
    for obs in observations {
        if let Some(arr) = obs.data.as_array() {
            for loc in arr {
                let key = serde_json::to_string(loc).unwrap_or_default();
                if seen.insert(key) {
                    result.push(loc.clone());
                }
            }
        }
    }
    result
}

pub fn merge_hovers_with_attribution(hovers: Vec<(String, Value)>) -> Value {
    let mut texts = Vec::new();
    for (source_id, hover) in hovers {
        if let Some(t) = extract_text_from_hover(&hover) {
            if !t.trim().is_empty() {
                texts.push(format!("### Source: {}\n\n{}", source_id, t));
            }
        }
    }
    if texts.is_empty() {
        return Value::Null;
    }
    let merged_text = texts.join("\n\n---\n\n");
    json!({
        "contents": {
            "kind": "markdown",
            "value": merged_text
        }
    })
}

pub(crate) fn merge_workspace_edits(mut base: Value, other: Value) -> Value {
    if base.is_null() {
        return other;
    }
    if other.is_null() {
        return base;
    }

    let base_obj = match base.as_object_mut() {
        Some(o) => o,
        None => return base,
    };
    let other_obj = match other.as_object() {
        Some(o) => o,
        None => return base,
    };

    // Merge "changes"
    if let Some(other_changes) = other_obj.get("changes").and_then(|v| v.as_object()) {
        if let Some(base_changes) = base_obj
            .entry("changes")
            .or_insert_with(|| json!({}))
            .as_object_mut()
        {
            for (uri, other_edits) in other_changes {
                if let Some(other_edits_arr) = other_edits.as_array() {
                    let base_edits = base_changes.entry(uri.clone()).or_insert_with(|| json!([]));
                    if let Some(base_edits_arr) = base_edits.as_array_mut() {
                        base_edits_arr.extend(other_edits_arr.clone());
                    }
                }
            }
        }
    }

    // Merge "documentChanges"
    if let Some(other_doc_changes) = other_obj.get("documentChanges").and_then(|v| v.as_array()) {
        if let Some(base_doc_changes) = base_obj
            .entry("documentChanges")
            .or_insert_with(|| json!([]))
            .as_array_mut()
        {
            base_doc_changes.extend(other_doc_changes.clone());
        }
    }

    base
}

pub(crate) fn merge_edits(base: Value, other: Value) -> Value {
    if base.is_null() {
        return other;
    }
    if other.is_null() {
        return base;
    }
    if let (Some(base_arr), Some(other_arr)) = (base.as_array(), other.as_array()) {
        let mut base_arr = base_arr.clone();
        base_arr.extend(other_arr.clone());
        return Value::Array(base_arr);
    }
    merge_workspace_edits(base, other)
}
