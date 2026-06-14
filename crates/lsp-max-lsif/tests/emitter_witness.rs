//! BUILDER-DRIVEN emitter witness tests for the extended LSIF 0.6 emit surface.
//!
//! These are intentionally NOT serde round-trips. A serde test exercises the
//! `Element` enum, not the emitter. Each test here constructs a real
//! `LsifBuilder` over a byte buffer, CALLS the emit method under test, then
//! parses the produced NDJSON and asserts on the actual emitted lines:
//!   (a) the expected `label` is present, and
//!   (b) for edges, `outV`/`inV` are wired to the exact ids passed in.
//!
//! Property (b) is the load-bearing one: it is what a mutation that swaps
//! `out_v`/`in_v` inside an emitter must break. Status of this surface is
//! CANDIDATE pending the mutation run.

use lsp_max_lsif::lsif::*;
use lsp_max_lsif::lsif_builder::LsifBuilder;
use lsp_types_max::{DocumentLink, FoldingRange, Position, Range, Uri};
use serde_json::Value;
use std::str::FromStr;

/// Parse the buffer into one JSON `Value` per NDJSON line.
fn lines(buf: &[u8]) -> Vec<Value> {
    String::from_utf8(buf.to_vec())
        .unwrap()
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(|l| serde_json::from_str::<Value>(l).unwrap())
        .collect()
}

/// First emitted line carrying the given `label`.
fn by_label<'a>(lines: &'a [Value], label: &str) -> &'a Value {
    lines
        .iter()
        .find(|v| v["label"] == label)
        .unwrap_or_else(|| panic!("no emitted element with label {label:?}"))
}

/// Emit the mandatory metaData first-element so subsequent emits are admitted
/// by the builder's metadata guard.
fn seed_metadata<W: std::io::Write>(b: &mut LsifBuilder<W>) {
    b.emit_metadata(
        "0.6.0",
        "file:///w",
        ToolInfo {
            name: "lsp-max-lsif".to_string(),
            version: None,
            args: None,
        },
    )
    .unwrap();
}

fn id_num(id: &Id) -> i64 {
    match id {
        Id::Number(n) => *n as i64,
        Id::String(s) => panic!("expected numeric id, got string {s:?}"),
    }
}

// ---- Group A: result-only pairs ----

#[test]
fn witness_bind_implementation_edge_wiring() {
    let mut buf = Vec::new();
    let mut b = LsifBuilder::new(&mut buf);
    seed_metadata(&mut b);
    let result_set = b.emit_result_set().unwrap();
    let doc = Id::Number(4242);
    let r1 = Id::Number(7);
    let result_id = b
        .bind_implementation(result_set.clone(), vec![r1.clone()], doc.clone())
        .unwrap();
    let ls = lines(&buf);

    let v = by_label(&ls, "implementationResult");
    assert_eq!(id_num(&result_id), v["id"].as_i64().unwrap());

    let e = by_label(&ls, "textDocument/implementation");
    assert_eq!(e["outV"].as_i64().unwrap(), id_num(&result_set));
    assert_eq!(e["inV"].as_i64().unwrap(), id_num(&result_id));

    let item = by_label(&ls, "item");
    assert_eq!(item["outV"].as_i64().unwrap(), id_num(&result_id));
    assert_eq!(item["inVs"][0].as_i64().unwrap(), id_num(&r1));
    assert_eq!(item["document"].as_i64().unwrap(), id_num(&doc));
}

#[test]
fn witness_bind_type_definition_edge_wiring() {
    let mut buf = Vec::new();
    let mut b = LsifBuilder::new(&mut buf);
    seed_metadata(&mut b);
    let result_set = b.emit_result_set().unwrap();
    let doc = Id::Number(909);
    let r1 = Id::Number(11);
    let result_id = b
        .bind_type_definition(result_set.clone(), vec![r1.clone()], doc.clone())
        .unwrap();
    let ls = lines(&buf);

    let v = by_label(&ls, "typeDefinitionResult");
    assert_eq!(id_num(&result_id), v["id"].as_i64().unwrap());

    let e = by_label(&ls, "textDocument/typeDefinition");
    assert_eq!(e["outV"].as_i64().unwrap(), id_num(&result_set));
    assert_eq!(e["inV"].as_i64().unwrap(), id_num(&result_id));

    let item = by_label(&ls, "item");
    assert_eq!(item["outV"].as_i64().unwrap(), id_num(&result_id));
    assert_eq!(item["inVs"][0].as_i64().unwrap(), id_num(&r1));
    assert_eq!(item["document"].as_i64().unwrap(), id_num(&doc));
}

#[test]
fn witness_bind_call_hierarchy_edge_wiring() {
    let mut buf = Vec::new();
    let mut b = LsifBuilder::new(&mut buf);
    seed_metadata(&mut b);
    let result_set = b.emit_result_set().unwrap();
    let result_id = b.bind_call_hierarchy(result_set.clone()).unwrap();
    let ls = lines(&buf);

    let v = by_label(&ls, "callHierarchyResult");
    assert_eq!(id_num(&result_id), v["id"].as_i64().unwrap());

    let e = by_label(&ls, "textDocument/callHierarchy");
    assert_eq!(e["outV"].as_i64().unwrap(), id_num(&result_set));
    assert_eq!(e["inV"].as_i64().unwrap(), id_num(&result_id));
}

#[test]
fn witness_bind_type_hierarchy_edge_wiring() {
    let mut buf = Vec::new();
    let mut b = LsifBuilder::new(&mut buf);
    seed_metadata(&mut b);
    let result_set = b.emit_result_set().unwrap();
    let result_id = b.bind_type_hierarchy(result_set.clone()).unwrap();
    let ls = lines(&buf);

    let v = by_label(&ls, "typeHierarchyResult");
    assert_eq!(id_num(&result_id), v["id"].as_i64().unwrap());

    let e = by_label(&ls, "textDocument/typeHierarchy");
    assert_eq!(e["outV"].as_i64().unwrap(), id_num(&result_set));
    assert_eq!(e["inV"].as_i64().unwrap(), id_num(&result_id));
}

// ---- Group B: data-bearing result + edge pairs ----

#[test]
fn witness_folding_range_result_vertex() {
    let mut buf = Vec::new();
    let mut b = LsifBuilder::new(&mut buf);
    seed_metadata(&mut b);
    let result_id = b
        .folding_range_result(vec![FoldingRange {
            start_line: 1,
            start_character: None,
            end_line: 4,
            end_character: None,
            kind: None,
            collapsed_text: None,
        }])
        .unwrap();
    let ls = lines(&buf);
    let v = by_label(&ls, "foldingRangeResult");
    assert_eq!(id_num(&result_id), v["id"].as_i64().unwrap());
    assert_eq!(v["result"][0]["startLine"].as_i64().unwrap(), 1);
}

#[test]
fn witness_folding_range_edge_wiring() {
    let mut buf = Vec::new();
    let mut b = LsifBuilder::new(&mut buf);
    seed_metadata(&mut b);
    let out_v = Id::Number(31);
    let in_v = Id::Number(32);
    b.folding_range_edge(out_v.clone(), in_v.clone()).unwrap();
    let ls = lines(&buf);
    let e = by_label(&ls, "textDocument/foldingRange");
    assert_eq!(e["outV"].as_i64().unwrap(), id_num(&out_v));
    assert_eq!(e["inV"].as_i64().unwrap(), id_num(&in_v));
}

#[test]
fn witness_document_link_result_vertex() {
    let mut buf = Vec::new();
    let mut b = LsifBuilder::new(&mut buf);
    seed_metadata(&mut b);
    let result_id = b
        .document_link_result(vec![DocumentLink {
            range: Range {
                start: Position::new(0, 0),
                end: Position::new(0, 5),
            },
            target: Some(Uri::from_str("file:///x").unwrap()),
            tooltip: None,
            data: None,
        }])
        .unwrap();
    let ls = lines(&buf);
    let v = by_label(&ls, "documentLinkResult");
    assert_eq!(id_num(&result_id), v["id"].as_i64().unwrap());
}

#[test]
fn witness_document_link_edge_wiring() {
    let mut buf = Vec::new();
    let mut b = LsifBuilder::new(&mut buf);
    seed_metadata(&mut b);
    let out_v = Id::Number(41);
    let in_v = Id::Number(42);
    b.document_link_edge(out_v.clone(), in_v.clone()).unwrap();
    let ls = lines(&buf);
    let e = by_label(&ls, "textDocument/documentLink");
    assert_eq!(e["outV"].as_i64().unwrap(), id_num(&out_v));
    assert_eq!(e["inV"].as_i64().unwrap(), id_num(&in_v));
}

#[test]
fn witness_document_symbol_result_vertex() {
    let mut buf = Vec::new();
    let mut b = LsifBuilder::new(&mut buf);
    seed_metadata(&mut b);
    let result_id = b
        .document_symbol_result(DocumentSymbolResultData::RangeBased(vec![
            RangeBasedDocumentSymbol {
                id: Id::Number(99),
                children: None,
            },
        ]))
        .unwrap();
    let ls = lines(&buf);
    let v = by_label(&ls, "documentSymbolResult");
    assert_eq!(id_num(&result_id), v["id"].as_i64().unwrap());
    assert_eq!(v["result"][0]["id"].as_i64().unwrap(), 99);
}

#[test]
fn witness_document_symbol_edge_wiring() {
    let mut buf = Vec::new();
    let mut b = LsifBuilder::new(&mut buf);
    seed_metadata(&mut b);
    let out_v = Id::Number(51);
    let in_v = Id::Number(52);
    b.document_symbol_edge(out_v.clone(), in_v.clone()).unwrap();
    let ls = lines(&buf);
    let e = by_label(&ls, "textDocument/documentSymbol");
    assert_eq!(e["outV"].as_i64().unwrap(), id_num(&out_v));
    assert_eq!(e["inV"].as_i64().unwrap(), id_num(&in_v));
}

#[test]
fn witness_semantic_tokens_result_vertex() {
    let mut buf = Vec::new();
    let mut b = LsifBuilder::new(&mut buf);
    seed_metadata(&mut b);
    let result_id = b
        .semantic_tokens_result(SemanticTokensData {
            data: vec![0, 1, 2, 3, 4],
        })
        .unwrap();
    let ls = lines(&buf);
    let v = by_label(&ls, "semanticTokensResult");
    assert_eq!(id_num(&result_id), v["id"].as_i64().unwrap());
    assert_eq!(v["result"]["data"][4].as_i64().unwrap(), 4);
}

#[test]
fn witness_semantic_tokens_edge_wiring() {
    let mut buf = Vec::new();
    let mut b = LsifBuilder::new(&mut buf);
    seed_metadata(&mut b);
    let out_v = Id::Number(61);
    let in_v = Id::Number(62);
    b.semantic_tokens_edge(out_v.clone(), in_v.clone()).unwrap();
    let ls = lines(&buf);
    let e = by_label(&ls, "textDocument/semanticTokens/full");
    assert_eq!(e["outV"].as_i64().unwrap(), id_num(&out_v));
    assert_eq!(e["inV"].as_i64().unwrap(), id_num(&in_v));
}

// ---- Group C: direct edges + metadata vertices ----

#[test]
fn witness_attach_edge_wiring() {
    let mut buf = Vec::new();
    let mut b = LsifBuilder::new(&mut buf);
    seed_metadata(&mut b);
    let out_v = Id::Number(71);
    let in_v = Id::Number(72);
    b.attach(out_v.clone(), in_v.clone()).unwrap();
    let ls = lines(&buf);
    let e = by_label(&ls, "attach");
    assert_eq!(e["outV"].as_i64().unwrap(), id_num(&out_v));
    assert_eq!(e["inV"].as_i64().unwrap(), id_num(&in_v));
}

#[test]
fn witness_package_information_edge_wiring() {
    let mut buf = Vec::new();
    let mut b = LsifBuilder::new(&mut buf);
    seed_metadata(&mut b);
    let out_v = Id::Number(81);
    let in_v = Id::Number(82);
    b.package_information_edge(out_v.clone(), in_v.clone())
        .unwrap();
    let ls = lines(&buf);
    let e = by_label(&ls, "packageInformation");
    assert_eq!(e["outV"].as_i64().unwrap(), id_num(&out_v));
    assert_eq!(e["inV"].as_i64().unwrap(), id_num(&in_v));
}

#[test]
fn witness_emit_source_vertex() {
    let mut buf = Vec::new();
    let mut b = LsifBuilder::new(&mut buf);
    seed_metadata(&mut b);
    let source_id = b
        .emit_source(
            "file:///workspace",
            Some(Repository {
                type_: "git".to_string(),
                url: "https://example/r".to_string(),
            }),
        )
        .unwrap();
    let ls = lines(&buf);
    let v = by_label(&ls, "source");
    assert_eq!(id_num(&source_id), v["id"].as_i64().unwrap());
    assert_eq!(v["workspaceRoot"].as_str().unwrap(), "file:///workspace");
}

#[test]
fn witness_emit_result_range_vertex() {
    let mut buf = Vec::new();
    let mut b = LsifBuilder::new(&mut buf);
    seed_metadata(&mut b);
    let range_id = b
        .emit_result_range(Position::new(2, 3), Position::new(2, 9))
        .unwrap();
    let ls = lines(&buf);
    let v = by_label(&ls, "resultRange");
    assert_eq!(id_num(&range_id), v["id"].as_i64().unwrap());
    assert_eq!(v["start"]["line"].as_i64().unwrap(), 2);
    assert_eq!(v["end"]["character"].as_i64().unwrap(), 9);
}

#[test]
fn witness_emit_package_information_vertex() {
    let mut buf = Vec::new();
    let mut b = LsifBuilder::new(&mut buf);
    seed_metadata(&mut b);
    let pkg_id = b
        .emit_package_information("lsp-max-lsif", "cargo", "26.6.9", None)
        .unwrap();
    let ls = lines(&buf);
    let v = by_label(&ls, "packageInformation");
    assert_eq!(id_num(&pkg_id), v["id"].as_i64().unwrap());
    assert_eq!(v["name"].as_str().unwrap(), "lsp-max-lsif");
    assert_eq!(v["manager"].as_str().unwrap(), "cargo");
    assert_eq!(v["version"].as_str().unwrap(), "26.6.9");
}
