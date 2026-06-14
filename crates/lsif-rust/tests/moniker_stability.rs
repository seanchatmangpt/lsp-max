//! Cross-product #1 witness: the moniker join key is the CONTENT identity
//! `(scheme, identifier)`, NOT the LSIF numeric vertex id.
//!
//! The load-bearing property is an INDEXER property, not a function-purity one:
//! when an unrelated `pub fn` is inserted ABOVE existing symbols, the numeric
//! LSIF vertex ids shift, but the moniker `identifier` strings assigned to the
//! original symbols must NOT change. A join key derived from numeric ids would
//! silently break under such an edit; the content address holds.
//!
//! This test asserts on the moniker `identifier` strings only — never on the
//! numeric `id` field, which is expected to shift.

use serde_json::Value;

/// Index a Rust source string through the public `lsif_rust::index_file` entry
/// point and return the NDJSON the builder emits.
fn index_to_ndjson(source: &str) -> String {
    let mut buf: Vec<u8> = Vec::new();
    {
        let mut builder = lsp_max_lsif::lsif_builder::LsifBuilder::new(&mut buf);
        let project_id = builder
            .emit_meta_project("file:///mod_a", "rust")
            .expect("emit_meta_project");
        builder
            .begin_project(project_id.clone())
            .expect("begin_project");
        // The module_path (and thus moniker identifier prefix) is the filename
        // stem, so `a.rs` yields identifiers like `a::alpha`.
        lsif_rust::index_file(source, "file:///mod/a.rs", &mut builder).expect("index_file");
        builder.end_project(project_id).expect("end_project");
    }
    String::from_utf8(buf).expect("utf8 ndjson")
}

/// Collect the `identifier` strings of every emitted moniker VERTEX line.
/// Selects on `label == "moniker"` and `type == "vertex"` — deliberately
/// ignoring the numeric `id` field.
fn moniker_identifiers(ndjson: &str) -> std::collections::BTreeSet<String> {
    ndjson
        .lines()
        .filter_map(|line| serde_json::from_str::<Value>(line).ok())
        .filter(|v| v["label"] == "moniker" && v["type"] == "vertex")
        .filter_map(|v| v["identifier"].as_str().map(str::to_owned))
        .collect()
}

#[test]
fn moniker_identifiers_are_stable_under_symbol_inserted_above() {
    // Original source: two pub symbols whose export monikers we pin.
    let original = "\
pub fn alpha() {}
pub struct Beta {}
";

    // Variant: an UNRELATED pub fn inserted ABOVE the originals. This shifts the
    // LSIF numeric vertex ids assigned to alpha/Beta, but must not change their
    // moniker content identifiers.
    let variant = "\
pub fn zzz_inserted_above() {}
pub fn alpha() {}
pub struct Beta {}
";

    let original_ids = moniker_identifiers(&index_to_ndjson(original));
    let variant_ids = moniker_identifiers(&index_to_ndjson(variant));

    // Sanity: the indexer actually emitted the export monikers we mean to pin.
    assert!(
        original_ids.contains("a::alpha"),
        "expected export moniker for alpha; got {original_ids:?}"
    );
    assert!(
        original_ids.contains("a::Beta"),
        "expected export moniker for Beta; got {original_ids:?}"
    );

    // The inserted symbol adds its own moniker but must not perturb the originals.
    assert!(
        variant_ids.contains("a::zzz_inserted_above"),
        "expected inserted symbol's own moniker; got {variant_ids:?}"
    );

    // The load-bearing assertion: every original moniker identifier survives the
    // insertion unchanged. (variant is a superset by exactly the inserted symbol.)
    assert!(
        original_ids.is_subset(&variant_ids),
        "original moniker identifiers shifted under an unrelated insert:\n  before: {original_ids:?}\n  after:  {variant_ids:?}"
    );
}
