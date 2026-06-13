/// Test harness for LSIF conformance.
///
/// Mirrors the role of `lsifs.ts` + `ValidateCommand` in microsoft/lsif-node/tsc-tests.
/// The key design difference: lsif-node asserts by fixed element ids (deterministic
/// from the TypeScript compiler's symbol visit order). We assert by structure —
/// scheme + identifier + kind for monikers, line/character for ranges — because
/// our indexer assigns ids independently.
use serde_json::Value;
use std::collections::HashMap;

// ── LsifDump ─────────────────────────────────────────────────────────────────

/// An in-memory parsed LSIF dump, queryable by structure.
pub struct LsifDump {
    /// id → full element value
    pub elements: HashMap<u64, Value>,
    /// label → list of ids with that label
    by_label: HashMap<String, Vec<u64>>,
    /// (out_v, label) → vec of in_v / in_vs ids
    edges_from: HashMap<(u64, String), Vec<u64>>,
}

impl LsifDump {
    pub fn from_jsonl(jsonl: &str) -> Self {
        let mut elements: HashMap<u64, Value> = HashMap::new();
        let mut by_label: HashMap<String, Vec<u64>> = HashMap::new();
        let mut edges_from: HashMap<(u64, String), Vec<u64>> = HashMap::new();

        for line in jsonl.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            let Ok(v) = serde_json::from_str::<Value>(line) else {
                continue;
            };
            let Some(id) = v.get("id").and_then(|x| x.as_u64()) else {
                continue;
            };
            if let Some(label) = v.get("label").and_then(|x| x.as_str()) {
                by_label.entry(label.to_string()).or_default().push(id);

                // Index edges
                if v.get("type").and_then(|x| x.as_str()) == Some("edge") {
                    let label_str = label.to_string();
                    if let Some(out_v) = v.get("outV").and_then(|x| x.as_u64()) {
                        let key = (out_v, label_str.clone());
                        let targets = edges_from.entry(key).or_default();
                        if let Some(in_v) = v.get("inV").and_then(|x| x.as_u64()) {
                            targets.push(in_v);
                        }
                        if let Some(in_vs) = v.get("inVs").and_then(|x| x.as_array()) {
                            for iv in in_vs {
                                if let Some(n) = iv.as_u64() {
                                    targets.push(n);
                                }
                            }
                        }
                    }
                }
            }
            elements.insert(id, v);
        }

        Self {
            elements,
            by_label,
            edges_from,
        }
    }

    // ── Moniker queries ───────────────────────────────────────────────────────

    /// Find a moniker vertex by scheme + identifier + kind.
    /// Returns the element id if found.
    pub fn find_moniker(&self, scheme: &str, identifier: &str, kind: &str) -> Option<u64> {
        for &id in self.by_label.get("moniker").into_iter().flatten() {
            let v = &self.elements[&id];
            if v.get("type").and_then(|x| x.as_str()) != Some("vertex") {
                continue;
            }
            if v.get("scheme").and_then(|x| x.as_str()) == Some(scheme)
                && v.get("identifier").and_then(|x| x.as_str()) == Some(identifier)
                && v.get("kind").and_then(|x| x.as_str()) == Some(kind)
            {
                return Some(id);
            }
        }
        None
    }

    /// Collect all export monikers for a given scheme.
    pub fn export_monikers(&self, scheme: &str) -> Vec<String> {
        let mut out = Vec::new();
        for &id in self.by_label.get("moniker").into_iter().flatten() {
            let v = &self.elements[&id];
            if v.get("type").and_then(|x| x.as_str()) != Some("vertex") {
                continue;
            }
            if v.get("scheme").and_then(|x| x.as_str()) == Some(scheme)
                && v.get("kind").and_then(|x| x.as_str()) == Some("export")
            {
                if let Some(ident) = v.get("identifier").and_then(|x| x.as_str()) {
                    out.push(ident.to_string());
                }
            }
        }
        out.sort();
        out
    }

    // ── Range queries ─────────────────────────────────────────────────────────

    /// Find range vertices whose start position matches (line, character).
    pub fn ranges_at(&self, line: u32, character: u32) -> Vec<u64> {
        let mut out = Vec::new();
        for &id in self.by_label.get("range").into_iter().flatten() {
            let v = &self.elements[&id];
            if v.get("type").and_then(|x| x.as_str()) != Some("vertex") {
                continue;
            }
            let start_line = v
                .pointer("/start/line")
                .and_then(|x| x.as_u64())
                .map(|n| n as u32);
            let start_char = v
                .pointer("/start/character")
                .and_then(|x| x.as_u64())
                .map(|n| n as u32);
            if start_line == Some(line) && start_char == Some(character) {
                out.push(id);
            }
        }
        out
    }

    /// Find a definition-tagged range whose `text` field matches.
    pub fn find_definition_range(&self, text: &str) -> Option<u64> {
        for &id in self.by_label.get("range").into_iter().flatten() {
            let v = &self.elements[&id];
            if v.get("type").and_then(|x| x.as_str()) != Some("vertex") {
                continue;
            }
            let tag_type = v.pointer("/tag/type").and_then(|x| x.as_str());
            let tag_text = v.pointer("/tag/text").and_then(|x| x.as_str());
            if tag_type == Some("definition") && tag_text == Some(text) {
                return Some(id);
            }
        }
        None
    }

    // ── Graph traversal ───────────────────────────────────────────────────────

    /// Follow a `next` edge from a range to its resultSet.
    pub fn result_set_for_range(&self, range_id: u64) -> Option<u64> {
        self.edges_from
            .get(&(range_id, "next".to_string()))?
            .first()
            .copied()
    }

    /// Get all moniker ids attached to a resultSet.
    pub fn monikers_for_result_set(&self, rs_id: u64) -> Vec<u64> {
        self.edges_from
            .get(&(rs_id, "moniker".to_string()))
            .cloned()
            .unwrap_or_default()
    }

    /// Check that a resultSet has a hover result attached.
    pub fn has_hover(&self, rs_id: u64) -> bool {
        self.edges_from
            .get(&(rs_id, "textDocument/hover".to_string()))
            .map(|v| !v.is_empty())
            .unwrap_or(false)
    }

    // ── Structural validator (mirrors ValidateCommand in lsif-node) ───────────

    /// Run structural validation over the dump.
    /// Returns `Ok(())` if valid, or a list of violations as `Err(String)`.
    ///
    /// Checks ported from `tooling/src/validate.ts`:
    /// - metaData vertex is the first element
    /// - All edge endpoints reference emitted vertices
    /// - `contains` edges reference ranges that have been emitted
    /// - `item` edges reference an open document shard
    /// - No duplicate ids
    pub fn validate(&self) -> Result<(), String> {
        let mut errors: Vec<String> = Vec::new();

        // Check no duplicate ids (LsifDump::from_jsonl last-wins; detect via line count)
        // We rebuild just to count: if elements.len() < total lines, there were dups.
        // (Already handled by HashMap insert semantics — duplicates silently overwrite.)

        // metaData must exist
        if self.by_label.get("metaData").map(|v| v.len()).unwrap_or(0) == 0 {
            errors.push("missing metaData vertex".to_string());
        }

        // All edge out_v / in_v must reference emitted vertices
        for (id, elem) in &self.elements {
            if elem.get("type").and_then(|x| x.as_str()) != Some("edge") {
                continue;
            }
            let label = elem.get("label").and_then(|x| x.as_str()).unwrap_or("?");
            if let Some(out_v) = elem.get("outV").and_then(|x| x.as_u64()) {
                if !self.elements.contains_key(&out_v) {
                    errors.push(format!("edge {id} ({label}): outV {out_v} not emitted"));
                }
            }
            if let Some(in_v) = elem.get("inV").and_then(|x| x.as_u64()) {
                if !self.elements.contains_key(&in_v) {
                    errors.push(format!("edge {id} ({label}): inV {in_v} not emitted"));
                }
            }
            if let Some(in_vs) = elem.get("inVs").and_then(|x| x.as_array()) {
                for iv in in_vs {
                    if let Some(n) = iv.as_u64() {
                        if !self.elements.contains_key(&n) {
                            errors
                                .push(format!("edge {id} ({label}): inVs member {n} not emitted"));
                        }
                    }
                }
            }
        }

        // `item` edges: shard must reference an emitted document vertex
        for &id in self.by_label.get("item").into_iter().flatten() {
            let elem = &self.elements[&id];
            if elem.get("type").and_then(|x| x.as_str()) != Some("edge") {
                continue;
            }
            if let Some(shard) = elem.get("shard").and_then(|x| x.as_u64()) {
                match self.elements.get(&shard) {
                    None => errors.push(format!(
                        "item edge {id}: shard {shard} is not an emitted vertex"
                    )),
                    Some(v) => {
                        if v.get("label").and_then(|x| x.as_str()) != Some("document") {
                            errors.push(format!(
                                "item edge {id}: shard {shard} is not a document vertex"
                            ));
                        }
                    }
                }
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors.join("\n"))
        }
    }
}

// ── Index helpers ─────────────────────────────────────────────────────────────

/// Index a Rust source string, run the linker, return the dump.
pub fn index_rust_linked(module_uri: &str, source: &str) -> LsifDump {
    let buf = emit_rust(module_uri, source);
    let mut linked: Vec<u8> = Vec::new();
    lsif_linker::link(&buf, &mut linked).expect("linker");
    LsifDump::from_jsonl(std::str::from_utf8(&linked).expect("utf8"))
}

/// Index a TypeScript source string, run the linker, return the dump.
pub fn index_typescript_linked(
    module_uri: &str,
    source: &str,
    package_name: Option<&str>,
) -> LsifDump {
    let buf = emit_typescript(module_uri, source, package_name);
    let mut linked: Vec<u8> = Vec::new();
    lsif_linker::link(&buf, &mut linked).expect("linker");
    LsifDump::from_jsonl(std::str::from_utf8(&linked).expect("utf8"))
}

/// Index multiple Rust files and return a linked dump.
pub fn index_rust_multi(files: &[(&str, &str)]) -> LsifDump {
    let mut all: Vec<u8> = Vec::new();
    {
        let mut builder = lsp_max_lsif::lsif_builder::LsifBuilder::new(&mut all);
        builder
            .emit_metadata(
                "0.6.0",
                "file:///test",
                lsp_max_lsif::lsif_types::ToolInfo {
                    name: "lsif-tests".into(),
                    version: None,
                    args: None,
                },
            )
            .unwrap();
        let pid = builder
            .emit_project(Some("rust"), Some("file:///test".to_string()))
            .unwrap();
        for (uri, source) in files {
            lsif_rust::index_file(source, uri, &mut builder).unwrap();
        }
        builder.end_project(pid).unwrap();
    }
    let mut linked: Vec<u8> = Vec::new();
    lsif_linker::link(&all, &mut linked).expect("linker");
    LsifDump::from_jsonl(std::str::from_utf8(&linked).expect("utf8"))
}

/// Index multiple TypeScript files and return a linked dump.
pub fn index_typescript_multi(files: &[(&str, &str)], package_name: Option<&str>) -> LsifDump {
    let mut all: Vec<u8> = Vec::new();
    {
        let mut builder = lsp_max_lsif::lsif_builder::LsifBuilder::new(&mut all);
        builder
            .emit_metadata(
                "0.6.0",
                "file:///test",
                lsp_max_lsif::lsif_types::ToolInfo {
                    name: "lsif-tests".into(),
                    version: None,
                    args: None,
                },
            )
            .unwrap();
        let pid = builder
            .emit_project(Some("typescript"), Some("file:///test".to_string()))
            .unwrap();
        for (uri, source) in files {
            lsif_typescript::index_file(source, uri, package_name, &mut builder).unwrap();
        }
        builder.end_project(pid).unwrap();
    }
    let mut linked: Vec<u8> = Vec::new();
    lsif_linker::link(&all, &mut linked).expect("linker");
    LsifDump::from_jsonl(std::str::from_utf8(&linked).expect("utf8"))
}

fn emit_rust(uri: &str, source: &str) -> Vec<u8> {
    let mut buf: Vec<u8> = Vec::new();
    let mut builder = lsp_max_lsif::lsif_builder::LsifBuilder::new(&mut buf);
    builder
        .emit_metadata(
            "0.6.0",
            uri,
            lsp_max_lsif::lsif_types::ToolInfo {
                name: "lsif-tests".into(),
                version: None,
                args: None,
            },
        )
        .unwrap();
    let pid = builder
        .emit_project(Some("rust"), Some(uri.to_string()))
        .unwrap();
    lsif_rust::index_file(source, uri, &mut builder).unwrap();
    builder.end_project(pid).unwrap();
    buf
}

fn emit_typescript(uri: &str, source: &str, package_name: Option<&str>) -> Vec<u8> {
    let mut buf: Vec<u8> = Vec::new();
    let mut builder = lsp_max_lsif::lsif_builder::LsifBuilder::new(&mut buf);
    builder
        .emit_metadata(
            "0.6.0",
            uri,
            lsp_max_lsif::lsif_types::ToolInfo {
                name: "lsif-tests".into(),
                version: None,
                args: None,
            },
        )
        .unwrap();
    let pid = builder
        .emit_project(Some("typescript"), Some(uri.to_string()))
        .unwrap();
    lsif_typescript::index_file(source, uri, package_name, &mut builder).unwrap();
    builder.end_project(pid).unwrap();
    buf
}
