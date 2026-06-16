use serde_json::json;

#[tokio::test]
#[ignore = "e2e: requires live server, run with --include-ignored"]
async fn test_f4_t3_diagnostics_filtering_contract() {
    let mut mock = crate::mock_server::MockServer::start().await;
    let upstreams = vec![("ggen-lsp".to_string(), mock.addr.to_string())];

    let (client_io, server_io) = tokio::io::duplex(1024 * 1024);
    let (service, socket) =
        lsp_max::LspService::new(|client| lsp_max::ComposedServer::new(client, upstreams.clone()));
    let (reader, writer) = tokio::io::split(server_io);
    tokio::spawn(async move {
        let _ = lsp_max::Server::new(reader, writer, socket)
            .serve(service)
            .await;
    });

    let mut client = crate::test_harness::TestClient::new(client_io);

    let init_params = json!({
        "capabilities": {},
        "rootUri": null,
        "processId": null
    });
    let _ = client.send_request("initialize", init_params).await;
    client.send_notification("initialized", json!({})).await;

    let diag_payload = json!({
        "jsonrpc": "2.0",
        "method": "textDocument/publishDiagnostics",
        "params": {
            "uri": "file:///Users/sac/tower-lsp-composition/main.rs",
            "diagnostics": [
                {
                    "range": {
                        "start": { "line": 0, "character": 0 },
                        "end": { "line": 0, "character": 5 }
                    },
                    "message": "Filtered out because missing source_id in data"
                },
                {
                    "range": {
                        "start": { "line": 1, "character": 0 },
                        "end": { "line": 1, "character": 5 }
                    },
                    "message": "Accepted because has source_id in data",
                    "data": {
                        "source_id": "ggen_lsp_observer"
                    }
                }
            ]
        }
    });

    {
        let mut s = mock.state.lock().await;
        s.diagnostics_to_send.push(diag_payload);
    }

    let msg = tokio::time::timeout(std::time::Duration::from_millis(500), client.read_message())
        .await
        .expect("Timeout waiting for diagnostics forwarding")
        .expect("Connection closed");

    let params = msg.get("params").unwrap();
    let diags = params.get("diagnostics").unwrap().as_array().unwrap();

    assert_eq!(diags.len(), 1);
    assert_eq!(
        diags[0].get("message").and_then(|m| m.as_str()),
        Some("Accepted because has source_id in data")
    );

    mock.shutdown();
}
