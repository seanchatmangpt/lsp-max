use crate::test_harness::TestHarness;
use serde_json::json;

/// 1. Composed Initialize: client sends initialize, upstream A advertises hover, upstream B does not;
///    verify downstream capabilities.
#[tokio::test]
#[ignore = "e2e: requires live server, run with --include-ignored"]
async fn test_gate3_composed_initialize() {
    let mut harness = TestHarness::new(2).await;

    // Upstream A advertises hoverProvider: true
    {
        let mut s0 = harness.mock_servers[0].state.lock().await;
        s0.custom_capabilities = json!({
            "hoverProvider": true
        });
    }

    // Upstream B does not advertise hoverProvider
    {
        let mut s1 = harness.mock_servers[1].state.lock().await;
        s1.custom_capabilities = json!({
            "hoverProvider": false
        });
    }

    let init_params = json!({
        "capabilities": {
            "textDocument": {
                "hover": {
                    "contentFormat": ["markdown", "plaintext"]
                }
            }
        },
        "rootUri": "file:///Users/sac/tower-lsp-composition",
        "processId": 1234
    });

    let resp = harness.client.send_request("initialize", init_params).await;
    let result = resp.get("result").expect("Initialize must return a result");
    let caps = result
        .get("capabilities")
        .expect("Result must contain capabilities");

    // Since Upstream A advertises hover, the composed server must advertise hover downstream
    assert_eq!(
        caps.get("hoverProvider").and_then(|h| h.as_bool()),
        Some(true),
        "Composed server must inherit hoverProvider capability from Upstream A"
    );

    harness.shutdown();

    // Now test when NEITHER upstream supports hover
    let mut harness2 = TestHarness::new(2).await;
    {
        let mut s0 = harness2.mock_servers[0].state.lock().await;
        s0.custom_capabilities = json!({ "hoverProvider": false });
    }
    {
        let mut s1 = harness2.mock_servers[1].state.lock().await;
        s1.custom_capabilities = json!({ "hoverProvider": false });
    }

    let init_params2 = json!({
        "capabilities": {
            "textDocument": {
                "hover": {
                    "contentFormat": ["markdown", "plaintext"]
                }
            }
        },
        "rootUri": "file:///Users/sac/tower-lsp-composition",
        "processId": 1234
    });

    let resp2 = harness2
        .client
        .send_request("initialize", init_params2)
        .await;
    let result2 = resp2
        .get("result")
        .expect("Initialize must return a result");
    let caps2 = result2
        .get("capabilities")
        .expect("Result must contain capabilities");

    let hover_provider = caps2.get("hoverProvider");
    assert!(
        hover_provider.is_none() || hover_provider == Some(&json!(false)),
        "Composed server must not advertise hoverProvider if no upstreams support it"
    );

    harness2.shutdown();
}
