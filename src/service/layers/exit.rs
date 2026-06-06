use futures::future::{self, Ready};
use std::marker::PhantomData;
use std::sync::Arc;
use std::task::{Context, Poll};
use tower::{Layer, Service};
use tracing::info;

use super::super::client::Client;
use super::super::pending::Pending;
use super::super::state::{ServerState, State};
use super::super::ExitedError;
use crate::jsonrpc::{Request, Response};

/// Middleware which implements `exit` notification semantics.
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
    type Future = Ready<Result<Self::Response, Self::Error>>;

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
