use std::collections::HashMap;
use wasm4pm_compat::ocel::validate::*;
use wasm4pm_compat::ocel::*;

#[test]
fn test_ocel_validation_missing_objects() {
    let log = OCEL {
        event_types: vec![],
        object_types: vec![],
        events: vec![OCELEvent::new("e1".to_string(), "Event")], // No relationships!
        objects: vec![OCELObject::new("o1".to_string(), "Object")],
    };

    let report = validate(&log, &HashMap::new());
    assert!(!report.valid);
    assert!(!report.errors.is_empty());
}

#[test]
fn test_ocel_validation_success() {
    let mut ev1 = OCELEvent::new("e1".to_string(), "Event");
    ev1.relationships
        .push(OCELRelationship::new("e1".to_string(), "o1".to_string()));

    let log = OCEL {
        event_types: vec![],
        object_types: vec![],
        events: vec![ev1],
        objects: vec![OCELObject::new("o1".to_string(), "Object")],
    };

    let report = validate(&log, &HashMap::new());
    assert!(report.valid);
    assert!(report.errors.is_empty());
}
