use super::types::MaterializedViewStore;
use lsp_types_max::{
    CallHierarchyIncomingCall, CallHierarchyItem, CallHierarchyOutgoingCall, Diagnostic, Hover,
    Location, Position, Range, TypeHierarchyItem,
};
use url::Url;

pub fn contains_position(range: &Range, pos: Position) -> bool {
    let after_start = if pos.line > range.start.line {
        true
    } else if pos.line == range.start.line {
        pos.character >= range.start.character
    } else {
        false
    };

    let before_end = if pos.line < range.end.line {
        true
    } else if pos.line == range.end.line {
        pos.character <= range.end.character
    } else {
        false
    };

    after_start && before_end
}

pub fn range_size(range: Range) -> u64 {
    if range.end.line < range.start.line {
        0
    } else {
        let lines = (range.end.line - range.start.line) as u64;
        let chars = if lines == 0 {
            if range.end.character >= range.start.character {
                (range.end.character - range.start.character) as u64
            } else {
                0
            }
        } else {
            range.end.character as u64
        };
        lines * 10000 + chars
    }
}

#[allow(clippy::mutable_key_type)]
pub fn lookup_definition(
    views: &MaterializedViewStore,
    uri: &Url,
    pos: Position,
) -> Option<Location> {
    views.wait_for_committed();
    let entries = views.definitions.get(uri)?;
    let mut best_match: Option<(Range, Location)> = None;
    for (range, loc) in entries.iter() {
        if contains_position(range, pos) {
            if let Some(ref best) = best_match {
                if range_size(*range) < range_size(best.0) {
                    best_match = Some((*range, loc.clone()));
                }
            } else {
                best_match = Some((*range, loc.clone()));
            }
        }
    }
    best_match.map(|entry| entry.1)
}

#[allow(clippy::mutable_key_type)]
pub fn lookup_references(
    views: &MaterializedViewStore,
    uri: &Url,
    pos: Position,
) -> Option<Vec<Location>> {
    views.wait_for_committed();
    let entries = views.references.get(uri)?;
    let mut best_match: Option<(Range, Vec<Location>)> = None;
    for (range, locs) in entries.iter() {
        if contains_position(range, pos) {
            if let Some(ref best) = best_match {
                if range_size(*range) < range_size(best.0) {
                    best_match = Some((*range, locs.clone()));
                }
            } else {
                best_match = Some((*range, locs.clone()));
            }
        }
    }
    best_match.map(|entry| entry.1)
}

#[allow(clippy::mutable_key_type)]
pub fn lookup_hover(views: &MaterializedViewStore, uri: &Url, pos: Position) -> Option<Hover> {
    views.wait_for_committed();
    let entries = views.hovers.get(uri)?;
    let mut best_match: Option<(Range, Hover)> = None;
    for (range, hover) in entries.iter() {
        if contains_position(range, pos) {
            if let Some(ref best) = best_match {
                if range_size(*range) < range_size(best.0) {
                    best_match = Some((*range, hover.clone()));
                }
            } else {
                best_match = Some((*range, hover.clone()));
            }
        }
    }
    best_match.map(|entry| entry.1)
}

#[allow(clippy::mutable_key_type)]
pub fn lookup_diagnostics(views: &MaterializedViewStore, uri: &Url) -> Option<Vec<Diagnostic>> {
    views.wait_for_committed();
    views.diagnostics.get(uri).map(|v| v.clone())
}

#[allow(clippy::mutable_key_type)]
pub fn lookup_call_hierarchy_prepare(
    views: &MaterializedViewStore,
    uri: &Url,
    pos: Position,
) -> Option<Vec<CallHierarchyItem>> {
    views.wait_for_committed();
    let entries = views.call_hierarchy_prepare.get(uri)?;
    let mut best_match: Option<(Range, Vec<CallHierarchyItem>)> = None;
    for (range, items) in entries.iter() {
        if contains_position(range, pos) {
            if let Some(ref best) = best_match {
                if range_size(*range) < range_size(best.0) {
                    best_match = Some((*range, items.clone()));
                }
            } else {
                best_match = Some((*range, items.clone()));
            }
        }
    }
    best_match.map(|entry| entry.1)
}

#[allow(clippy::mutable_key_type)]
pub fn lookup_call_hierarchy_incoming(
    views: &MaterializedViewStore,
    uri: &Url,
    pos: Position,
) -> Option<Vec<CallHierarchyIncomingCall>> {
    views.wait_for_committed();
    let entries = views.call_hierarchy_incoming.get(uri)?;
    let mut best_match: Option<(Range, Vec<CallHierarchyIncomingCall>)> = None;
    for (range, calls) in entries.iter() {
        if contains_position(range, pos) {
            if let Some(ref best) = best_match {
                if range_size(*range) < range_size(best.0) {
                    best_match = Some((*range, calls.clone()));
                }
            } else {
                best_match = Some((*range, calls.clone()));
            }
        }
    }
    best_match.map(|entry| entry.1)
}

#[allow(clippy::mutable_key_type)]
pub fn lookup_call_hierarchy_outgoing(
    views: &MaterializedViewStore,
    uri: &Url,
    pos: Position,
) -> Option<Vec<CallHierarchyOutgoingCall>> {
    views.wait_for_committed();
    let entries = views.call_hierarchy_outgoing.get(uri)?;
    let mut best_match: Option<(Range, Vec<CallHierarchyOutgoingCall>)> = None;
    for (range, calls) in entries.iter() {
        if contains_position(range, pos) {
            if let Some(ref best) = best_match {
                if range_size(*range) < range_size(best.0) {
                    best_match = Some((*range, calls.clone()));
                }
            } else {
                best_match = Some((*range, calls.clone()));
            }
        }
    }
    best_match.map(|entry| entry.1)
}

#[allow(clippy::mutable_key_type)]
pub fn lookup_type_hierarchy_prepare(
    views: &MaterializedViewStore,
    uri: &Url,
    pos: Position,
) -> Option<Vec<TypeHierarchyItem>> {
    views.wait_for_committed();
    let entries = views.type_hierarchy_prepare.get(uri)?;
    let mut best_match: Option<(Range, Vec<TypeHierarchyItem>)> = None;
    for (range, items) in entries.iter() {
        if contains_position(range, pos) {
            if let Some(ref best) = best_match {
                if range_size(*range) < range_size(best.0) {
                    best_match = Some((*range, items.clone()));
                }
            } else {
                best_match = Some((*range, items.clone()));
            }
        }
    }
    best_match.map(|entry| entry.1)
}

#[allow(clippy::mutable_key_type)]
pub fn lookup_type_hierarchy_supertypes(
    views: &MaterializedViewStore,
    uri: &Url,
    pos: Position,
) -> Option<Vec<TypeHierarchyItem>> {
    views.wait_for_committed();
    let entries = views.type_hierarchy_supertypes.get(uri)?;
    let mut best_match: Option<(Range, Vec<TypeHierarchyItem>)> = None;
    for (range, items) in entries.iter() {
        if contains_position(range, pos) {
            if let Some(ref best) = best_match {
                if range_size(*range) < range_size(best.0) {
                    best_match = Some((*range, items.clone()));
                }
            } else {
                best_match = Some((*range, items.clone()));
            }
        }
    }
    best_match.map(|entry| entry.1)
}

#[allow(clippy::mutable_key_type)]
pub fn lookup_type_hierarchy_subtypes(
    views: &MaterializedViewStore,
    uri: &Url,
    pos: Position,
) -> Option<Vec<TypeHierarchyItem>> {
    views.wait_for_committed();
    let entries = views.type_hierarchy_subtypes.get(uri)?;
    let mut best_match: Option<(Range, Vec<TypeHierarchyItem>)> = None;
    for (range, items) in entries.iter() {
        if contains_position(range, pos) {
            if let Some(ref best) = best_match {
                if range_size(*range) < range_size(best.0) {
                    best_match = Some((*range, items.clone()));
                }
            } else {
                best_match = Some((*range, items.clone()));
            }
        }
    }
    best_match.map(|entry| entry.1)
}
