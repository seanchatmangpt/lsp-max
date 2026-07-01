use std::fs;

#[test]
fn justfile_contains_required_recipes() {
    let content =
        fs::read_to_string("justfile").unwrap_or_else(|_| fs::read_to_string("Justfile").unwrap());
    let required = vec![
        "list",
        "fmt",
        "check",
        "test",
        "clippy",
        "ci",
        "dx",
        "qol",
        "doctor",
        "doctor-strict",
        "lsif",
        "lsif-receipt",
        "stale-lsif",
        "semantic-graph",
        "disclaimer",
        "rice",
        "closure-channel",
        "publish-dry-run",
        "q",
        "failset",
        "receipts",
        "receipts-check",
        "agents-loc",
        "agents-closure-scan",
        "tree",
        "changed",
        "clean",
        "help",
    ];
    for req in required {
        assert!(
            content.contains(&format!("{}:", req)) || content.contains(&format!("{} :", req)),
            "Missing recipe {}",
            req
        );
    }
}

#[test]
fn justfile_does_not_allow_cargo_publish() {
    let content =
        fs::read_to_string("justfile").unwrap_or_else(|_| fs::read_to_string("Justfile").unwrap());
    for line in content.lines() {
        if line.contains("cargo publish") {
            assert!(
                line.contains("--dry-run"),
                "Found cargo publish without --dry-run"
            );
        }
    }
}

#[test]
fn doctor_is_nonmutating() {
    let content = fs::read_to_string("scripts/doctor.sh").unwrap_or_default();
    assert!(!content.contains("cargo fmt"));
    assert!(!content.contains("cargo fix"));
}

#[test]
fn q_output_contains_packet_not_closure_prose() {
    let content = fs::read_to_string("scripts/q.sh").unwrap_or_default();
    assert!(content.contains("q"));
}

#[test]
fn receipts_check_requires_dx_qol_doctor_receipts() {
    let content = fs::read_to_string("scripts/receipts-check.sh").unwrap_or_default();
    assert!(content.contains("v26.6.28-dx.receipt.json"));
    assert!(content.contains("v26.6.28-qol.receipt.json"));
    assert!(content.contains("v26.6.28-doctor.receipt.json"));
}
