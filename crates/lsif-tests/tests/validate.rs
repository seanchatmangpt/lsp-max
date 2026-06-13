/// Structural validation tests — ported from microsoft/lsif-node/tooling/src/validate.ts
///
/// The original `ValidateCommand` enforces:
///   - metaData vertex exists
///   - every edge endpoint references an emitted vertex
///   - `contains` edges only reference emitted ranges
///   - `item` edges' shard references an emitted document vertex
///   - 1:1 cardinality on `next` edges
///
/// These tests exercise the `LsifDump::validate()` method which mirrors
/// that logic, and also verify that valid dumps pass and malformed ones fail.
use lsif_tests::{index_rust_linked, index_rust_multi, index_typescript_multi, LsifDump};

fn assert_valid(dump: &LsifDump) {
    if let Err(e) = dump.validate() {
        panic!("LSIF structural validation failed:\n{e}");
    }
}

// =============================================================================
// Valid dumps pass validation
// =============================================================================

#[test]
fn empty_rust_file_is_valid() {
    let dump = index_rust_linked("file:///test/empty.rs", "");
    assert_valid(&dump);
}

#[test]
fn single_function_dump_is_valid() {
    let dump = index_rust_linked("file:///test/a.rs", "pub fn hello() {}");
    assert_valid(&dump);
}

#[test]
fn multi_file_rust_dump_is_valid() {
    let dump = index_rust_multi(&[
        ("file:///test/a.rs", "pub fn greet() {}"),
        ("file:///test/b.rs", "fn main() { greet(); }"),
    ]);
    assert_valid(&dump);
}

#[test]
fn typescript_single_file_is_valid() {
    let dump = index_typescript_multi(
        &[("file:///test/a.ts", "export function hello(): void {}")],
        Some("pkg"),
    );
    assert_valid(&dump);
}

#[test]
fn typescript_cross_file_is_valid() {
    let dump = index_typescript_multi(
        &[
            (
                "file:///test/a.ts",
                "export class Greeter { hello(): void {} }",
            ),
            (
                "file:///test/b.ts",
                "import { Greeter } from './a';\nnew Greeter().hello();",
            ),
        ],
        Some("pkg"),
    );
    assert_valid(&dump);
}

// =============================================================================
// Malformed JSONL → validation should catch violations
// =============================================================================

#[test]
fn missing_metadata_fails_validation() {
    // A dump without the metaData vertex
    let jsonl = r#"{"id":2,"type":"vertex","label":"project","kind":"rust"}
{"id":3,"type":"vertex","label":"$event","kind":"begin","scope":"project","data":2}
{"id":4,"type":"vertex","label":"$event","kind":"end","scope":"project","data":2}"#;

    let dump = LsifDump::from_jsonl(jsonl);
    let result = dump.validate();
    assert!(
        result.is_err(),
        "dump without metaData should fail validation"
    );
    assert!(
        result.unwrap_err().contains("metaData"),
        "error should mention metaData"
    );
}

#[test]
fn edge_to_nonexistent_vertex_fails_validation() {
    // A `next` edge whose inV references an id that was never emitted
    let jsonl = r#"{"id":1,"type":"vertex","label":"metaData","version":"0.6.0","projectRoot":"file:///","positionEncoding":"utf-16"}
{"id":2,"type":"vertex","label":"range","start":{"line":0,"character":0},"end":{"line":0,"character":3}}
{"id":99,"type":"edge","label":"next","outV":2,"inV":9999}"#;

    let dump = LsifDump::from_jsonl(jsonl);
    let result = dump.validate();
    assert!(result.is_err(), "edge to missing vertex should fail");
    let err = result.unwrap_err();
    assert!(
        err.contains("9999"),
        "error should reference the missing vertex id, got: {err}"
    );
}

#[test]
fn item_edge_with_nonexistent_shard_fails_validation() {
    let jsonl = r#"{"id":1,"type":"vertex","label":"metaData","version":"0.6.0","projectRoot":"file:///","positionEncoding":"utf-16"}
{"id":2,"type":"vertex","label":"referenceResult"}
{"id":3,"type":"vertex","label":"range","start":{"line":0,"character":0},"end":{"line":0,"character":3}}
{"id":10,"type":"edge","label":"item","outV":2,"inVs":[3],"shard":9999}"#;

    let dump = LsifDump::from_jsonl(jsonl);
    let result = dump.validate();
    assert!(result.is_err(), "item edge with missing shard should fail");
    let err = result.unwrap_err();
    assert!(
        err.contains("9999"),
        "error should reference the missing shard id, got: {err}"
    );
}

// =============================================================================
// Graph invariants checked across real indexer output
// =============================================================================

/// Every `next` edge must connect a range → resultSet.
/// Mirrors the edge descriptor table in lsif-protocol (next: range → resultSet).
#[test]
fn next_edges_connect_range_to_result_set() {
    let dump = index_rust_linked("file:///test/a.rs", "pub fn alpha() {}\npub fn beta() {}");
    assert_valid(&dump);

    for (&_id, elem) in &dump.elements {
        if elem.get("label").and_then(|x| x.as_str()) == Some("next")
            && elem.get("type").and_then(|x| x.as_str()) == Some("edge")
        {
            let out_v = elem.get("outV").and_then(|x| x.as_u64()).unwrap();
            let in_v = elem.get("inV").and_then(|x| x.as_u64()).unwrap();

            let out_label = dump
                .elements
                .get(&out_v)
                .and_then(|v| v.get("label"))
                .and_then(|x| x.as_str())
                .unwrap_or("?");
            let in_label = dump
                .elements
                .get(&in_v)
                .and_then(|v| v.get("label"))
                .and_then(|x| x.as_str())
                .unwrap_or("?");

            assert_eq!(out_label, "range", "next edge outV should be a range");
            assert_eq!(in_label, "resultSet", "next edge inV should be a resultSet");
        }
    }
}

/// Every `moniker` edge must connect a resultSet → moniker vertex.
#[test]
fn moniker_edges_connect_result_set_to_moniker() {
    let dump = index_rust_linked("file:///test/a.rs", "pub fn foo() {}\npub struct Bar {}");
    assert_valid(&dump);

    for (&_id, elem) in &dump.elements {
        if elem.get("label").and_then(|x| x.as_str()) == Some("moniker")
            && elem.get("type").and_then(|x| x.as_str()) == Some("edge")
        {
            let out_v = elem.get("outV").and_then(|x| x.as_u64()).unwrap();
            let in_v = elem.get("inV").and_then(|x| x.as_u64()).unwrap();

            let out_label = dump
                .elements
                .get(&out_v)
                .and_then(|v| v.get("label"))
                .and_then(|x| x.as_str())
                .unwrap_or("?");
            let in_label = dump
                .elements
                .get(&in_v)
                .and_then(|v| v.get("label"))
                .and_then(|x| x.as_str())
                .unwrap_or("?");

            assert_eq!(
                out_label, "resultSet",
                "moniker edge outV should be a resultSet"
            );
            assert_eq!(
                in_label, "moniker",
                "moniker edge inV should be a moniker vertex"
            );
        }
    }
}

/// The `contains` edge from a document must reference range vertices.
#[test]
fn contains_edges_reference_range_vertices() {
    let dump = index_rust_linked(
        "file:///test/a.rs",
        "pub fn one() {}\npub fn two() {}\npub fn three() {}",
    );
    assert_valid(&dump);

    for (&_id, elem) in &dump.elements {
        if elem.get("label").and_then(|x| x.as_str()) == Some("contains")
            && elem.get("type").and_then(|x| x.as_str()) == Some("edge")
        {
            if let Some(in_vs) = elem.get("inVs").and_then(|x| x.as_array()) {
                for iv in in_vs {
                    if let Some(range_id) = iv.as_u64() {
                        let label = dump
                            .elements
                            .get(&range_id)
                            .and_then(|v| v.get("label"))
                            .and_then(|x| x.as_str())
                            .unwrap_or("?");
                        assert_eq!(
                            label, "range",
                            "contains inVs[{range_id}] should be a range vertex, got '{label}'"
                        );
                    }
                }
            }
        }
    }
}

/// No ids are duplicated in a single dump.
#[test]
fn no_duplicate_ids_in_dump() {
    let raw = {
        let mut buf: Vec<u8> = Vec::new();
        let mut builder = lsp_max_lsif::lsif_builder::LsifBuilder::new(&mut buf);
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
        lsif_rust::index_file(
            "pub fn a() {}\npub fn b() {}\npub fn c() {}",
            "file:///test/a.rs",
            &mut builder,
        )
        .unwrap();
        builder.end_project(pid).unwrap();
        buf
    };

    let mut seen: std::collections::HashSet<u64> = std::collections::HashSet::new();
    let mut dupes: Vec<u64> = Vec::new();
    for line in std::str::from_utf8(&raw).unwrap().lines() {
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(line) {
            if let Some(id) = v.get("id").and_then(|x| x.as_u64()) {
                if !seen.insert(id) {
                    dupes.push(id);
                }
            }
        }
    }
    assert!(dupes.is_empty(), "duplicate ids found: {dupes:?}");
}
