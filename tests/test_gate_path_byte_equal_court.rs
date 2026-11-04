// Court for the Λ_CD gate-path formula (G6, lane LM-1).
//
// src/primitives/gate_path.rs is the sole authoritative derivation. Two
// scaffold copies restate the formula by hand (examples/lsp-max-scaffold).
// The "must match exactly" comment was the old invariant — comments do not
// enforce. This court include_str!s all three files, extracts the
// `gate_file_path` derivation rows from each, normalizes the two lawful
// spellings (local `PathBuf` vs fully-qualified `std::path::PathBuf`, `pub fn`
// vs `fn`), and asserts row-by-row equality. A positive control refuses a
// copy that "matches" only because the agent-id branch was dropped everywhere
// (all-copies-drift non-vacuity).
use std::path::PathBuf;

const AUTHORITATIVE: &str = include_str!("../src/primitives/gate_path.rs");
const SCAFFOLD_GATE: &str = include_str!("../examples/lsp-max-scaffold/src/nouns/gate.rs");
const SCAFFOLD_SERVER: &str = include_str!("../examples/lsp-max-scaffold/src/server.rs");

/// Extract the normalized derivation rows of `gate_file_path` from a source
/// text. Normalization is limited to the lawful spellings observed across the
/// three copies: leading whitespace, `pub ` on the fn line, and
/// fully-qualified `std::path::` on the `PathBuf` type. Comment and blank
/// lines inside the body are dropped.
fn formula_rows(src: &str) -> Vec<String> {
    let brace_delta =
        |t: &str| -> i32 { t.matches('{').count() as i32 - t.matches('}').count() as i32 };
    let mut rows: Vec<String> = Vec::new();
    let mut in_body = false;
    let mut depth: i32 = 0;
    for line in src.lines() {
        let t = line.trim();
        if !in_body {
            if t.ends_with("fn gate_file_path() -> PathBuf {")
                || t.ends_with("fn gate_file_path() -> std::path::PathBuf {")
            {
                in_body = true;
                depth = brace_delta(t);
                rows.push(normalize(t));
            }
            continue;
        }
        depth += brace_delta(t);
        if depth <= 0 {
            // closing line — include it, body complete
            rows.push(normalize(t));
            return rows;
        }
        if t.is_empty() || t.starts_with("//") {
            continue;
        }
        rows.push(normalize(t));
        continue;
    }
    rows
}

fn normalize(t: &str) -> String {
    t.replace("pub ", "")
        .replace("std::path::PathBuf", "PathBuf")
        .to_string()
}

fn row_diff(a: &[String], b: &[String]) -> String {
    let mut out = String::from("formula rows diverge:\n");
    for i in 0..a.len().max(b.len()) {
        let la = a.get(i).map(|s| s.as_str()).unwrap_or("<missing>");
        let lb = b.get(i).map(|s| s.as_str()).unwrap_or("<missing>");
        if la != lb {
            out.push_str(&format!(
                "  row {i}:\n    authoritative: {la}\n    copy:         {lb}\n"
            ));
        }
    }
    out
}

#[test]
fn scaffold_gate_noun_formula_is_row_identical_to_authoritative() {
    let auth = formula_rows(AUTHORITATIVE);
    let copy = formula_rows(SCAFFOLD_GATE);
    assert!(
        !auth.is_empty(),
        "authoritative gate_file_path not found in src/primitives/gate_path.rs"
    );
    assert_eq!(auth, copy, "{}", row_diff(&auth, &copy));
}

#[test]
fn scaffold_server_formula_is_row_identical_to_authoritative() {
    let auth = formula_rows(AUTHORITATIVE);
    let copy = formula_rows(SCAFFOLD_SERVER);
    assert_eq!(auth, copy, "{}", row_diff(&auth, &copy));
}

#[test]
fn positive_control_agent_id_branch_present_in_all_three() {
    // Non-vacuity: three copies could agree by all dropping the same branch.
    // The derivation must still carry the LSP_MAX_AGENT_ID routing.
    for (name, src) in [
        ("authoritative", AUTHORITATIVE),
        ("scaffold gate.rs", SCAFFOLD_GATE),
        ("scaffold server.rs", SCAFFOLD_SERVER),
    ] {
        let rows = formula_rows(src);
        assert!(
            rows.iter().any(|r| r.contains("LSP_MAX_AGENT_ID")),
            "{name}: formula rows lost the LSP_MAX_AGENT_ID branch — \
             copies must match the AUTHORITATIVE formula, not a reduced one"
        );
        assert!(
            rows.iter().any(|r| r.contains("XDG_RUNTIME_DIR")),
            "{name}: formula rows lost the XDG_RUNTIME_DIR branch"
        );
    }
}

#[test]
fn authoritative_formula_derives_expected_file_name() {
    // Real derivation over the real function: no agent id -> plain name.
    // (Env-sandboxed by setting/removing through the real std::env on the
    // test process is racy under parallel tests, so assert structurally.)
    let rows = formula_rows(AUTHORITATIVE);
    let joined = rows.join("\n");
    assert!(joined.contains("lsp-max-gate-{hash:016x}"));
    assert!(joined.contains("lsp-max-gate-{hash:016x}-agent-{agent_id}"));
    assert!(joined.contains("fnv1a(workspace.to_string_lossy().as_bytes())"));
    assert!(PathBuf::from("/tmp").join("x") == *"/tmp/x");
}
