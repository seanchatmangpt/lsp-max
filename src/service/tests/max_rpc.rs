use async_trait::async_trait;
use lsp_types_max::*;
use serde_json::json;
use tower::ServiceExt;

use super::super::*;
use super::*;
use crate::jsonrpc::Result;

#[derive(Debug)]
struct MockLsp318;

#[async_trait]
impl LanguageServer for MockLsp318 {
    async fn initialize(&self, _: InitializeParams) -> Result<InitializeResult> {
        Ok(InitializeResult::default())
    }

    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }

    async fn inline_completion(
        &self,
        _params: lsp_types_max::InlineCompletionParams,
    ) -> Result<Option<lsp_types_max::InlineCompletionResponse>> {
        Ok(Some(lsp_types_max::InlineCompletionResponse::List(
            lsp_types_max::InlineCompletionList {
                items: vec![lsp_types_max::InlineCompletionItem {
                    insert_text: lsp_types_max::StringOrStringValue::String("hello_world".to_string()),
                    filter_text: None,
                    range: None,
                    command: None,
                    insert_text_format: None,
                }],
            },
        )))
    }

    async fn text_document_content(
        &self,
        _params: crate::max_protocol::lsp_3_18::TextDocumentContentParams,
    ) -> Result<crate::max_protocol::lsp_3_18::TextDocumentContentResult> {
        Ok(crate::max_protocol::lsp_3_18::TextDocumentContentResult {
            text: "test content".to_string(),
        })
    }
}

#[tokio::test(flavor = "current_thread")]
async fn test_max_rpc_endpoints() {
    // Remove any stale receipt files from previous failed runs
    let _ = std::fs::remove_file("admission.receipt");
    let _ = std::fs::remove_file("security.receipt");
    let _ = std::fs::remove_file("auth.receipt");
    let _ = std::fs::remove_file("debug.log");

    let (mut service, _) = LspService::new(|_| Mock);

    // 1. Initialize
    let initialize = initialize_request(1);
    let response = service.ready().await.unwrap().call(initialize).await;
    assert!(response.is_ok());

    // 2. Call max/snapshot
    let req = Request::build("max/snapshot").id(2).finish();
    let response = service
        .ready()
        .await
        .unwrap()
        .call(req)
        .await
        .unwrap()
        .unwrap();
    let (_, res) = response.into_parts();
    let snapshot_id: max_protocol::SnapshotId = serde_json::from_value(res.unwrap()).unwrap();
    assert!(snapshot_id.0.starts_with("snap-"));

    // 3. After initialize, gate-state-check is active (initialized != Uninitialized),
    //    so diag-uninitialized-admission must NOT exist.
    let req = Request::build("max/explainDiagnostic")
        .params("diag-uninitialized-admission".to_string())
        .id(3)
        .finish();
    let response = service
        .ready()
        .await
        .unwrap()
        .call(req)
        .await
        .unwrap()
        .unwrap();
    let (_, res) = response.into_parts();
    let err = res.unwrap_err();
    assert_eq!(
        err.message,
        "Diagnostic 'diag-uninitialized-admission' not found"
    );

    // 3b. Verify conformanceVector has no refused axes (gate-state-check now passes).
    let req = Request::build("max/conformanceVector").id(4).finish();
    let response = service
        .ready()
        .await
        .unwrap()
        .call(req)
        .await
        .unwrap()
        .unwrap();
    let (_, res) = response.into_parts();
    let cv: max_protocol::ConformanceVector = serde_json::from_value(res.unwrap()).unwrap();
    assert!(
        cv.refused.is_empty(),
        "Expected no refused axes after initialize, got: {:?}",
        cv.refused
    );
    // Note: under strict mode, unknown axes block release, so admits_release() is false.
    // We only verify that there are no active refused axes.
    // 6. Test Law 3: Receipt Integrity (Missing Validation Receipt)
    // Get the repair plan for diag-missing-receipt
    let req = Request::build("max/repairPlan")
        .params("diag-missing-receipt".to_string())
        .id(7)
        .finish();
    let response = service
        .ready()
        .await
        .unwrap()
        .call(req)
        .await
        .unwrap()
        .unwrap();
    let (_, res) = response.into_parts();
    let plans2: Vec<max_protocol::MaxCodeAction> = serde_json::from_value(res.unwrap()).unwrap();
    let action_with_dep = plans2[0].clone();

    // Attempting to apply it fails because expected_receipts has "rcpt-security-auth" which is not registered yet
    let req = Request::build("max/applyRepairTransaction")
        .params(serde_json::to_value(action_with_dep.clone()).unwrap())
        .id(8)
        .finish();
    let response = service
        .ready()
        .await
        .unwrap()
        .call(req)
        .await
        .unwrap()
        .unwrap();
    let (_, res) = response.into_parts();
    let err = res.unwrap_err();
    assert!(err.message.contains("Receipt integrity violation"));

    // Retrieve generator action to get security auth receipt
    let req = Request::build("max/repairPlan")
        .params("diag-auth-generator".to_string())
        .id(9)
        .finish();
    let response = service
        .ready()
        .await
        .unwrap()
        .call(req)
        .await
        .unwrap()
        .unwrap();
    let (_, res) = response.into_parts();
    let plans3: Vec<max_protocol::MaxCodeAction> = serde_json::from_value(res.unwrap()).unwrap();
    let gen_action = plans3[0].clone();

    // Apply generator action to obtain "rcpt-security-auth"
    let req = Request::build("max/applyRepairTransaction")
        .params(serde_json::to_value(gen_action).unwrap())
        .id(10)
        .finish();
    let response = service
        .ready()
        .await
        .unwrap()
        .call(req)
        .await
        .unwrap()
        .unwrap();
    let (_, res) = response.into_parts();
    let gen_receipt: max_protocol::Receipt = serde_json::from_value(res.unwrap()).unwrap();
    assert_eq!(gen_receipt.receipt_id, "rcpt-security-auth");

    // Now apply action_with_dep again - it should succeed!
    let req = Request::build("max/applyRepairTransaction")
        .params(serde_json::to_value(action_with_dep).unwrap())
        .id(11)
        .finish();
    let response = service
        .ready()
        .await
        .unwrap()
        .call(req)
        .await
        .unwrap()
        .unwrap();
    let (_, res) = response.into_parts();
    let final_receipt: max_protocol::Receipt = serde_json::from_value(res.unwrap()).unwrap();
    assert!(final_receipt.receipt_id.starts_with("rcpt-"));

    // 7. Verify we can lookup receipt
    let req = Request::build("max/receipt")
        .params("rcpt-security-auth".to_string())
        .id(12)
        .finish();
    let response = service
        .ready()
        .await
        .unwrap()
        .call(req)
        .await
        .unwrap()
        .unwrap();
    let (_, res) = response.into_parts();
    let retrieved: max_protocol::Receipt = serde_json::from_value(res.unwrap()).unwrap();
    assert_eq!(retrieved.hash, gen_receipt.hash);

    // 8. Test max/runGate
    let req = Request::build("max/runGate")
        .params(serde_json::to_value(max_protocol::GateId("some-gate".to_string())).unwrap())
        .id(13)
        .finish();
    let response = service
        .ready()
        .await
        .unwrap()
        .call(req)
        .await
        .unwrap()
        .unwrap();
    let (_, res) = response.into_parts();
    let gate_result: bool = serde_json::from_value(res.unwrap()).unwrap();
    assert!(gate_result);

    // 9. Export Analysis Bundle for the snapshot
    let req = Request::build("max/exportAnalysisBundle")
        .params(serde_json::to_value(snapshot_id.clone()).unwrap())
        .id(14)
        .finish();
    let response = service
        .ready()
        .await
        .unwrap()
        .call(req)
        .await
        .unwrap()
        .unwrap();
    let (_, res) = response.into_parts();
    let bundle: max_protocol::AnalysisBundle = serde_json::from_value(res.unwrap()).unwrap();
    assert_eq!(bundle.snapshot_id.0, snapshot_id.0);
    assert!(!bundle.diagnostics.is_empty());

    // Cleanup created receipt files from disk
    let _ = std::fs::remove_file("admission.receipt");
    let _ = std::fs::remove_file("security.receipt");
    let _ = std::fs::remove_file("auth.receipt");
}

#[tokio::test(flavor = "current_thread")]
async fn test_lsp_3_18_methods() {
    let (mut service, _) = LspService::new(|_| Mock);

    let initialize = initialize_request(1);
    let response = service.ready().await.unwrap().call(initialize).await;
    assert!(response.is_ok());

    // 1. textDocument/inlineCompletion
    let req = Request::build("textDocument/inlineCompletion")
        .params(json!({
            "textDocument": { "uri": "file:///foo.rs" },
            "position": { "line": 0, "character": 0 },
            "context": { "triggerKind": 1 }
        }))
        .id(2)
        .finish();
    let response = service
        .ready()
        .await
        .unwrap()
        .call(req)
        .await
        .unwrap()
        .unwrap();
    let (_, res) = response.into_parts();
    assert_eq!(res.unwrap_err().code, ErrorCode::MethodNotFound);

    // 2. workspace/textDocumentContent
    let req = Request::build("workspace/textDocumentContent")
        .params(json!({
            "uri": "file:///foo.rs"
        }))
        .id(3)
        .finish();
    let response = service
        .ready()
        .await
        .unwrap()
        .call(req)
        .await
        .unwrap()
        .unwrap();
    let (_, res) = response.into_parts();
    assert_eq!(res.unwrap_err().code, ErrorCode::MethodNotFound);

    // 3. workspace/textDocumentContent/refresh
    let req = Request::build("workspace/textDocumentContent/refresh")
        .params(json!({
            "uri": "file:///foo.rs"
        }))
        .id(4)
        .finish();
    let response = service
        .ready()
        .await
        .unwrap()
        .call(req)
        .await
        .unwrap()
        .unwrap();
    let (_, res) = response.into_parts();
    assert_eq!(res.unwrap_err().code, ErrorCode::MethodNotFound);
}

#[tokio::test(flavor = "current_thread")]
async fn test_lsp_3_18_methods_routing() {
    let (mut service, _) = LspService::new(|_| MockLsp318);

    let initialize = initialize_request(1);
    let response = service.ready().await.unwrap().call(initialize).await;
    assert!(response.is_ok());

    // 1. textDocument/inlineCompletion
    let req = Request::build("textDocument/inlineCompletion")
        .params(json!({
            "textDocument": { "uri": "file:///foo.rs" },
            "position": { "line": 0, "character": 0 },
            "context": { "triggerKind": 1 }
        }))
        .id(2)
        .finish();
    let response = service
        .ready()
        .await
        .unwrap()
        .call(req)
        .await
        .unwrap()
        .unwrap();
    let (_, res) = response.into_parts();
    let val = res.unwrap();
    assert_eq!(
        val,
        json!({
            "items": [
                {
                    "insertText": "hello_world",
                }
            ]
        })
    );

    // 2. workspace/textDocumentContent
    let req = Request::build("workspace/textDocumentContent")
        .params(json!({
            "uri": "file:///foo.rs"
        }))
        .id(3)
        .finish();
    let response = service
        .ready()
        .await
        .unwrap()
        .call(req)
        .await
        .unwrap()
        .unwrap();
    let (_, res) = response.into_parts();
    let val = res.unwrap();
    assert_eq!(
        val,
        json!({
            "text": "test content"
        })
    );

    // 3. workspace/textDocumentContent/refresh
    let req = Request::build("workspace/textDocumentContent/refresh")
        .params(json!({
            "uri": "file:///foo.rs"
        }))
        .id(4)
        .finish();
    let response = service
        .ready()
        .await
        .unwrap()
        .call(req)
        .await
        .unwrap()
        .unwrap();
    let (_, res) = response.into_parts();
    assert_eq!(res.unwrap_err().code, ErrorCode::MethodNotFound);
}
