use futures::future::{BoxFuture, FutureExt};
use std::sync::Arc;
use std::task::{Context, Poll};
use tower::{Layer, Service};

use super::super::pending::Pending;
use super::super::state::ServerState;
use super::super::ExitedError;
use super::Cancellable;
use crate::jsonrpc::{Request, Response};

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
