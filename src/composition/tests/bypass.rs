//! Bypass and sabotage verification tests.
//! These confirm that each bypass/sabotage env var flips the expected behavior.

use super::super::*;
use serde_json::{json, Value};

fn make_source(id: &str, caps: Value) -> UpstreamSource {
    let mut src = UpstreamSource::new(id, "127.0.0.1:0");
    src.server_capabilities = serde_json::from_value(caps).ok();
    src
}

#[test]
fn test_gate8_method_routing_matrix() {
    let routed_methods = vec![
        "initialize",
        "initialized",
        "shutdown",
        "exit",
        "textDocument/didOpen",
        "textDocument/didChange",
        "textDocument/willSave",
        "textDocument/willSaveWaitUntil",
        "textDocument/didSave",
        "textDocument/didClose",
        "textDocument/declaration",
        "textDocument/definition",
        "textDocument/typeDefinition",
        "textDocument/implementation",
        "textDocument/references",
        "textDocument/prepareCallHierarchy",
        "callHierarchy/incomingCalls",
        "callHierarchy/outgoingCalls",
        "textDocument/prepareTypeHierarchy",
        "typeHierarchy/supertypes",
        "typeHierarchy/subtypes",
        "textDocument/documentHighlight",
        "textDocument/documentLink",
        "documentLink/resolve",
        "textDocument/hover",
        "textDocument/completion",
        "completionItem/resolve",
        "textDocument/semanticTokens/full",
        "textDocument/semanticTokens/full/delta",
        "textDocument/semanticTokens/range",
        "textDocument/codeLens",
        "codeLens/resolve",
        "textDocument/foldingRange",
        "textDocument/selectionRange",
        "textDocument/documentSymbol",
        "workspace/symbol",
        "workspaceSymbol/resolve",
        "workspace/didChangeConfiguration",
        "workspace/didChangeWorkspaceFolders",
        "workspace/willCreateFiles",
        "workspace/didCreateFiles",
        "workspace/willRenameFiles",
        "workspace/didRenameFiles",
        "workspace/willDeleteFiles",
        "workspace/didDeleteFiles",
        "workspace/didChangeWatchedFiles",
        "workspace/executeCommand",
        "textDocument/signatureHelp",
        "textDocument/codeAction",
        "codeAction/resolve",
        "textDocument/documentColor",
        "textDocument/colorPresentation",
        "textDocument/formatting",
        "textDocument/rangeFormatting",
        "textDocument/onTypeFormatting",
        "textDocument/rename",
        "textDocument/prepareRename",
        "textDocument/linkedEditingRange",
        "textDocument/moniker",
        "textDocument/inlayHint",
        "inlayHint/resolve",
        "textDocument/inlineValue",
        "textDocument/diagnostic",
        "workspace/diagnostic",
        "textDocument/inlineCompletion",
        "workspace/textDocumentContent",
        "textDocument/rangesFormatting",
        "notebookDocument/didOpen",
        "notebookDocument/didChange",
        "notebookDocument/didSave",
        "notebookDocument/didClose",
        "$/cancelRequest",
        "$/progress",
        "window/workDoneProgress/cancel",
        "$/setTrace",
    ];

    for method in &routed_methods {
        let strategy = method_strategy(method);
        assert_ne!(
            strategy,
            CompositionStrategy::Deny,
            "Method '{}' must map to an explicit routing strategy, not the default Deny strategy",
            method
        );
    }

    assert_eq!(
        method_strategy("textDocument/nonExistentMethod"),
        CompositionStrategy::Deny
    );
    assert_eq!(
        method_strategy("workspace/invalidRandomAction"),
        CompositionStrategy::Deny
    );
}

#[test]
fn test_bypass_capability_tracker() {
    let mut tracker = CapabilityTracker::new();
    let mut source1 = UpstreamSource::new("A", "127.0.0.1:1");
    let caps1: lsp_types_max::ServerCapabilities = serde_json::from_value(json!({
        "completionProvider": { "resolveProvider": true, "triggerCharacters": ["."] }
    }))
    .unwrap();
    source1.server_capabilities = Some(caps1);
    tracker.add_source(source1);

    let mut source2 = UpstreamSource::new("B", "127.0.0.1:2");
    let caps2: lsp_types_max::ServerCapabilities = serde_json::from_value(json!({
        "completionProvider": { "resolveProvider": false, "triggerCharacters": [".", ":"] }
    }))
    .unwrap();
    source2.server_capabilities = Some(caps2);
    tracker.add_source(source2);

    let client_caps = lsp_types_max::ClientCapabilities {
        text_document: Some(lsp_types_max::TextDocumentClientCapabilities {
            completion: Some(lsp_types_max::CompletionClientCapabilities {
                ..Default::default()
            }),
            ..Default::default()
        }),
        ..Default::default()
    };

    let effective = tracker.derive_effective_capabilities(&client_caps);
    let comp = effective.completion_provider.as_ref().unwrap();
    assert_eq!(comp.resolve_provider, Some(false));
    assert_eq!(
        comp.trigger_characters.as_ref().unwrap(),
        &vec![".".to_string()]
    );
}

#[test]
fn test_bypass_document_version_tracker() {
    let mut tracker = DocumentVersionTracker::new();
    let uri = "file:///test.rs";
    tracker.did_open(uri, 5);

    let outcome = tracker.did_change(uri, 3);
    assert!(matches!(outcome, VersionCheckResult::OutOfOrder { .. }));
    let staleness = tracker.check_staleness(uri, 4);
    assert!(matches!(staleness, VersionCheckResult::Stale { .. }));
}

#[test]
fn test_bypass_transaction_edit_gate() {
    let gate = TransactionEditGate::new();
    let mut doc = DocumentVersionTracker::new();
    let mut caps = CapabilityTracker::new();
    let uri = "file:///test.rs";

    let source = UpstreamSource::new("A", "127.0.0.1:1");
    caps.add_source(source);

    doc.did_open(uri, 5);

    let proposed = ProposedEdit {
        source_id: "A".to_string(),
        uri: uri.to_string(),
        version: 4,
        method: "textDocument/formatting".to_string(),
        edit: json!([]),
    };

    let outcome = gate.validate(&proposed, &doc, &caps);
    assert_eq!(outcome, EditGateOutcome::Stale);
}

#[test]
fn test_bypass_routing_matrix() {
    assert_eq!(
        method_strategy("textDocument/hover"),
        CompositionStrategy::FirstSuccess
    );
}

#[test]
fn test_bypass_source_health() {
    let mut tracker = CapabilityTracker::new();
    let source = UpstreamSource::new("A", "127.0.0.1:1");
    tracker.add_source(source);

    tracker.degrade_source("A", SourceHealth::Crashed);
    assert_eq!(tracker.sources["A"].health, SourceHealth::Crashed);
}

#[tokio::test]
async fn test_bypass_static_graph() {
    use crate::LspService;
    let (service, _) = LspService::new(|client| {
        ComposedServer::new(client, vec![("A".to_string(), "127.0.0.1:1".to_string())])
    });
    let server = service.inner();

    {
        let mut s = server.state.lock().await;
        s.doc_tracker.did_open("file:///test.rs", 5);
        s.capability_tracker.add_source(make_source("A", json!({})));
    }

    let params = json!({
        "textDocument": { "uri": "file:///test.rs" },
        "position": { "line": 0, "character": 0 }
    });
    let mut params_obj = params.as_object().unwrap().clone();
    params_obj.insert("context".to_string(), json!({ "version": 1 }));

    let resp = server
        .route_request::<_, serde_json::Value>("textDocument/definition", Value::Object(params_obj))
        .await;
    assert!(resp.is_ok());
    assert_eq!(resp.unwrap(), None);
}
