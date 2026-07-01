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

// ── L7 Speciation — concrete production server isolation ────────────────────
//
// Mirrors the actual lsp-max.toml configuration:
//   wasm4pm-lsp        → ["WASM4PM-", "GGEN-EVIDENCE-", "CLAP-PACK-HANDLER-UNBOUND", "COG-"]
//   diagnostics-only-lsp → ["ANTI-LLM-"]
//   ggen-lsp           → ["GGEN-"]
//
// Isolation invariant: a code prefix declared only by server X must NOT set ANDON
// when the diagnostic is attributed to server Y. The workspace union (which covers
// all prefix families) must never leak into a configured server's C_D routing.

fn production_ctx() -> lsp_max_compositor::MergeContext {
    // Workspace-wide union exactly as produced by CompositorConfig::all_andon_prefixes()
    // for the three servers above.
    let union = vec![
        "WASM4PM-".to_string(),
        "GGEN-EVIDENCE-".to_string(),
        "CLAP-PACK-HANDLER-UNBOUND".to_string(),
        "COG-".to_string(),
        "ANTI-LLM-".to_string(),
        "GGEN-".to_string(),
    ];
    let mut ctx = lsp_max_compositor::MergeContext::new(union);
    ctx.add_server_prefix_override(
        "wasm4pm-lsp".to_string(),
        vec![
            "WASM4PM-".to_string(),
            "GGEN-EVIDENCE-".to_string(),
            "CLAP-PACK-HANDLER-UNBOUND".to_string(),
            "COG-".to_string(),
        ],
    );
    ctx.add_server_prefix_override(
        "diagnostics-only-lsp".to_string(),
        vec!["ANTI-LLM-".to_string()],
    );
    ctx.add_server_prefix_override("ggen-lsp".to_string(), vec!["GGEN-".to_string()]);
    ctx
}

// wasm4pm-lsp emits WASM4PM-CROWN-001 → must be ANDON (own prefix).
#[test]
fn wasm4pm_own_prefix_is_andon() {
    let ctx = production_ctx();
    assert!(
        ctx.is_andon_for_server("WASM4PM-CROWN-001", Some("wasm4pm-lsp")),
        "WASM4PM-CROWN-001 from wasm4pm-lsp must be ANDON (wasm4pm-lsp declared WASM4PM-)"
    );
}

// wasm4pm-lsp emits GGEN-EVIDENCE-MISSING-001 → must be ANDON (own prefix).
#[test]
fn wasm4pm_ggen_evidence_prefix_is_andon() {
    let ctx = production_ctx();
    assert!(
        ctx.is_andon_for_server("GGEN-EVIDENCE-MISSING-001", Some("wasm4pm-lsp")),
        "GGEN-EVIDENCE-MISSING-001 from wasm4pm-lsp must be ANDON (wasm4pm-lsp declared GGEN-EVIDENCE-)"
    );
}

// diagnostics-only-lsp emits ANTI-LLM-CHEAT-C001 → must be ANDON (own prefix).
#[test]
fn anti_llm_own_prefix_is_andon() {
    let ctx = production_ctx();
    assert!(
        ctx.is_andon_for_server("ANTI-LLM-CHEAT-C001", Some("diagnostics-only-lsp")),
        "ANTI-LLM-CHEAT-C001 from diagnostics-only-lsp must be ANDON (declared ANTI-LLM-)"
    );
}

// ggen-lsp emits GGEN-TPL-001 → must be ANDON (own prefix).
#[test]
fn ggen_lsp_own_prefix_is_andon() {
    let ctx = production_ctx();
    assert!(
        ctx.is_andon_for_server("GGEN-TPL-001", Some("ggen-lsp")),
        "GGEN-TPL-001 from ggen-lsp must be ANDON (ggen-lsp declared GGEN-)"
    );
}

// Cross-isolation: WASM4PM- code attributed to diagnostics-only-lsp must NOT be ANDON.
// diagnostics-only-lsp only declared ANTI-LLM-; it must not inherit WASM4PM- from the union.
#[test]
fn wasm4pm_code_not_andon_for_anti_llm_server() {
    let ctx = production_ctx();
    assert!(
        !ctx.is_andon_for_server("WASM4PM-CROWN-001", Some("diagnostics-only-lsp")),
        "WASM4PM-CROWN-001 attributed to diagnostics-only-lsp must NOT be ANDON — \
         diagnostics-only-lsp declared only ANTI-LLM-, union must not leak"
    );
}

// Cross-isolation: ANTI-LLM- code attributed to wasm4pm-lsp must NOT be ANDON.
// wasm4pm-lsp declared WASM4PM-/GGEN-EVIDENCE-/CLAP-PACK-HANDLER-UNBOUND/COG-; not ANTI-LLM-.
#[test]
fn anti_llm_code_not_andon_for_wasm4pm_server() {
    let ctx = production_ctx();
    assert!(
        !ctx.is_andon_for_server("ANTI-LLM-CHEAT-C001", Some("wasm4pm-lsp")),
        "ANTI-LLM-CHEAT-C001 attributed to wasm4pm-lsp must NOT be ANDON — \
         wasm4pm-lsp did not declare ANTI-LLM-, union must not leak"
    );
}

// Cross-isolation: GGEN- code (not GGEN-EVIDENCE-) attributed to wasm4pm-lsp must NOT be ANDON.
// wasm4pm-lsp declared GGEN-EVIDENCE- (a strict sub-namespace), not the broader GGEN- prefix.
#[test]
fn ggen_tpl_not_andon_for_wasm4pm_server() {
    let ctx = production_ctx();
    assert!(
        !ctx.is_andon_for_server("GGEN-TPL-001", Some("wasm4pm-lsp")),
        "GGEN-TPL-001 attributed to wasm4pm-lsp must NOT be ANDON — \
         wasm4pm-lsp declared only GGEN-EVIDENCE- (sub-namespace), not the broader GGEN-"
    );
}

// Cross-isolation: GGEN-EVIDENCE- code attributed to ggen-lsp IS ANDON because ggen-lsp
// declared GGEN- (a broader prefix that covers GGEN-EVIDENCE- as a sub-namespace).
#[test]
fn ggen_evidence_is_andon_for_ggen_lsp() {
    let ctx = production_ctx();
    assert!(
        ctx.is_andon_for_server("GGEN-EVIDENCE-MISSING-001", Some("ggen-lsp")),
        "GGEN-EVIDENCE-MISSING-001 attributed to ggen-lsp must be ANDON — \
         ggen-lsp declared GGEN- which is a broader prefix covering GGEN-EVIDENCE-"
    );
}
