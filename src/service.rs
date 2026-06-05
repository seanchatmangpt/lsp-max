//! Service abstraction for language servers.

pub use self::client::{progress, Client, ClientSocket, RequestStream, ResponseSink};

pub(crate) use self::pending::Pending;
pub(crate) use self::state::{ServerState, State};

use std::fmt::{self, Debug, Display, Formatter};
use std::sync::Arc;
use std::task::{Context, Poll};

use futures::future::{self, BoxFuture, FutureExt};
use serde_json::Value;
use tower::Service;

use crate::jsonrpc::{
    Error, ErrorCode, FromParams, IntoResponse, Method, Request, Response, Router,
};
use crate::LanguageServer;

pub(crate) mod layers;

mod client;
mod pending;
mod state;

/// Error that occurs when attempting to call the language server after it has already exited.
#[derive(Clone, Debug, Eq, PartialEq)]
#[repr(transparent)]
pub struct ExitedError(pub i32);

impl std::error::Error for ExitedError {}

impl Display for ExitedError {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "language server has exited with status: {}", self.0)
    }
}

impl ExitedError {
    /// Returns the exit status code.
    pub fn code(&self) -> i32 {
        self.0
    }
}

/// Service abstraction for the Language Server Protocol.
///
/// This service takes an incoming JSON-RPC message as input and produces an outgoing message as
/// output. If the incoming message is a server notification or a client response, then the
/// corresponding response will be `None`.
///
/// This implements [`tower::Service`] in order to remain independent from the underlying transport
/// and to facilitate further abstraction with middleware.
///
/// Pending requests can be canceled by issuing a [`$/cancelRequest`] notification.
///
/// [`$/cancelRequest`]: https://microsoft.github.io/language-server-protocol/specification#cancelRequest
///
/// The service shuts down and stops serving requests after the [`exit`] notification is received.
///
/// [`exit`]: https://microsoft.github.io/language-server-protocol/specification#exit
#[derive(Debug)]
pub struct LspService<S> {
    inner: Router<S, ExitedError>,
    state: Arc<ServerState>,
}

impl<S: LanguageServer> LspService<S> {
    /// Creates a new `LspService` with the given server backend, also returning a channel for
    /// server-to-client communication.
    pub fn new<F>(init: F) -> (Self, ClientSocket)
    where
        F: FnOnce(Client) -> S,
    {
        LspService::build(init).finish()
    }

    /// Starts building a new `LspService`.
    ///
    /// Returns an `LspServiceBuilder`, which allows adding custom JSON-RPC methods to the server.
    pub fn build<F>(init: F) -> LspServiceBuilder<S>
    where
        F: FnOnce(Client) -> S,
    {
        let state = Arc::new(ServerState::new());

        let (client, socket) = Client::new(state.clone());
        let inner = Router::new(init(client.clone()));
        let pending = Arc::new(Pending::new());

        LspServiceBuilder {
            inner: crate::generated::register_lsp_methods(
                inner,
                state.clone(),
                pending.clone(),
                client,
            ),
            state,
            pending,
            socket,
        }
    }

    /// Returns a reference to the inner server.
    pub fn inner(&self) -> &S {
        self.inner.inner()
    }
}

impl<S: LanguageServer> Service<Request> for LspService<S> {
    type Response = Option<Response>;
    type Error = ExitedError;
    type Future = BoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        if self.state.get() == State::Exited {
            let code = self.state.get_exit_code();
            return Poll::Ready(Err(ExitedError(code)));
        }
        if self.state.poll_initializing(cx).is_pending() {
            return Poll::Pending;
        }
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: Request) -> Self::Future {
        if self.state.get() == State::Exited {
            let code = self.state.get_exit_code();
            return future::err(ExitedError(code)).boxed();
        }

        let fut = self.inner.call(req);

        Box::pin(async move {
            let response = fut.await?;

            match response.as_ref().and_then(|res| res.error()) {
                Some(Error {
                    code: ErrorCode::MethodNotFound,
                    data: Some(Value::String(m)),
                    ..
                }) if m.starts_with("$/") => Ok(None),
                _ => Ok(response),
            }
        })
    }
}

/// A builder to customize the properties of an `LspService`.
///
/// To construct an `LspServiceBuilder`, refer to [`LspService::build`].
pub struct LspServiceBuilder<S> {
    inner: Router<S, ExitedError>,
    state: Arc<ServerState>,
    pending: Arc<Pending>,
    socket: ClientSocket,
}

impl<S: LanguageServer> LspServiceBuilder<S> {
    /// Defines a custom JSON-RPC request or notification with the given method `name` and handler.
    ///
    /// # Handler varieties
    ///
    /// Fundamentally, any inherent `async fn(&self)` method defined directly on the language
    /// server backend could be considered a valid method handler.
    ///
    /// Handlers may optionally include a single `params` argument. This argument may be of any
    /// type that implements [`Serialize`](serde::Serialize).
    ///
    /// Handlers which return `()` are treated as **notifications**, while those which return
    /// [`jsonrpc::Result<T>`](crate::jsonrpc::Result) are treated as **requests**.
    ///
    /// Similar to the `params` argument, the `T` in the `Result<T>` return values may be of any
    /// type which implements [`DeserializeOwned`](serde::de::DeserializeOwned). Additionally, this
    /// type _must_ be convertible into a [`serde_json::Value`] using [`serde_json::to_value`]. If
    /// this latter constraint is not met, the client will receive a JSON-RPC error response with
    /// code `-32603` (Internal Error) instead of the expected response.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use serde_json::{json, Value};
    /// use tower_lsp_max::jsonrpc::Result;
    /// use tower_lsp_max::lsp_types::*;
    /// use tower_lsp_max::{LanguageServer, LspService};
    ///
    /// struct Mock;
    ///
    /// // Implementation of `LanguageServer` omitted...
    /// # #[tower_lsp_max::async_trait]
    /// # impl LanguageServer for Mock {
    /// #     async fn initialize(&self, _: InitializeParams) -> Result<InitializeResult> {
    /// #         Ok(InitializeResult::default())
    /// #     }
    /// #
    /// #     async fn shutdown(&self) -> Result<()> {
    /// #         Ok(())
    /// #     }
    /// # }
    ///
    /// impl Mock {
    ///     async fn request(&self) -> Result<i32> {
    ///         Ok(123)
    ///     }
    ///
    ///     async fn request_params(&self, params: Vec<String>) -> Result<Value> {
    ///         Ok(json!({"num_elems":params.len()}))
    ///     }
    ///
    ///     async fn notification(&self) {
    ///         // ...
    ///     }
    ///
    ///     async fn notification_params(&self, params: Value) {
    ///         // ...
    /// #       let _ = params;
    ///     }
    /// }
    ///
    /// let (service, socket) = LspService::build(|_| Mock)
    ///     .custom_method("custom/request", Mock::request)
    ///     .custom_method("custom/requestParams", Mock::request_params)
    ///     .custom_method("custom/notification", Mock::notification)
    ///     .custom_method("custom/notificationParams", Mock::notification_params)
    ///     .finish();
    /// ```
    pub fn custom_method<P, R, F>(mut self, name: &'static str, callback: F) -> Self
    where
        P: FromParams,
        R: IntoResponse,
        F: for<'a> Method<&'a S, P, R> + Clone + Send + Sync + 'static,
    {
        let layer = layers::Normal::new(self.state.clone(), self.pending.clone());
        self.inner.method(name, callback, layer);
        self
    }

    /// Constructs the `LspService` and returns it, along with a channel for server-to-client
    /// communication.
    pub fn finish(self) -> (LspService<S>, ClientSocket) {
        let LspServiceBuilder {
            inner,
            state,
            socket,
            ..
        } = self;

        (LspService { inner, state }, socket)
    }
}

impl<S: Debug> Debug for LspServiceBuilder<S> {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.debug_struct("LspServiceBuilder")
            .field("inner", &self.inner)
            .finish_non_exhaustive()
    }
}

#[cfg(test)]
mod tests {
    use async_trait::async_trait;
    use lsp_types::*;
    use serde_json::json;
    use tower::ServiceExt;

    use super::*;
    use crate::jsonrpc::Result;

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
            future::pending().await
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

        let ready = future::poll_fn(|cx| service.poll_ready(cx)).await;
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

        let ready = future::poll_fn(|cx| service.poll_ready(cx)).await;
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

        // 3. Call max/explainDiagnostic
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
        let diagnostic: max_protocol::MaxDiagnostic = serde_json::from_value(res.unwrap()).unwrap();
        assert_eq!(diagnostic.diagnostic_id, "diag-uninitialized-admission");

        // 4. Call max/repairPlan
        let req = Request::build("max/repairPlan")
            .params("diag-uninitialized-admission".to_string())
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
        let plans: Vec<max_protocol::MaxCodeAction> = serde_json::from_value(res.unwrap()).unwrap();
        assert_eq!(plans.len(), 1);
        let action = plans[0].clone();

        // Reset server registry state back to Uninitialized to satisfy precondition check
        if let Ok(mut reg) = crate::get_registry().lock() {
            reg.current_state = crate::service::State::Uninitialized;
        }

        // 5. Call max/applyRepairTransaction
        let req = Request::build("max/applyRepairTransaction")
            .params(serde_json::to_value(action.clone()).unwrap())
            .id(5)
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
        let receipt: max_protocol::Receipt = serde_json::from_value(res.unwrap()).unwrap();
        assert!(receipt.receipt_id.starts_with("rcpt-"));

        // Verify the diagnostic has been cleared/resolved
        let req = Request::build("max/explainDiagnostic")
            .params("diag-uninitialized-admission".to_string())
            .id(6)
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
        let plans2: Vec<max_protocol::MaxCodeAction> =
            serde_json::from_value(res.unwrap()).unwrap();
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
        let plans3: Vec<max_protocol::MaxCodeAction> =
            serde_json::from_value(res.unwrap()).unwrap();
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
}
