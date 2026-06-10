//! Composition substrate for `tower-lsp-max v26.6.5`.
//!
//! Implements:
//! - R2: Capability discovery, intersection, dynamic registration/unregistration.
//! - R3: Method routing and composition matrix.
//! - R4: Source attribution and document version causality.
//! - R5: Guarded mutations via TransactionEditGate.
//! - R6: Failure isolation and source degradation.

pub mod capability_tracker;
pub mod edit_gate;
pub mod language_server_impl;
pub mod merge;
pub mod routing;
pub mod server;
pub mod strategy;
pub mod upstream;
pub mod version_tracker;

#[cfg(test)]
mod tests;

// Items used by lib.rs — must remain `pub`.
pub use server::{ComposedServer, CompositionState, SharedCompositionState};
pub use strategy::SourceHealth;

// Items consumed via `use super::*` in submodule tests.
// The glob import makes these appear "unused" to clippy, but they are used.
#[allow(unused_imports)]
pub(crate) use capability_tracker::{client_supports, CapabilityTracker, DynamicRegistration};
#[allow(unused_imports)]
pub(crate) use edit_gate::{
    extract_all_uris, extract_ranges_for_uri, extract_version_from_edit, pos_lt, ranges_overlap,
    EditGateOutcome, ProposedEdit, TransactionEditGate,
};
#[allow(unused_imports)]
pub(crate) use merge::{
    merge_attributed, merge_deduped_locations, merge_hovers_with_attribution, AttributedObservation,
};
#[allow(unused_imports)]
pub(crate) use merge::{merge_edits, merge_workspace_edits};
#[allow(unused_imports)]
pub(crate) use strategy::{
    capability_supports_method, method_strategy, CompositionStrategy, UpstreamSource,
};
#[allow(unused_imports)]
pub(crate) use upstream::{PersistentUpstream, UpstreamNotification};
#[allow(unused_imports)]
pub(crate) use version_tracker::{DocumentVersionTracker, VersionCheckResult};
