//! Tests for `indexer_rust::emit::index_rust_source` — the tree-sitter-driven
//! Rust-to-LSIF emitter. This path previously had zero unit test coverage
//! (only the LSIF vertex/edge *builder* was exercised elsewhere in this
//! `tests/` directory; `index_rust_source` itself was never called).
//!
//! Follows the same builder-driven, NDJSON-parsing witness style as
//! `emitter_witness.rs`: construct a real `LsifBuilder`, call the function
//! under test, then assert on the actual emitted lines.

use lsp_max_lsif::indexer_rust::index_rust_source;
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
    index_rust_source(source, uri, &mut b).expect("index_rust_source must not error");
    lines(&buf)
}

fn range_tags(ls: &[Value]) -> Vec<&Value> {
    ls.iter()
        .filter(|v| v["label"] == "range" && v.get("tag").is_some())
        .map(|v| &v["tag"])
        .collect()
}

#[test]
fn function_definition_is_emitted_with_correct_name_and_kind() {
    let ls = index(
        "pub fn add(a: i32, b: i32) -> i32 { a + b }",
        "file:///lib.rs",
    );
    let tags = range_tags(&ls);
    let def = tags
        .iter()
        .find(|t| t["type"] == "definition" && t["text"] == "add")
        .expect("expected a definition tag for `add`");
    assert_eq!(
        def["kind"],
        serde_json::json!(12),
        "SymbolKind::FUNCTION == 12"
    );
}

#[test]
fn struct_and_enum_definitions_are_emitted() {
    let src = "pub struct Point { x: i32, y: i32 }\npub enum Color { Red, Green, Blue }";
    let ls = index(src, "file:///types.rs");
    let tags = range_tags(&ls);

    assert!(
        tags.iter()
            .any(|t| t["type"] == "definition" && t["text"] == "Point"),
        "expected a definition tag for struct `Point`"
    );
    assert!(
        tags.iter()
            .any(|t| t["type"] == "definition" && t["text"] == "Color"),
        "expected a definition tag for enum `Color`"
    );
}

#[test]
fn doc_comments_are_captured_into_hover_markdown() {
    let src = "/// Adds two numbers together.\npub fn add(a: i32, b: i32) -> i32 { a + b }";
    let ls = index(src, "file:///lib.rs");
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
    // `helper` is not defined in this file — it comes from `use other::helper;`
    // and is then called. This is the core cross-file value of the LSIF
    // pipeline: the call site must resolve to an Import moniker naming the
    // module it came from, not silently drop the reference.
    let src = "use other::helper;\nfn run() { helper(); }";
    let ls = index(src, "file:///caller.rs");

    let moniker = ls
        .iter()
        .find(|v| v["label"] == "moniker" && v["kind"] == "import")
        .expect("expected an import moniker for the unresolved call to `helper`");
    assert_eq!(moniker["identifier"], "other::helper");
}

#[test]
fn local_call_resolves_to_a_reference_not_an_import_moniker() {
    let src = "fn helper() {}\nfn run() { helper(); }";
    let ls = index(src, "file:///local.rs");

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
    // Deliberately broken Rust — unbalanced braces / garbage tokens. The
    // tree-sitter parser produces ERROR nodes rather than failing, and the
    // walk must tolerate that instead of panicking.
    let src = "pub fn broken( {{{ this is not rust @@@ }}}";
    let ls = index(src, "file:///broken.rs");
    assert!(
        ls.iter().any(|v| v["label"] == "document"),
        "even malformed input should still emit a document vertex, got: {ls:?}"
    );
}
