use lsp_max_compositor::registry::ChildTier;
use lsp_max_compositor::routing::{
    route_lsif_fallback, should_forward_notification, RoutingDecision,
};

// --- route_lsif_fallback ---

#[test]
fn lsif_nav_methods_return_fallback_to_lsif() {
    let nav_methods = [
        "textDocument/definition",
        "textDocument/references",
        "textDocument/hover",
        "textDocument/documentSymbol",
        "textDocument/implementation",
        "textDocument/typeDefinition",
    ];
    for method in nav_methods {
        let decision = route_lsif_fallback(method, &ChildTier::Lsif);
        assert!(
            matches!(decision, RoutingDecision::FallbackToLsif { .. }),
            "expected FallbackToLsif for {method}, got {decision:?}"
        );
    }
}

#[test]
fn lsif_write_methods_return_excluded() {
    let write_methods = [
        "textDocument/didOpen",
        "textDocument/didChange",
        "textDocument/didClose",
        "window/showMessage",
        "$/cancelRequest",
        "initialized",
        "shutdown",
    ];
    for method in write_methods {
        let decision = route_lsif_fallback(method, &ChildTier::Lsif);
        assert!(
            matches!(decision, RoutingDecision::Unroutable { .. }),
            "expected Unroutable for {method}, got {decision:?}"
        );
    }
}

#[test]
fn lsif_unknown_method_returns_excluded() {
    let decision = route_lsif_fallback("textDocument/someCustomMethod", &ChildTier::Lsif);
    assert!(matches!(decision, RoutingDecision::Unroutable { .. }));
}

#[test]
fn non_lsif_tier_returns_excluded_for_nav_methods() {
    // LSIF fallback logic only applies to the Lsif tier.
    for tier in [
        ChildTier::Primary,
        ChildTier::Secondary,
        ChildTier::DiagnosticsOnly,
    ] {
        let decision = route_lsif_fallback("textDocument/definition", &tier);
        assert!(
            matches!(decision, RoutingDecision::Unroutable { .. }),
            "expected Unroutable for non-Lsif tier {}, got {decision:?}",
            tier.as_str()
        );
    }
}

// --- should_forward_notification ---

#[test]
fn lsif_tier_never_receives_notifications() {
    let methods = [
        "textDocument/didOpen",
        "textDocument/didChange",
        "textDocument/didClose",
        "textDocument/definition",
        "initialized",
        "$/cancelRequest",
    ];
    for method in methods {
        assert!(
            !should_forward_notification(method, &ChildTier::Lsif),
            "Lsif tier must not receive {method}"
        );
    }
}

#[test]
fn primary_secondary_forward_all_notifications() {
    let methods = [
        "textDocument/didOpen",
        "textDocument/didChange",
        "textDocument/didClose",
        "initialized",
        "$/cancelRequest",
    ];
    for tier in [ChildTier::Primary, ChildTier::Secondary] {
        for method in methods {
            assert!(
                should_forward_notification(method, &tier),
                "{} tier must forward {method}",
                tier.as_str()
            );
        }
    }
}

#[test]
fn diagnostics_only_skips_doc_lifecycle_but_passes_others() {
    assert!(!should_forward_notification(
        "textDocument/didChange",
        &ChildTier::DiagnosticsOnly
    ));
    assert!(!should_forward_notification(
        "textDocument/didOpen",
        &ChildTier::DiagnosticsOnly
    ));
    assert!(!should_forward_notification(
        "textDocument/didClose",
        &ChildTier::DiagnosticsOnly
    ));
    // Non-lifecycle notifications pass through.
    assert!(should_forward_notification(
        "textDocument/publishDiagnostics",
        &ChildTier::DiagnosticsOnly
    ));
    assert!(should_forward_notification(
        "initialized",
        &ChildTier::DiagnosticsOnly
    ));
}

// Falsification: FallbackToLsif carries the method string correctly.
#[test]
fn fallback_decision_carries_method_name() {
    let method = "textDocument/definition";
    let decision = route_lsif_fallback(method, &ChildTier::Lsif);
    match decision {
        RoutingDecision::FallbackToLsif { method: m } => assert_eq!(m, method),
        other => panic!("unexpected decision: {other:?}"),
    }
}
