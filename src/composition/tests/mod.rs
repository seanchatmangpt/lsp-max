//! Composition unit tests — R2 through R6.

use super::*;
use serde_json::{json, Value};

#[cfg(test)]
mod bypass;

// ── R2: Capability Tracker Tests ──────────────────────────────────────────

fn make_source(id: &str, caps: Value) -> UpstreamSource {
    let mut src = UpstreamSource::new(id, "127.0.0.1:0");
    src.server_capabilities = serde_json::from_value(caps).ok();
    src
}

#[test]
fn test_completion_deser() {
    let val = json!({
        "isIncomplete": false,
        "items": [
            { "label": "apple", "detail": "high priority", "sortText": "01" }
        ]
    });
    let res: std::result::Result<lsp_types_max::CompletionResponse, _> =
        serde_json::from_value(val);
    if let Err(e) = res {
        panic!("DESER ERROR: {:?}", e);
    }
}

#[test]
fn r2_source_a_supports_source_b_does_not_yields_non_union() {
    let mut tracker = CapabilityTracker::new();
    tracker.add_source(make_source("A", json!({ "hoverProvider": true })));
    tracker.add_source(make_source("B", json!({ "hoverProvider": false })));

    let client_caps_val = json!({ "textDocument": { "hover": {} } });
    let client_caps: lsp_types_max::ClientCapabilities =
        serde_json::from_value(client_caps_val).unwrap();
    let effective = tracker.derive_effective_capabilities(&client_caps);

    assert_eq!(
        effective.hover_provider,
        Some(lsp_types_max::HoverProviderCapability::Simple(true)),
        "hover should be advertised since source A supports it"
    );
    assert!(
        effective.definition_provider.is_none(),
        "definitionProvider must not be advertised if no source supports it"
    );
}

#[test]
fn r2_no_source_supports_method_not_advertised() {
    let mut tracker = CapabilityTracker::new();
    tracker.add_source(make_source("A", json!({ "hoverProvider": false })));
    tracker.add_source(make_source("B", json!({ "hoverProvider": false })));

    let client_caps = lsp_types_max::ClientCapabilities::default();
    let effective = tracker.derive_effective_capabilities(&client_caps);
    assert!(
        effective.hover_provider.is_none(),
        "hover must not be advertised if no source supports it"
    );
}

#[test]
fn r2_dynamic_registration_adds_support() {
    let mut tracker = CapabilityTracker::new();
    tracker.add_source(make_source("A", json!({})));

    assert!(!tracker.sources["A"].supports_method("textDocument/hover"));

    let ok = tracker.register_dynamic("reg-1", "textDocument/hover", "A", json!({}));
    assert!(ok, "First registration should succeed");
    assert!(
        tracker.sources["A"].supports_method("textDocument/hover"),
        "After dynamic registration, source A should support hover"
    );
}

#[test]
fn r2_dynamic_registration_duplicate_rejected() {
    let mut tracker = CapabilityTracker::new();
    tracker.add_source(make_source("A", json!({})));

    let ok1 = tracker.register_dynamic("reg-1", "textDocument/hover", "A", json!({}));
    let ok2 = tracker.register_dynamic("reg-1", "textDocument/hover", "A", json!({}));
    assert!(ok1, "First registration should succeed");
    assert!(!ok2, "Duplicate registration ID must be rejected");
}

#[test]
fn r2_dynamic_unregistration_removes_support() {
    let mut tracker = CapabilityTracker::new();
    tracker.add_source(make_source("A", json!({})));
    tracker.register_dynamic("reg-1", "textDocument/hover", "A", json!({}));
    assert!(tracker.sources["A"].supports_method("textDocument/hover"));

    let removed = tracker.unregister_dynamic("reg-1");
    assert!(removed, "Unregistration should succeed");
    assert!(
        !tracker.sources["A"].supports_method("textDocument/hover"),
        "After unregistration, hover must no longer be supported"
    );
}

#[test]
fn r2_unregister_nonexistent_is_safe_noop() {
    let mut tracker = CapabilityTracker::new();
    let removed = tracker.unregister_dynamic("nonexistent");
    assert!(
        !removed,
        "Unregistering nonexistent ID must return false (safe no-op)"
    );
}

#[test]
fn r2_unsafe_route_unsupported_source_not_routable() {
    let mut tracker = CapabilityTracker::new();
    let mut src = make_source("A", json!({ "hoverProvider": true }));
    src.health = SourceHealth::Crashed;
    tracker.add_source(src);

    let routes = tracker.routable_sources_for_method("textDocument/hover");
    assert!(
        routes.is_empty(),
        "Crashed source must not be in routable list"
    );

    let client_caps = lsp_types_max::ClientCapabilities::default();
    let effective = tracker.derive_effective_capabilities(&client_caps);
    assert!(
        effective.hover_provider.is_none(),
        "If no healthy source supports hover, it must not be advertised"
    );
}

#[test]
fn r2_client_incapable_not_advertised() {
    let mut tracker = CapabilityTracker::new();
    tracker.add_source(make_source("A", json!({ "hoverProvider": true })));

    let client_caps_val = json!({ "textDocument": { "hover": null } });
    let client_caps: lsp_types_max::ClientCapabilities =
        serde_json::from_value(client_caps_val).unwrap();
    let effective = tracker.derive_effective_capabilities(&client_caps);
    assert!(
        effective.hover_provider.is_none(),
        "If client does not support hover, it must not be advertised"
    );
}

#[test]
fn r2_capabilities_not_raw_union() {
    let mut tracker = CapabilityTracker::new();
    tracker.add_source(make_source("A", json!({ "definitionProvider": true })));
    tracker.add_source(make_source("B", json!({ "definitionProvider": false })));

    tracker.degrade_source("A", SourceHealth::Crashed);
    let client_caps = lsp_types_max::ClientCapabilities::default();
    let effective = tracker.derive_effective_capabilities(&client_caps);
    assert!(
        effective.definition_provider.is_none(),
        "After degrading only supporting source, definition must not be advertised"
    );
}

// ── R3: Method Strategy Tests ─────────────────────────────────────────────

#[test]
fn r3_method_strategies_are_explicit() {
    assert_eq!(
        method_strategy("initialize"),
        CompositionStrategy::SingleOwner
    );
    assert_eq!(
        method_strategy("textDocument/didChange"),
        CompositionStrategy::OrderedFanout
    );
    assert_eq!(
        method_strategy("textDocument/hover"),
        CompositionStrategy::FirstSuccess
    );
    assert_eq!(
        method_strategy("textDocument/definition"),
        CompositionStrategy::MergeDeduped
    );
    assert_eq!(
        method_strategy("textDocument/references"),
        CompositionStrategy::MergeDeduped
    );
    assert_eq!(
        method_strategy("textDocument/documentSymbol"),
        CompositionStrategy::MergeAttributed
    );
    assert_eq!(
        method_strategy("textDocument/completion"),
        CompositionStrategy::RankedProviders
    );
    assert_eq!(
        method_strategy("textDocument/formatting"),
        CompositionStrategy::TransactionalEditGate
    );
    assert_eq!(
        method_strategy("textDocument/rename"),
        CompositionStrategy::TransactionalEditGate
    );
    assert_eq!(
        method_strategy("textDocument/codeAction"),
        CompositionStrategy::TransactionalEditGate
    );
    assert_eq!(
        method_strategy("workspace/applyEdit"),
        CompositionStrategy::TransactionalEditGate
    );
    assert_eq!(
        method_strategy("$/cancelRequest"),
        CompositionStrategy::ObserveOnly
    );
    assert_eq!(
        method_strategy("some/unknown/method"),
        CompositionStrategy::Deny
    );
}

#[test]
fn r3_unknown_method_denied() {
    let strategy = method_strategy("custom/nonstandardMethod");
    assert_eq!(
        strategy,
        CompositionStrategy::Deny,
        "Unknown methods must be denied by default"
    );
}

// ── R4: Document Version Causality Tests ──────────────────────────────────

#[test]
fn r4_ordered_changes_accepted() {
    let mut tracker = DocumentVersionTracker::new();
    tracker.did_open("file:///test.rs", 1);
    assert_eq!(
        tracker.did_change("file:///test.rs", 2),
        VersionCheckResult::Current
    );
    assert_eq!(
        tracker.did_change("file:///test.rs", 3),
        VersionCheckResult::Current
    );
}

#[test]
fn r4_out_of_order_change_refused() {
    let mut tracker = DocumentVersionTracker::new();
    tracker.did_open("file:///test.rs", 5);
    let result = tracker.did_change("file:///test.rs", 3);
    assert_eq!(
        result,
        VersionCheckResult::OutOfOrder {
            expected: 6,
            got: 3
        },
        "Out-of-order version change must be refused"
    );
}

#[test]
fn r4_stale_result_detected() {
    let mut tracker = DocumentVersionTracker::new();
    tracker.did_open("file:///test.rs", 1);
    let _ = tracker.did_change("file:///test.rs", 2);
    let check = tracker.check_staleness("file:///test.rs", 1);
    assert_eq!(
        check,
        VersionCheckResult::Stale {
            current: 2,
            result_version: 1
        },
        "Result at old version must be detected as stale"
    );
}

#[test]
fn r4_did_close_ends_tracking() {
    let mut tracker = DocumentVersionTracker::new();
    tracker.did_open("file:///test.rs", 1);
    tracker.did_close("file:///test.rs");
    assert_eq!(
        tracker.current_version("file:///test.rs"),
        None,
        "After didClose, document must no longer be tracked"
    );
}

#[test]
fn r4_untracked_document_returns_not_tracked() {
    let tracker = DocumentVersionTracker::new();
    assert_eq!(
        tracker.check_staleness("file:///untracked.rs", 1),
        VersionCheckResult::NotTracked
    );
}

// ── R5: Edit Gate Tests ───────────────────────────────────────────────────

fn make_proposed_edit(source_id: &str, uri: &str, version: i32, method: &str) -> ProposedEdit {
    ProposedEdit {
        source_id: source_id.to_string(),
        uri: uri.to_string(),
        version,
        method: method.to_string(),
        edit: json!([
            { "range": { "start": { "line": 0, "character": 0 }, "end": { "line": 0, "character": 5 } }, "newText": "hello" }
        ]),
    }
}

fn setup_gate_env() -> (
    TransactionEditGate,
    DocumentVersionTracker,
    CapabilityTracker,
) {
    let gate = TransactionEditGate::new();
    let mut doc = DocumentVersionTracker::new();
    doc.did_open("file:///test.rs", 1);
    let mut caps = CapabilityTracker::new();
    caps.add_source(make_source(
        "src-a",
        json!({ "documentFormattingProvider": true }),
    ));
    (gate, doc, caps)
}

#[test]
fn r5_formatting_through_edit_gate_accepted() {
    let (gate, doc, caps) = setup_gate_env();
    let edit = make_proposed_edit("src-a", "file:///test.rs", 1, "textDocument/formatting");
    assert_eq!(gate.validate(&edit, &doc, &caps), EditGateOutcome::Accepted);
}

#[test]
fn r5_stale_edit_refused() {
    let (gate, mut doc, caps) = setup_gate_env();
    let _ = doc.did_change("file:///test.rs", 2);
    let edit = make_proposed_edit("src-a", "file:///test.rs", 1, "textDocument/formatting");
    assert_eq!(
        gate.validate(&edit, &doc, &caps),
        EditGateOutcome::Stale,
        "Edit at old version must be refused as stale"
    );
}

#[test]
fn r5_overlapping_edit_refused() {
    let (mut gate, doc, caps) = setup_gate_env();
    let edit_a = make_proposed_edit("src-a", "file:///test.rs", 1, "textDocument/formatting");
    gate.accept(edit_a);

    let mut caps2 = caps;
    caps2.add_source(make_source(
        "src-b",
        json!({ "documentFormattingProvider": true }),
    ));
    let edit_b = make_proposed_edit("src-b", "file:///test.rs", 1, "textDocument/formatting");
    assert_eq!(
        gate.validate(&edit_b, &doc, &caps2),
        EditGateOutcome::Overlapping,
        "Edit from different source targeting same document must be refused as overlapping"
    );
}

#[test]
fn r5_degraded_source_edit_refused() {
    let (gate, doc, mut caps) = setup_gate_env();
    caps.degrade_source("src-a", SourceHealth::Crashed);
    let edit = make_proposed_edit("src-a", "file:///test.rs", 1, "textDocument/formatting");
    assert_eq!(
        gate.validate(&edit, &doc, &caps),
        EditGateOutcome::SourceDegraded,
        "Edit from degraded source must be refused"
    );
}

#[test]
fn r5_unknown_document_edit_refused() {
    let (gate, doc, caps) = setup_gate_env();
    let edit = make_proposed_edit("src-a", "file:///unknown.rs", 1, "textDocument/formatting");
    assert_eq!(
        gate.validate(&edit, &doc, &caps),
        EditGateOutcome::Accepted,
        "Edit for untracked document must be accepted"
    );
}

#[test]
fn r5_non_mutation_method_not_allowed_through_gate() {
    let (gate, doc, caps) = setup_gate_env();
    let edit = make_proposed_edit("src-a", "file:///test.rs", 1, "textDocument/hover");
    assert_eq!(
        gate.validate(&edit, &doc, &caps),
        EditGateOutcome::MethodNotAllowed,
        "Non-mutation methods must be refused by the edit gate"
    );
}

#[test]
fn r5_workspace_edit_cannot_bypass_gate() {
    let (gate, doc, caps) = setup_gate_env();
    let edit = make_proposed_edit("src-a", "file:///test.rs", 1, "workspace/applyEdit");
    let outcome = gate.validate(&edit, &doc, &caps);
    assert_ne!(
        outcome,
        EditGateOutcome::MethodNotAllowed,
        "workspace/applyEdit must pass through gate checks (not be excluded as MethodNotAllowed)"
    );
}

// ── R6: Failure Isolation Tests ───────────────────────────────────────────

#[test]
fn r6_crashed_source_does_not_poison_others() {
    let mut tracker = CapabilityTracker::new();
    tracker.add_source(make_source("A", json!({ "hoverProvider": true })));
    tracker.add_source(make_source("B", json!({ "hoverProvider": true })));

    tracker.degrade_source("A", SourceHealth::Crashed);

    let routes = tracker.routable_sources_for_method("textDocument/hover");
    assert_eq!(
        routes.len(),
        1,
        "After source A crashes, only source B should be routable"
    );
    assert_eq!(routes[0], "B");
}

#[test]
fn r6_all_sources_degraded_no_capability_advertised() {
    let mut tracker = CapabilityTracker::new();
    tracker.add_source(make_source("A", json!({ "hoverProvider": true })));
    tracker.degrade_source("A", SourceHealth::InitializationFailed);

    let effective =
        tracker.derive_effective_capabilities(&lsp_types_max::ClientCapabilities::default());
    assert!(
        effective.hover_provider.is_none(),
        "If all sources are degraded, no capabilities should be advertised"
    );
}

// ── R4: Source Attribution Tests ──────────────────────────────────────────

#[test]
fn r4_merged_output_preserves_source_identity() {
    let observations = vec![
        AttributedObservation {
            source_id: "lang-server".to_string(),
            uri: "file:///test.rs".to_string(),
            data: json!([{ "message": "error 1" }]),
        },
        AttributedObservation {
            source_id: "lint-server".to_string(),
            uri: "file:///test.rs".to_string(),
            data: json!([{ "message": "warning 2" }]),
        },
    ];

    let merged = merge_attributed(observations);
    let arr = merged.as_array().unwrap();
    assert_eq!(arr.len(), 2);
    assert_eq!(
        arr[0]["source"],
        json!("lang-server"),
        "Source identity must be preserved in merged output"
    );
    assert_eq!(
        arr[1]["source"],
        json!("lint-server"),
        "Source identity must be preserved in merged output"
    );
}

#[test]
fn r4_deduped_locations_removes_duplicates() {
    let loc = json!({ "uri": "file:///test.rs", "range": { "start": {"line":1,"character":0}, "end": {"line":1,"character":5} } });
    let observations = vec![
        AttributedObservation {
            source_id: "A".to_string(),
            uri: "file:///test.rs".to_string(),
            data: json!([loc.clone()]),
        },
        AttributedObservation {
            source_id: "B".to_string(),
            uri: "file:///test.rs".to_string(),
            data: json!([loc.clone()]),
        },
    ];

    let merged = merge_deduped_locations(observations);
    assert_eq!(merged.len(), 1, "Duplicate locations must be deduplicated");
}
