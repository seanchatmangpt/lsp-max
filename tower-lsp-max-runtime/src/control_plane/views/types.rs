use dashmap::DashMap;
use lsp_types_max::{
    CallHierarchyIncomingCall, CallHierarchyItem, CallHierarchyOutgoingCall, Diagnostic, Hover,
    Location, Range, TypeHierarchyItem,
};
use std::sync::Arc;
use url::Url;

#[derive(Debug, Clone)]
pub struct DefinitionEntry {
    pub src_range: Range,
    pub dest_location: Location,
}

#[derive(Debug, Clone)]
pub struct ReferenceEntry {
    pub src_range: Range,
    pub ref_locations: Vec<Location>,
}

#[derive(Debug, Clone)]
pub struct HoverEntry {
    pub src_range: Range,
    pub hover: Hover,
}

#[derive(Debug, Clone)]
pub struct MaterializedViews {
    pub definitions: Arc<DashMap<String, Vec<DefinitionEntry>>>,
    pub references: Arc<DashMap<String, Vec<ReferenceEntry>>>,
    pub hovers: Arc<DashMap<String, Vec<HoverEntry>>>,
    pub diagnostics: Arc<DashMap<String, Vec<Diagnostic>>>,
}

impl Default for MaterializedViews {
    fn default() -> Self {
        Self::new()
    }
}

impl MaterializedViews {
    pub fn new() -> Self {
        Self {
            definitions: Arc::new(DashMap::new()),
            references: Arc::new(DashMap::new()),
            hovers: Arc::new(DashMap::new()),
            diagnostics: Arc::new(DashMap::new()),
        }
    }

    pub fn clear(&self) {
        self.definitions.clear();
        self.references.clear();
        self.hovers.clear();
        self.diagnostics.clear();
    }
}

#[allow(clippy::mutable_key_type)]
#[derive(Debug)]
pub struct MaterializedViewStore {
    pub definitions: DashMap<Url, Vec<(Range, Location)>>,
    pub references: DashMap<Url, Vec<(Range, Vec<Location>)>>,
    pub hovers: DashMap<Url, Vec<(Range, Hover)>>,
    pub diagnostics: DashMap<Url, Vec<Diagnostic>>,

    // Call and Type Hierarchy maps
    pub call_hierarchy_prepare: DashMap<Url, Vec<(Range, Vec<CallHierarchyItem>)>>,
    pub call_hierarchy_incoming: DashMap<Url, Vec<(Range, Vec<CallHierarchyIncomingCall>)>>,
    pub call_hierarchy_outgoing: DashMap<Url, Vec<(Range, Vec<CallHierarchyOutgoingCall>)>>,
    pub type_hierarchy_prepare: DashMap<Url, Vec<(Range, Vec<TypeHierarchyItem>)>>,
    pub type_hierarchy_supertypes: DashMap<Url, Vec<(Range, Vec<TypeHierarchyItem>)>>,
    pub type_hierarchy_subtypes: DashMap<Url, Vec<(Range, Vec<TypeHierarchyItem>)>>,

    pub committed_epoch: std::sync::atomic::AtomicU64,
    pub applied_epoch: std::sync::atomic::AtomicU64,
    pub sync_mutex: std::sync::Mutex<()>,
    pub sync_condvar: std::sync::Condvar,
}

impl Default for MaterializedViewStore {
    fn default() -> Self {
        Self::new()
    }
}

impl MaterializedViewStore {
    pub fn new() -> Self {
        Self {
            definitions: DashMap::new(),
            references: DashMap::new(),
            hovers: DashMap::new(),
            diagnostics: DashMap::new(),

            call_hierarchy_prepare: DashMap::new(),
            call_hierarchy_incoming: DashMap::new(),
            call_hierarchy_outgoing: DashMap::new(),
            type_hierarchy_prepare: DashMap::new(),
            type_hierarchy_supertypes: DashMap::new(),
            type_hierarchy_subtypes: DashMap::new(),

            committed_epoch: std::sync::atomic::AtomicU64::new(0),
            applied_epoch: std::sync::atomic::AtomicU64::new(0),
            sync_mutex: std::sync::Mutex::new(()),
            sync_condvar: std::sync::Condvar::new(),
        }
    }

    pub fn clear(&self) {
        self.definitions.clear();
        self.references.clear();
        self.hovers.clear();
        self.diagnostics.clear();

        self.call_hierarchy_prepare.clear();
        self.call_hierarchy_incoming.clear();
        self.call_hierarchy_outgoing.clear();
        self.type_hierarchy_prepare.clear();
        self.type_hierarchy_supertypes.clear();
        self.type_hierarchy_subtypes.clear();
    }

    pub fn wait_for_epoch(&self, target_epoch: u64) {
        let mut lock = self.sync_mutex.lock().unwrap();
        while self
            .applied_epoch
            .load(std::sync::atomic::Ordering::Acquire)
            < target_epoch
        {
            lock = self.sync_condvar.wait(lock).unwrap();
        }
    }

    pub fn wait_for_committed(&self) {
        let target = self
            .committed_epoch
            .load(std::sync::atomic::Ordering::Acquire);
        self.wait_for_epoch(target);
    }
}
