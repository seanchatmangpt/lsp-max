use serde_json::Value;
use std::fs;
use std::path::Path;

const EXPECTED_COUNT: usize = 10;
const EXPECTED_IDS: &[&str] = &[
    "asp",
    "bayesian_network",
    "cbr",
    "eliza",
    "frames_inheritance",
    "ltl_monitor",
    "meta_reasoning",
    "mycin",
    "pomdp",
    "llm",
];

fn load_registry() -> Value {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("breeds/registry.json");
    let raw = fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("registry.json not readable at {:?}: {}", path, e));
    serde_json::from_str(&raw).expect("registry.json valid JSON")
}

#[test]
fn test_breed_registry_scaffold_structure() {
    let registry = load_registry();
    let breeds = registry
        .get("breeds")
        .and_then(|b| b.as_array())
        .expect("registry must have a 'breeds' array");

    assert_eq!(breeds.len(), EXPECTED_COUNT, "breed count mismatch");

    for breed in breeds {
        let breed_id = breed
            .get("breed_id")
            .and_then(|v| v.as_str())
            .expect("each breed must have a non-null 'breed_id' string");
        assert!(!breed_id.is_empty(), "breed_id must be non-empty");

        let status = breed
            .get("status")
            .and_then(|v| v.as_str())
            .unwrap_or_else(|| panic!("breed '{}' missing 'status'", breed_id));
        assert_eq!(
            status, "CANDIDATE",
            "breed '{}' status must be CANDIDATE, not '{}'",
            breed_id, status
        );

        assert!(
            breed.get("oracle_value").and_then(|v| v.as_f64()).is_some(),
            "breed '{}' must have a numeric 'oracle_value'",
            breed_id
        );
        assert!(
            breed.get("module_stem").and_then(|v| v.as_str()).is_some(),
            "breed '{}' must have a 'module_stem' string",
            breed_id
        );
    }
}

#[test]
fn test_breed_registry_expected_ids_present() {
    let registry = load_registry();
    let breeds = registry
        .get("breeds")
        .and_then(|b| b.as_array())
        .expect("breeds array");

    let present_ids: Vec<&str> = breeds
        .iter()
        .filter_map(|b| b.get("breed_id").and_then(|v| v.as_str()))
        .collect();

    for expected_id in EXPECTED_IDS {
        assert!(
            present_ids.contains(expected_id),
            "expected breed_id '{}' not found in registry",
            expected_id
        );
    }
}

#[test]
fn test_breed_fixture_paper_values() {
    let fixtures_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/papers");

    let fixtures_present: Vec<&&str> = EXPECTED_IDS
        .iter()
        .filter(|id| fixtures_dir.join(format!("{}.json", id)).exists())
        .collect();

    assert!(
        fixtures_present.len() >= 3,
        "expected fixtures for at least 3 breeds, found {}",
        fixtures_present.len()
    );

    for breed_id in &fixtures_present {
        let path = fixtures_dir.join(format!("{}.json", breed_id));
        let raw = fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("fixture {:?} unreadable: {}", path, e));
        let fixture: Value = serde_json::from_str(&raw)
            .unwrap_or_else(|e| panic!("fixture {:?} invalid JSON: {}", path, e));

        assert!(
            fixture
                .get("paper_value")
                .and_then(|v| v.as_f64())
                .is_some(),
            "fixture for '{}' must have a numeric 'paper_value' (COG-005 analog)",
            breed_id
        );
    }
}
