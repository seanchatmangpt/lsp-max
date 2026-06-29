//! Chicago acceptance test for CC-007: LSIF DiagnosticsOnly tier routing
//! Status: CANDIDATE — implement ChildTier::Lsif and route_lsif_fallback to make this pass.
//! Ticket: docs/jira/v26.6.30/CC-007-lsif-tier.md

use chicago_tdd_tools::chicago_test;
use lsp_max_compositor::registry::ChildTier;

#[chicago_test(
    ticket      = "docs/jira/v26.6.30/CC-007-lsif-tier.md",
    scaffold_fn = "lsp_max_compositor::routing::route_lsif_fallback"
)]
fn did_change_not_forwarded_to_lsif_tier() {
    // Given: the LSIF tier
    let tier = ChildTier::Lsif;
    // When: routing decision for didChange is queried
    let should_forward = lsp_max_compositor::routing::should_forward_notification("textDocument/didChange", &tier);
    // Then: didChange is NOT forwarded to LSIF servers (read-only snapshot)
    assert!(!should_forward, "didChange must never be forwarded to LSIF tier");
}

#[chicago_test(
    ticket      = "docs/jira/v26.6.30/CC-007-lsif-tier.md",
    scaffold_fn = "lsp_max_compositor::routing::route_lsif_fallback"
)]
fn definition_falls_back_to_lsif_when_primary_returns_null() {
    // Given: LSIF tier is registered for textDocument/definition
    // When: route_lsif_fallback is called with method = definition
    let decision = lsp_max_compositor::routing::route_lsif_fallback(
        "textDocument/definition",
        &ChildTier::Lsif,
    );
    // Then: decision is FallbackToLsif (not excluded)
    assert!(
        matches!(decision, lsp_max_compositor::routing::RoutingDecision::FallbackToLsif),
        "textDocument/definition should produce FallbackToLsif routing for LSIF tier, got: {decision:?}"
    );
}
