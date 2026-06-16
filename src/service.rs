//! Service abstraction for language servers.

pub use self::cancellation::{CancellationGuard, OnDrop};
pub use self::client::{progress, Client, ClientSocket, RequestStream, ResponseSink};

pub use self::pending::Pending;
pub use self::state::{ServerState, State};

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

mod cancellation;
mod client;
mod pending;
mod state;
mod watchdog;

/// Error that occurs when attempting to call the language server after it has already exited.
///
/// See also: [`examples/transport_utilities_explained.rs`] — a run-to-exit witness that
/// demonstrates `ExitedError`, `ClientSocket`, and `Loopback` with real `assert!`s.
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
    inner: layers::CatchUnwindService<Router<S, ExitedError>>,
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
        let doc_sync = layers::DocumentSync::new();

        LspServiceBuilder {
            inner: crate::generated::register_lsp_methods(
                inner,
                state.clone(),
                pending.clone(),
                client,
                doc_sync.clone(),
            ),
            state,
            pending,
            socket,
            doc_sync,
        }
    }

    /// Returns a reference to the inner server.
    pub fn inner(&self) -> &S {
        self.inner.inner().inner()
    }
}

impl<S: LanguageServer> Service<Request> for LspService<S> {
    type Response = Option<Response>;
    type Error = ExitedError;
    type Future = BoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        // The service is ready only after initialization completes.
        // An already-exited service propagates the exit error through inner.poll_ready.
        let cur_state = self.state.get();
        if cur_state == State::Exited {
            let code = self.state.get_exit_code();
            return Poll::Ready(Err(ExitedError(code)));
        }
        if self.state.poll_initializing(cx).is_pending() {
            return Poll::Pending;
        }
        tracing::trace!("LspService::poll_ready delegating to inner (state={:?})", cur_state);
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
    doc_sync: layers::DocumentSync,
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
    /// ```rust,no_run
    /// use serde_json::{json, Value};
    /// use lsp_max::jsonrpc::Result;
    /// use lsp_max::lsp_types_max::*;
    /// use lsp_max::{LanguageServer, LspService};
    ///
    /// struct Mock;
    ///
    /// // Implementation of `LanguageServer` omitted...
    /// # #[lsp_max::async_trait]
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
        let layer = tower::ServiceBuilder::new()
            .layer(layers::Normal::new(
                self.state.clone(),
                self.pending.clone(),
            ))
            .layer(self.doc_sync.clone())
            .into_inner();
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

        let inner = tower::Layer::layer(&layers::CatchUnwind, inner);

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

#[allow(dead_code)]
fn handle_mesh_rpc(
    state: &ServerState,
    method: &str,
    params: Option<Value>,
) -> Result<Value, String> {
    let mut registry = crate::get_registry()
        .lock()
        .unwrap_or_else(|p| p.into_inner());
    crate::update_diagnostics(&mut registry);

    let mut mesh = state.mesh.lock().unwrap_or_else(|p| p.into_inner());

    // Sync registry -> mesh
    let instance_id = "LSP_1";
    if !mesh.instances.contains_key(instance_id) {
        mesh.add_instance(crate::max_runtime::LspInstance::new(instance_id));
    }
    {
        let instance = match mesh.instances.get_mut(instance_id) {
            Some(inst) => inst,
            None => return Err("LSP_1 instance not found".to_string()),
        };
        instance.phase = match registry.current_state {
            State::Uninitialized => crate::max_runtime::LspPhase::Uninitialized,
            State::Initializing => crate::max_runtime::LspPhase::Initializing,
            State::Initialized => crate::max_runtime::LspPhase::Initialized,
            State::ShutDown => crate::max_runtime::LspPhase::ShutDown,
            State::Exited => crate::max_runtime::LspPhase::Exited,
        };
        instance.diagnostics = registry.diagnostics.values().cloned().collect();
        let mut sorted_receipts: Vec<_> = registry.receipts.values().cloned().collect();
        sorted_receipts.sort_by_key(|r| {
            if r.receipt_id == "rcpt-uninitialized" {
                0
            } else if r
                .receipt_id
                .starts_with("rcpt-uninitialized-to-initializing:")
            {
                1
            } else if r
                .receipt_id
                .starts_with("rcpt-initializing-to-initialized:")
            {
                2
            } else if r.receipt_id == "rcpt-initialized-to-shutdown" {
                3
            } else if r.receipt_id == "rcpt-shutdown-to-exited" {
                4
            } else {
                5
            }
        });
        instance.receipts = sorted_receipts;
    }

    // Dispatch RPC
    let result = mesh.dispatch_rpc(instance_id, method, params.unwrap_or(Value::Null))?;

    // Sync mesh -> registry
    if let Some(instance) = mesh.instances.get(instance_id) {
        let mesh_diagnostic_ids: std::collections::HashSet<String> = instance
            .diagnostics
            .iter()
            .map(|d| d.diagnostic_id.clone())
            .collect();
        for id in registry
            .diagnostics
            .keys()
            .cloned()
            .collect::<Vec<String>>()
        {
            if !mesh_diagnostic_ids.contains(&id) {
                registry.diagnostics.remove(&id);
                registry.cleared_diagnostics.insert(id);
            }
        }
        for d in &instance.diagnostics {
            registry
                .diagnostics
                .insert(d.diagnostic_id.clone(), d.clone());
        }
        for r in &instance.receipts {
            registry.receipts.insert(r.receipt_id.clone(), r.clone());
        }
    }

    // Custom post-sync logic for snapshot: register snapshot in registry
    if method == "max/snapshot" {
        let snapshot_id: crate::max_protocol::SnapshotId =
            serde_json::from_value(result.clone())
                .map_err(|e| format!("Invalid snapshot ID: {}", e))?;

        let capability_vector = crate::max_protocol::MaxCapabilityVector {
            client: registry.client_capabilities.clone().unwrap_or_default(),
            server: registry.server_capabilities.clone().unwrap_or_default(),
            negotiated: serde_json::json!({
                "conformance": "maximal",
                "law_framework": "v1"
            }),
            experimental: serde_json::json!({}),
            gaps: vec![],
        };

        let diagnostics = registry.diagnostics.values().cloned().collect();
        let actions = registry.repair_plans.values().flatten().cloned().collect();

        let score = if registry.diagnostics.is_empty() {
            100.0
        } else {
            let severity_penalty: f64 = registry
                .diagnostics
                .values()
                .map(|d| match d.lsp.severity {
                    Some(crate::lsp_types_max::DiagnosticSeverity::ERROR) => 30.0,
                    Some(crate::lsp_types_max::DiagnosticSeverity::WARNING) => 15.0,
                    _ => 5.0,
                })
                .sum();
            (100.0 - severity_penalty).max(0.0)
        };

        let (refused_diags, admitted_diags): (Vec<_>, Vec<_>) =
            registry.diagnostics.values().partition(|d| {
                matches!(
                    d.lsp.severity,
                    Some(crate::lsp_types_max::DiagnosticSeverity::ERROR)
                )
            });
        let refused: Vec<max_protocol::LawAxis> =
            refused_diags.iter().map(|d| d.law_axis.clone()).collect();
        let admitted: Vec<max_protocol::LawAxis> =
            admitted_diags.iter().map(|d| d.law_axis.clone()).collect();
        let derived_score = if admitted.is_empty() && refused.is_empty() {
            None
        } else {
            let total = (admitted.len() + refused.len()) as f64;
            Some(100.0 * admitted.len() as f64 / total)
        };
        let _ = score; // superseded by derived_score
        let conformance_vector = crate::max_protocol::ConformanceVector {
            admitted,
            refused,
            unknown: Vec::new(),
            score: derived_score,
            strict_mode: true,
            process_quality: None,
            ..Default::default()
        };

        let receipts = registry.receipts.values().cloned().collect();

        let record = crate::SnapshotRecord {
            id: snapshot_id.clone(),
            capability_vector,
            diagnostics,
            actions,
            conformance_vector,
            receipts,
        };

        registry.snapshots.insert(snapshot_id.0.clone(), record);
    }

    Ok(result)
}

#[cfg(test)]
mod tests;
