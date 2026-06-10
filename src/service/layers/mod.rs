//! Assorted middleware that implements LSP server semantics.

pub(crate) use self::cancellation::Cancellable;
pub use self::catch_unwind::{CatchUnwind, CatchUnwindService};
pub use self::document_sync::DocumentSync;
pub use self::exit::Exit;
pub use self::initialize::Initialize;
pub use self::normal::Normal;
pub use self::shutdown::Shutdown;

pub(super) mod cancellation;
mod catch_unwind;
mod document_sync;
mod exit;
mod initialize;
mod normal;
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
