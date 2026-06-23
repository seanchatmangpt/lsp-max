//! COG-010 Oracle Injection Scan
//!
//! Verifies that no breed source file contains a hard-coded oracle value literal
//! on a non-comment line (which would constitute oracle injection — a COG-010 violation).
//!
//! Pairs scanned: (module_stem, oracle_literal). Literals with oracle_value 0.0 or 1.0
//! are excluded — they appear too frequently in algorithm code to distinguish injection.
//!
//! On pass: writes tests/receipts/cog010-scan.json as an admission receipt.

use std::path::PathBuf;

fn crate_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// (module_stem, oracle_literal) pairs to scan for non-comment occurrences.
const SCAN_PAIRS: &[(&str, &str)] = &[
    ("bayesian_network", "0.284"),
    ("bayesian_network", "0.2842"),
    ("production_rules", "0.693"),
    ("cbr",              "0.85"),
    ("pomdp",            "0.969"),
];

struct Violation {
    module_stem:    &'static str,
    oracle_literal: &'static str,
    line_number:    usize,
    line_content:   String,
}

fn scan_for_violations() -> Vec<Violation> {
    let root = crate_root();
    let mut violations = Vec::new();

    for &(stem, literal) in SCAN_PAIRS {
        let path = root.join(format!("src/breeds/{stem}.rs"));
        let source = match std::fs::read_to_string(&path) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("COG-010 WARN: could not read {}: {e}", path.display());
                continue;
            }
        };

        for (idx, line) in source.lines().enumerate() {
            let trimmed = line.trim();
            if trimmed.starts_with("//") {
                continue;
            }
            if trimmed.contains(literal) {
                violations.push(Violation {
                    module_stem: stem,
                    oracle_literal: literal,
                    line_number: idx + 1,
                    line_content: line.to_string(),
                });
            }
        }
    }

    violations
}

fn write_receipt(violations: &[Violation]) {
    let root = crate_root();
    let receipts_dir = root.join("tests/receipts");
    let _ = std::fs::create_dir_all(&receipts_dir);

    let violation_records: Vec<serde_json::Value> = violations
        .iter()
        .map(|v| {
            serde_json::json!({
                "module_stem":    v.module_stem,
                "oracle_literal": v.oracle_literal,
                "line_number":    v.line_number,
                "line_content":   v.line_content
            })
        })
        .collect();

    let result = if violations.is_empty() { "ADMITTED" } else { "REFUSED" };

    let scanned_stems: Vec<&str> = {
        let mut seen = std::collections::HashSet::new();
        SCAN_PAIRS
            .iter()
            .filter(|(s, _)| seen.insert(*s))
            .map(|(s, _)| *s)
            .collect()
    };

    let receipt = serde_json::json!({
        "receipt_id":         format!("cog010-oracle-scan-{}", epoch_day()),
        "law":                "COG-010",
        "scanned_stems":      scanned_stems,
        "scan_pairs_checked": SCAN_PAIRS.len(),
        "result":             result,
        "violations":         violation_records,
        "scanned_at":         epoch_now()
    });

    let path = receipts_dir.join("cog010-scan.json");
    if let Ok(s) = serde_json::to_string_pretty(&receipt) {
        let _ = std::fs::write(&path, s);
    }
}

fn epoch_day() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    format!("{}", secs / 86400)
}

fn epoch_now() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let s = secs % 60;
    let m = (secs / 60) % 60;
    let h = (secs / 3600) % 24;
    let days = secs / 86400;
    format!("epoch+{}d {:02}:{:02}:{:02}Z", days, h, m, s)
}

#[test]
fn cog010_no_oracle_injection() {
    let violations = scan_for_violations();
    write_receipt(&violations);

    if !violations.is_empty() {
        for v in &violations {
            eprintln!(
                "COG-010 VIOLATION: {}:{} oracle '{}' on non-comment line: {}",
                v.module_stem, v.line_number, v.oracle_literal, v.line_content
            );
        }
        panic!(
            "COG-010: {} oracle injection violation(s) detected — see output above",
            violations.len()
        );
    }

    let root = crate_root();
    let receipt_path = root.join("tests/receipts/cog010-scan.json");
    assert!(
        receipt_path.exists(),
        "COG-010 receipt not written to {}",
        receipt_path.display()
    );

    println!(
        "COG-010 ADMITTED: {} scan pairs checked, 0 violations, receipt written to {}",
        SCAN_PAIRS.len(),
        receipt_path.display()
    );
}
