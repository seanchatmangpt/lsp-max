//! Assorted middleware that implements LSP server semantics.

pub use self::initialize::{Initialize, InitializeService};
pub use self::shutdown::{Shutdown, ShutdownService};
pub use self::exit::{Exit, ExitService};
pub use self::normal::{Normal, NormalService};
pub use self::permissive::{Permissive, PermissiveService};
pub use self::catch_unwind::{CatchUnwind, CatchUnwindService};
pub use self::document_sync::{DocumentSync, DocumentSyncService};
pub(crate) use self::cancellation::Cancellable;

mod initialize;
mod shutdown;
mod exit;
mod normal;
mod permissive;
mod catch_unwind;
pub(super) mod cancellation;
mod document_sync;

#[cfg(test)]
mod tests;

use crate::jsonrpc::{not_initialized_error, Error, Id, Response};
use super::state::State;

fn not_initialized_response(id: Option<Id>, server_state: State) -> Option<Response> {
    let id = id?;
    let error = match server_state {
        State::Uninitialized | State::Initializing => not_initialized_error(),
        _ => Error::invalid_request(),
    };

    Some(Response::from_error(id, error))
}
