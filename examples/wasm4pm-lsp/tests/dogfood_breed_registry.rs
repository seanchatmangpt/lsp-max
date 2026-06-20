use serde_json::Value;
use std::fs;
use std::path::Path;

const EXPECTED_BREED_COUNT: usize = 9;

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
];

#[test]
fn test_breed_registry_scaffold_structure() {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let registry_path = manifest_dir.join("breeds/registry.json");

    let raw = fs::read_to_string(&registry_path)
        .unwrap_or_else(|e| panic!("registry.json not readable at {:?}: {}", registry_path, e));

    let registry: Value =
        serde_json::from_str(&raw).unwrap_or_else(|e| panic!("registry.json parse error: {}", e));

    let breeds = registry
        .get("breeds")
        .and_then(|b| b.as_array())
        .expect("registry must have a 'breeds' array");

    assert_eq!(
        breeds.len(),
        EXPECTED_BREED_COUNT,
        "expected {} breed entries, found {}",
        EXPECTED_BREED_COUNT,
        breeds.len()
    );

    for breed in breeds {
        let breed_id = breed
            .get("breed_id")
            .and_then(|v| v.as_str())
            .expect("each breed must have a non-null 'breed_id' string");
        assert!(
            !breed_id.is_empty(),
            "breed_id must be non-empty (got empty string)"
        );

        let status = breed
            .get("status")
            .and_then(|v| v.as_str())
            .unwrap_or_else(|| panic!("breed '{}' missing 'status'", breed_id));
        assert_eq!(
            status, "CANDIDATE",
            "breed '{}' status must be CANDIDATE, got '{}'",
            breed_id, status
        );

        assert!(
            breed.get("oracle_value").and_then(|v| v.as_f64()).is_some(),
            "breed '{}' must have a numeric 'oracle_value'",
            breed_id
        );

        assert!(
            breed
                .get("module_stem")
                .and_then(|v| v.as_str())
                .is_some(),
            "breed '{}' must have a 'module_stem' string",
            breed_id
        );
    }
}

#[test]
fn test_breed_registry_expected_ids_present() {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let registry_path = manifest_dir.join("breeds/registry.json");

    let raw = fs::read_to_string(&registry_path).expect("registry.json readable");
    let registry: Value = serde_json::from_str(&raw).expect("registry.json valid JSON");

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
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let fixtures_dir = manifest_dir.join("tests/fixtures/papers");

    let fixtures_checked = EXPECTED_IDS
        .iter()
        .filter(|breed_id| {
            let path = fixtures_dir.join(format!("{}.json", breed_id));
            path.exists()
        })
        .count();

    assert!(
        fixtures_checked >= 3,
        "expected fixtures for at least 3 breeds, found {}",
        fixtures_checked
    );

    for breed_id in EXPECTED_IDS {
        let path = fixtures_dir.join(format!("{}.json", breed_id));
        if !path.exists() {
            continue;
        }

        let raw = fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("fixture {:?} unreadable: {}", path, e));
        let fixture: Value = serde_json::from_str(&raw)
            .unwrap_or_else(|e| panic!("fixture {:?} invalid JSON: {}", path, e));

        assert!(
            fixture.get("paper_value").and_then(|v| v.as_f64()).is_some(),
            "fixture for '{}' must have a numeric 'paper_value' (COG-005 analog)",
            breed_id
        );
    }
}
