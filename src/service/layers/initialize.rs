use futures::future::{BoxFuture, FutureExt};
use std::sync::Arc;
use std::task::{Context, Poll};
use tower::{Layer, Service};
use tracing::warn;

use super::super::pending::Pending;
use super::super::state::{ServerState, State};
use super::super::ExitedError;
use super::Cancellable;
use crate::jsonrpc::{Error, Request, Response};

/// Middleware which implements `initialize` request semantics.
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
        let client_caps: Option<lsp_types_max::ClientCapabilities> =
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
                        let server_caps_parsed: Option<lsp_types_max::ServerCapabilities> =
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
            futures::future::ok(id.map(|id| Response::from_error(id, Error::invalid_request())))
                .boxed()
        }
    }
}
