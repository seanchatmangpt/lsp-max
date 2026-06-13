/// Chicago TDD integration test for the lsp-max indexing pipeline.
///
/// Uses `chicago-tdd-tools` (dev-dependency only, never distributed) to
/// enforce the Arrange-Act-Assert structure and verify state-based outcomes
/// for the LSIF indexer crates.
use chicago_tdd_tools::prelude::*;

// ── Fixture ───────────────────────────────────────────────────────────────────

struct LsifFixture {
    fixture: TestFixture,
}

impl LsifFixture {
    fn new() -> Self {
        Self {
            fixture: TestFixture::new().expect("TestFixture setup failed"),
        }
    }

    fn index_rust(&self, source: &str) -> Vec<u8> {
        let mut buf: Vec<u8> = Vec::new();
        let mut builder = lsp_max_lsif::lsif_builder::LsifBuilder::new(&mut buf);
        builder
            .emit_metadata(
                "0.6.0",
                "file:///test",
                lsp_max_lsif::lsif_types::ToolInfo {
                    name: "lsp-max-lsif".into(),
                    version: None,
                    args: None,
                },
            )
            .expect("emit_metadata failed");
        let pid = builder
            .emit_project(Some("rust"), Some("file:///test".to_string()))
            .expect("emit_project failed");
        lsif_rust::index_file(source, "file:///test/a.rs", &mut builder)
            .expect("index_file failed");
        builder.end_project(pid).expect("end_project failed");
        buf
    }

    fn index_typescript(&self, source: &str) -> Vec<u8> {
        let mut buf: Vec<u8> = Vec::new();
        let mut builder = lsp_max_lsif::lsif_builder::LsifBuilder::new(&mut buf);
        builder
            .emit_metadata(
                "0.6.0",
                "file:///test",
                lsp_max_lsif::lsif_types::ToolInfo {
                    name: "lsp-max-lsif".into(),
                    version: None,
                    args: None,
                },
            )
            .expect("emit_metadata failed");
        let pid = builder
            .emit_project(Some("typescript"), Some("file:///test".to_string()))
            .expect("emit_project failed");
        lsif_typescript::index_file(source, "file:///test/a.ts", None, &mut builder)
            .expect("index_file failed");
        builder.end_project(pid).expect("end_project failed");
        buf
    }

    fn teardown(self) {
        drop(self.fixture);
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

/// A public Rust function produces at least one LSIF vertex in the output.
#[test]
fn rust_pub_fn_produces_lsif_output() {
    // Arrange
    let f = LsifFixture::new();

    // Act
    let output = f.index_rust("pub fn greet() {}");

    // Assert — output is non-empty valid UTF-8 JSONL
    let text = std::str::from_utf8(&output).expect("output must be UTF-8");
    assert!(!text.is_empty(), "LSIF output must not be empty");
    let line_count = text.lines().count();
    assert!(
        line_count >= 3,
        "expected at least metaData + project + document, got {line_count} lines"
    );

    f.teardown();
}

/// Export monikers are emitted for public items, not for private ones.
#[test]
fn rust_export_moniker_only_for_pub() {
    // Arrange
    let f = LsifFixture::new();
    let source = "pub fn visible() {}\nfn hidden() {}";

    // Act
    let output = f.index_rust(source);
    let text = std::str::from_utf8(&output).expect("output must be UTF-8");

    // Assert
    let moniker_count = text
        .lines()
        .filter(|l| l.contains("\"moniker\"") && l.contains("\"export\""))
        .count();
    assert_eq!(
        moniker_count, 1,
        "exactly one export moniker expected (for `visible`), got {moniker_count}"
    );

    f.teardown();
}

/// A TypeScript `export function` produces an export moniker with scheme "typescript".
#[test]
fn typescript_export_fn_produces_typescript_moniker() {
    // Arrange
    let f = LsifFixture::new();

    // Act
    let output = f.index_typescript("export function hello(): void {}");
    let text = std::str::from_utf8(&output).expect("output must be UTF-8");

    // Assert
    let has_ts_moniker = text
        .lines()
        .any(|l| l.contains("\"typescript\"") && l.contains("\"export\""));
    assert!(
        has_ts_moniker,
        "expected a typescript-scheme export moniker in:\n{text}"
    );

    f.teardown();
}

/// `export const` in TypeScript produces a definition range and export moniker.
#[test]
fn typescript_export_const_indexed() {
    // Arrange
    let f = LsifFixture::new();

    // Act
    let output = f.index_typescript("export const MAX_RETRIES: number = 3;");
    let text = std::str::from_utf8(&output).expect("output must be UTF-8");

    // Assert — definition range tagged for MAX_RETRIES
    let has_def = text
        .lines()
        .any(|l| l.contains("MAX_RETRIES") && l.contains("definition"));
    assert!(
        has_def,
        "expected definition range for MAX_RETRIES in:\n{text}"
    );

    f.teardown();
}

/// No duplicate IDs are emitted in any indexer output.
#[test]
fn no_duplicate_ids_in_rust_output() {
    // Arrange
    let f = LsifFixture::new();

    // Act
    let output = f.index_rust("pub fn a() {}\npub fn b() {}\npub struct C {}");
    let text = std::str::from_utf8(&output).expect("output must be UTF-8");

    // Assert
    let mut seen = std::collections::HashSet::new();
    let mut dupes = Vec::new();
    for line in text.lines() {
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(line) {
            if let Some(id) = v.get("id").and_then(|x| x.as_u64()) {
                if !seen.insert(id) {
                    dupes.push(id);
                }
            }
        }
    }
    assert!(dupes.is_empty(), "duplicate LSIF ids: {dupes:?}");

    f.teardown();
}
