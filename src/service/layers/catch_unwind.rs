use futures::future::{BoxFuture, FutureExt};
use std::task::{Context, Poll};
use tower::{Layer, Service};

use super::super::ExitedError;
use crate::jsonrpc::{Error, ErrorCode, Request, Response};

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
        f.debug_struct("CatchUnwindService").finish_non_exhaustive()
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
