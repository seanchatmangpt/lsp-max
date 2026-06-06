//! Assorted middleware that implements LSP server semantics.

pub(crate) use self::cancellation::Cancellable;
pub use self::catch_unwind::{CatchUnwind, CatchUnwindService};
pub use self::document_sync::{DocumentSync, DocumentSyncService};
pub use self::exit::{Exit, ExitService};
pub use self::initialize::{Initialize, InitializeService};
pub use self::normal::{Normal, NormalService};
pub use self::permissive::{Permissive, PermissiveService};
pub use self::shutdown::{Shutdown, ShutdownService};

pub(super) mod cancellation;
mod catch_unwind;
mod document_sync;
mod exit;
mod initialize;
mod normal;
mod permissive;
mod shutdown;

#[cfg(test)]
mod tests;

use super::state::State;
use crate::jsonrpc::{not_initialized_error, Error, Id, Response};

fn not_initialized_response(id: Option<Id>, server_state: State) -> Option<Response> {
    let id = id?;
    let error = match server_state {
        State::Uninitialized | State::Initializing => not_initialized_error(),
        _ => Error::invalid_request(),
    };

    Some(Response::from_error(id, error))
}
