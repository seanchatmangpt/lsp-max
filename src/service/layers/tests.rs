use super::*;
use crate::jsonrpc::{Request, Response};
use crate::service::{ExitedError, Pending, ServerState, State};
use futures::future::BoxFuture;
use serde_json::json;
use std::sync::Arc;
use std::task::{Context, Poll};
use tower::ServiceExt;
use tower::{Layer, Service};

#[derive(Clone)]
struct DummyService;

impl Service<Request> for DummyService {
    type Response = Option<Response>;
    type Error = ExitedError;
    type Future = BoxFuture<'static, std::result::Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, _: &mut Context<'_>) -> Poll<std::result::Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: Request) -> Self::Future {
        let (_, id, _) = req.into_parts();
        let res = id.map(|id| Response::from_ok(id, json!({})));
        Box::pin(futures::future::ok(res))
    }
}

#[tokio::test]
async fn test_initialize_layer() {
    let state = Arc::new(ServerState::new());
    let pending = Arc::new(Pending::new());
    let layer = Initialize::new(state.clone(), pending.clone());
    let mut service = layer.layer(DummyService);

    let req = Request::build("initialize")
        .params(json!({"capabilities": {}}))
        .id(1)
        .finish();

    assert_eq!(state.get(), State::Uninitialized);
    let res = service.ready().await.unwrap().call(req).await.unwrap();
    assert!(res.is_some());
    assert_eq!(state.get(), State::Initialized);
}

#[tokio::test]
async fn test_shutdown_layer() {
    let state = Arc::new(ServerState::new());
    state.try_initialize(json!({"capabilities": {}}));
    state.transition_to_initialized(json!({}));
    let pending = Arc::new(Pending::new());
    let layer = Shutdown::new(state.clone(), pending.clone());
    let mut service = layer.layer(DummyService);

    let req = Request::build("shutdown").id(1).finish();
    assert_eq!(state.get(), State::Initialized);
    let res = service.ready().await.unwrap().call(req).await.unwrap();
    assert!(res.is_some());
    assert_eq!(state.get(), State::ShutDown);
}

#[tokio::test]
async fn test_normal_layer() {
    let state = Arc::new(ServerState::new());
    let pending = Arc::new(Pending::new());
    let layer = Normal::new(state.clone(), pending.clone());
    let mut service = layer.layer(DummyService);

    let req = Request::build("textDocument/hover").id(1).finish();
    // Should refuse since state is not Initialized
    let res = service
        .ready()
        .await
        .unwrap()
        .call(req.clone())
        .await
        .unwrap();
    assert!(res.unwrap().error().is_some());

    // Now initialize
    state.try_initialize(json!({"capabilities": {}}));
    state.transition_to_initialized(json!({}));

    let res = service.ready().await.unwrap().call(req).await.unwrap();
    assert!(res.unwrap().result().is_some());
}

#[tokio::test]
async fn test_document_sync_layer() {
    let layer = DocumentSync::new();
    let mut service = layer.layer(DummyService);

    let req1 = Request::build("textDocument/didChange")
        .params(json!({"textDocument": {"uri": "file:///foo.rs"}}))
        .finish();
    let req2 = Request::build("textDocument/didSave")
        .params(json!({"textDocument": {"uri": "file:///foo.rs"}}))
        .finish();

    let res1 = service.ready().await.unwrap().call(req1).await;
    let res2 = service.ready().await.unwrap().call(req2).await;

    assert!(res1.is_ok());
    assert!(res2.is_ok());
}

#[tokio::test]
async fn test_catch_unwind_layer() {
    struct PanickingService;

    impl Service<Request> for PanickingService {
        type Response = Option<Response>;
        type Error = ExitedError;
        type Future = BoxFuture<'static, std::result::Result<Self::Response, Self::Error>>;

        fn poll_ready(
            &mut self,
            _: &mut Context<'_>,
        ) -> Poll<std::result::Result<(), Self::Error>> {
            Poll::Ready(Ok(()))
        }

        fn call(&mut self, _: Request) -> Self::Future {
            Box::pin(async move {
                panic!("inner service panic");
            })
        }
    }

    let layer = CatchUnwind;
    let mut service = layer.layer(PanickingService);

    let req = Request::build("test").id(1).finish();
    let res = service.ready().await.unwrap().call(req).await.unwrap();

    assert!(res.is_some());
    let response = res.unwrap();
    assert!(response.error().is_some());
    assert_eq!(
        response.error().unwrap().code,
        crate::jsonrpc::ErrorCode::InternalError
    );
    assert_eq!(response.error().unwrap().message, "panic occurred");
}
