use async_trait::async_trait;
use lsp_types_max::*;
use serde_json::json;
use tower::ServiceExt;

use super::*;
use crate::jsonrpc::Result;

mod max_rpc;

#[derive(Debug)]
struct Mock;

#[async_trait]
impl LanguageServer for Mock {
    async fn initialize(&self, _: InitializeParams) -> Result<InitializeResult> {
        Ok(InitializeResult::default())
    }

    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }

    // This handler should never resolve...
    async fn code_action_resolve(&self, _: CodeAction) -> Result<CodeAction> {
        futures::future::pending().await
    }
}

impl Mock {
    async fn custom_request(&self, params: i32) -> Result<i32> {
        Ok(params)
    }
}

fn initialize_request(id: i64) -> Request {
    Request::build("initialize")
        .params(json!({"capabilities":{}}))
        .id(id)
        .finish()
}

#[tokio::test(flavor = "current_thread")]
async fn initializes_only_once() {
    let (mut service, _) = LspService::new(|_| Mock);

    let request = initialize_request(1);

    let response = service.ready().await.unwrap().call(request.clone()).await;
    let ok = Response::from_ok(1.into(), json!({"capabilities":{}}));
    assert_eq!(response, Ok(Some(ok)));

    let response = service.ready().await.unwrap().call(request).await;
    let err = Response::from_error(1.into(), Error::invalid_request());
    assert_eq!(response, Ok(Some(err)));
}

#[tokio::test(flavor = "current_thread")]
async fn refuses_requests_after_shutdown() {
    let (mut service, _) = LspService::new(|_| Mock);

    let initialize = initialize_request(1);
    let response = service.ready().await.unwrap().call(initialize).await;
    let ok = Response::from_ok(1.into(), json!({"capabilities":{}}));
    assert_eq!(response, Ok(Some(ok)));

    let shutdown = Request::build("shutdown").id(1).finish();
    let response = service.ready().await.unwrap().call(shutdown.clone()).await;
    let ok = Response::from_ok(1.into(), json!(null));
    assert_eq!(response, Ok(Some(ok)));

    let response = service.ready().await.unwrap().call(shutdown).await;
    let err = Response::from_error(1.into(), Error::invalid_request());
    assert_eq!(response, Ok(Some(err)));
}

#[tokio::test(flavor = "current_thread")]
async fn exit_notification() {
    let (mut service, _) = LspService::new(|_| Mock);

    let exit = Request::build("exit").finish();
    let response = service.ready().await.unwrap().call(exit.clone()).await;
    assert_eq!(response, Ok(None));

    let ready = futures::future::poll_fn(|cx| service.poll_ready(cx)).await;
    assert_eq!(ready, Err(ExitedError(1)));
    assert_eq!(service.call(exit).await, Err(ExitedError(1)));
}

#[tokio::test(flavor = "current_thread")]
async fn exit_notification_after_shutdown() {
    let (mut service, _) = LspService::new(|_| Mock);

    let initialize = initialize_request(1);
    let response = service.ready().await.unwrap().call(initialize).await;
    assert!(response.is_ok());

    let shutdown = Request::build("shutdown").id(1).finish();
    let response = service.ready().await.unwrap().call(shutdown).await;
    assert!(response.is_ok());

    let exit = Request::build("exit").finish();
    let response = service.ready().await.unwrap().call(exit).await;
    assert_eq!(response, Ok(None));

    let ready = futures::future::poll_fn(|cx| service.poll_ready(cx)).await;
    assert_eq!(ready, Err(ExitedError(0)));
}

#[tokio::test(flavor = "current_thread")]
async fn cancels_pending_requests() {
    let (mut service, _) = LspService::new(|_| Mock);

    let initialize = initialize_request(1);
    let response = service.ready().await.unwrap().call(initialize).await;
    let ok = Response::from_ok(1.into(), json!({"capabilities":{}}));
    assert_eq!(response, Ok(Some(ok)));

    let pending_request = Request::build("codeAction/resolve")
        .params(json!({"title":""}))
        .id(1)
        .finish();

    let cancel_request = Request::build("$/cancelRequest")
        .params(json!({"id":1i32}))
        .finish();

    let pending_fut = service.ready().await.unwrap().call(pending_request);
    let cancel_fut = service.ready().await.unwrap().call(cancel_request);
    let (pending_response, cancel_response) = futures::join!(pending_fut, cancel_fut);

    let canceled = Response::from_error(1.into(), Error::request_cancelled());
    assert_eq!(pending_response, Ok(Some(canceled)));
    assert_eq!(cancel_response, Ok(None));
}

#[tokio::test(flavor = "current_thread")]
async fn serves_custom_requests() {
    let (mut service, _) = LspService::build(|_| Mock)
        .custom_method("custom", Mock::custom_request)
        .finish();

    let initialize = initialize_request(1);
    let response = service.ready().await.unwrap().call(initialize).await;
    let ok = Response::from_ok(1.into(), json!({"capabilities":{}}));
    assert_eq!(response, Ok(Some(ok)));

    let custom = Request::build("custom").params(123i32).id(1).finish();
    let response = service.ready().await.unwrap().call(custom).await;
    let ok = Response::from_ok(1.into(), json!(123i32));
    assert_eq!(response, Ok(Some(ok)));
}

#[tokio::test(flavor = "current_thread")]
async fn get_inner() {
    let (service, _) = LspService::build(|_| Mock).finish();

    service
        .inner()
        .initialize(InitializeParams::default())
        .await
        .unwrap();
}
