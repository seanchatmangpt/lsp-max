use futures::future::{BoxFuture, FutureExt};
use std::sync::Arc;
use std::task::{Context, Poll};
use tower::Service;

use super::super::pending::Pending;
use super::super::ExitedError;
use crate::jsonrpc::{Request, Response};

/// Wraps an inner service `S` and implements `$/cancelRequest` semantics for all requests.
pub(crate) struct Cancellable<S> {
    inner: S,
    pending: Arc<Pending>,
}

impl<S> Cancellable<S> {
    pub(crate) fn new(inner: S, pending: Arc<Pending>) -> Self {
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
