use serde_json::json;
use tower_lsp_max::lsp_types::{
    CodeAction, CodeActionDisabled, CodeActionKind, CodeActionKindDocumentation,
    CodeActionTag, Command, NumberOrString,
};
use tower_lsp_max_lsif::lsif::{
    Element, PositionEncoding, ToolInfo, Vertex, VertexType,
};

// ============================================================================
// 1. R1: Protocol Type Completeness - Positive Test Cases
// ============================================================================

#[test]
fn test_r1_positive_code_action_serialization() {
    let tag = CodeActionTag::LLM_GENERATED;
    let doc = CodeActionKindDocumentation {
        kind: CodeActionKind::QUICKFIX,
        command: Command {
            title: "Quick Fix".to_string(),
            command: "quickfix.run".to_string(),
            arguments: None,
        },
    };
    let disabled = CodeActionDisabled {
        reason: "Deprecated method".to_string(),
    };

    let action = CodeAction {
        title: "Apply LLM Refactor".to_string(),
        kind: Some(CodeActionKind::REFACTOR),
        diagnostics: None,
        edit: None,
        command: None,
        is_preferred: Some(true),
        disabled: Some(disabled),
        tags: Some(vec![tag]),
        documentation: Some(vec![doc]),
        data: Some(json!({ "refactor_id": 123 })),
    };

    let serialized = serde_json::to_string(&action).expect("Should serialize CodeAction");
    
    // Verify serialized JSON contains the new 3.18.0 fields
    assert!(serialized.contains("tags"), "Serialized JSON must contain 'tags'");
    assert!(serialized.contains("documentation"), "Serialized JSON must contain 'documentation'");
    assert!(serialized.contains("disabled"), "Serialized JSON must contain 'disabled'");
    assert!(serialized.contains("data"), "Serialized JSON must contain 'data'");

    let deserialized: CodeAction = serde_json::from_str(&serialized).expect("Should deserialize CodeAction");
    assert_eq!(action.title, deserialized.title);
    assert_eq!(action.is_preferred, deserialized.is_preferred);
    assert_eq!(action.disabled, deserialized.disabled);
    assert_eq!(action.tags, deserialized.tags);
    assert_eq!(action.documentation, deserialized.documentation);
    assert_eq!(action.data, deserialized.data);
}

#[test]
fn test_r1_positive_lsif_graph_types_serialization() {
    // Test that LSIF vertices can be serialized/deserialized strongly typed
    let vertex = Vertex::MetaData {
        id: NumberOrString::Number(1),
        type_: VertexType::Vertex,
        version: "0.6.0".to_string(),
        project_root: "file:///".to_string(),
        position_encoding: PositionEncoding::Utf16,
        tool_info: Some(ToolInfo {
            name: "tower-lsp-max".to_string(),
            version: Some("26.6.5".to_string()),
            args: None,
        }),
    };

    let element = Element::Vertex(vertex);
    let serialized = serde_json::to_string(&element).expect("Should serialize LSIF element");
    
    assert!(serialized.contains(r#""label":"metaData""#));
    assert!(serialized.contains(r#""version":"0.6.0""#));

    let deserialized: Element = serde_json::from_str(&serialized).expect("Should deserialize LSIF element");
    match deserialized {
        Element::Vertex(Vertex::MetaData { version, .. }) => {
            assert_eq!(version, "0.6.0");
        }
        _ => panic!("Expected Vertex::MetaData"),
    }
}

// ============================================================================
// 2. R1: Protocol Type Completeness - Negative Test Cases
// ============================================================================

#[test]
fn test_r1_negative_invalid_code_action_rejected() {
    // Attempting to deserialize a malformed tag (wrong type)
    let bad_json = r#"{
        "title": "Bad Action",
        "tags": ["invalid-tag-format"]
    }"#;
    let res: std::result::Result<CodeAction, _> = serde_json::from_str(bad_json);
    assert!(res.is_err(), "Should reject invalid tags type");
}

#[test]
fn test_r1_negative_malformed_lsif_vertex_rejected() {
    // MetaData with invalid version type (integer instead of string)
    let bad_json = r#"{
        "id": 1,
        "type": "vertex",
        "label": "metaData",
        "version": 60,
        "positionEncoding": "utf-16"
    }"#;
    let res: std::result::Result<Element, _> = serde_json::from_str(bad_json);
    assert!(res.is_err(), "Should reject non-string version in LSIF MetaData");
}

// ============================================================================
// 3. R2: Capability Discovery - Bypass / Fake-Risk Test Cases
// ============================================================================

/// Proves that if the capability computer or type layer returns a static mock/stub, the test suite fails.
/// This test checks if the server's advertised capabilities match the client's supported subset.
/// Since the server's current implementation (ComposedServer) returns a static mock/stub
/// advertising hoverProvider/completionProvider unconditionally, this test will fail on the static mock.
#[tokio::test]
async fn test_r2_bypass_fails_on_static_mock() {
    use tower_lsp_max::lsp_types as lsp;
    use tower_lsp_max::{LanguageServer, LspService};

    // Construct a client initialize request that explicitly disables hoverProvider support
    let init_params = lsp::InitializeParams {
        capabilities: lsp::ClientCapabilities {
            text_document: Some(lsp::TextDocumentClientCapabilities {
                hover: Some(lsp::HoverClientCapabilities {
                    dynamic_registration: Some(false),
                    content_format: None,
                }),
                ..Default::default()
            }),
            ..Default::default()
        },
        ..Default::default()
    };

    // Instantiate ComposedServer with no upstreams (to test intersection with empty set)
    let (service, _) = LspService::new(|client| {
        tower_lsp_max::ComposedServer::new(client, vec![] as Vec<(String, String)>)
    });

    // Directly call the initialize handler to retrieve negotiated capabilities
    let server = service.inner();
    let result = server.initialize(init_params).await.expect("Initialize failed");
    
    let caps = result.capabilities;

    // A correct capability computer must perform an intersection and NOT advertise
    // support if the client or upstreams do not support it, or if it is gated.
    // If the server returns a static mock/stub that always advertises hover_provider: Some(Simple(true)),
    // this assert will fail, proving the bypass/fake-risk implementation is active.
    
    // We expect hover_provider to be None or Some(Simple(false)) because it's a mock check.
    match caps.hover_provider {
        Some(lsp::HoverProviderCapability::Simple(true)) => {
            panic!(
                "FAIL: Static mock/stub detected! hover_provider is unconditionally set to true \
                 regardless of client support. The capability computer must dynamically negotiate \
                 capabilities rather than returning a static mock."
            );
        }
        _ => {}
    }
}
