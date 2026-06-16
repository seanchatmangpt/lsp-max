//! Validates the compositor-scale admission receipt.
//!
//! This test is the gate check: it reads receipts/compositor-scale.receipt.json,
//! asserts required fields are present and non-empty, confirms the checkpoint
//! token matches the expected CalVer value, and verifies WASM4PM-CROWN aliases.
//!
//! To generate the receipt: `just bench-compositor`
//! To run this test:        `just test-compositor-admission`

use serde_json::Value;
use std::fs;
use std::path::Path;

const RECEIPT_PATH: &str = "receipts/compositor-scale.receipt.json";
const EXPECTED_CHECKPOINT: &str = "COMPOSITOR-SCALE-ADMITTED-26.6.9";

#[test]
#[ignore = "stress/perf: slow by design, run with --include-ignored"]
fn compositor_scale_receipt_exists() {
    assert!(
        Path::new(RECEIPT_PATH).exists(),
        "Receipt not found at {RECEIPT_PATH}. Run `just bench-compositor` to generate it.",
    );
}

#[test]
#[ignore = "stress/perf: slow by design, run with --include-ignored"]
fn compositor_scale_receipt_is_valid_json() {
    let content = fs::read_to_string(RECEIPT_PATH)
        .unwrap_or_else(|_| panic!("Cannot read {RECEIPT_PATH}. Run `just bench-compositor`."));
    serde_json::from_str::<Value>(&content)
        .unwrap_or_else(|e| panic!("Receipt at {RECEIPT_PATH} is not valid JSON: {e}"));
}

#[test]
#[ignore = "stress/perf: slow by design, run with --include-ignored"]
fn compositor_scale_receipt_has_required_fields() {
    let content = fs::read_to_string(RECEIPT_PATH)
        .unwrap_or_else(|_| panic!("Cannot read {RECEIPT_PATH}. Run `just bench-compositor`."));
    let val: Value = serde_json::from_str(&content).expect("valid json");

    // Core receipt fields + WASM4PM-CROWN law aliases (output_hash, run_id, replay_pointer).
    let required = [
        "checkpoint",
        "boundary",
        "digest",
        "digest_algorithm",
        "output_digest",
        "raw_command",
        "output_hash",
        "run_id",
        "replay_pointer",
    ];

    for field in &required {
        let present = val
            .get(field)
            .and_then(|v| v.as_str())
            .map(|s| !s.is_empty())
            .unwrap_or(false);
        assert!(
            present,
            "Receipt missing required field '{field}'. Re-run `just bench-compositor`.",
        );
    }
}

#[test]
#[ignore = "stress/perf: slow by design, run with --include-ignored"]
fn compositor_scale_receipt_checkpoint_matches_calver() {
    let content = fs::read_to_string(RECEIPT_PATH)
        .unwrap_or_else(|_| panic!("Cannot read {RECEIPT_PATH}. Run `just bench-compositor`."));
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
#[ignore = "stress/perf: slow by design, run with --include-ignored"]
fn compositor_scale_receipt_claims_cover_all_six_benchmarks() {
    let content = fs::read_to_string(RECEIPT_PATH)
        .unwrap_or_else(|_| panic!("Cannot read {RECEIPT_PATH}. Run `just bench-compositor`."));
    let val: Value = serde_json::from_str(&content).expect("valid json");

    let claims = val
        .get("claims")
        .and_then(|c| c.as_object())
        .expect("receipt must have a 'claims' object");

    for key in ["CS1", "CS2", "CS3", "CS4", "CS5", "CS6"] {
        assert!(
            claims.contains_key(key),
            "Receipt claims missing benchmark '{key}'. Re-run `just bench-compositor`.",
        );
    }
}

#[test]
#[ignore = "stress/perf: slow by design, run with --include-ignored"]
fn compositor_scale_receipt_status_is_admitted() {
    let content = fs::read_to_string(RECEIPT_PATH)
        .unwrap_or_else(|_| panic!("Cannot read {RECEIPT_PATH}. Run `just bench-compositor`."));
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
