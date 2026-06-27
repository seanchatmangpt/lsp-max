use std::collections::HashMap;
use std::io::{BufRead, BufReader, BufWriter, Read, Write};

use serde::{Deserialize, Serialize};
use serde_json::Value;

// ── Minimal LSIF element shapes we need to read ──────────────────────────────

#[derive(Debug, Deserialize)]
struct Element {
    id: u64,
    #[serde(rename = "type")]
    kind: String,
    label: String,
    #[serde(flatten)]
    rest: Value,
}

#[derive(Debug, Deserialize)]
struct MonikerVertex {
    scheme: String,
    identifier: String,
    #[serde(rename = "kind")]
    moniker_kind: String,
}

#[derive(Debug, Serialize)]
struct AttachEdge {
    id: u64,
    #[serde(rename = "type")]
    kind: &'static str,
    label: &'static str,
    #[serde(rename = "outV")]
    out_v: u64,
    #[serde(rename = "inV")]
    in_v: u64,
}

#[derive(Debug, Serialize)]
struct ItemEdge {
    id: u64,
    #[serde(rename = "type")]
    kind: &'static str,
    label: &'static str,
    #[serde(rename = "outV")]
    out_v: u64,
    #[serde(rename = "inVs")]
    in_vs: Vec<u64>,
    shard: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    property: Option<&'static str>,
}

// ── Linker ────────────────────────────────────────────────────────────────────

pub struct Linker {
    monikers: HashMap<u64, MonikerVertex>,
    /// (scheme, identifier) → export moniker id
    exports: HashMap<(String, String), u64>,
    import_ids: Vec<u64>,
    /// (scheme, identifier) → referenceResult vertex id
    ref_results: HashMap<(String, String), u64>,
    /// (scheme, identifier) → pending (range_id, doc_id) pairs from reference sites
    pending_refs: HashMap<(String, String), Vec<(u64, u64)>>,
    next_id: u64,
}

impl Linker {
    pub fn new(max_id: u64) -> Self {
        Self {
            monikers: HashMap::new(),
            exports: HashMap::new(),
            import_ids: Vec::new(),
            ref_results: HashMap::new(),
            pending_refs: HashMap::new(),
            next_id: max_id + 1,
        }
    }

    fn alloc(&mut self) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        id
    }

    /// First pass: collect moniker vertices from the dump.
    pub fn collect<R: Read>(&mut self, reader: R) {
        let reader = BufReader::new(reader);
        for line in reader.lines().map_while(Result::ok) {
            let Ok(elem) = serde_json::from_str::<Element>(&line) else {
                continue;
            };
            match (elem.kind.as_str(), elem.label.as_str()) {
                ("vertex", "moniker") => {
                    let Ok(m) = serde_json::from_value::<MonikerVertex>(elem.rest.clone()) else {
                        continue;
                    };
                    match m.moniker_kind.as_str() {
                        "export" => {
                            self.exports
                                .insert((m.scheme.clone(), m.identifier.clone()), elem.id);
                        }
                        "import" => {
                            self.import_ids.push(elem.id);
                        }
                        _ => {}
                    }
                    self.monikers.insert(elem.id, m);
                }
                // The indexer may annotate referenceResult vertices with
                // `ogse:scheme` / `ogse:moniker` properties so the linker can
                // wire cross-file reference ranges into the existing result.
                ("vertex", "referenceResult") => {
                    if let (Some(Value::String(scheme)), Some(Value::String(ident))) =
                        (elem.rest.get("ogse:scheme"), elem.rest.get("ogse:moniker"))
                    {
                        self.ref_results
                            .insert((scheme.clone(), ident.clone()), elem.id);
                    }
                }
                _ => {}
            }
        }
    }

    /// Second pass: emit `attach` edges (import → export moniker) and any
    /// `item` edges for pending cross-file reference ranges.
    pub fn emit_links<W: Write>(&mut self, writer: &mut W) -> std::io::Result<()> {
        let mut out = BufWriter::new(writer);

        let import_ids: Vec<u64> = self.import_ids.drain(..).collect();
        for import_id in import_ids {
            let Some(m) = self.monikers.get(&import_id) else {
                continue;
            };
            let key = (m.scheme.clone(), m.identifier.clone());

            if let Some(&export_id) = self.exports.get(&key) {
                let edge = AttachEdge {
                    id: self.alloc(),
                    kind: "edge",
                    label: "attach",
                    out_v: import_id,
                    in_v: export_id,
                };
                serde_json::to_writer(&mut out, &edge)?;
                out.write_all(b"\n")?;
            }

            // Wire pending cross-file reference ranges into the referenceResult
            // if the indexer created one annotated with ogse:moniker.
            if let Some(pending) = self.pending_refs.remove(&key) {
                if let Some(&ref_result_id) = self.ref_results.get(&key) {
                    let mut by_doc: HashMap<u64, Vec<u64>> = HashMap::new();
                    for (range_id, doc_id) in pending {
                        by_doc.entry(doc_id).or_default().push(range_id);
                    }
                    for (doc_id, range_ids) in by_doc {
                        let edge = ItemEdge {
                            id: self.alloc(),
                            kind: "edge",
                            label: "item",
                            out_v: ref_result_id,
                            in_vs: range_ids,
                            shard: doc_id,
                            property: Some("references"),
                        };
                        serde_json::to_writer(&mut out, &edge)?;
                        out.write_all(b"\n")?;
                    }
                }
            }
        }

        out.flush()
    }
}

// ── Public API ────────────────────────────────────────────────────────────────

/// Read a complete LSIF dump (as bytes), append cross-file `attach`/`item`
/// edges, and write a valid extended LSIF dump to `output`.
///
/// The input is taken as `&[u8]` rather than `Read + Clone` so callers are
/// explicit that the dump must be fully buffered before linking — a `File`
/// cannot be rewound in all contexts.
pub fn link(dump: &[u8], output: &mut impl Write) -> std::io::Result<()> {
    let max_id = find_max_id(dump);
    let mut linker = Linker::new(max_id);
    linker.collect(dump);

    // Pass through original lines
    {
        let mut out = BufWriter::new(&mut *output);
        for line in BufReader::new(dump).lines().map_while(Result::ok) {
            out.write_all(line.as_bytes())?;
            out.write_all(b"\n")?;
        }
    }

    linker.emit_links(output)
}

fn find_max_id(dump: &[u8]) -> u64 {
    let reader = BufReader::new(dump);
    let mut max = 0u64;
    for line in reader.lines().map_while(Result::ok) {
        if let Ok(Value::Object(map)) = serde_json::from_str::<Value>(&line) {
            if let Some(Value::Number(n)) = map.get("id") {
                if let Some(id) = n.as_u64() {
                    max = max.max(id);
                }
            }
        }
    }
    max
}

#[cfg(test)]
mod tests {
    use super::*;

    fn moniker_line(id: u64, scheme: &str, ident: &str, kind: &str) -> String {
        format!(
            r#"{{"id":{id},"type":"vertex","label":"moniker","scheme":"{scheme}","identifier":"{ident}","kind":"{kind}","unique":"project"}}"#
        )
    }

    #[test]
    fn attach_edge_emitted_for_matching_pair() {
        let export = moniker_line(10, "rust", "my_crate::foo", "export");
        let import = moniker_line(20, "rust", "my_crate::foo", "import");
        let meta = r#"{"id":1,"type":"vertex","label":"metaData","version":"0.6.0","projectRoot":"file:///","positionEncoding":"utf-16"}"#;

        let dump = format!("{meta}\n{export}\n{import}\n");

        let mut out: Vec<u8> = Vec::new();
        link(dump.as_bytes(), &mut out).unwrap();

        let text = String::from_utf8(out).unwrap();
        assert!(text.contains(r#""label":"attach""#), "attach edge missing");
        assert!(text.contains(r#""outV":20"#), "import id wrong");
        assert!(text.contains(r#""inV":10"#), "export id wrong");
    }

    #[test]
    fn no_attach_when_no_matching_export() {
        let import = moniker_line(20, "rust", "other_crate::bar", "import");
        let meta = r#"{"id":1,"type":"vertex","label":"metaData","version":"0.6.0","projectRoot":"file:///","positionEncoding":"utf-16"}"#;

        let dump = format!("{meta}\n{import}\n");
        let mut out: Vec<u8> = Vec::new();
        link(dump.as_bytes(), &mut out).unwrap();

        let text = String::from_utf8(out).unwrap();
        assert!(
            !text.contains(r#""label":"attach""#),
            "spurious attach edge"
        );
    }
}
