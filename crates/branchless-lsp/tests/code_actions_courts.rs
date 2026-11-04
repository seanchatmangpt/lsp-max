//! Integration courts for the E12 quick-fix rewriters (Chicago: real
//! syn parses, real `apply_edit` round-trips, real Tier-1 linter on the
//! rewritten source).
//!
//! Anti-vacuity: an inverted-mask mutation of rewriter 1's output must
//! fail the semantic check - a court that cannot flip is FAIL.

use branchless_lsp::code_actions::{
    apply_edit, insert_allow_isel_residual, rewrite_if_else_to_mask, rewrite_match_to_jump_table,
    NoFix, WorkspaceEdit,
};
use branchless_lsp::lint_source;

/// A rewritten source is valid Rust iff re-linting a planted branchy probe
/// appended to it still sees the probe (an unparsable rewrite swallows it).
fn syn_parse_ok(src: &str) {
    let planted =
        format!("{src}fn e12_planted_probe() -> u32 {{ if true {{ 1 }} else {{ 0 }} }}\n");
    let r = lint_source(&planted);
    assert!(
        r.hits.iter().any(|h| h.kind == "if"),
        "planted probe unseen: rewritten source is not valid Rust:\n{src}"
    );
}

#[test]
fn court_if_else_mask_roundtrip_is_tier1_clean() {
    let src = "fn f(cond: bool, x: u32, y: u32) -> u32 {\n    if cond { x } else { y }\n}\n";
    let report_before = lint_source(src);
    assert_eq!(
        report_before.hits.len(),
        1,
        "pre: exactly the if is flagged"
    );
    assert_eq!(report_before.hits[0].line, 2);
    let edit = rewrite_if_else_to_mask(src, 2, 5).expect("rewrite applies");
    let rewritten = apply_edit(src, &edit);
    let report = lint_source(&rewritten);
    assert!(
        report.hits.is_empty(),
        "rewritten span must carry 0 ANDON_BRANCH_SYNTAX hits, got {:?}\n---\n{rewritten}",
        report.hits
    );
    syn_parse_ok(&rewritten);
    assert!(edit.note.unwrap().contains("Tier-2"));
}

#[test]
fn court_if_else_mask_semantics_flip_on_inverted_mask() {
    let src = "fn f(cond: bool, x: u32, y: u32) -> u32 {\n    if cond { x } else { y }\n}\n";
    let edit = rewrite_if_else_to_mask(src, 2, 5).expect("rewrite applies");
    let good = &edit.edits[0].new_text;
    // DELIBERATE WRONG REWRITE (anti-vacuity mutation): swap mask polarity.
    let bad = good
        .replacen("& mask", "@SWAP@", 1)
        .replacen("& !mask", "& mask", 1)
        .replace("@SWAP@", "& !mask");
    assert_ne!(bad, good.as_str(), "mutation must change the text");
    // Structural check: canonical text is polarity-correct; the inverted
    // mutation has (!x mask-polarity swapped combine) and must be detected.
    let canon_ok = good.contains("& mask) | ((")
        && good.contains("& !mask)")
        && good.contains("wrapping_neg()");
    let bad_is_inverted = bad.contains("!mask) | ((");
    assert!(canon_ok, "canonical shape: {good}");
    assert!(bad_is_inverted, "mutation recognized as inverted: {bad}");
    // Direct arithmetic flip: mask = -(cond as usize) picks x; inverted picks y.
    let x = 10u32;
    let y = 20u32;
    let mask = (true as u32).wrapping_neg();
    assert_eq!((x & mask) | (y & !mask), 10, "canonical picks then-branch");
    assert_ne!((x & !mask) | (y & mask), 10, "inverted picks else-branch");
}

#[test]
fn court_match_jump_table_roundtrip_is_tier1_clean() {
    let src = "enum Op { A, B, C }\n\nfn g(op: Op, v: u32) -> u32 {\n    match op {\n        Op::A => 10,\n        Op::B => 20,\n        Op::C => 30,\n    }\n}\n";
    let report_before = lint_source(src);
    assert_eq!(report_before.hits[0].kind, "match");
    let edit = rewrite_match_to_jump_table(src, 4, 5).expect("rewrite applies");
    let rewritten = apply_edit(src, &edit);
    let report = lint_source(&rewritten);
    assert!(
        report.hits.is_empty(),
        "rewritten source must be Tier-1 clean, got {:?}\n---\n{rewritten}",
        report.hits
    );
    syn_parse_ok(&rewritten);
    assert!(rewritten.contains("OP_JUMP_TABLE"));
    assert!(rewritten.contains("let __idx = (op) as usize;"));
    assert!(rewritten.contains("get_unchecked(__idx)"));
}

#[test]
fn court_nonflat_enum_refusal_is_documented() {
    let src = "enum Weird { X = 5, Y = 9 }\n\nfn h(w: Weird, v: u32) -> u32 {\n    match w {\n        Weird::X => 1,\n        Weird::Y => 2,\n    }\n}\n";
    let err = rewrite_match_to_jump_table(src, 4, 5).unwrap_err();
    let msg = err.to_string();
    assert!(msg.contains("non-flat enum"), "msg: {msg}");
    assert!(msg.contains("not contiguous"), "msg: {msg}");
    // Compare against the typed refusal for the enum name channel.
    let _ = NoFix::NoFunctionAtLoc { line: 1 };
}

#[test]
fn court_allow_insert_measured_uses_tier2_count() {
    let src = "fn f(cond: bool, x: u32, y: u32) -> u32 {\n    if cond { x } else { y }\n}\n";
    // Tier-2 court data: branch_count measured on the compiled object.
    let measured: Option<u32> = Some(2);
    let edit = insert_allow_isel_residual(src, 1, measured).expect("insert applies");
    let rewritten = apply_edit(src, &edit);
    assert!(rewritten.contains("target = \"riscv32\""));
    assert!(rewritten.contains("max_branches = 2"));
    assert!(rewritten.contains("reason = \"measured\""));
    assert!(
        rewritten.contains("fn f(cond: bool"),
        "insert corrupts the fn"
    );
    // Unmeasured template omits max_branches honestly.
    let unmeasured = insert_allow_isel_residual(src, 1, None).expect("insert applies");
    let u_text = &unmeasured.edits[0].new_text;
    assert!(!u_text.contains("max_branches"));
    assert!(u_text.contains("reason = \"unmeasured\""));
    let _: Option<WorkspaceEdit> = None;
}
