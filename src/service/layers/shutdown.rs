use futures::future::{BoxFuture, FutureExt};
use std::sync::Arc;
use std::task::{Context, Poll};
use tower::{Layer, Service};
use tracing::info;

use super::super::pending::Pending;
use super::super::state::{ServerState, State};
use super::super::ExitedError;
use super::Cancellable;
use crate::jsonrpc::{Request, Response};

/// Middleware which implements `shutdown` request semantics.
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
            futures::future::ok(super::not_initialized_response(id, cur_state)).boxed()
        }
    }
}
