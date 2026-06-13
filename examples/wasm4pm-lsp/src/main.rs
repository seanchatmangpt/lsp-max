use dashmap::DashMap;
use gc005_wasm4pm_adapter::analyze_ocel;
use lsp_max::jsonrpc::Result;
use lsp_max::lsp_types::*;
use lsp_max::{Client, LspService, Server};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;

// ── OCEL index structures ────────────────────────────────────────────────────

#[derive(Debug, Clone)]
struct OcelObject {
    id: String,
    obj_type: String,
    range: Range,
}

#[derive(Debug, Clone)]
struct OcelEvent {
    id: String,
    event_type: String,
    time: String,
    range: Range,
    /// (objectId, qualifier, range-of-the-objectId-string-value)
    relationships: Vec<(String, String, Range)>,
}

#[derive(Debug, Clone, Default)]
struct OcelIndex {
    events: Vec<OcelEvent>,
    objects: Vec<OcelObject>,
    event_types: Vec<String>,
    object_types: Vec<String>,
    /// objectId → index in objects
    obj_by_id: HashMap<String, usize>,
    /// eventId → index in events
    event_by_id: HashMap<String, usize>,
    /// objectId → Vec of Ranges (relationship objectId string value positions)
    obj_refs: HashMap<String, Vec<Range>>,
    /// line of "events": [ token
    events_key_line: u32,
    /// line of "objects": [ token
    objects_key_line: u32,
    /// line of closing ] of top-level events array
    events_end_line: u32,
    /// line of closing ] of top-level objects array
    objects_end_line: u32,
}

#[derive(Debug, Clone)]
enum GallVerdict {
    Fit {
        fitness: f32,
    },
    Deviation {
        #[allow(dead_code)]
        fitness: f32,
        missing: Vec<String>,
    },
    Blocked {
        #[allow(dead_code)]
        reason: String,
    },
    Inconclusive,
}

#[derive(Debug, Clone)]
struct ConformanceResult {
    verdict: GallVerdict,
    fitness: Option<f32>,
}

#[derive(Debug, Clone, Default)]
struct DocumentState {
    text: String,
    index: Option<OcelIndex>,
    conformance: Option<ConformanceResult>,
}

// ── Semantic token legend ────────────────────────────────────────────────────
// Types: namespace=0, class=1, function=2, variable=3, string=4, number=5, keyword=6, comment=7
// Modifiers: declaration=0, definition=1, readonly=2, deprecated=3
//
// OCEL mapping:
//   eventId value   → function (2)
//   objectId value  → class (1)
//   activityName    → keyword (6)
//   timestamp       → string (4)
//   attributeName   → variable (3)

fn token_legend() -> SemanticTokensLegend {
    SemanticTokensLegend {
        token_types: vec![
            SemanticTokenType::NAMESPACE,
            SemanticTokenType::CLASS,
            SemanticTokenType::FUNCTION,
            SemanticTokenType::VARIABLE,
            SemanticTokenType::new("string"),
            SemanticTokenType::new("number"),
            SemanticTokenType::KEYWORD,
            SemanticTokenType::COMMENT,
        ],
        token_modifiers: vec![
            SemanticTokenModifier::DECLARATION,
            SemanticTokenModifier::DEFINITION,
            SemanticTokenModifier::READONLY,
            SemanticTokenModifier::DEPRECATED,
        ],
    }
}

// ── Text helpers ─────────────────────────────────────────────────────────────

#[allow(dead_code)]
fn pos_to_offset(text: &str, line: u32, character: u32) -> Option<usize> {
    let mut cur_line = 0u32;
    let mut offset = 0usize;
    for ch in text.chars() {
        if cur_line == line {
            // character is UTF-16 code units; approximate as char count for ASCII-heavy JSON
            if character == 0 {
                return Some(offset);
            }
            // walk character columns
            break;
        }
        if ch == '\n' {
            cur_line += 1;
        }
        offset += ch.len_utf8();
    }
    // Re-do with explicit column walk
    let mut cur_line = 0u32;
    let mut offset = 0usize;
    for ch in text.chars() {
        if cur_line == line {
            let mut col_offset = offset;
            for (col, c) in text[offset..].chars().enumerate() {
                if col as u32 == character {
                    return Some(col_offset);
                }
                col_offset += c.len_utf8();
            }
            return Some(col_offset);
        }
        if ch == '\n' {
            cur_line += 1;
        }
        offset += ch.len_utf8();
    }
    None
}

#[allow(dead_code)]
fn word_at(text: &str, offset: usize) -> Option<(String, usize, usize)> {
    if offset >= text.len() {
        return None;
    }
    // Walk backwards to find start of quoted string or identifier
    let bytes = text.as_bytes();
    // Find enclosing quotes
    let mut start = offset;
    while start > 0 && bytes[start] != b'"' {
        start -= 1;
    }
    if bytes[start] == b'"' {
        start += 1; // skip opening quote
    }
    let mut end = offset;
    while end < text.len() && bytes[end] != b'"' {
        end += 1;
    }
    if start < end {
        Some((text[start..end].to_string(), start, end))
    } else {
        None
    }
}

/// Convert a byte offset to (line, character).
fn offset_to_position(text: &str, offset: usize) -> Position {
    let mut line = 0u32;
    let mut col = 0u32;
    for (i, ch) in text.char_indices() {
        if i == offset {
            break;
        }
        if ch == '\n' {
            line += 1;
            col = 0;
        } else {
            col += 1;
        }
    }
    Position {
        line,
        character: col,
    }
}

/// Find all occurrences of a quoted string value in text and return their Ranges.
/// Matches `"<needle>"` (exact quoted value).
fn find_all_quoted(text: &str, needle: &str) -> Vec<Range> {
    let pattern = format!("\"{}\"", needle);
    let mut results = Vec::new();
    let mut search_from = 0;
    while let Some(pos) = text[search_from..].find(&pattern) {
        let abs = search_from + pos + 1; // +1 to skip opening quote
        let start = offset_to_position(text, abs);
        let end = offset_to_position(text, abs + needle.len());
        results.push(Range { start, end });
        search_from = abs + needle.len();
    }
    results
}

// ── OCEL parser ──────────────────────────────────────────────────────────────

fn parse_ocel(text: &str) -> Option<OcelIndex> {
    let v: Value = serde_json::from_str(text).ok()?;
    let obj = v.as_object()?;

    let mut idx = OcelIndex::default();

    // Collect event_types
    if let Some(et) = obj.get("eventTypes").and_then(|v| v.as_array()) {
        for t in et {
            if let Some(name) = t.get("name").and_then(|n| n.as_str()) {
                idx.event_types.push(name.to_string());
            }
        }
    }

    // Collect object_types
    if let Some(ot) = obj.get("objectTypes").and_then(|v| v.as_array()) {
        for t in ot {
            if let Some(name) = t.get("name").and_then(|n| n.as_str()) {
                idx.object_types.push(name.to_string());
            }
        }
    }

    // Parse objects
    if let Some(objects) = obj.get("objects").and_then(|v| v.as_array()) {
        for (i, o) in objects.iter().enumerate() {
            let id = o
                .get("id")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let obj_type = o
                .get("type")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            // Find range of this object entry in text
            let range = find_value_range(text, &id).unwrap_or_default();
            idx.objects.push(OcelObject {
                id: id.clone(),
                obj_type,
                range,
            });
            idx.obj_by_id.insert(id, i);
        }
    }

    // Parse events
    if let Some(events) = obj.get("events").and_then(|v| v.as_array()) {
        for (i, e) in events.iter().enumerate() {
            let id = e
                .get("id")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let event_type = e
                .get("type")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let time = e
                .get("time")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let range = find_value_range(text, &id).unwrap_or_default();

            let mut rels = Vec::new();
            if let Some(relationships) = e.get("relationships").and_then(|v| v.as_array()) {
                for r in relationships {
                    let oid = r
                        .get("objectId")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();
                    let qual = r
                        .get("qualifier")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();
                    let rel_range =
                        find_quoted_value_range(text, "objectId", &oid).unwrap_or_default();
                    idx.obj_refs.entry(oid.clone()).or_default().push(rel_range);
                    rels.push((oid, qual, rel_range));
                }
            }

            idx.events.push(OcelEvent {
                id: id.clone(),
                event_type,
                time,
                range,
                relationships: rels,
            });
            idx.event_by_id.insert(id, i);
        }
    }

    // Compute line numbers for top-level arrays
    idx.events_key_line = find_key_line(text, "\"events\"");
    idx.objects_key_line = find_key_line(text, "\"objects\"");
    idx.events_end_line = find_array_end_line(text, idx.events_key_line);
    idx.objects_end_line = find_array_end_line(text, idx.objects_key_line);

    Some(idx)
}

/// Find the line number of the first occurrence of `key` in text.
fn find_key_line(text: &str, key: &str) -> u32 {
    for (line, l) in text.lines().enumerate() {
        if l.contains(key) {
            return line as u32;
        }
    }
    0
}

/// Find the closing bracket line for an array starting at `start_line`.
/// Naive bracket counting.
fn find_array_end_line(text: &str, start_line: u32) -> u32 {
    let lines: Vec<&str> = text.lines().collect();
    let mut depth = 0i32;
    let mut found_start = false;
    for (i, line) in lines.iter().enumerate() {
        if i as u32 >= start_line {
            for ch in line.chars() {
                if ch == '[' {
                    depth += 1;
                    found_start = true;
                } else if ch == ']' && found_start {
                    depth -= 1;
                    if depth == 0 {
                        return i as u32;
                    }
                }
            }
        }
    }
    start_line
}

/// Find the Range of a quoted string id value (e.g. the id itself) in text.
fn find_value_range(text: &str, value: &str) -> Option<Range> {
    let pattern = format!("\"{}\"", value);
    let pos = text.find(&pattern)?;
    let start = offset_to_position(text, pos + 1);
    let end = offset_to_position(text, pos + 1 + value.len());
    Some(Range { start, end })
}

/// Find the Range of a quoted value that is preceded by the given JSON key.
/// Searches for `"<key>": "<value>"` pattern.
fn find_quoted_value_range(text: &str, key: &str, value: &str) -> Option<Range> {
    let pattern = format!("\"{}\": \"{}\"", key, value);
    // Try both spacing variants
    let pos = text.find(&pattern).or_else(|| {
        let p2 = format!("\"{}\":\"{}\"", key, value);
        text.find(&p2)
    })?;
    // Find the value's opening quote
    let key_end = pos + key.len() + 2; // past closing quote of key
    let rest = &text[key_end..];
    let val_start_rel = rest.find(&format!("\"{}\"", value))?;
    let abs_start = key_end + val_start_rel + 1; // skip opening quote
    let start = offset_to_position(text, abs_start);
    let end = offset_to_position(text, abs_start + value.len());
    Some(Range { start, end })
}

// ── Conformance result from adapter issues ───────────────────────────────────

fn conformance_from_issues(
    issues: &[gc005_wasm4pm_adapter::ConformanceIssue],
) -> Option<ConformanceResult> {
    for issue in issues {
        match issue.code.as_str() {
            "WASM4PM-VERDICT-FIT" => {
                // parse fitness from message "... FIT (Fitness: 1.0)"
                let fitness = parse_fitness(&issue.message);
                return Some(ConformanceResult {
                    verdict: GallVerdict::Fit {
                        fitness: fitness.unwrap_or(1.0),
                    },
                    fitness,
                });
            }
            "WASM4PM-VERDICT-DEVIATION" => {
                let fitness = parse_fitness(&issue.message);
                return Some(ConformanceResult {
                    verdict: GallVerdict::Deviation {
                        fitness: fitness.unwrap_or(0.0),
                        missing: vec![],
                    },
                    fitness,
                });
            }
            "WASM4PM-VERDICT-BLOCKED" => {
                return Some(ConformanceResult {
                    verdict: GallVerdict::Blocked {
                        reason: issue.message.clone(),
                    },
                    fitness: None,
                });
            }
            "WASM4PM-VERDICT-INCONCLUSIVE" => {
                return Some(ConformanceResult {
                    verdict: GallVerdict::Inconclusive,
                    fitness: None,
                });
            }
            _ => {}
        }
    }
    None
}

fn parse_fitness(msg: &str) -> Option<f32> {
    // Look for "Fitness: 0.9" pattern
    let marker = "Fitness: ";
    let pos = msg.find(marker)?;
    let rest = &msg[pos + marker.len()..];
    let end = rest
        .find(|c: char| !c.is_ascii_digit() && c != '.')
        .unwrap_or(rest.len());
    rest[..end].parse().ok()
}

// ── Backend ──────────────────────────────────────────────────────────────────

#[derive(Debug)]
struct Backend {
    client: Client,
    documents: Arc<DashMap<Url, DocumentState>>,
}

impl Backend {
    async fn store_and_diagnose(&self, uri: Url, text: String) {
        let issues = if uri.path().as_str().ends_with(".ocel.json") {
            analyze_ocel(&text)
        } else {
            vec![]
        };

        let index = parse_ocel(&text);
        let conformance = conformance_from_issues(&issues);

        self.documents.insert(
            uri.clone(),
            DocumentState {
                text: text.clone(),
                index,
                conformance,
            },
        );

        // Build push diagnostics
        let mut diags = Vec::new();
        let path = uri.path().to_string();

        if path.ends_with(".ocel.json") {
            for issue in &issues {
                let severity = match issue.severity.as_str() {
                    "INFORMATION" => DiagnosticSeverity::INFORMATION,
                    "WARNING" => DiagnosticSeverity::WARNING,
                    _ => DiagnosticSeverity::ERROR,
                };
                diags.push(Diagnostic {
                    range: Range::default(),
                    severity: Some(severity),
                    code: Some(NumberOrString::String(issue.code.clone())),
                    message: issue.message.clone(),
                    source: Some("wasm4pm-lsp".to_string()),
                    ..Default::default()
                });
            }

            // Structural diagnostics from index
            if let Some(ref idx) = self.documents.get(&uri).and_then(|d| d.index.clone()) {
                // (a) dangling references
                for ev in &idx.events {
                    for (oid, _qual, range) in &ev.relationships {
                        if !idx.obj_by_id.contains_key(oid.as_str()) {
                            diags.push(Diagnostic {
                                range: *range,
                                severity: Some(DiagnosticSeverity::ERROR),
                                code: Some(NumberOrString::String(
                                    "WASM4PM-DANGLING-REF".to_string(),
                                )),
                                message: format!(
                                    "Dangling objectId reference: \"{}\" not found in objects",
                                    oid
                                ),
                                source: Some("wasm4pm-lsp".to_string()),
                                ..Default::default()
                            });
                        }
                    }
                }
                // (b) time ordering
                let mut prev_time: Option<&str> = None;
                for ev in &idx.events {
                    if let Some(pt) = prev_time {
                        if ev.time.as_str() < pt {
                            diags.push(Diagnostic {
                                range: ev.range,
                                severity: Some(DiagnosticSeverity::WARNING),
                                code: Some(NumberOrString::String(
                                    "WASM4PM-TIME-ORDER".to_string(),
                                )),
                                message: format!(
                                    "Event \"{}\" is out of chronological order",
                                    ev.id
                                ),
                                source: Some("wasm4pm-lsp".to_string()),
                                ..Default::default()
                            });
                            break;
                        }
                    }
                    prev_time = Some(&ev.time);
                }
                // (c) unknown activity types
                for ev in &idx.events {
                    if !idx.event_types.contains(&ev.event_type) {
                        diags.push(Diagnostic {
                            range: ev.range,
                            severity: Some(DiagnosticSeverity::ERROR),
                            code: Some(NumberOrString::String(
                                "WASM4PM-UNKNOWN-ACTIVITY".to_string(),
                            )),
                            message: format!(
                                "Event type \"{}\" not declared in eventTypes",
                                ev.event_type
                            ),
                            source: Some("wasm4pm-lsp".to_string()),
                            ..Default::default()
                        });
                    }
                }
            }
        }

        // cli.rs and .rs diagnostics (preserved from original)
        if path.ends_with("cli.rs") {
            let has_verb = text.contains("#[verb")
                || text.contains("clap_noun_verb")
                || text.contains("clap-noun-verb");
            if !has_verb {
                diags.push(Diagnostic {
                    range: Range::default(),
                    severity: Some(DiagnosticSeverity::ERROR),
                    code: Some(NumberOrString::String(
                        "CLAP-PACK-HANDLER-UNBOUND".to_string(),
                    )),
                    message: "cli.rs has no clap-noun-verb handler binding".to_string(),
                    source: Some("wasm4pm-lsp".to_string()),
                    data: Some(serde_json::json!({ "source_id": "clap_noun_verb_pack_lsp" })),
                    ..Default::default()
                });
            }
            if text.contains("ggen:override") {
                diags.push(Diagnostic {
                    range: Range::default(),
                    severity: Some(DiagnosticSeverity::WARNING),
                    code: Some(NumberOrString::String(
                        "GGEN-PROJECTION-OVERRIDE".to_string(),
                    )),
                    message: "ggen projection state override detected in cli.rs".to_string(),
                    source: Some("wasm4pm-lsp".to_string()),
                    data: Some(serde_json::json!({ "source_id": "ggen_lsp_observer" })),
                    ..Default::default()
                });
            }
        }
        if path.ends_with("receipts.json") || (path.contains("receipt") && path.ends_with(".json"))
        {
            // Validate receipt JSON structure
            let valid = serde_json::from_str::<serde_json::Value>(&text)
                .map(|v| v.get("input_hash").is_some() && v.get("output_hash").is_some())
                .unwrap_or(false);
            if !valid {
                diags.push(Diagnostic {
                    range: Range::default(),
                    severity: Some(DiagnosticSeverity::ERROR),
                    code: Some(NumberOrString::String("GGEN-EVIDENCE-001".to_string())),
                    message: "Receipt file is missing required input_hash/output_hash fields"
                        .to_string(),
                    source: Some("wasm4pm-lsp".to_string()),
                    data: Some(serde_json::json!({ "source_id": "ggen_lsp_observer" })),
                    ..Default::default()
                });
            }
        }
        if path.ends_with(".rs") && !path.ends_with("cli.rs") {
            let has_mutation = text.contains("write_to_disk")
                || text.contains("fn write_")
                || text.contains("fn delete_")
                || text.contains("fn mutate_")
                || text.contains("fn update_file");
            if has_mutation {
                diags.push(Diagnostic {
                    range: Range::default(),
                    severity: Some(DiagnosticSeverity::ERROR),
                    code: Some(NumberOrString::String(
                        "TOWER-PACK-UNGUARDED-MUTATION".to_string(),
                    )),
                    message: "LSP surface must be read-only; direct file mutation detected"
                        .to_string(),
                    source: Some("wasm4pm-lsp".to_string()),
                    data: Some(serde_json::json!({ "source_id": "lsp_max_pack_lsp" })),
                    ..Default::default()
                });
            }
        }

        self.client.publish_diagnostics(uri, diags, None).await;
    }

    fn get_doc(&self, uri: &Url) -> Option<dashmap::mapref::one::Ref<'_, Url, DocumentState>> {
        self.documents.get(uri)
    }
}

// ── LanguageServer impl ───────────────────────────────────────────────────────

#[lsp_max::async_trait]
impl lsp_max::LanguageServer for Backend {
    // LSP 3.18 §3.1 — Initialize
    async fn initialize(&self, _: InitializeParams) -> Result<InitializeResult> {
        Ok(InitializeResult {
            capabilities: ServerCapabilities {
                // LSP 3.18 §3.15 — Text Document Sync
                text_document_sync: Some(TextDocumentSyncCapability::Kind(
                    TextDocumentSyncKind::FULL,
                )),
                // LSP 3.18 §3.5 — Hover
                hover_provider: Some(HoverProviderCapability::Simple(true)),
                // LSP 3.18 §3.6 — Completion
                completion_provider: Some(CompletionOptions {
                    trigger_characters: Some(vec!["\"".to_string(), ":".to_string()]),
                    ..Default::default()
                }),
                // LSP 3.18 §3.10 — Document Symbol
                document_symbol_provider: Some(OneOf::Left(true)),
                // LSP 3.18 §3.22 — Semantic Tokens
                semantic_tokens_provider: Some(
                    SemanticTokensServerCapabilities::SemanticTokensOptions(
                        SemanticTokensOptions {
                            legend: token_legend(),
                            full: Some(SemanticTokensFullOptions::Bool(true)),
                            range: None,
                            ..Default::default()
                        },
                    ),
                ),
                // LSP 3.18 §3.20 — Inlay Hints
                inlay_hint_provider: Some(OneOf::Left(true)),
                // LSP 3.18 §3.16 — Code Lens
                code_lens_provider: Some(CodeLensOptions {
                    resolve_provider: Some(false),
                }),
                // LSP 3.18 §3.19 — Folding Range
                folding_range_provider: Some(FoldingRangeProviderCapability::Simple(true)),
                // LSP 3.18 §3.9 — Go to Definition
                definition_provider: Some(OneOf::Left(true)),
                // LSP 3.18 §3.11 — References
                references_provider: Some(OneOf::Left(true)),
                // LSP 3.18 §3.13 — Rename
                rename_provider: Some(OneOf::Right(RenameOptions {
                    prepare_provider: Some(true),
                    work_done_progress_options: Default::default(),
                })),
                // LSP 3.18 §3.17 — Diagnostics (pull model)
                diagnostic_provider: Some(DiagnosticServerCapabilities::Options(
                    DiagnosticOptions {
                        identifier: Some("wasm4pm".to_string()),
                        inter_file_dependencies: false,
                        workspace_diagnostics: false,
                        work_done_progress_options: Default::default(),
                    },
                )),
                // LSP 3.18 §3.12 — Document Highlight
                document_highlight_provider: Some(OneOf::Left(true)),
                // LSP 3.18 §3.18 — Formatting
                document_formatting_provider: Some(OneOf::Left(true)),
                // LSP 3.18 §3.4 — Code Action
                code_action_provider: Some(CodeActionProviderCapability::Simple(true)),
                ..Default::default()
            },
            ..Default::default()
        })
    }

    async fn initialized(&self, _: InitializedParams) {
        self.client
            .log_message(MessageType::INFO, "wasm4pm-lsp initialized")
            .await;
    }

    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }

    // LSP 3.18 §3.15.1 — didOpen
    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        self.store_and_diagnose(params.text_document.uri, params.text_document.text)
            .await;
    }

    // LSP 3.18 §3.15.2 — didChange
    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        if let Some(change) = params.content_changes.into_iter().next() {
            self.store_and_diagnose(params.text_document.uri, change.text)
                .await;
        }
    }

    // LSP 3.18 §3.5 — Hover
    async fn hover(&self, params: HoverParams) -> Result<Option<Hover>> {
        let uri = &params.text_document_position_params.text_document.uri;
        let pos = params.text_document_position_params.position;
        let doc = match self.get_doc(uri) {
            Some(d) => d,
            None => return Ok(None),
        };
        let idx = match &doc.index {
            Some(i) => i.clone(),
            None => return Ok(None),
        };
        let text = doc.text.clone();
        drop(doc);

        // Find the word under cursor
        let line_str = text.lines().nth(pos.line as usize).unwrap_or("");
        let col = pos.character as usize;
        // Extract quoted token at cursor position
        let token = extract_token_at(line_str, col);
        if token.is_empty() {
            return Ok(None);
        }

        // Check if it's an eventId
        if let Some(ei) = idx.event_by_id.get(&token) {
            let ev = &idx.events[*ei];
            let md = format!(
                "**Event** `{}`\nType: `{}`\nTimestamp: `{}`\nRelationships: {}",
                ev.id,
                ev.event_type,
                ev.time,
                ev.relationships.len()
            );
            return Ok(Some(Hover {
                contents: HoverContents::Markup(MarkupContent {
                    kind: MarkupKind::Markdown,
                    value: md,
                }),
                range: None,
            }));
        }

        // Check if it's an objectId
        if let Some(oi) = idx.obj_by_id.get(&token) {
            let obj = &idx.objects[*oi];
            let ref_count = idx.obj_refs.get(&token).map(|v| v.len()).unwrap_or(0);
            let md = format!(
                "**Object** `{}`\nType: `{}`\nReferenced by {} event(s)",
                obj.id, obj.obj_type, ref_count
            );
            return Ok(Some(Hover {
                contents: HoverContents::Markup(MarkupContent {
                    kind: MarkupKind::Markdown,
                    value: md,
                }),
                range: None,
            }));
        }

        // Check if it's an activity name (event type)
        if idx.event_types.contains(&token) {
            let count = idx.events.iter().filter(|e| e.event_type == token).count();
            let md = format!("**Activity** `{}`\nOccurrences in log: {}", token, count);
            return Ok(Some(Hover {
                contents: HoverContents::Markup(MarkupContent {
                    kind: MarkupKind::Markdown,
                    value: md,
                }),
                range: None,
            }));
        }

        Ok(None)
    }

    // LSP 3.18 §3.6 — Completion
    async fn completion(&self, params: CompletionParams) -> Result<Option<CompletionResponse>> {
        let uri = &params.text_document_position.text_document.uri;
        let pos = params.text_document_position.position;
        let doc = match self.get_doc(uri) {
            Some(d) => d,
            None => return Ok(None),
        };
        let idx = match &doc.index {
            Some(i) => i.clone(),
            None => return Ok(None),
        };
        let text = doc.text.clone();
        drop(doc);

        let context = detect_json_context(&text, pos);
        let items: Vec<CompletionItem> = match context.as_str() {
            "relationship_objectId" => idx
                .obj_by_id
                .keys()
                .map(|id| {
                    let oi = idx.obj_by_id[id];
                    let obj_type = &idx.objects[oi].obj_type;
                    CompletionItem {
                        label: id.clone(),
                        kind: Some(CompletionItemKind::REFERENCE),
                        detail: Some(obj_type.clone()),
                        ..Default::default()
                    }
                })
                .collect(),
            "event_type" => idx
                .event_types
                .iter()
                .map(|t| CompletionItem {
                    label: t.clone(),
                    kind: Some(CompletionItemKind::ENUM_MEMBER),
                    ..Default::default()
                })
                .collect(),
            "object_type" => idx
                .object_types
                .iter()
                .map(|t| CompletionItem {
                    label: t.clone(),
                    kind: Some(CompletionItemKind::ENUM_MEMBER),
                    ..Default::default()
                })
                .collect(),
            "qualifier" => ["subject", "proves", "uses", "produces"]
                .iter()
                .map(|q| CompletionItem {
                    label: q.to_string(),
                    kind: Some(CompletionItemKind::VALUE),
                    ..Default::default()
                })
                .collect(),
            _ => return Ok(None),
        };

        Ok(Some(CompletionResponse::Array(items)))
    }

    // LSP 3.18 §3.10 — Document Symbol
    async fn document_symbol(
        &self,
        params: DocumentSymbolParams,
    ) -> Result<Option<DocumentSymbolResponse>> {
        let uri = &params.text_document.uri;
        let doc = match self.get_doc(uri) {
            Some(d) => d,
            None => return Ok(None),
        };
        let idx = match &doc.index {
            Some(i) => i.clone(),
            None => return Ok(None),
        };
        let conformance = doc.conformance.clone();
        drop(doc);

        let fitness_prefix = conformance
            .as_ref()
            .and_then(|c| c.fitness)
            .map(|f| format!("Fitness: {:.2} — ", f))
            .unwrap_or_default();

        let events_range = Range {
            start: Position {
                line: idx.events_key_line,
                character: 0,
            },
            end: Position {
                line: idx.events_end_line,
                character: 0,
            },
        };
        let objects_range = Range {
            start: Position {
                line: idx.objects_key_line,
                character: 0,
            },
            end: Position {
                line: idx.objects_end_line,
                character: 0,
            },
        };

        let event_children: Vec<DocumentSymbol> = idx
            .events
            .iter()
            .map(|ev| {
                let rel_children: Vec<DocumentSymbol> = ev
                    .relationships
                    .iter()
                    .map(|(oid, _qual, range)| {
                        #[allow(deprecated)]
                        DocumentSymbol {
                            name: oid.clone(),
                            kind: SymbolKind::KEY,
                            range: *range,
                            selection_range: *range,
                            detail: None,
                            children: None,
                            deprecated: None,
                            tags: None,
                        }
                    })
                    .collect();
                #[allow(deprecated)]
                DocumentSymbol {
                    name: ev.id.clone(),
                    kind: SymbolKind::EVENT,
                    detail: Some(ev.event_type.clone()),
                    range: ev.range,
                    selection_range: ev.range,
                    children: Some(rel_children),
                    deprecated: None,
                    tags: None,
                }
            })
            .collect();

        let object_children: Vec<DocumentSymbol> = idx
            .objects
            .iter()
            .map(|obj| {
                #[allow(deprecated)]
                DocumentSymbol {
                    name: obj.id.clone(),
                    kind: SymbolKind::OBJECT,
                    detail: Some(obj.obj_type.clone()),
                    range: obj.range,
                    selection_range: obj.range,
                    children: None,
                    deprecated: None,
                    tags: None,
                }
            })
            .collect();

        #[allow(deprecated)]
        let root = vec![
            DocumentSymbol {
                name: "events".to_string(),
                kind: SymbolKind::ARRAY,
                detail: Some(format!("{}{}  event(s)", fitness_prefix, idx.events.len())),
                range: events_range,
                selection_range: events_range,
                children: Some(event_children),
                deprecated: None,
                tags: None,
            },
            DocumentSymbol {
                name: "objects".to_string(),
                kind: SymbolKind::ARRAY,
                detail: Some(format!("{} object(s)", idx.objects.len())),
                range: objects_range,
                selection_range: objects_range,
                children: Some(object_children),
                deprecated: None,
                tags: None,
            },
        ];

        Ok(Some(DocumentSymbolResponse::Nested(root)))
    }

    // LSP 3.18 §3.22 — Semantic Tokens Full
    async fn semantic_tokens_full(
        &self,
        params: SemanticTokensParams,
    ) -> Result<Option<SemanticTokensResult>> {
        let uri = &params.text_document.uri;
        let doc = match self.get_doc(uri) {
            Some(d) => d,
            None => return Ok(None),
        };
        let idx = match &doc.index {
            Some(i) => i.clone(),
            None => return Ok(None),
        };
        drop(doc);

        // Token type indices (per legend):
        // namespace=0, class=1, function=2, variable=3, string=4, number=5, keyword=6, comment=7
        // OCEL mapping: eventId→function(2), objectId→class(1), activityName→keyword(6), timestamp→string(4), attributeName→variable(3)

        let mut raw: Vec<(Range, u32)> = Vec::new();

        // eventId values → function (2)
        for ev in &idx.events {
            raw.push((ev.range, 2));
            // activity name (event type) → keyword (6)
            // find type value range
            // timestamp → string (4)
        }

        // objectId values in objects → class (1) with declaration modifier
        for obj in &idx.objects {
            raw.push((obj.range, 1));
        }

        // relationship objectId values → class (1)
        for ev in &idx.events {
            for (_oid, _qual, range) in &ev.relationships {
                raw.push((*range, 1));
            }
        }

        // Sort by position
        raw.sort_by(|a, b| {
            a.0.start
                .line
                .cmp(&b.0.start.line)
                .then(a.0.start.character.cmp(&b.0.start.character))
        });

        // Encode as LSP delta-encoded semantic tokens
        let mut data: Vec<SemanticToken> = Vec::new();
        let mut prev_line = 0u32;
        let mut prev_char = 0u32;

        for (range, token_type) in &raw {
            let delta_line = range.start.line - prev_line;
            let delta_char = if delta_line == 0 {
                range.start.character - prev_char
            } else {
                range.start.character
            };
            let length = range.end.character.saturating_sub(range.start.character);
            data.push(SemanticToken {
                delta_line,
                delta_start: delta_char,
                length,
                token_type: *token_type,
                token_modifiers_bitset: 0,
            });
            prev_line = range.start.line;
            prev_char = range.start.character;
        }

        Ok(Some(SemanticTokensResult::Tokens(SemanticTokens {
            result_id: None,
            data,
        })))
    }

    // LSP 3.18 §3.20 — Inlay Hints
    async fn inlay_hint(&self, params: InlayHintParams) -> Result<Option<Vec<InlayHint>>> {
        let uri = &params.text_document.uri;
        let doc = match self.get_doc(uri) {
            Some(d) => d,
            None => return Ok(None),
        };
        let idx = match &doc.index {
            Some(i) => i.clone(),
            None => return Ok(None),
        };
        let conformance = doc.conformance.clone();
        drop(doc);

        let mut hints: Vec<InlayHint> = Vec::new();

        // Fitness hint on events key
        if let Some(ref conf) = conformance {
            if let Some(fitness) = conf.fitness {
                hints.push(InlayHint {
                    position: Position {
                        line: idx.events_key_line,
                        character: 8,
                    },
                    label: InlayHintLabel::String(format!(" fitness:{:.2}", fitness)),
                    kind: Some(InlayHintKind::PARAMETER),
                    text_edits: None,
                    tooltip: None,
                    padding_left: Some(true),
                    padding_right: None,
                    data: None,
                });
            }
        }

        // Orphan event hints
        let referenced_ids: std::collections::HashSet<String> = idx
            .events
            .iter()
            .flat_map(|ev| ev.relationships.iter().map(|(oid, _, _)| oid.clone()))
            .collect();
        // Events not referenced by any relationship
        for ev in &idx.events {
            let is_orphan = !referenced_ids.contains(&ev.id);
            if is_orphan && ev.relationships.is_empty() {
                // Only hint events with no outgoing relationships (truly unlinked)
                hints.push(InlayHint {
                    position: ev.range.end,
                    label: InlayHintLabel::String(" ⚠ unlinked".to_string()),
                    kind: Some(InlayHintKind::TYPE),
                    text_edits: None,
                    tooltip: None,
                    padding_left: Some(true),
                    padding_right: None,
                    data: None,
                });
            }
        }

        Ok(Some(hints))
    }

    // LSP 3.18 §3.16 — Code Lens
    async fn code_lens(&self, params: CodeLensParams) -> Result<Option<Vec<CodeLens>>> {
        let uri = &params.text_document.uri;
        let doc = match self.get_doc(uri) {
            Some(d) => d,
            None => return Ok(None),
        };
        let idx = match &doc.index {
            Some(i) => i.clone(),
            None => return Ok(None),
        };
        let conformance = doc.conformance.clone();
        drop(doc);

        let mut lenses: Vec<CodeLens> = Vec::new();

        // (1) Check Conformance lens at line 0
        lenses.push(CodeLens {
            range: Range {
                start: Position {
                    line: 0,
                    character: 0,
                },
                end: Position {
                    line: 0,
                    character: 1,
                },
            },
            command: Some(Command {
                title: "▶ Check Conformance".to_string(),
                command: "wasm4pm.checkConformance".to_string(),
                arguments: Some(vec![serde_json::to_value(uri).unwrap_or_default()]),
            }),
            data: None,
        });

        // (2) Verdict-dependent lens on events key line
        if let Some(ref conf) = conformance {
            let events_line = idx.events_key_line;
            match &conf.verdict {
                GallVerdict::Fit { fitness } => {
                    lenses.push(CodeLens {
                        range: Range {
                            start: Position {
                                line: events_line,
                                character: 0,
                            },
                            end: Position {
                                line: events_line,
                                character: 1,
                            },
                        },
                        command: Some(Command {
                            title: "⬡ Bind Receipt".to_string(),
                            command: "conformance-receipt.bind".to_string(),
                            arguments: Some(vec![
                                serde_json::to_value(uri).unwrap_or_default(),
                                serde_json::json!(fitness),
                            ]),
                        }),
                        data: None,
                    });
                }
                GallVerdict::Deviation { missing, .. } => {
                    lenses.push(CodeLens {
                        range: Range {
                            start: Position {
                                line: events_line,
                                character: 0,
                            },
                            end: Position {
                                line: events_line,
                                character: 1,
                            },
                        },
                        command: Some(Command {
                            title: format!("⚑ {} Missing Admissions", missing.len()),
                            command: String::new(),
                            arguments: None,
                        }),
                        data: None,
                    });
                }
                _ => {}
            }
        }

        Ok(Some(lenses))
    }

    // LSP 3.18 §3.19 — Folding Range
    async fn folding_range(&self, params: FoldingRangeParams) -> Result<Option<Vec<FoldingRange>>> {
        let uri = &params.text_document.uri;
        let doc = match self.get_doc(uri) {
            Some(d) => d,
            None => return Ok(None),
        };
        let idx = match &doc.index {
            Some(i) => i.clone(),
            None => return Ok(None),
        };
        drop(doc);

        let mut ranges: Vec<FoldingRange> = Vec::new();

        // Top-level events array
        if idx.events_end_line > idx.events_key_line {
            ranges.push(FoldingRange {
                start_line: idx.events_key_line,
                end_line: idx.events_end_line,
                kind: Some(FoldingRangeKind::Region),
                start_character: None,
                end_character: None,
                collapsed_text: None,
            });
        }

        // Top-level objects array
        if idx.objects_end_line > idx.objects_key_line {
            ranges.push(FoldingRange {
                start_line: idx.objects_key_line,
                end_line: idx.objects_end_line,
                kind: Some(FoldingRangeKind::Region),
                start_character: None,
                end_character: None,
                collapsed_text: None,
            });
        }

        // Individual event objects
        for ev in &idx.events {
            if ev.range.end.line > ev.range.start.line {
                ranges.push(FoldingRange {
                    start_line: ev.range.start.line,
                    end_line: ev.range.end.line,
                    kind: None,
                    start_character: None,
                    end_character: None,
                    collapsed_text: None,
                });
            }
        }

        // Individual object entries
        for obj in &idx.objects {
            if obj.range.end.line > obj.range.start.line {
                ranges.push(FoldingRange {
                    start_line: obj.range.start.line,
                    end_line: obj.range.end.line,
                    kind: None,
                    start_character: None,
                    end_character: None,
                    collapsed_text: None,
                });
            }
        }

        Ok(Some(ranges))
    }

    // LSP 3.18 §3.9 — Go to Definition
    // Triggered when cursor is on a relationship objectId value; returns the object definition range.
    async fn goto_definition(
        &self,
        params: GotoDefinitionParams,
    ) -> Result<Option<GotoDefinitionResponse>> {
        let uri = &params.text_document_position_params.text_document.uri;
        let pos = params.text_document_position_params.position;
        let doc = match self.get_doc(uri) {
            Some(d) => d,
            None => return Ok(None),
        };
        let idx = match &doc.index {
            Some(i) => i.clone(),
            None => return Ok(None),
        };
        let text = doc.text.clone();
        drop(doc);

        let line_str = text.lines().nth(pos.line as usize).unwrap_or("");
        let token = extract_token_at(line_str, pos.character as usize);
        if token.is_empty() {
            return Ok(None);
        }

        // Must be an objectId
        if let Some(&oi) = idx.obj_by_id.get(&token) {
            let obj = &idx.objects[oi];
            return Ok(Some(GotoDefinitionResponse::Scalar(Location {
                uri: uri.clone(),
                range: obj.range,
            })));
        }

        Ok(None)
    }

    // LSP 3.18 §3.11 — References
    // Given cursor on an object id, find all event relationships pointing to it.
    async fn references(&self, params: ReferenceParams) -> Result<Option<Vec<Location>>> {
        let uri = &params.text_document_position.text_document.uri;
        let pos = params.text_document_position.position;
        let doc = match self.get_doc(uri) {
            Some(d) => d,
            None => return Ok(None),
        };
        let idx = match &doc.index {
            Some(i) => i.clone(),
            None => return Ok(None),
        };
        let text = doc.text.clone();
        drop(doc);

        let line_str = text.lines().nth(pos.line as usize).unwrap_or("");
        let token = extract_token_at(line_str, pos.character as usize);
        if token.is_empty() {
            return Ok(None);
        }

        let mut locations: Vec<Location> = Vec::new();

        // include declaration if requested
        if params.context.include_declaration {
            if let Some(&oi) = idx.obj_by_id.get(&token) {
                locations.push(Location {
                    uri: uri.clone(),
                    range: idx.objects[oi].range,
                });
            }
        }

        // All relationship references
        if let Some(refs) = idx.obj_refs.get(&token) {
            for r in refs {
                locations.push(Location {
                    uri: uri.clone(),
                    range: *r,
                });
            }
        }

        if locations.is_empty() {
            return Ok(None);
        }
        Ok(Some(locations))
    }

    // LSP 3.18 §3.13 — Prepare Rename
    async fn prepare_rename(
        &self,
        params: TextDocumentPositionParams,
    ) -> Result<Option<PrepareRenameResponse>> {
        let uri = &params.text_document.uri;
        let pos = params.position;
        let doc = match self.get_doc(uri) {
            Some(d) => d,
            None => return Ok(None),
        };
        let idx = match &doc.index {
            Some(i) => i.clone(),
            None => return Ok(None),
        };
        let text = doc.text.clone();
        drop(doc);

        let line_str = text.lines().nth(pos.line as usize).unwrap_or("");
        let token = extract_token_at(line_str, pos.character as usize);
        if token.is_empty() {
            return Ok(None);
        }

        // Only allow rename of objectIds
        if idx.obj_by_id.contains_key(&token) {
            let range = find_value_range(&text, &token).unwrap_or_default();
            return Ok(Some(PrepareRenameResponse::Range(range)));
        }

        Ok(None)
    }

    // LSP 3.18 §3.13 — Rename
    async fn rename(&self, params: RenameParams) -> Result<Option<WorkspaceEdit>> {
        let uri = &params.text_document_position.text_document.uri;
        let pos = params.text_document_position.position;
        let new_name = &params.new_name;
        let doc = match self.get_doc(uri) {
            Some(d) => d,
            None => return Ok(None),
        };
        let idx = match &doc.index {
            Some(i) => i.clone(),
            None => return Ok(None),
        };
        let text = doc.text.clone();
        drop(doc);

        // Reject if new_name already exists
        if idx.obj_by_id.contains_key(new_name.as_str()) {
            return Err(lsp_max::jsonrpc::Error {
                code: lsp_max::jsonrpc::ErrorCode::InvalidParams,
                message: format!("Name '{}' already exists", new_name).into(),
                data: None,
            });
        }

        let line_str = text.lines().nth(pos.line as usize).unwrap_or("");
        let token = extract_token_at(line_str, pos.character as usize);
        if token.is_empty() || !idx.obj_by_id.contains_key(&token) {
            return Ok(None);
        }

        // Collect all occurrences
        let mut edits: Vec<TextEdit> = Vec::new();
        for range in find_all_quoted(&text, &token) {
            edits.push(TextEdit {
                range,
                new_text: new_name.clone(),
            });
        }

        #[allow(clippy::mutable_key_type)]
        let mut changes = HashMap::new();
        changes.insert(uri.clone(), edits);

        Ok(Some(WorkspaceEdit {
            changes: Some(changes),
            document_changes: None,
            change_annotations: None,
            metadata: None,
        }))
    }

    // LSP 3.18 §3.17 — Diagnostic (pull model)
    async fn diagnostic(
        &self,
        params: DocumentDiagnosticParams,
    ) -> Result<DocumentDiagnosticReportResult> {
        let uri = &params.text_document.uri;
        let doc = match self.get_doc(uri) {
            Some(d) => d,
            None => {
                return Ok(DocumentDiagnosticReportResult::Report(
                    DocumentDiagnosticReport::Full(RelatedFullDocumentDiagnosticReport {
                        related_documents: None,
                        full_document_diagnostic_report: FullDocumentDiagnosticReport {
                            result_id: None,
                            items: vec![],
                        },
                    }),
                ))
            }
        };
        let idx = doc.index.clone();
        let text = doc.text.clone();
        drop(doc);

        let mut diags = Vec::new();

        // Re-run OCEL analysis
        if uri.path().as_str().ends_with(".ocel.json") {
            let issues = analyze_ocel(&text);
            for issue in &issues {
                let severity = match issue.severity.as_str() {
                    "INFORMATION" => DiagnosticSeverity::INFORMATION,
                    "WARNING" => DiagnosticSeverity::WARNING,
                    _ => DiagnosticSeverity::ERROR,
                };
                diags.push(Diagnostic {
                    range: Range::default(),
                    severity: Some(severity),
                    code: Some(NumberOrString::String(issue.code.clone())),
                    message: issue.message.clone(),
                    source: Some("wasm4pm".to_string()),
                    ..Default::default()
                });
            }

            if let Some(ref idx) = idx {
                // (a) dangling refs
                for ev in &idx.events {
                    for (oid, _qual, range) in &ev.relationships {
                        if !idx.obj_by_id.contains_key(oid.as_str()) {
                            diags.push(Diagnostic {
                                range: *range,
                                severity: Some(DiagnosticSeverity::ERROR),
                                code: Some(NumberOrString::String(
                                    "WASM4PM-DANGLING-REF".to_string(),
                                )),
                                message: format!("Dangling objectId: \"{}\" not in objects", oid),
                                source: Some("wasm4pm".to_string()),
                                ..Default::default()
                            });
                        }
                    }
                }
                // (b) time order
                let mut prev: Option<&str> = None;
                for ev in &idx.events {
                    if let Some(p) = prev {
                        if ev.time.as_str() < p {
                            diags.push(Diagnostic {
                                range: ev.range,
                                severity: Some(DiagnosticSeverity::WARNING),
                                code: Some(NumberOrString::String(
                                    "WASM4PM-TIME-ORDER".to_string(),
                                )),
                                message: format!("Event \"{}\" out of time order", ev.id),
                                source: Some("wasm4pm".to_string()),
                                ..Default::default()
                            });
                            break;
                        }
                    }
                    prev = Some(&ev.time);
                }
                // (c) unknown activity
                for ev in &idx.events {
                    if !idx.event_types.contains(&ev.event_type) {
                        diags.push(Diagnostic {
                            range: ev.range,
                            severity: Some(DiagnosticSeverity::ERROR),
                            code: Some(NumberOrString::String(
                                "WASM4PM-UNKNOWN-ACTIVITY".to_string(),
                            )),
                            message: format!("Event type \"{}\" not in eventTypes", ev.event_type),
                            source: Some("wasm4pm".to_string()),
                            ..Default::default()
                        });
                    }
                }
            }
        }

        Ok(DocumentDiagnosticReportResult::Report(
            DocumentDiagnosticReport::Full(RelatedFullDocumentDiagnosticReport {
                related_documents: None,
                full_document_diagnostic_report: FullDocumentDiagnosticReport {
                    result_id: None,
                    items: diags,
                },
            }),
        ))
    }

    // LSP 3.18 §3.12 — Document Highlight
    async fn document_highlight(
        &self,
        params: DocumentHighlightParams,
    ) -> Result<Option<Vec<DocumentHighlight>>> {
        let uri = &params.text_document_position_params.text_document.uri;
        let pos = params.text_document_position_params.position;
        let doc = match self.get_doc(uri) {
            Some(d) => d,
            None => return Ok(None),
        };
        let idx = match &doc.index {
            Some(i) => i.clone(),
            None => return Ok(None),
        };
        let text = doc.text.clone();
        drop(doc);

        let line_str = text.lines().nth(pos.line as usize).unwrap_or("");
        let token = extract_token_at(line_str, pos.character as usize);
        if token.is_empty() {
            return Ok(None);
        }

        // objectId highlight
        if idx.obj_by_id.contains_key(&token) {
            let mut highlights = Vec::new();
            // definition site → Write
            if let Some(&oi) = idx.obj_by_id.get(&token) {
                highlights.push(DocumentHighlight {
                    range: idx.objects[oi].range,
                    kind: Some(DocumentHighlightKind::WRITE),
                });
            }
            // relationship occurrences → Read
            if let Some(refs) = idx.obj_refs.get(&token) {
                for r in refs {
                    highlights.push(DocumentHighlight {
                        range: *r,
                        kind: Some(DocumentHighlightKind::READ),
                    });
                }
            }
            return Ok(Some(highlights));
        }

        // activity name highlight
        if idx.event_types.contains(&token) {
            let highlights: Vec<DocumentHighlight> = idx
                .events
                .iter()
                .filter(|ev| ev.event_type == token)
                .map(|ev| DocumentHighlight {
                    range: ev.range,
                    kind: Some(DocumentHighlightKind::TEXT),
                })
                .collect();
            return Ok(Some(highlights));
        }

        Ok(None)
    }

    // LSP 3.18 §3.18 — Formatting
    async fn formatting(&self, params: DocumentFormattingParams) -> Result<Option<Vec<TextEdit>>> {
        let uri = &params.text_document.uri;
        let doc = match self.get_doc(uri) {
            Some(d) => d,
            None => return Ok(None),
        };
        let text = doc.text.clone();
        let tab_size = params.options.tab_size;
        drop(doc);

        let v: Value = match serde_json::from_str(&text) {
            Ok(v) => v,
            Err(_) => return Ok(Some(vec![])),
        };

        let pretty = if tab_size == 2 {
            serde_json::to_string_pretty(&v).unwrap_or_default()
        } else {
            // Custom indent
            indent_json(&v, tab_size as usize)
        };
        let new_text = format!("{}\n", pretty);

        let line_count = text.lines().count() as u32;
        let last_line = text.lines().last().unwrap_or("");
        let end_char = last_line.len() as u32;

        Ok(Some(vec![TextEdit {
            range: Range {
                start: Position {
                    line: 0,
                    character: 0,
                },
                end: Position {
                    line: line_count,
                    character: end_char,
                },
            },
            new_text,
        }]))
    }

    // LSP 3.18 §3.4 — Code Action
    async fn code_action(&self, params: CodeActionParams) -> Result<Option<CodeActionResponse>> {
        let mut actions = Vec::new();
        for diag in params.context.diagnostics {
            if let Some(NumberOrString::String(code)) = &diag.code {
                if code == "WASM4PM-VERDICT-FIT" {
                    actions.push(CodeActionOrCommand::CodeAction(CodeAction {
                        title: "Bind Conformance Receipt".to_string(),
                        kind: Some(CodeActionKind::QUICKFIX),
                        diagnostics: Some(vec![diag.clone()]),
                        command: Some(Command {
                            title: "Bind Conformance Receipt".to_string(),
                            command: "conformance-receipt.bind".to_string(),
                            arguments: Some(vec![serde_json::to_value(
                                params.text_document.uri.clone(),
                            )
                            .unwrap_or_default()]),
                        }),
                        ..Default::default()
                    }));
                }
            }
        }
        Ok(Some(actions))
    }
}

// ── JSON context detection ────────────────────────────────────────────────────

/// Very lightweight JSON path context detector.
/// Returns a string key describing what kind of value the cursor is in.
fn detect_json_context(text: &str, pos: Position) -> String {
    // Walk backwards from cursor scanning for the enclosing key
    let lines: Vec<&str> = text.lines().collect();
    let target = pos.line as usize;
    // Scan current and preceding lines for nearest key
    let scan_start = target.saturating_sub(5);
    let snippet: String = lines[scan_start..=target.min(lines.len().saturating_sub(1))].join("\n");

    if snippet.contains("\"objectId\"") && snippet.contains("\"relationships\"") {
        return "relationship_objectId".to_string();
    }
    if snippet.contains("\"qualifier\"") {
        return "qualifier".to_string();
    }
    // Determine if we are inside events[*].type or objects[*].type
    // Check surrounding context for "events" vs "objects"
    let broader_start = target.saturating_sub(20);
    let broader: String =
        lines[broader_start..=target.min(lines.len().saturating_sub(1))].join("\n");
    let current_line = lines.get(pos.line as usize).unwrap_or(&"");
    if current_line.contains("\"type\"") || current_line.trim_start().starts_with("\"type\"") {
        // Check if inside events or objects block
        if broader.rfind("\"events\"").unwrap_or(0) > broader.rfind("\"objects\"").unwrap_or(0) {
            return "event_type".to_string();
        } else {
            return "object_type".to_string();
        }
    }

    "unknown".to_string()
}

// ── Token extraction ──────────────────────────────────────────────────────────

/// Extract the quoted string token that the cursor (column col) is inside on a single line.
fn extract_token_at(line: &str, col: usize) -> String {
    let bytes = line.as_bytes();
    let col = col.min(bytes.len().saturating_sub(1));

    // Walk left to find opening quote
    let mut start = col;
    while start > 0 && bytes[start] != b'"' {
        start = start.saturating_sub(1);
    }
    if start >= bytes.len() || bytes[start] != b'"' {
        return String::new();
    }
    start += 1; // skip quote

    // Walk right to find closing quote
    let mut end = start;
    while end < bytes.len() && bytes[end] != b'"' {
        end += 1;
    }

    if end > start {
        line[start..end].to_string()
    } else {
        String::new()
    }
}

// ── Custom JSON indenter ──────────────────────────────────────────────────────

fn indent_json(v: &Value, spaces: usize) -> String {
    // Serialize with serde_json pretty (2 spaces) then re-indent
    let pretty = serde_json::to_string_pretty(v).unwrap_or_default();
    let indent_str = " ".repeat(spaces);
    let mut result = String::new();
    let mut depth = 0usize;
    let mut in_string = false;
    let mut chars = pretty.chars().peekable();
    while let Some(ch) = chars.next() {
        match ch {
            '"' => {
                in_string = !in_string;
                result.push(ch);
            }
            '\\' if in_string => {
                result.push(ch);
                if let Some(next) = chars.next() {
                    result.push(next);
                }
            }
            '{' | '[' if !in_string => {
                result.push(ch);
                depth += 1;
            }
            '}' | ']' if !in_string => {
                depth = depth.saturating_sub(1);
                result.push(ch);
            }
            '\n' if !in_string => {
                result.push('\n');
                result.push_str(&indent_str.repeat(depth));
            }
            ' ' if !in_string => {
                // Skip original spaces at line starts (already re-indented)
                // Keep single spaces after colons/commas
                result.push(ch);
            }
            _ => result.push(ch),
        }
    }
    result
}

// ── main ──────────────────────────────────────────────────────────────────────

#[tokio::main]
async fn main() {
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    let documents: Arc<DashMap<Url, DocumentState>> = Arc::new(DashMap::new());
    let (service, socket) = LspService::new(|client| Backend {
        client,
        documents: documents.clone(),
    });
    Server::new(stdin, stdout, socket)
        .serve(service)
        .await
        .unwrap();
}
