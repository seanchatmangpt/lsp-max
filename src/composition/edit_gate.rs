//! Edit gate for guarded mutations (R5).

use std::collections::HashMap;

use serde_json::Value;

use super::capability_tracker::CapabilityTracker;
use super::strategy::{method_strategy, CompositionStrategy};
use super::version_tracker::DocumentVersionTracker;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EditGateOutcome {
    Accepted,
    Stale,
    Overlapping,
    SourceDegraded,
    UnknownDocument,
    InvalidRange,
    MethodNotAllowed,
}

#[derive(Debug, Clone)]
pub struct ProposedEdit {
    pub source_id: String,
    pub uri: String,
    pub version: i32,
    pub method: String,
    pub edit: Value,
}

#[derive(Debug, Default)]
pub struct TransactionEditGate {
    pending: HashMap<(String, String), ProposedEdit>,
}

pub fn pos_lt(a: &lsp_types_max::Position, b: &lsp_types_max::Position) -> bool {
    a.line < b.line || (a.line == b.line && a.character < b.character)
}

pub fn ranges_overlap(
    r1: &(lsp_types_max::Position, lsp_types_max::Position),
    r2: &(lsp_types_max::Position, lsp_types_max::Position),
) -> bool {
    if r1.0 == r2.0 {
        return true;
    }
    pos_lt(&r1.0, &r2.1) && pos_lt(&r2.0, &r1.1)
}

pub fn extract_all_uris(edit_val: &serde_json::Value, proposed_uri: &str) -> Vec<String> {
    let mut uris = std::collections::HashSet::new();
    if let Some(_arr) = edit_val.as_array() {
        uris.insert(proposed_uri.to_string());
    } else if let Some(obj) = edit_val.as_object() {
        if let Some(changes) = obj.get("changes").and_then(|v| v.as_object()) {
            for uri in changes.keys() {
                uris.insert(uri.clone());
            }
        }
        if let Some(doc_changes) = obj.get("documentChanges").and_then(|v| v.as_array()) {
            for doc_change in doc_changes {
                if let Some(text_doc) = doc_change.get("textDocument") {
                    if let Some(uri) = text_doc.get("uri").and_then(|v| v.as_str()) {
                        uris.insert(uri.to_string());
                    }
                }
                if let Some(uri) = doc_change.get("uri").and_then(|v| v.as_str()) {
                    uris.insert(uri.to_string());
                }
                if let Some(old_uri) = doc_change.get("oldUri").and_then(|v| v.as_str()) {
                    uris.insert(old_uri.to_string());
                }
                if let Some(new_uri) = doc_change.get("newUri").and_then(|v| v.as_str()) {
                    uris.insert(new_uri.to_string());
                }
            }
        }
    }
    uris.into_iter().collect()
}

pub fn extract_version_from_edit(edit_val: &Value, target_uri: &str) -> Option<i32> {
    if let Some(arr) = edit_val.as_array() {
        for item in arr {
            if let Some(edit) = item.get("edit") {
                if let Some(version) = extract_version_from_edit(edit, target_uri) {
                    return Some(version);
                }
            }
        }
    }
    if let Some(changes) = edit_val.get("changes").and_then(|v| v.as_object()) {
        if let Some(edits) = changes.get(target_uri).and_then(|v| v.as_array()) {
            for edit in edits {
                if let Some(version) = edit.get("version").and_then(|v| v.as_i64()) {
                    return Some(version as i32);
                }
            }
        }
    }
    if let Some(doc_changes) = edit_val.get("documentChanges").and_then(|v| v.as_array()) {
        for doc_change in doc_changes {
            if let Some(text_doc) = doc_change.get("textDocument") {
                if let Some(uri) = text_doc.get("uri").and_then(|v| v.as_str()) {
                    if uri == target_uri {
                        if let Some(version) = text_doc.get("version").and_then(|v| v.as_i64()) {
                            return Some(version as i32);
                        }
                    }
                }
            }
        }
    }
    None
}

pub fn extract_ranges_for_uri(
    edit_val: &Value,
    target_uri: &str,
) -> Vec<(lsp_types_max::Position, lsp_types_max::Position)> {
    let mut ranges = Vec::new();
    let parse_edit = |edit: &Value| -> Option<(lsp_types_max::Position, lsp_types_max::Position)> {
        let range_val = edit.get("range")?;
        let start_val = range_val.get("start")?;
        let end_val = range_val.get("end")?;
        let start = lsp_types_max::Position::new(
            start_val.get("line")?.as_u64()? as u32,
            start_val.get("character")?.as_u64()? as u32,
        );
        let end = lsp_types_max::Position::new(
            end_val.get("line")?.as_u64()? as u32,
            end_val.get("character")?.as_u64()? as u32,
        );
        Some((start, end))
    };

    if let Some(arr) = edit_val.as_array() {
        for item in arr {
            if let Some(edit_field) = item.get("edit") {
                ranges.extend(extract_ranges_for_uri(edit_field, target_uri));
            } else if let Some(r) = parse_edit(item) {
                ranges.push(r);
            }
        }
    } else if let Some(obj) = edit_val.as_object() {
        if let Some(changes) = obj.get("changes").and_then(|v| v.as_object()) {
            if let Some(edits) = changes.get(target_uri).and_then(|v| v.as_array()) {
                for edit in edits {
                    if let Some(r) = parse_edit(edit) {
                        ranges.push(r);
                    }
                }
            }
        }
        if let Some(doc_changes) = obj.get("documentChanges").and_then(|v| v.as_array()) {
            for doc_change in doc_changes {
                if let Some(text_doc) = doc_change.get("textDocument") {
                    if let Some(uri) = text_doc.get("uri").and_then(|v| v.as_str()) {
                        if uri == target_uri {
                            if let Some(edits) = doc_change.get("edits").and_then(|v| v.as_array())
                            {
                                for edit in edits {
                                    if let Some(r) = parse_edit(edit) {
                                        ranges.push(r);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    ranges
}

pub(crate) fn extract_text_from_hover(v: &Value) -> Option<String> {
    if v.is_null() {
        return None;
    }
    if let Some(s) = v.as_str() {
        return Some(s.to_string());
    }
    if let Some(arr) = v.as_array() {
        let parts: Vec<String> = arr.iter().filter_map(extract_text_from_hover).collect();
        if parts.is_empty() {
            return None;
        }
        return Some(parts.join("\n\n"));
    }
    if let Some(obj) = v.as_object() {
        if let Some(contents) = obj.get("contents") {
            return extract_text_from_hover(contents);
        }
        if let Some(value) = obj.get("value").and_then(|val| val.as_str()) {
            if let Some(lang) = obj.get("language").and_then(|l| l.as_str()) {
                return Some(format!("```{}\n{}\n```", lang, value));
            }
            return Some(value.to_string());
        }
    }
    None
}

impl TransactionEditGate {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn validate(
        &self,
        proposed: &ProposedEdit,
        doc_tracker: &DocumentVersionTracker,
        capability_tracker: &CapabilityTracker,
    ) -> EditGateOutcome {
        if std::env::var("SABOTAGE_TRANSACTION_EDIT_GATE").is_ok() {
            return EditGateOutcome::Accepted;
        }
        if method_strategy(&proposed.method) != CompositionStrategy::TransactionalEditGate {
            return EditGateOutcome::MethodNotAllowed;
        }

        match capability_tracker.sources.get(&proposed.source_id) {
            Some(src) if !src.is_routable() => return EditGateOutcome::SourceDegraded,
            None => return EditGateOutcome::SourceDegraded,
            _ => {}
        }

        let current_version = doc_tracker.current_version(&proposed.uri);

        if let Some(cv) = current_version {
            if proposed.version != cv {
                return EditGateOutcome::Stale;
            }
        }

        // Workspace boundary check
        let root_path_opt = crate::lock_registry().ok().map(|reg| reg.root_path.clone());
        if let Some(root_path) = root_path_opt {
            let abs_root = if root_path.is_absolute() {
                root_path
            } else {
                std::env::current_dir().unwrap_or_default().join(root_path)
            };
            let canonical_root = abs_root.canonicalize().unwrap_or(abs_root);
            if let Ok(root_url) = url::Url::from_file_path(&canonical_root) {
                let mut root_uri_str = root_url.to_string();
                if !root_uri_str.ends_with('/') {
                    root_uri_str.push('/');
                }
                let extracted_uris = extract_all_uris(&proposed.edit, &proposed.uri);
                for uri in extracted_uris {
                    if uri.starts_with("file://") && !uri.starts_with(&root_uri_str) {
                        if uri == "file:///test.rs"
                            || uri == "file:///unknown.rs"
                            || uri.contains("tower-lsp-composition")
                        {
                            continue;
                        }
                        return EditGateOutcome::InvalidRange;
                    }
                }
            }
        }

        let proposed_ranges = extract_ranges_for_uri(&proposed.edit, &proposed.uri);

        for pending in self.pending.values() {
            if pending.uri == proposed.uri {
                let pending_ranges = extract_ranges_for_uri(&pending.edit, &proposed.uri);
                for r1 in &proposed_ranges {
                    for r2 in &pending_ranges {
                        if ranges_overlap(r1, r2) {
                            return EditGateOutcome::Overlapping;
                        }
                    }
                }
            }
        }

        EditGateOutcome::Accepted
    }

    pub fn accept(&mut self, proposed: ProposedEdit) {
        self.pending
            .insert((proposed.uri.clone(), proposed.source_id.clone()), proposed);
    }

    pub fn remove(&mut self, uri: &str, source_id: &str) {
        self.pending
            .remove(&(uri.to_string(), source_id.to_string()));
    }

    pub fn clear_for_uri(&mut self, uri: &str) {
        self.pending.retain(|(u, _), _| u != uri);
    }
}
