pub mod helpers;
pub mod lookups;
pub mod populate_call_hierarchy;
pub mod populate_defs_refs;
pub mod populate_hover_diag;
pub mod populate_type_hierarchy;
pub mod types;
pub mod update;

pub use helpers::*;
pub use lookups::*;
pub use types::*;
pub use update::*;

use std::sync::OnceLock;

pub static VIEWS: OnceLock<MaterializedViewStore> = OnceLock::new();

pub fn get_views() -> &'static MaterializedViewStore {
    VIEWS.get_or_init(MaterializedViewStore::new)
}

#[cfg(test)]
mod tests;
