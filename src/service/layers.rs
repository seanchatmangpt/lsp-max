//! Assorted middleware that implements LSP server semantics.

use std::marker::PhantomData;
use std::sync::Arc;
use std::task::{Context, Poll};

use dashmap::DashMap;
use futures::future::{self, BoxFuture, FutureExt};
use futures::lock::Mutex;
use tower::{Layer, Service, ServiceExt};
use tracing::{info, warn};
use url::Url;

use super::ExitedError;
use crate::jsonrpc::{not_initialized_error, Error, ErrorCode, Id, Request, Response};

use super::client::Client;
use super::pending::Pending;
use super::state::{ServerState, State};

/// Middleware which implements `initialize` request semantics.
///
/// # Specification
///
/// https://microsoft.github.io/language-server-protocol/specification#initialize
pub struct Initialize {
    state: Arc<ServerState>,
    pending: Arc<Pending>,
}

impl Initialize {
    pub fn new(state: Arc<ServerState>, pending: Arc<Pending>) -> Self {
        Initialize { state, pending }
    }
}

impl<S> Layer<S> for Initialize {
    type Service = InitializeService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        InitializeService {
            inner: Cancellable::new(inner, self.pending.clone()),
            state: self.state.clone(),
        }
    }
}

/// Service created from [`Initialize`] layer.
pub struct InitializeService<S> {
    inner: Cancellable<S>,
    state: Arc<ServerState>,
}

impl<S> Service<Request> for InitializeService<S>
where
    S: Service<Request, Response = Option<Response>, Error = ExitedError>,
    S::Future: Send + 'static,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = BoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: Request) -> Self::Future {
        let params = req.params().cloned().unwrap_or(serde_json::Value::Null);
        let client_caps_val = params
            .get("capabilities")
            .cloned()
            .unwrap_or(serde_json::Value::Null);
        let client_caps: Option<lsp_types::ClientCapabilities> =
            serde_json::from_value(client_caps_val).ok();

        if let Some(pid) = params.get("processId").and_then(|v| v.as_u64()) {
            self.state.set_parent_pid(pid as u32);
            crate::service::watchdog::spawn_watchdog(self.state.clone());
        }

        if self.state.try_initialize(params) {
            let state = self.state.clone();
            let fut = self.inner.call(req);

            Box::pin(async move {
                let response = fut.await?;

                match &response {
                    Some(res) if res.is_ok() => {
                        let server_caps = res.result().cloned().unwrap_or(serde_json::Value::Null);
                        let server_caps_val = server_caps
                            .get("capabilities")
                            .cloned()
                            .unwrap_or(serde_json::Value::Null);
                        let server_caps_parsed: Option<lsp_types::ServerCapabilities> =
                            serde_json::from_value(server_caps_val).ok();

                        {
                            let mut registry = crate::get_registry().lock().unwrap();
                            registry.client_capabilities = client_caps;
                            registry.server_capabilities = server_caps_parsed;
                        }

                        state.transition_to_initialized(server_caps);
                    }
                    _ => {
                        state.transition_to_uninitialized();
                    }
                }

                Ok(response)
            })
        } else {
            warn!("received duplicate `initialize` request, ignoring");
            let (_, id, _) = req.into_parts();
            future::ok(id.map(|id| Response::from_error(id, Error::invalid_request()))).boxed()
        }
    }
}

/// Middleware which implements `shutdown` request semantics.
///
/// # Specification
///
/// https://microsoft.github.io/language-server-protocol/specification#shutdown
pub struct Shutdown {
    state: Arc<ServerState>,
    pending: Arc<Pending>,
}

impl Shutdown {
    pub fn new(state: Arc<ServerState>, pending: Arc<Pending>) -> Self {
        Shutdown { state, pending }
    }
}

impl<S> Layer<S> for Shutdown {
    type Service = ShutdownService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        ShutdownService {
            inner: Cancellable::new(inner, self.pending.clone()),
            state: self.state.clone(),
        }
    }
}

/// Service created from [`Shutdown`] layer.
pub struct ShutdownService<S> {
    inner: Cancellable<S>,
    state: Arc<ServerState>,
}

impl<S> Service<Request> for ShutdownService<S>
where
    S: Service<Request, Response = Option<Response>, Error = ExitedError>,
    S::Future: Into<BoxFuture<'static, Result<Option<Response>, S::Error>>> + Send + 'static,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = BoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: Request) -> Self::Future {
        if self.state.transition_to_shutdown() {
            info!("shutdown request received, shutting down");
            self.inner.call(req)
        } else {
            let cur_state = self.state.get();
            let (_, id, _) = req.into_parts();
            future::ok(not_initialized_response(id, cur_state)).boxed()
        }
    }
}

/// Middleware which implements `exit` notification semantics.
///
/// # Specification
///
/// https://microsoft.github.io/language-server-protocol/specification#exit
pub struct Exit {
    state: Arc<ServerState>,
    pending: Arc<Pending>,
    client: Client,
}

impl Exit {
    pub fn new(state: Arc<ServerState>, pending: Arc<Pending>, client: Client) -> Self {
        Exit {
            state,
            pending,
            client,
        }
    }
}

impl<S> Layer<S> for Exit {
    type Service = ExitService<S>;

    fn layer(&self, _: S) -> Self::Service {
        ExitService {
            state: self.state.clone(),
            pending: self.pending.clone(),
            client: self.client.clone(),
            _marker: PhantomData,
        }
    }
}

/// Service created from [`Exit`] layer.
pub struct ExitService<S> {
    state: Arc<ServerState>,
    pending: Arc<Pending>,
    client: Client,
    _marker: PhantomData<S>,
}

impl<S> Service<Request> for ExitService<S> {
    type Response = Option<Response>;
    type Error = ExitedError;
    type Future = future::Ready<Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, _: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        if self.state.get() == State::Exited {
            Poll::Ready(Err(ExitedError(self.state.get_exit_code())))
        } else {
            Poll::Ready(Ok(()))
        }
    }

    fn call(&mut self, _: Request) -> Self::Future {
        info!("exit notification received, stopping");
        self.state.transition_to_exited();
        self.pending.cancel_all();
        self.client.close();
        future::ok(None)
    }
}

/// Middleware which implements LSP semantics for all other kinds of requests.
pub struct Normal {
    state: Arc<ServerState>,
    pending: Arc<Pending>,
}

impl Normal {
    pub fn new(state: Arc<ServerState>, pending: Arc<Pending>) -> Self {
        Normal { state, pending }
    }
}

impl<S> Layer<S> for Normal {
    type Service = NormalService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        NormalService {
            inner: Cancellable::new(inner, self.pending.clone()),
            state: self.state.clone(),
        }
    }
}

/// Service created from [`Normal`] layer.
pub struct NormalService<S> {
    inner: Cancellable<S>,
    state: Arc<ServerState>,
}

impl<S> Service<Request> for NormalService<S>
where
    S: Service<Request, Response = Option<Response>, Error = ExitedError>,
    S::Future: Into<BoxFuture<'static, Result<Option<Response>, S::Error>>> + Send + 'static,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = BoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: Request) -> Self::Future {
        match self.state.get() {
            State::Initialized => self.inner.call(req),
            cur_state => {
                println!(
                    "NormalService: rejected request/notification '{}' because state is {:?}",
                    req.method(),
                    cur_state
                );
                let (_, id, _) = req.into_parts();
                future::ok(not_initialized_response(id, cur_state)).boxed()
            }
        }
    }
}

/// Middleware which implements LSP semantics without checking initialization state.
pub struct Permissive {
    pending: Arc<Pending>,
}

impl Permissive {
    /// Creates a new `Permissive` layer.
    pub fn new(_state: Arc<ServerState>, pending: Arc<Pending>) -> Self {
        Permissive { pending }
    }
}

impl<S> Layer<S> for Permissive {
    type Service = PermissiveService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        PermissiveService {
            inner: Cancellable::new(inner, self.pending.clone()),
        }
    }
}

/// Service created from [`Permissive`] layer.
pub struct PermissiveService<S> {
    inner: Cancellable<S>,
}

impl<S> Service<Request> for PermissiveService<S>
where
    S: Service<Request, Response = Option<Response>, Error = ExitedError>,
    S::Future: Into<BoxFuture<'static, Result<Option<Response>, S::Error>>> + Send + 'static,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = BoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: Request) -> Self::Future {
        self.inner.call(req)
    }
}

/// Middleware which catches panics and returns a JSON-RPC internal error.
#[derive(Debug)]
pub struct CatchUnwind;

impl<S> Layer<S> for CatchUnwind {
    type Service = CatchUnwindService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        CatchUnwindService { inner }
    }
}

/// Service created from [`CatchUnwind`] layer.
pub struct CatchUnwindService<S> {
    inner: S,
}

impl<S> CatchUnwindService<S> {
    /// Returns a reference to the inner service.
    pub fn inner(&self) -> &S {
        &self.inner
    }
}

impl<S> std::fmt::Debug for CatchUnwindService<S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CatchUnwindService")
            .finish_non_exhaustive()
    }
}

impl<S> Service<Request> for CatchUnwindService<S>
where
    S: Service<Request, Response = Option<Response>, Error = ExitedError>,
    S::Future: Send + 'static,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = BoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: Request) -> Self::Future {
        let id = req.id().cloned();
        let fut = self.inner.call(req);

        async move {
            match std::panic::AssertUnwindSafe(fut).catch_unwind().await {
                Ok(res) => res,
                Err(_) => {
                    let response = id.map(|id| {
                        Response::from_error(
                            id,
                            Error {
                                code: ErrorCode::InternalError,
                                message: "panic occurred".into(),
                                data: None,
                            },
                        )
                    });
                    Ok(response)
                }
            }
        }
        .boxed()
    }
}

/// Wraps an inner service `S` and implements `$/cancelRequest` semantics for all requests.
///
/// # Specification
///
/// https://microsoft.github.io/language-server-protocol/specification#cancelRequest
struct Cancellable<S> {
    inner: S,
    pending: Arc<Pending>,
}

impl<S> Cancellable<S> {
    fn new(inner: S, pending: Arc<Pending>) -> Self {
        Cancellable { inner, pending }
    }
}

impl<S> Service<Request> for Cancellable<S>
where
    S: Service<Request, Response = Option<Response>, Error = ExitedError>,
    S::Future: Send + 'static,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = BoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: Request) -> Self::Future {
        match req.id().cloned() {
            Some(id) => self.pending.execute(id, self.inner.call(req)).boxed(),
            None => self.inner.call(req).boxed(),
        }
    }
}

fn not_initialized_response(id: Option<Id>, server_state: State) -> Option<Response> {
    let id = id?;
    let error = match server_state {
        State::Uninitialized | State::Initializing => not_initialized_error(),
        _ => Error::invalid_request(),
    };

    Some(Response::from_error(id, error))
}

/// Middleware which serializes requests/notifications based on their document URI.
#[derive(Clone)]
pub struct DocumentSync {
    locks: Arc<DashMap<Url, Arc<Mutex<()>>>>,
}

impl DocumentSync {
    /// Creates a new `DocumentSync` layer.
    pub fn new() -> Self {
        DocumentSync {
            locks: Arc::new(DashMap::new()),
        }
    }
}

impl Default for DocumentSync {
    fn default() -> Self {
        Self::new()
    }
}

impl<S> Layer<S> for DocumentSync {
    type Service = DocumentSyncService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        DocumentSyncService {
            inner,
            locks: self.locks.clone(),
        }
    }
}

/// Service created from [`DocumentSync`] layer.
#[derive(Clone)]
pub struct DocumentSyncService<S> {
    inner: S,
    locks: Arc<DashMap<Url, Arc<Mutex<()>>>>,
}

impl<S> Service<Request> for DocumentSyncService<S>
where
    S: Service<Request, Response = Option<Response>, Error = ExitedError> + Clone + Send + 'static,
    S::Future: Send + 'static,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = BoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: Request) -> Self::Future {
        let uri = extract_uri(req.params());
        let method = req.method().to_string();
        let params = req.params().cloned();
        let locks = self.locks.clone();
        let mut inner = self.inner.clone();

        Box::pin(async move {
            if let Some(uri) = uri {
                let lock = locks
                    .entry(uri.clone())
                    .or_insert_with(|| Arc::new(Mutex::new(())))
                    .value()
                    .clone();
                let _guard = lock.lock().await;

                // Update version if this is a didOpen or didChange notification
                match method.as_str() {
                    "textDocument/didOpen" => {
                        if let Some(version) = params
                            .as_ref()
                            .and_then(|p| p.get("textDocument"))
                            .and_then(|t| t.get("version"))
                            .and_then(|v| v.as_i64())
                        {
                            if let Ok(mut reg) = crate::get_registry().lock() {
                                reg.document_versions.insert(uri, version as i32);
                            }
                        }
                    }
                    "textDocument/didChange" => {
                        if let Some(version) = params
                            .as_ref()
                            .and_then(|p| p.get("textDocument"))
                            .and_then(|t| t.get("version"))
                            .and_then(|v| v.as_i64())
                        {
                            if let Ok(mut reg) = crate::get_registry().lock() {
                                reg.document_versions.insert(uri, version as i32);
                            }
                        }
                    }
                    _ => {}
                }

                inner.ready().await?.call(req).await
            } else {
                inner.ready().await?.call(req).await
            }
        })
    }
}

fn extract_uri(params: Option<&serde_json::Value>) -> Option<Url> {
    let params = params?;
    if let Some(uri) = params
        .get("textDocument")
        .and_then(|t| t.get("uri"))
        .and_then(|u| u.as_str())
    {
        if let Ok(url) = Url::parse(uri) {
            return Some(url);
        }
    }
    if let Some(uri) = params.get("uri").and_then(|u| u.as_str()) {
        if let Ok(url) = Url::parse(uri) {
            return Some(url);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use tower::ServiceExt;

    #[derive(Clone)]
    struct DummyService;

    impl Service<Request> for DummyService {
        type Response = Option<Response>;
        type Error = ExitedError;
        type Future = BoxFuture<'static, std::result::Result<Self::Response, Self::Error>>;

        fn poll_ready(
            &mut self,
            _: &mut Context<'_>,
        ) -> Poll<std::result::Result<(), Self::Error>> {
            Poll::Ready(Ok(()))
        }

        fn call(&mut self, req: Request) -> Self::Future {
            let (_, id, _) = req.into_parts();
            let res = id.map(|id| Response::from_ok(id, json!({})));
            Box::pin(future::ok(res))
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
        assert_eq!(response.error().unwrap().code, ErrorCode::InternalError);
        assert_eq!(response.error().unwrap().message, "panic occurred");
    }
}
