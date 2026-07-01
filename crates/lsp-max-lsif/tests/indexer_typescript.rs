//! Tests for `indexer_typescript::emit::index_typescript_source` — the
//! tree-sitter-driven TypeScript-to-LSIF emitter. Like its Rust sibling,
//! this path previously had zero unit test coverage.
//!
//! Same builder-driven, NDJSON-parsing witness style as `emitter_witness.rs`
//! and `indexer_rust.rs`.

use lsp_max_lsif::indexer_typescript::index_typescript_source;
use lsp_max_lsif::lsif::ToolInfo;
use lsp_max_lsif::lsif_builder::LsifBuilder;
use serde_json::Value;

fn lines(buf: &[u8]) -> Vec<Value> {
    String::from_utf8(buf.to_vec())
        .unwrap()
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(|l| serde_json::from_str::<Value>(l).unwrap())
        .collect()
}

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

fn index(source: &str, uri: &str) -> Vec<Value> {
    let mut buf = Vec::new();
    let mut b = LsifBuilder::new(&mut buf);
    seed_metadata(&mut b);
    index_typescript_source(source, uri, None, &mut b)
        .expect("index_typescript_source must not error");
    lines(&buf)
}

fn range_tags(ls: &[Value]) -> Vec<&Value> {
    ls.iter()
        .filter(|v| v["label"] == "range" && v.get("tag").is_some())
        .map(|v| &v["tag"])
        .collect()
}

#[test]
fn exported_function_definition_is_emitted_with_correct_name_and_kind() {
    let ls = index("export function add(a: number, b: number): number { return a + b; }", "file:///lib.ts");
    let tags = range_tags(&ls);
    let def = tags
        .iter()
        .find(|t| t["type"] == "definition" && t["text"] == "add")
        .expect("expected a definition tag for `add`");
    assert_eq!(def["kind"], serde_json::json!(12), "SymbolKind::FUNCTION == 12");
}

#[test]
fn class_and_interface_definitions_are_emitted() {
    let src = "export class Point {}\nexport interface Shape {}";
    let ls = index(src, "file:///types.ts");
    let tags = range_tags(&ls);

    assert!(
        tags.iter()
            .any(|t| t["type"] == "definition" && t["text"] == "Point"),
        "expected a definition tag for class `Point`"
    );
    assert!(
        tags.iter()
            .any(|t| t["type"] == "definition" && t["text"] == "Shape"),
        "expected a definition tag for interface `Shape`"
    );
}

#[test]
fn doc_comments_are_captured_into_hover_markdown() {
    let src = "/** Adds two numbers together. */\nexport function add(a: number, b: number): number { return a + b; }";
    let ls = index(src, "file:///lib.ts");
    let hover = ls
        .iter()
        .find(|v| v["label"] == "hoverResult")
        .expect("expected a hoverResult vertex");
    let markdown = hover["result"]["contents"]["value"]
        .as_str()
        .expect("hover contents must be a markdown string");
    assert!(
        markdown.contains("Adds two numbers together."),
        "hover markdown must include the doc comment; got: {markdown}"
    );
}

#[test]
fn imported_call_resolves_to_a_cross_file_moniker() {
    let src = "import { helper } from './other';\nfunction run() { helper(); }";
    let ls = index(src, "file:///caller.ts");

    let moniker = ls
        .iter()
        .find(|v| v["label"] == "moniker" && v["kind"] == "import")
        .expect("expected an import moniker for the unresolved call to `helper`");
    assert_eq!(moniker["identifier"], "other::helper");
}

#[test]
fn local_call_resolves_to_a_reference_not_an_import_moniker() {
    let src = "function helper() {}\nfunction run() { helper(); }";
    let ls = index(src, "file:///local.ts");

    let has_import_moniker = ls
        .iter()
        .any(|v| v["label"] == "moniker" && v["kind"] == "import");
    assert!(
        !has_import_moniker,
        "a call to a function defined in the same file must not produce an import moniker"
    );

    let has_reference_edge = ls.iter().any(|v| v["label"] == "textDocument/references");
    assert!(
        has_reference_edge,
        "expected a references edge binding the call site to the local definition"
    );
}

#[test]
fn malformed_source_does_not_panic_and_still_emits_a_document() {
    let src = "export function broken( {{{ this is not typescript @@@ }}}";
    let ls = index(src, "file:///broken.ts");
    assert!(
        ls.iter().any(|v| v["label"] == "document"),
        "even malformed input should still emit a document vertex, got: {ls:?}"
    );
}
