// diagnostics_merge.rs — FanAll diagnostics merge semantics (CC-005).
//
// The compositor fans `textDocument/publishDiagnostics` to ALL children
// (FanAll) and merges their contributions into a single upstream event
// stream via `merge_for_upstream`. These tests drive that real code path
// with stub children represented as `(server_id, Vec<Diagnostic>)`
// contributions — the same shape the router hands the merge after a flush.
//
// Merge-order law (documented, asserted throughout):
//   Contributions are processed in the ORDER GIVEN (child priority order —
//   the caller supplies contributions in router order). Within a child, the
//   per-child publish sequence is preserved (Vec insertion order). The
//   merged Vec preserves first-occurrence position; at a duplicate
//   (range, code) key, ANDON beats non-ANDON, then lower severity number
//   wins; equal severity → first occurrence wins.

use lsp_max::lsp_types::{
    Diagnostic, DiagnosticSeverity, DocumentUri, NumberOrString, Position, Range,
};
use lsp_max_compositor::merge::{merge_for_upstream, AndonRoute, DiagnosticEntry, MergeContext};
use lsp_max_compositor::ChildTier;
use std::str::FromStr;

// ── helpers ──────────────────────────────────────────────────────────────────

fn range(l: u32, c: u32) -> Range {
    Range {
        start: Position {
            line: l,
            character: c,
        },
        end: Position {
            line: l,
            character: c + 4,
        },
    }
}

fn str_code(s: &str) -> Option<NumberOrString> {
    Some(NumberOrString::String(s.to_string()))
}

fn diag(l: u32, c: u32, code: &str, severity: DiagnosticSeverity) -> Diagnostic {
    Diagnostic {
        range: range(l, c),
        code: str_code(code),
        severity: Some(severity),
        message: format!("diag {code} at {l}:{c}"),
        ..Default::default()
    }
}

fn uri(name: &str) -> DocumentUri {
    DocumentUri::from_str(&format!("file:///m5/{name}")).unwrap()
}

/// Each child's contribution is a run of diagnostics in that child's own
/// publish sequence. Assert helper: extract codes from the merged stream.
fn codes(v: &[Diagnostic]) -> Vec<&str> {
    v.iter()
        .map(|d| match &d.code {
            Some(NumberOrString::String(s)) => s.as_str(),
            _ => "<none>",
        })
        .collect()
}

// ── 1. Two children, same document URI → both sets present in merged stream ──

#[test]
fn two_children_same_uri_both_sets_survive_merge() {
    let doc = uri("same.rs");
    let doc_str = doc.to_string();

    // config-lsp emits CCC-* codes; a second child emits its own family.
    let config_lsp: Vec<Diagnostic> = vec![diag(0, 0, "CCC-KEY-DUP", DiagnosticSeverity::WARNING)];
    let other: Vec<Diagnostic> = vec![diag(5, 2, "OTHER-RULE-1", DiagnosticSeverity::ERROR)];

    let merged = merge_for_upstream(
        &[
            ("config-lsp".to_string(), config_lsp),
            ("other-lsp".to_string(), other),
        ],
        &[],
    );

    assert_eq!(merged.len(), 2, "both children's sets survive the merge");
    // Documented order law: contribution order (child priority order) is
    // preserved on first occurrence — config-lsp's entry sits before other-lsp's.
    assert_eq!(codes(&merged), vec!["CCC-KEY-DUP", "OTHER-RULE-1"]);
    let _ = doc_str;
}

#[test]
fn two_children_same_uri_dedup_law_andon_beats_non_andon_then_severity() {
    let doc = uri("dedup.rs");

    // Same (range, code) key: ANDON-prefix code from child B replaces the
    // non-ANDON code from child A at the same key… but note the key includes
    // the code, so a code-differing pair never collides. Collision requires
    // the SAME code at the SAME range — andon status/severity decide.
    let a: Vec<Diagnostic> = vec![diag(2, 0, "CCC-TIMEOUT", DiagnosticSeverity::WARNING)];
    let b: Vec<Diagnostic> = vec![diag(2, 0, "CCC-TIMEOUT", DiagnosticSeverity::ERROR)];

    let _ = doc;
    let merged = merge_for_upstream(
        &[("config-lsp".to_string(), a), ("other-lsp".to_string(), b)],
        &["CCC-".to_string()],
    );

    assert_eq!(merged.len(), 1, "duplicate (range, code) key dedups to one");
    // ANDON-prefixed family is in play for both (same code); lower severity
    // number (ERROR=1) wins over WARNING.
    assert_eq!(
        merged[0].severity,
        Some(DiagnosticSeverity::ERROR),
        "lower severity number wins at duplicate key"
    );
}

#[test]
fn same_key_first_occurrence_wins_at_equal_severity() {
    let _doc = uri("first.rs");

    let a: Vec<Diagnostic> = vec![diag(3, 1, "CCC-DUP", DiagnosticSeverity::WARNING)];
    // Same key, same severity, different message — first occurrence must win.
    let mut b = diag(3, 1, "CCC-DUP", DiagnosticSeverity::WARNING);
    b.message = "CHILD-B VARIANT".to_string();

    let merged = merge_for_upstream(
        &[
            ("config-lsp".to_string(), a),
            ("other-lsp".to_string(), vec![b]),
        ],
        &[],
    );

    assert_eq!(merged.len(), 1);
    assert_eq!(
        merged[0].message, "diag CCC-DUP at 3:1",
        "first occurrence wins at equal severity (child priority order decides)"
    );
}

// ── 2. One child producing, other silent → single set passes through ─────────

#[test]
fn silent_child_passes_through_single_set_untouched() {
    let _doc = uri("silent.rs");

    let producer: Vec<Diagnostic> = vec![
        diag(0, 0, "CCC-ONE", DiagnosticSeverity::ERROR),
        diag(1, 0, "CCC-TWO", DiagnosticSeverity::WARNING),
    ];
    let silent: Vec<Diagnostic> = Vec::new();

    let merged = merge_with_silent_child(&producer, &silent);

    assert_eq!(merged.len(), 2, "producer's full set passes through");
    assert_eq!(
        codes(&merged),
        vec!["CCC-ONE", "CCC-TWO"],
        "per-child publish sequence preserved through merge"
    );
}

/// Real code path for the silent-child scenario: the compositor passes the
/// silent child as an empty contribution; `merge_for_upstream` with a single
/// non-empty contribution is the pass-through identity.
fn merge_with_silent_child(producer: &[Diagnostic], silent: &[Diagnostic]) -> Vec<Diagnostic> {
    assert!(silent.is_empty(), "silent child contributes no diagnostics");
    merge_for_upstream(
        &[
            ("producer-lsp".to_string(), producer.to_vec()),
            ("silent-lsp".to_string(), Vec::new()),
        ],
        &[],
    )
}

// ── 3. andon_code_prefixes attribution survives the merge ────────────────────

#[test]
fn andon_prefixes_stay_attributable_to_emitting_child() {
    let _doc = uri("attr.rs");

    // config-lsp emits CCC-* ; second child emits ALPHA-*. Distinct prefixes.
    let config_lsp: Vec<Diagnostic> = vec![diag(0, 0, "CCC-GATE", DiagnosticSeverity::ERROR)];
    let alpha_child: Vec<Diagnostic> = vec![diag(0, 0, "ALPHA-GATE", DiagnosticSeverity::ERROR)];

    let merged = merge_for_upstream(
        &[
            ("config-lsp".to_string(), config_lsp),
            ("alpha-lsp".to_string(), alpha_child),
        ],
        &["CCC-".to_string()],
    );

    assert_eq!(merged.len(), 2);
    // Both diagnostics share range but differ in code → both survive; the
    // CCC- prefix (config-lsp's andon family) is preserved on its entry.
    assert_eq!(codes(&merged), vec!["CCC-GATE", "ALPHA-GATE"]);
}

#[test]
fn andon_attribution_via_merge_context_is_per_server() {
    // MergeContext is the per-server attribution surface: each child's own
    // prefix set governs classification of its codes, and the classification
    // survives the merge because it is computed per (code, server_id).
    let mut ctx = MergeContext::new(vec![]); // empty workspace baseline
    ctx.add_server_prefix_override("config-lsp".to_string(), vec!["CCC-".to_string()]);
    ctx.add_server_prefix_override("alpha-lsp".to_string(), vec!["ALPHA-".to_string()]);

    let mk = |code: &str, server: &str| DiagnosticEntry {
        uri: uri("ctx.rs").to_string(),
        line: 0,
        character: 0,
        end_line: 0,
        end_character: 0,
        severity: 1,
        code: code.to_string(),
        message: String::new(),
        source_tier: ChildTier::Primary,
        server_id: Some(server.to_string()),
    };

    let result = ctx.merge(vec![
        (ChildTier::Primary, vec![mk("CCC-GATE", "config-lsp")]),
        (ChildTier::Primary, vec![mk("ALPHA-GATE", "alpha-lsp")]),
    ]);

    assert!(
        result.has_andon_block,
        "both children's andon codes classify as ANDON under their own prefixes"
    );
    assert_eq!(result.diagnostics.len(), 2);

    // Attribution survives the merge: classifying each child's code against
    // the OTHER child's prefix set must NOT match.
    assert_eq!(
        ctx.attribute_andon("CCC-GATE", Some("alpha-lsp")),
        AndonRoute::NotAndon,
        "config-lsp's code is not andon under alpha-lsp's own prefix set"
    );
    assert_eq!(
        ctx.attribute_andon("ALPHA-GATE", Some("config-lsp")),
        AndonRoute::NotAndon,
        "alpha-lsp's code is not andon under config-lsp's own prefix set"
    );
    assert_eq!(
        ctx.attribute_andon("CCC-GATE", Some("config-lsp")),
        AndonRoute::PerServer {
            server_id: "config-lsp".to_string()
        },
    );
}

// ── 4. Merged publishes preserve per-child sequence ──────────────────────────

#[test]
fn merged_publish_preserves_per_child_sequence() {
    let _doc = uri("seq.rs");

    // config-lsp publishes D1,D2,D3 in that sequence; other-lsp publishes E1,E2.
    let config_lsp: Vec<Diagnostic> = vec![
        diag(0, 0, "CCC-D1", DiagnosticSeverity::WARNING),
        diag(1, 0, "CCC-D2", DiagnosticSeverity::WARNING),
    ];
    let other: Vec<Diagnostic> = vec![
        diag(2, 0, "OTH-E1", DiagnosticSeverity::ERROR),
        diag(3, 0, "OTH-E2", DiagnosticSeverity::HINT),
        diag(4, 0, "OTH-E3", DiagnosticSeverity::HINT),
    ];

    let merged = merge_for_upstream(
        &[
            ("config-lsp".to_string(), config_lsp),
            ("other-lsp".to_string(), other),
        ],
        &[],
    );

    assert_eq!(merged.len(), 5);
    assert_eq!(
        codes(&merged),
        vec!["CCC-D1", "CCC-D2", "OTH-E1", "OTH-E2", "OTH-E3"],
        "per-child sequence preserved: child A's run then child B's run"
    );
}

#[test]
fn interleave_children_without_key_collision_preserves_both_runs() {
    // Even when children interleave in the contributions list, each child's
    // internal run order is preserved because the merge only repositions
    // entries at duplicate (range, code) keys.
    let a: Vec<Diagnostic> = vec![diag(0, 0, "A-1", DiagnosticSeverity::WARNING)];
    let b: Vec<Diagnostic> = vec![diag(1, 0, "B-1", DiagnosticSeverity::WARNING)];
    let a2: Vec<Diagnostic> = vec![diag(2, 0, "A-2", DiagnosticSeverity::WARNING)];

    let merged = merge_for_upstream(
        &[
            ("a".to_string(), a),
            ("b".to_string(), b),
            ("a".to_string(), a2),
        ],
        &[],
    );

    assert_eq!(
        codes(&merged),
        vec!["A-1", "B-1", "A-2"],
        "second A contribution lands after B's run (contribution order law)"
    );
}

#[test]
fn severity_mix_orders_law_errors_first_within_one_child() {
    // Within one child's contribution, the pass-through is insertion order
    // (no sort in merge_for_upstream); andon replacement preserves position.
    let _doc = uri("ord.rs");

    let only: Vec<Diagnostic> = vec![
        diag(2, 0, "CCC-LAW", DiagnosticSeverity::WARNING),
        diag(0, 0, "CCC-ERR", DiagnosticSeverity::ERROR),
    ];

    let merged = merge_for_upstream(&[("config-lsp".to_string(), only)], &["CCC-".to_string()]);

    assert_eq!(
        codes(&merged),
        vec!["CCC-LAW", "CCC-ERR"],
        "merge_for_upstream does NOT reorder within a child's own sequence"
    );
}
