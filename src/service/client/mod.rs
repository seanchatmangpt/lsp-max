//! Types for sending data to and from the language client.

pub use self::socket::{ClientSocket, RequestStream, ResponseSink};

use std::fmt::{self, Debug, Formatter};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use std::task::{Context, Poll};

use futures::channel::mpsc::{self, Sender};
use futures::future::BoxFuture;
use futures::sink::SinkExt;
use tower::Service;
use tracing::{error, trace};

use self::pending::Pending;
pub use self::progress::Progress;
use super::state::{ServerState, State};
use super::ExitedError;
use crate::jsonrpc::{self, Error, ErrorCode, Id, Request, Response};

mod lsp_methods;
mod pending;
pub mod progress;
pub mod progress_ext;
mod socket;

#[cfg(test)]
mod tests;

struct ClientInner {
    tx: Sender<Request>,
    request_id: AtomicU32,
    pending: Arc<Pending>,
    state: Arc<ServerState>,
}

/// Handle for communicating with the language client.
#[derive(Clone)]
pub struct Client {
    inner: Arc<ClientInner>,
}

impl Client {
    pub(super) fn new(state: Arc<ServerState>) -> (Self, ClientSocket) {
        let (tx, rx) = mpsc::channel(1);
        let pending = Arc::new(Pending::new());

        let client = Client {
            inner: Arc::new(ClientInner {
                tx,
                request_id: AtomicU32::new(0),
                pending: pending.clone(),
                state: state.clone(),
            }),
        };

        (client, ClientSocket { rx, pending, state })
    }

    /// Disconnects the `Client` from its corresponding `LspService`.
    pub(crate) fn close(&self) {
        self.inner.tx.clone().close_channel();
    }
}

impl Client {
    /// Sends a custom notification to the client.
    pub async fn send_notification<N>(&self, params: N::Params)
    where
        N: lsp_types_max::notification::Notification,
    {
        if let State::Initialized | State::ShutDown = self.inner.state.get() {
            self.send_notification_unchecked::<N>(params).await;
        } else {
            let msg = Request::from_notification::<N>(params);
            trace!("server not initialized, supressing message: {}", msg);
        }
    }

    async fn send_notification_unchecked<N>(&self, params: N::Params)
    where
        N: lsp_types_max::notification::Notification,
    {
        let request = Request::from_notification::<N>(params);
        if self.clone().call(request).await.is_err() {
            error!("failed to send notification");
        }
    }

    /// Sends a custom request to the client.
    pub async fn send_request<R>(&self, params: R::Params) -> jsonrpc::Result<R::Result>
    where
        R: lsp_types_max::request::Request,
    {
        if let State::Initialized | State::ShutDown = self.inner.state.get() {
            self.send_request_unchecked::<R>(params).await
        } else {
            let id = self.inner.request_id.load(Ordering::SeqCst) as i64 + 1;
            let msg = Request::from_request::<R>(id.into(), params);
            trace!("server not initialized, supressing message: {}", msg);
            Err(jsonrpc::not_initialized_error())
        }
    }

    async fn send_request_unchecked<R>(&self, params: R::Params) -> jsonrpc::Result<R::Result>
    where
        R: lsp_types_max::request::Request,
    {
        let id = self.next_request_id();
        let request = Request::from_request::<R>(id, params);

        let response = match self.clone().call(request).await {
            Ok(Some(response)) => response,
            Ok(None) | Err(_) => return Err(Error::internal_error()),
        };

        let (_, result) = response.into_parts();
        result.and_then(|v| {
            serde_json::from_value(v).map_err(|e| Error {
                code: ErrorCode::ParseError,
                message: e.to_string().into(),
                data: None,
            })
        })
    }
}

impl Client {
    /// Increments the internal request ID counter and returns the previous value.
    pub fn next_request_id(&self) -> Id {
        let num = self.inner.request_id.fetch_add(1, Ordering::Relaxed);
        Id::Number(num as i64)
    }
}

impl Debug for Client {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.debug_struct("Client")
            .field("tx", &self.inner.tx)
            .field("pending", &self.inner.pending)
            .field("request_id", &self.inner.request_id)
            .field("state", &self.inner.state)
            .finish()
    }
}

impl Service<Request> for Client {
    type Response = Option<Response>;
    type Error = ExitedError;
    type Future = BoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        let state = self.inner.state.clone();
        self.inner.tx.clone().poll_ready(cx).map_err(move |_| {
            let code = if state.get() == State::Exited {
                state.get_exit_code()
            } else {
                1
            };
            ExitedError(code)
        })
    }

    fn call(&mut self, req: Request) -> Self::Future {
        let mut tx = self.inner.tx.clone();
        let response_waiter = req.id().cloned().map(|id| self.inner.pending.wait(id));
        let state = self.inner.state.clone();

        Box::pin(async move {
            if tx.send(req).await.is_err() {
                let code = if state.get() == State::Exited {
                    state.get_exit_code()
                } else {
                    1
                };
                return Err(ExitedError(code));
            }

            match response_waiter {
                Some(fut) => Ok(Some(fut.await)),
                None => Ok(None),
            }
        })
    }
}

#[derive(Debug)]
struct FoldingRangeRefresh;

impl lsp_types_max::request::Request for FoldingRangeRefresh {
    type Params = ();
    type Result = ();
    const METHOD: &'static str = "workspace/foldingRange/refresh";
}
