// L7 Speciation — per-server C_D routing integration tests.
//
// Law: server-A with andon_code_prefixes=["WASM4PM-"] must NOT trigger ANDON
// for a diagnostic with code "GGEN-001", even when the workspace-level prefix
// union includes "GGEN-". The workspace union is the fallback only for servers
// with no override and for entries with no server_id.

use lsp_max_compositor::MergeContext;

/// Build a MergeContext where:
///   - workspace union = ["WASM4PM-", "GGEN-"]
///   - server-a override = ["WASM4PM-"] only
///   - server-b has no override → falls back to workspace union
fn ctx() -> MergeContext {
    let workspace_prefixes = vec!["WASM4PM-".to_string(), "GGEN-".to_string()];
    let mut ctx = MergeContext::new(workspace_prefixes);

    // Wire per-server override for server-a.
    ctx.add_server_prefix_override("server-a".to_string(), vec!["WASM4PM-".to_string()]);

    ctx
}

// ── assertion 1 ──────────────────────────────────────────────────────────────
// WASM4PM-CHEAT-C001 from server-a: prefix in server-a override → ANDON.
#[test]
fn wasm4pm_code_is_andon_for_server_a() {
    let c = ctx();
    assert!(
        c.is_andon_for_server("WASM4PM-CHEAT-C001", Some("server-a")),
        "WASM4PM-CHEAT-C001 must be ANDON for server-a (prefix in server-a override)"
    );
}

// ── assertion 2 ──────────────────────────────────────────────────────────────
// GGEN-TPL-001 from server-a: prefix NOT in server-a override → NOT ANDON.
#[test]
fn ggen_code_is_not_andon_for_server_a() {
    let c = ctx();
    assert!(
        !c.is_andon_for_server("GGEN-TPL-001", Some("server-a")),
        "GGEN-TPL-001 must NOT be ANDON for server-a (GGEN- excluded from server-a override)"
    );
}

// ── assertion 3 ──────────────────────────────────────────────────────────────
// GGEN-TPL-001 with no server_id: workspace union includes GGEN- → ANDON.
#[test]
fn ggen_code_is_andon_for_no_server() {
    let c = ctx();
    assert!(
        c.is_andon_for_server("GGEN-TPL-001", None),
        "GGEN-TPL-001 must be ANDON when no server_id (workspace union includes GGEN-)"
    );
}

// ── assertion 4 ──────────────────────────────────────────────────────────────
// GGEN-TPL-001 from server-b: no override → falls back to workspace union → ANDON.
#[test]
fn ggen_code_is_andon_for_server_b_no_override() {
    let c = ctx();
    assert!(
        c.is_andon_for_server("GGEN-TPL-001", Some("server-b")),
        "GGEN-TPL-001 must be ANDON for server-b (no override → workspace union includes GGEN-)"
    );
}
