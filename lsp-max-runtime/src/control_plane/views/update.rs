//! Coordinator that drives all materialized-view population sub-modules.
//!
//! Each logical group of SPARQL queries lives in its own module:
//! - [`populate_defs_refs`]   — definitions and references
//! - [`populate_hover_diag`]  — hovers and diagnostics
//! - [`populate_call_hierarchy`] — call hierarchy (prepare / incoming / outgoing)
//! - [`populate_type_hierarchy`] — type hierarchy (prepare / supertypes / subtypes)

use super::populate_call_hierarchy::populate_call_hierarchy;
use super::populate_defs_refs::{populate_definitions, populate_references};
use super::populate_hover_diag::{populate_diagnostics, populate_hovers};
use super::populate_type_hierarchy::populate_type_hierarchy;
use super::types::MaterializedViewStore;
use oxigraph::store::Store;

/// Rebuild all materialized views from the Oxigraph triple store.
///
/// Called after every admitted commit. Clears existing views, re-runs all
/// SPARQL projections, then advances the epoch sync barrier so that any
/// readers blocked on `sync_condvar` are unblocked.
#[allow(clippy::mutable_key_type)]
pub fn update_views(store: &Store, views: &MaterializedViewStore) {
    views.clear();

    populate_definitions(store, views);
    populate_references(store, views);
    populate_hovers(store, views);
    populate_diagnostics(store, views);
    populate_call_hierarchy(store, views);
    populate_type_hierarchy(store, views);

    // Monotonic epoch synchronisation — unblock waiting readers.
    let committed = views
        .committed_epoch
        .load(std::sync::atomic::Ordering::Acquire);
    views
        .applied_epoch
        .store(committed, std::sync::atomic::Ordering::Release);
    let _lock = views.sync_mutex.lock().unwrap();
    views.sync_condvar.notify_all();
}
