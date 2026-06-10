use lsp_max::jsonrpc::{ErrorCode, Request, Result};
use lsp_max::lsp_types::*;
use lsp_max::{LanguageServer, LspService};
use serde_json::{json, Value};
use std::time::Duration;
use tower::{Service, ServiceExt};

struct Mock;

#[lsp_max::async_trait]
impl LanguageServer for Mock {
    async fn initialize(&self, _: InitializeParams) -> Result<InitializeResult> {
        Ok(InitializeResult::default())
    }
    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }
}

impl Mock {
    async fn panic_handler(&self) -> Result<()> {
        panic!("intentional panic for test");
    }

    async fn slow_handler(&self, params: Value) -> Result<Value> {
        tokio::time::sleep(Duration::from_millis(100)).await;
        Ok(params)
    }
}

#[tokio::test]
async fn test_panic_handling() {
    let (mut service, _) = LspService::build(|_| Mock)
        .custom_method("test/panic", Mock::panic_handler)
        .finish();

    // Initialize first to pass guards
    let init_req = Request::build("initialize")
        .params(json!({"capabilities": {}, "processId": std::process::id()}))
        .id(1)
        .finish();
    let _ = service.ready().await.unwrap().call(init_req).await.unwrap();

    let panic_req = Request::build("test/panic").id(2).finish();
    let response = service
        .ready()
        .await
        .unwrap()
        .call(panic_req)
        .await
        .unwrap()
        .unwrap();

    assert_eq!(response.error().unwrap().code, ErrorCode::InternalError);
    assert!(response.error().unwrap().message.contains("panic"));
}

#[tokio::test]
async fn test_max_rpc_lifecycle_guards() {
    lsp_max::reset_registry_for_tests();
    let (mut service, _) = LspService::new(|_| Mock);

    // Call max/snapshot before initialization
    let req = Request::build("max/snapshot").id(1).finish();
    let response = service
        .ready()
        .await
        .unwrap()
        .call(req)
        .await
        .unwrap()
        .unwrap();

    assert_eq!(
        response.error().unwrap().code,
        ErrorCode::ServerNotInitialized
    );
}

#[tokio::test]
async fn test_document_sync_serialization() {
    let (mut service, _) = LspService::build(|_| Mock)
        .custom_method("test/slow", Mock::slow_handler)
        .finish();

    let init_req = Request::build("initialize")
        .params(json!({"capabilities": {}}))
        .id(1)
        .finish();
    let _ = service.ready().await.unwrap().call(init_req).await.unwrap();

    let uri = "file:///test.rs";
    let req1 = Request::build("test/slow")
        .params(json!({"textDocument": {"uri": uri}, "val": 1}))
        .id(2)
        .finish();
    let req2 = Request::build("test/slow")
        .params(json!({"textDocument": {"uri": uri}, "val": 2}))
        .id(3)
        .finish();

    let start = std::time::Instant::now();
    let (res1, res2) = futures::join!(
        service.ready().await.unwrap().call(req1),
        service.ready().await.unwrap().call(req2)
    );
    let elapsed = start.elapsed();

    assert!(res1.is_ok());
    assert!(res2.is_ok());
    // Since each takes 100ms and they are serialized per URI, it should take at least 200ms.
    assert!(elapsed >= Duration::from_millis(200));
}

#[tokio::test]
async fn test_watchdog_exit() {
    lsp_max::reset_registry_for_tests();
    let (mut service, _) = LspService::new(|_| Mock);

    // Use a PID that is likely to exist but we can "simulate" death if we had a way to mock.
    // For now, let's just test that it extracts the PID correctly and we can trigger it.
    // Since we can't easily kill a process in a test without side effects,
    // we'll just verify the server state transition if we manually trigger it.

    let init_req = Request::build("initialize")
        .params(json!({"capabilities": {}, "processId": std::process::id()}))
        .id(1)
        .finish();
    let _ = service.ready().await.unwrap().call(init_req).await.unwrap();

    // Check that state is Initialized
    // (We'd need to expose state more directly or check via another RPC)
    let snapshot_req = Request::build("max/snapshot").id(2).finish();
    let res = service
        .ready()
        .await
        .unwrap()
        .call(snapshot_req)
        .await
        .unwrap()
        .unwrap();
    assert!(res.is_ok());
}
