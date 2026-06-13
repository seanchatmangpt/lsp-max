//! Validates the perf-refactors admission receipt.
//!
//! This test is the gate check: it reads receipts/perf-refactors.receipt.json,
//! asserts required fields are present and non-empty, and confirms the
//! checkpoint token matches the expected value for this CalVer release.
//!
//! To generate the receipt: `just bench-admit`
//! To run this test:        `just test-perf-admission`

use serde_json::Value;
use std::fs;
use std::path::Path;

const RECEIPT_PATH: &str = "receipts/perf-refactors.receipt.json";
const EXPECTED_CHECKPOINT: &str = "PERF-REFACTORS-ADMITTED-26.6.9";

#[test]
fn perf_refactors_receipt_exists() {
    assert!(
        Path::new(RECEIPT_PATH).exists(),
        "Receipt not found at {RECEIPT_PATH}. Run `just bench-admit` to generate it.",
    );
}

#[test]
fn perf_refactors_receipt_is_valid_json() {
    let content = fs::read_to_string(RECEIPT_PATH)
        .unwrap_or_else(|_| panic!("Cannot read {RECEIPT_PATH}. Run `just bench-admit`."));
    serde_json::from_str::<Value>(&content)
        .unwrap_or_else(|e| panic!("Receipt at {RECEIPT_PATH} is not valid JSON: {e}"));
}

#[test]
fn perf_refactors_receipt_has_required_fields() {
    let content = fs::read_to_string(RECEIPT_PATH)
        .unwrap_or_else(|_| panic!("Cannot read {RECEIPT_PATH}. Run `just bench-admit`."));
    let val: Value = serde_json::from_str(&content).expect("valid json");

    let required = [
        "checkpoint",
        "boundary",
        "digest",
        "digest_algorithm",
        "output_digest",
        "raw_command",
    ];

    for field in &required {
        let present = val
            .get(field)
            .and_then(|v| v.as_str())
            .map(|s| !s.is_empty())
            .unwrap_or(false);
        assert!(
            present,
            "Receipt missing required field '{field}'. Re-run `just bench-admit`.",
        );
    }
}

#[test]
fn perf_refactors_receipt_checkpoint_matches_calver() {
    let content = fs::read_to_string(RECEIPT_PATH)
        .unwrap_or_else(|_| panic!("Cannot read {RECEIPT_PATH}. Run `just bench-admit`."));
    let val: Value = serde_json::from_str(&content).expect("valid json");

    let checkpoint = val
        .get("checkpoint")
        .and_then(|v| v.as_str())
        .expect("checkpoint field must be a string");

    assert_eq!(
        checkpoint, EXPECTED_CHECKPOINT,
        "Receipt checkpoint '{checkpoint}' does not match expected '{EXPECTED_CHECKPOINT}'",
    );
}

#[test]
fn perf_refactors_receipt_claims_cover_all_four_benchmarks() {
    let content = fs::read_to_string(RECEIPT_PATH)
        .unwrap_or_else(|_| panic!("Cannot read {RECEIPT_PATH}. Run `just bench-admit`."));
    let val: Value = serde_json::from_str(&content).expect("valid json");

    let claims = val
        .get("claims")
        .and_then(|c| c.as_object())
        .expect("receipt must have a 'claims' object");

    for key in ["B1", "B2", "B3", "B4", "B5", "B6"] {
        assert!(
            claims.contains_key(key),
            "Receipt claims missing benchmark '{key}'",
        );
    }
}

#[test]
fn perf_refactors_receipt_status_is_admitted() {
    let content = fs::read_to_string(RECEIPT_PATH)
        .unwrap_or_else(|_| panic!("Cannot read {RECEIPT_PATH}. Run `just bench-admit`."));
    let val: Value = serde_json::from_str(&content).expect("valid json");

    let status = val
        .get("status")
        .and_then(|v| v.as_str())
        .unwrap_or("UNKNOWN");

    assert_eq!(
        status, "ADMITTED",
        "Receipt status is '{status}', expected 'ADMITTED'",
    );
}
