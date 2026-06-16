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

#[cfg(test)]
mod tests {
    use super::*;

    fn obs(source_id: &str, uri: &str, data: Value) -> AttributedObservation {
        AttributedObservation {
            source_id: source_id.to_string(),
            uri: uri.to_string(),
            data,
        }
    }

    // ── merge_attributed ─────────────────────────────────────────────────────────

    #[test]
    fn merge_attributed_empty_input_produces_empty_array() {
        let result = merge_attributed(vec![]);
        assert_eq!(result, Value::Array(vec![]));
    }

    #[test]
    fn merge_attributed_single_source_preserves_fields() {
        let result = merge_attributed(vec![obs("src-1", "file:///a.rs", json!({"x": 1}))]);
        let arr = result.as_array().unwrap();
        assert_eq!(arr.len(), 1);
        assert_eq!(arr[0]["source"], "src-1");
        assert_eq!(arr[0]["uri"], "file:///a.rs");
        assert_eq!(arr[0]["data"]["x"], 1);
    }

    #[test]
    fn merge_attributed_two_sources_produces_two_items() {
        let result = merge_attributed(vec![
            obs("src-1", "file:///a.rs", json!({"x": 1})),
            obs("src-2", "file:///b.rs", json!({"x": 2})),
        ]);
        let arr = result.as_array().unwrap();
        assert_eq!(arr.len(), 2);
        assert_eq!(arr[0]["source"], "src-1");
        assert_eq!(arr[1]["source"], "src-2");
    }

    // ── merge_deduped_locations ───────────────────────────────────────────────────

    #[test]
    fn merge_deduped_locations_empty_input_produces_empty_vec() {
        let result = merge_deduped_locations(vec![]);
        assert!(result.is_empty());
    }

    #[test]
    fn merge_deduped_locations_single_source() {
        let loc = json!({"uri": "file:///a.rs", "range": {"start": {"line": 0, "character": 0}, "end": {"line": 0, "character": 5}}});
        let result = merge_deduped_locations(vec![obs("src-1", "file:///a.rs", json!([loc.clone()]))]);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0], loc);
    }

    #[test]
    fn merge_deduped_locations_deduplicates_identical_entries() {
        let loc = json!({"uri": "file:///a.rs", "range": {"start": {"line": 0, "character": 0}, "end": {"line": 0, "character": 5}}});
        let result = merge_deduped_locations(vec![
            obs("src-1", "file:///a.rs", json!([loc.clone()])),
            obs("src-2", "file:///a.rs", json!([loc.clone()])),
        ]);
        // Duplicate location should appear only once
        assert_eq!(result.len(), 1);
    }

    #[test]
    fn merge_deduped_locations_keeps_distinct_entries() {
        let loc_a = json!({"uri": "file:///a.rs"});
        let loc_b = json!({"uri": "file:///b.rs"});
        let result = merge_deduped_locations(vec![
            obs("src-1", "file:///a.rs", json!([loc_a.clone()])),
            obs("src-2", "file:///b.rs", json!([loc_b.clone()])),
        ]);
        assert_eq!(result.len(), 2);
    }

    // ── merge_hovers_with_attribution ────────────────────────────────────────────

    #[test]
    fn merge_hovers_empty_input_produces_null() {
        let result = merge_hovers_with_attribution(vec![]);
        assert_eq!(result, Value::Null);
    }

    #[test]
    fn merge_hovers_single_source_with_plain_text() {
        let hover = json!({"contents": {"kind": "markdown", "value": "hover text"}});
        let result = merge_hovers_with_attribution(vec![("src-1".to_string(), hover)]);
        let contents = &result["contents"];
        assert_eq!(contents["kind"], "markdown");
        let val = contents["value"].as_str().unwrap();
        assert!(val.contains("src-1"));
        assert!(val.contains("hover text"));
    }

    #[test]
    fn merge_hovers_two_sources_produces_separator() {
        let hover_a = json!({"contents": {"kind": "markdown", "value": "text A"}});
        let hover_b = json!({"contents": {"kind": "markdown", "value": "text B"}});
        let result = merge_hovers_with_attribution(vec![
            ("src-1".to_string(), hover_a),
            ("src-2".to_string(), hover_b),
        ]);
        let val = result["contents"]["value"].as_str().unwrap();
        assert!(val.contains("src-1"));
        assert!(val.contains("src-2"));
        assert!(val.contains("---"));
    }

    #[test]
    fn merge_hovers_null_hover_ignored() {
        let result =
            merge_hovers_with_attribution(vec![("src-null".to_string(), Value::Null)]);
        // Null hover has no extractable text, result should be Null
        assert_eq!(result, Value::Null);
    }

    // ── merge_workspace_edits ────────────────────────────────────────────────────

    #[test]
    fn merge_workspace_edits_null_base_returns_other() {
        let other = json!({"changes": {}});
        let result = merge_workspace_edits(Value::Null, other.clone());
        assert_eq!(result, other);
    }

    #[test]
    fn merge_workspace_edits_null_other_returns_base() {
        let base = json!({"changes": {}});
        let result = merge_workspace_edits(base.clone(), Value::Null);
        assert_eq!(result, base);
    }

    #[test]
    fn merge_workspace_edits_merges_changes_for_same_uri() {
        let edit_a = json!({"range": {"start": {"line": 0, "character": 0}, "end": {"line": 0, "character": 1}}, "newText": "a"});
        let edit_b = json!({"range": {"start": {"line": 1, "character": 0}, "end": {"line": 1, "character": 1}}, "newText": "b"});
        let base = json!({"changes": {"file:///a.rs": [edit_a.clone()]}});
        let other = json!({"changes": {"file:///a.rs": [edit_b.clone()]}});
        let result = merge_workspace_edits(base, other);
        let edits = result["changes"]["file:///a.rs"].as_array().unwrap();
        assert_eq!(edits.len(), 2);
    }

    // ── merge_edits ───────────────────────────────────────────────────────────────

    #[test]
    fn merge_edits_null_base_returns_other() {
        let other = json!([{"edit": 1}]);
        let result = merge_edits(Value::Null, other.clone());
        assert_eq!(result, other);
    }

    #[test]
    fn merge_edits_null_other_returns_base() {
        let base = json!([{"edit": 1}]);
        let result = merge_edits(base.clone(), Value::Null);
        assert_eq!(result, base);
    }

    #[test]
    fn merge_edits_two_arrays_concatenated() {
        let base = json!([{"edit": 1}]);
        let other = json!([{"edit": 2}]);
        let result = merge_edits(base, other);
        let arr = result.as_array().unwrap();
        assert_eq!(arr.len(), 2);
        assert_eq!(arr[0]["edit"], 1);
        assert_eq!(arr[1]["edit"], 2);
    }
}
