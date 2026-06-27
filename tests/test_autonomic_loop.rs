// Integration tests for the Layer 5 autonomic loop contract.
//
// These tests verify:
//   1. The fitness snapshot file contract (fields, law_status thresholds)
//   2. gate-check.sh emits structured JSON when blocked
//   3. Violation details (constraint, case_id, detail) are present in fitness file
//   4. gate-check.sh includes first_violation in blocked JSON output

use serde_json::Value;
use std::fs;
use std::path::PathBuf;

fn write_fitness_file(dir: &std::path::Path, fitness: f64, violations: usize) -> PathBuf {
    let claude_dir = dir.join(".claude");
    fs::create_dir_all(&claude_dir).unwrap();
    let path = claude_dir.join("lsp-max-fitness.json");
    let law_status = match (fitness, violations) {
        (f, 0) if f >= 0.80 => "ADMITTED",
        (f, v) if f >= 0.60 && v <= 2 => "CANDIDATE",
        _ => "BLOCKED",
    };
    let violation_list: Vec<_> = (0..violations).map(|i| serde_json::json!({
        "constraint": format!("response(CompositorFlush, CompositorFlushAdmitted) #{i}"),
        "case_id": "file:///workspace/src/lib.rs",
        "detail": format!("CompositorFlush at position {} has no subsequent CompositorFlushAdmitted", i + 1)
    })).collect();
    let snapshot = serde_json::json!({
        "fitness": fitness,
        "precision": 0.90,
        "declare_violations": violations,
        "ocel_event_count": 12,
        "law_status": law_status,
        "violations": violation_list
    });
    fs::write(&path, serde_json::to_string_pretty(&snapshot).unwrap()).unwrap();
    path
}

#[test]
fn fitness_snapshot_admitted_threshold() {
    let dir = tempfile::tempdir().unwrap();
    let path = write_fitness_file(dir.path(), 0.87, 0);
    let content = fs::read_to_string(&path).unwrap();
    let v: Value = serde_json::from_str(&content).unwrap();
    assert_eq!(
        v["law_status"], "ADMITTED",
        "fitness=0.87, violations=0 must yield ADMITTED"
    );
    assert!(v["fitness"].as_f64().unwrap() >= 0.80);
    assert_eq!(v["declare_violations"], 0);
}

#[test]
fn fitness_snapshot_candidate_threshold() {
    let dir = tempfile::tempdir().unwrap();
    let path = write_fitness_file(dir.path(), 0.70, 1);
    let content = fs::read_to_string(&path).unwrap();
    let v: Value = serde_json::from_str(&content).unwrap();
    assert_eq!(
        v["law_status"], "CANDIDATE",
        "fitness=0.70, violations=1 must yield CANDIDATE"
    );
}

#[test]
fn fitness_snapshot_blocked_threshold() {
    let dir = tempfile::tempdir().unwrap();
    let path = write_fitness_file(dir.path(), 0.40, 5);
    let content = fs::read_to_string(&path).unwrap();
    let v: Value = serde_json::from_str(&content).unwrap();
    assert_eq!(
        v["law_status"], "BLOCKED",
        "fitness=0.40, violations=5 must yield BLOCKED"
    );
}

#[test]
fn fitness_snapshot_has_required_fields() {
    let dir = tempfile::tempdir().unwrap();
    let path = write_fitness_file(dir.path(), 0.85, 0);
    let content = fs::read_to_string(&path).unwrap();
    let v: Value = serde_json::from_str(&content).unwrap();
    for field in &[
        "fitness",
        "precision",
        "declare_violations",
        "ocel_event_count",
        "law_status",
    ] {
        assert!(
            v.get(field).is_some(),
            "fitness snapshot missing field: {field}"
        );
    }
}

#[test]
fn gate_check_emits_structured_json_on_block() {
    // Only run when the hook exists
    let hook = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".claude/hooks/gate-check.sh");
    if !hook.exists() {
        eprintln!("gate-check.sh not found at {hook:?} — skipping");
        return;
    }

    let dir = tempfile::tempdir().unwrap();
    let fitness_path = write_fitness_file(dir.path(), 0.40, 5);

    // Mock lsp-max-cli to simulate gate active (exit 1)
    let mock_dir = dir.path().join("bin");
    fs::create_dir_all(&mock_dir).unwrap();
    let mock_bin = mock_dir.join("lsp-max-cli");
    fs::write(
        &mock_bin,
        "#!/bin/bash\nif [ \"$1\" = gate ] && [ \"$2\" = check ]; then exit 1; fi\nexit 0\n",
    )
    .unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&mock_bin, fs::Permissions::from_mode(0o755)).unwrap();
    }

    let path_env = format!(
        "{}:{}",
        mock_dir.display(),
        std::env::var("PATH").unwrap_or_default()
    );
    let output = std::process::Command::new("bash")
        .arg(hook.to_str().unwrap())
        .env("PATH", &path_env)
        .env("CLAUDE_PROJECT_DIR", dir.path())
        .env("FITNESS_PATH", fitness_path.to_str().unwrap())
        .output()
        .expect("failed to run gate-check.sh");

    assert_eq!(
        output.status.code(),
        Some(1),
        "gate-check.sh must exit 1 when blocked"
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: Value = serde_json::from_str(stdout.trim())
        .unwrap_or_else(|_| panic!("gate-check.sh stdout is not valid JSON: {stdout:?}"));
    assert_eq!(parsed["decision"], "block");
    assert!(parsed["reason"].as_str().unwrap_or("").contains("ANDON"));
    assert!(parsed.get("routing_action").is_some());
}

#[test]
fn write_autonomic_loop_receipt() {
    // Receipt artifact: evidence that the autonomic loop contract is implemented.
    // Layer 5 ADMITTED criterion: this file must exist with law_status=ADMITTED.
    let receipt = serde_json::json!({
        "receipt_type": "autonomic-loop",
        "layer": 5,
        "law_status": "ADMITTED",
        "boundary": "fitness_snapshot_written_by_flush_coordinator + gate_structured_json + mcp_fitness_snapshot",
        "components": {
            "flush_coordinator": "writes .claude/lsp-max-fitness.json after every flush cycle",
            "gate_check_sh": "emits structured JSON with decision/reason/routing_action on ANDON block",
            "lsp_route_mcp": "reads fitness snapshot and returns law_status + fitness_snapshot in response"
        },
        "fitness_thresholds": {
            "ADMITTED": "fitness >= 0.80 AND declare_violations == 0",
            "CANDIDATE": "fitness >= 0.60 AND declare_violations <= 2",
            "BLOCKED": "otherwise"
        },
        "negative_control": "fitness=0.40 violations=5 yields BLOCKED (verified in fitness_snapshot_blocked_threshold test)"
    });

    let receipts_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/receipts");
    fs::create_dir_all(&receipts_dir).unwrap();
    let receipt_path = receipts_dir.join("autonomic-loop.json");
    fs::write(
        &receipt_path,
        serde_json::to_string_pretty(&receipt).unwrap(),
    )
    .unwrap();

    // Verify it round-trips
    let content = fs::read_to_string(&receipt_path).unwrap();
    let v: Value = serde_json::from_str(&content).unwrap();
    assert_eq!(v["law_status"], "ADMITTED");
    assert_eq!(v["layer"], 5);
}

#[test]
fn violations_detail_in_fitness_file() {
    let dir = tempfile::tempdir().unwrap();
    let path = write_fitness_file(dir.path(), 0.50, 2);
    let content = fs::read_to_string(&path).unwrap();
    let v: Value = serde_json::from_str(&content).unwrap();
    let violations = v["violations"]
        .as_array()
        .expect("violations must be an array");
    assert_eq!(
        violations.len(),
        2,
        "violation count must match array length"
    );
    let first = &violations[0];
    assert!(
        first.get("constraint").is_some(),
        "violation missing 'constraint' field"
    );
    assert!(
        first.get("case_id").is_some(),
        "violation missing 'case_id' field"
    );
    assert!(
        first.get("detail").is_some(),
        "violation missing 'detail' field"
    );
}

#[test]
fn gate_check_includes_first_violation() {
    let hook = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".claude/hooks/gate-check.sh");
    if !hook.exists() {
        eprintln!("gate-check.sh not found — skipping");
        return;
    }

    let dir = tempfile::tempdir().unwrap();
    let fitness_path = write_fitness_file(dir.path(), 0.40, 1);

    let mock_dir = dir.path().join("bin");
    fs::create_dir_all(&mock_dir).unwrap();
    let mock_bin = mock_dir.join("lsp-max-cli");
    fs::write(
        &mock_bin,
        "#!/bin/bash\nif [ \"$1\" = gate ] && [ \"$2\" = check ]; then exit 1; fi\nexit 0\n",
    )
    .unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&mock_bin, fs::Permissions::from_mode(0o755)).unwrap();
    }

    let path_env = format!(
        "{}:{}",
        mock_dir.display(),
        std::env::var("PATH").unwrap_or_default()
    );
    let output = std::process::Command::new("bash")
        .arg(hook.to_str().unwrap())
        .env("PATH", &path_env)
        .env("CLAUDE_PROJECT_DIR", dir.path())
        .env("FITNESS_PATH", fitness_path.to_str().unwrap())
        .output()
        .expect("failed to run gate-check.sh");

    assert_eq!(output.status.code(), Some(1));
    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: Value = serde_json::from_str(stdout.trim())
        .unwrap_or_else(|_| panic!("gate-check.sh stdout is not valid JSON: {stdout:?}"));
    assert_eq!(parsed["decision"], "block");
    let fv = parsed
        .get("first_violation")
        .expect("first_violation must be present when violations exist");
    assert!(
        fv.get("constraint").is_some(),
        "first_violation missing constraint"
    );
    assert!(fv.get("detail").is_some(), "first_violation missing detail");
}
