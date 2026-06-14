use futures::future::{BoxFuture, FutureExt};
use std::sync::Arc;
use std::task::{Context, Poll};
use tower::{Layer, Service};

use super::super::pending::Pending;
use super::super::state::{ServerState, State};
use super::super::ExitedError;
use super::Cancellable;
use crate::jsonrpc::{Request, Response};

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
                tracing::trace!(
                    "NormalService: rejected request/notification '{}' because state is {:?}",
                    req.method(),
                    cur_state
                );
                let (_, id, _) = req.into_parts();
                futures::future::ok(super::not_initialized_response(id, cur_state)).boxed()
            }
        }
    }
}
