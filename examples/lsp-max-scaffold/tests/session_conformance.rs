//! Process-Mined Session Conformance — integration test coverage.
//!
//! Tests the OCEL 2.0 event log, Declare constraint model, Oracle class
//! detectors (A8–A12), and the van der Aalst fitness metric from outside the
//! crate boundary.  Uses only the public API.

use lsp_max_scaffold::session_conformance::{
    replay_session, EventActivity, EventObjects, OracleClass, SessionLog,
};

fn append(log: &mut SessionLog, activity: EventActivity) {
    log.append(activity, EventObjects::default());
}

fn analysis() -> EventActivity {
    EventActivity::AnalysisRun {
        source_digest: "d41d8cd98f00b204e9800998ecf8427e".to_string(),
    }
}

fn finding() -> EventActivity {
    EventActivity::FindingProduced {
        code: "TEST-001".to_string(),
    }
}

fn receipt() -> EventActivity {
    EventActivity::ReceiptProduced {
        chain_head: "genesis".to_string(),
    }
}

// ---------------------------------------------------------------------------
// Fitness and baseline

#[test]
fn empty_session_is_fully_admitted() {
    let log = SessionLog::new();
    let r = replay_session(&log);
    assert_eq!(r.status, "ADMITTED");
    assert!((r.fitness - 1.0).abs() < f64::EPSILON);
}

#[test]
fn well_formed_session_is_admitted() {
    let mut log = SessionLog::new();
    append(&mut log, EventActivity::DocumentOpened);
    append(&mut log, analysis());
    append(&mut log, finding());
    append(&mut log, receipt());
    append(&mut log, EventActivity::ReceiptVerified { admitted: true });
    append(&mut log, EventActivity::ChainVerified { intact: true });
    let r = replay_session(&log);
    assert_eq!(r.status, "ADMITTED", "fitness={}", r.fitness);
    assert!(r.violations.is_empty());
    assert!(r.oracle_hits.is_empty());
}

#[test]
fn fitness_is_in_unit_interval() {
    // Maximally violated session: every Oracle class triggered.
    let mut log = SessionLog::new();
    append(&mut log, receipt()); // A10: no prior analysis
    append(&mut log, EventActivity::ChainVerified { intact: false });
    append(&mut log, finding()); // A9: after broken chain
    for _ in 0..5 {
        append(&mut log, EventActivity::GateChecked { blocked: true }); // A12
    }
    let r = replay_session(&log);
    assert!(r.fitness >= 0.0, "fitness must not go negative");
    assert!(r.fitness <= 1.0, "fitness must not exceed 1.0");
}

// ---------------------------------------------------------------------------
// Declare constraint violations

#[test]
fn finding_with_no_following_receipt_violates_response() {
    let mut log = SessionLog::new();
    append(&mut log, analysis());
    append(&mut log, finding()); // no ReceiptProduced ever follows
    let r = replay_session(&log);
    let has = r.violations.iter().any(|v| v.constraint_name == "Response");
    assert!(
        has,
        "Response constraint must fire when finding has no receipt"
    );
}

#[test]
fn receipt_without_analysis_violates_precedence() {
    let mut log = SessionLog::new();
    append(&mut log, receipt()); // AnalysisRun never occurred
    let r = replay_session(&log);
    let has = r
        .violations
        .iter()
        .any(|v| v.constraint_name == "Precedence");
    assert!(
        has,
        "Precedence constraint must fire for ReceiptProduced without AnalysisRun"
    );
}

#[test]
fn verify_without_receipt_violates_precedence() {
    let mut log = SessionLog::new();
    append(&mut log, analysis());
    append(&mut log, EventActivity::ReceiptVerified { admitted: true }); // no ReceiptProduced
    let r = replay_session(&log);
    let has = r
        .violations
        .iter()
        .any(|v| v.constraint_name == "Precedence");
    assert!(
        has,
        "Precedence constraint must fire for ReceiptVerified without ReceiptProduced"
    );
}

// ---------------------------------------------------------------------------
// Oracle A10 — causal violation

#[test]
fn a10_fires_for_orphan_receipt() {
    let mut log = SessionLog::new();
    append(&mut log, receipt());
    let r = replay_session(&log);
    let has = r
        .oracle_hits
        .iter()
        .any(|h| h.class == OracleClass::A10CausalViolation);
    assert!(has, "A10 must fire for receipt without prior analysis");
}

#[test]
fn a10_clear_when_analysis_precedes_receipt() {
    let mut log = SessionLog::new();
    append(&mut log, analysis());
    append(&mut log, finding());
    append(&mut log, receipt());
    let r = replay_session(&log);
    let has = r
        .oracle_hits
        .iter()
        .any(|h| h.class == OracleClass::A10CausalViolation);
    assert!(!has, "A10 must not fire when analysis precedes receipt");
}

// ---------------------------------------------------------------------------
// Oracle A11 — Unknown collapse

#[test]
fn a11_fires_for_collapse_without_evidence() {
    let mut log = SessionLog::new();
    append(
        &mut log,
        EventActivity::AxisTransitioned {
            axis: "Protocol".to_string(),
            from: "Unknown".to_string(),
            to: "Admitted".to_string(),
        },
    );
    let r = replay_session(&log);
    let has = r
        .oracle_hits
        .iter()
        .any(|h| h.class == OracleClass::A11UnknownCollapse);
    assert!(
        has,
        "A11 must fire when Unknown collapses without ReceiptVerified evidence"
    );
}

#[test]
fn a11_clear_when_evidence_precedes_transition() {
    let mut log = SessionLog::new();
    append(&mut log, analysis());
    append(&mut log, finding());
    append(&mut log, receipt());
    append(&mut log, EventActivity::ReceiptVerified { admitted: true });
    append(
        &mut log,
        EventActivity::AxisTransitioned {
            axis: "Receipt".to_string(),
            from: "Unknown".to_string(),
            to: "Admitted".to_string(),
        },
    );
    let r = replay_session(&log);
    let has = r
        .oracle_hits
        .iter()
        .any(|h| h.class == OracleClass::A11UnknownCollapse);
    assert!(
        !has,
        "A11 must not fire when ReceiptVerified evidence precedes the transition"
    );
}

// ---------------------------------------------------------------------------
// Oracle A9 — temporal anomaly

#[test]
fn a9_fires_for_finding_after_broken_chain() {
    let mut log = SessionLog::new();
    append(&mut log, analysis());
    append(&mut log, EventActivity::ChainVerified { intact: false });
    append(&mut log, finding()); // illegal: chain is broken
    let r = replay_session(&log);
    let has = r
        .oracle_hits
        .iter()
        .any(|h| h.class == OracleClass::A9TemporalAnomaly);
    assert!(has, "A9 must fire for finding produced after broken chain");
}

// ---------------------------------------------------------------------------
// Oracle A12 — cyclic dependency

#[test]
fn a12_fires_at_threshold() {
    let mut log = SessionLog::new();
    for _ in 0..5 {
        append(&mut log, EventActivity::GateChecked { blocked: true });
    }
    let r = replay_session(&log);
    let has = r
        .oracle_hits
        .iter()
        .any(|h| h.class == OracleClass::A12CyclicDependency);
    assert!(has, "A12 must fire after 5 consecutive blocked gate checks");
}

#[test]
fn a12_resets_on_resolution() {
    let mut log = SessionLog::new();
    for _ in 0..4 {
        append(&mut log, EventActivity::GateChecked { blocked: true });
    }
    append(&mut log, EventActivity::GateChecked { blocked: false }); // reset
    for _ in 0..4 {
        append(&mut log, EventActivity::GateChecked { blocked: true });
    }
    let r = replay_session(&log);
    let has = r
        .oracle_hits
        .iter()
        .any(|h| h.class == OracleClass::A12CyclicDependency);
    assert!(
        !has,
        "A12 must not fire when gate resolves before threshold"
    );
}

// ---------------------------------------------------------------------------
// Oracle A8 — audit tampering

#[test]
fn a8_fires_when_refused_receipt_precedes_intact_chain() {
    let mut log = SessionLog::new();
    append(&mut log, analysis());
    append(&mut log, finding());
    append(&mut log, receipt());
    append(&mut log, EventActivity::ReceiptVerified { admitted: false });
    append(&mut log, EventActivity::ChainVerified { intact: true }); // suspicious
    let r = replay_session(&log);
    let has = r
        .oracle_hits
        .iter()
        .any(|h| h.class == OracleClass::A8AuditTampering);
    assert!(
        has,
        "A8 must fire when intact chain follows a refused receipt"
    );
}

// ---------------------------------------------------------------------------
// Serialization round-trip

#[test]
fn session_log_serializes_and_deserializes() {
    let mut log = SessionLog::new();
    append(&mut log, EventActivity::DocumentOpened);
    append(&mut log, analysis());
    append(&mut log, finding());
    append(&mut log, receipt());

    let json = serde_json::to_string(&log).expect("serialize");
    let restored: SessionLog = serde_json::from_str(&json).expect("deserialize");

    assert_eq!(log.events().len(), restored.events().len());
    assert_eq!(log.events(), restored.events());
}

#[test]
fn digest_is_stable_across_serialization_roundtrip() {
    let mut log = SessionLog::new();
    append(&mut log, EventActivity::DocumentOpened);
    append(&mut log, analysis());
    let before = log.digest();

    let json = serde_json::to_string(&log).unwrap();
    let restored: SessionLog = serde_json::from_str(&json).unwrap();
    let after = restored.digest();

    assert_eq!(
        before, after,
        "digest must survive serialization round-trip"
    );
}

// ---------------------------------------------------------------------------
// Server-wiring pattern: the event sequence emitted by ScaffoldServer handlers
// is the pattern that `record_and_check` appends.  These tests verify the
// session model against the exact event shapes the server produces, so that
// a regression in the server's event emission is caught here without needing
// a live LSP connection.

#[test]
fn server_did_open_event_sequence_is_conformant() {
    // ScaffoldServer::did_open emits: DocumentOpened -> AnalysisRun ->
    // [FindingProduced*] -- mirrored here.  The Declare model requires
    // AnalysisRun before any FindingProduced (Precedence) and that each
    // FindingProduced is followed by a ReceiptProduced (Response).
    // A clean document (no findings) produces no Response violation.
    let mut log = SessionLog::new();
    append(&mut log, EventActivity::DocumentOpened);
    append(
        &mut log,
        EventActivity::AnalysisRun { source_digest: "abc".into() },
    );
    // No findings for a clean document.
    let r = replay_session(&log);
    assert_eq!(
        r.status, "ADMITTED",
        "clean did_open sequence must be ADMITTED; violations: {:?}",
        r.violations
    );
}

#[test]
fn server_finding_without_receipt_is_partial() {
    // Mirrors what the server would produce for a document with one finding
    // that has not yet been receipt-wrapped (i.e., a mid-flight analysis).
    // The Response constraint fires because FindingProduced has no subsequent
    // ReceiptProduced.
    let mut log = SessionLog::new();
    append(&mut log, EventActivity::DocumentOpened);
    append(
        &mut log,
        EventActivity::AnalysisRun { source_digest: "xyz".into() },
    );
    append(
        &mut log,
        EventActivity::FindingProduced { code: "RVD-TEST-001".into() },
    );
    // No ReceiptProduced follows -- Declare Response constraint fires.
    let r = replay_session(&log);
    let has_response = r.violations.iter().any(|v| v.constraint_name == "Response");
    assert!(
        has_response,
        "FindingProduced without ReceiptProduced must trigger Response violation"
    );
    assert!(
        r.fitness < 1.0,
        "fitness must be below 1.0 when Declare constraint is violated"
    );
}

#[test]
fn server_finding_with_receipt_is_conformant() {
    // Full honest server cycle: DocumentOpened -> AnalysisRun -> FindingProduced
    // -> ReceiptProduced -> ReceiptVerified -> ChainVerified.
    let mut log = SessionLog::new();
    append(&mut log, EventActivity::DocumentOpened);
    append(
        &mut log,
        EventActivity::AnalysisRun { source_digest: "d8e8fca".into() },
    );
    append(
        &mut log,
        EventActivity::FindingProduced { code: "RVD-FORK-001".into() },
    );
    append(
        &mut log,
        EventActivity::ReceiptProduced { chain_head: "genesis".into() },
    );
    append(&mut log, EventActivity::ReceiptVerified { admitted: true });
    append(&mut log, EventActivity::ChainVerified { intact: true });
    let r = replay_session(&log);
    assert_eq!(
        r.status, "ADMITTED",
        "full honest cycle must be ADMITTED; fitness={}, violations={:?}",
        r.fitness,
        r.violations
    );
}

#[test]
fn incremental_conformance_check_detects_first_violation() {
    // Simulates the incremental checking done by `record_and_check`: after
    // each append the model is checked.  Verify that the first violation
    // appears at the correct position in the event sequence.
    let mut log = SessionLog::new();
    // Step 1: after DocumentOpened, no violations yet.
    append(&mut log, EventActivity::DocumentOpened);
    assert!(
        replay_session(&log).violations.is_empty(),
        "no violations after DocumentOpened alone"
    );
    // Step 2: after AnalysisRun, still no violations.
    append(
        &mut log,
        EventActivity::AnalysisRun { source_digest: "step2".into() },
    );
    assert!(
        replay_session(&log).violations.is_empty(),
        "no violations after AnalysisRun"
    );
    // Step 3: FindingProduced with no subsequent ReceiptProduced -- violation
    // will appear when we check after this append (Response constraint).
    append(
        &mut log,
        EventActivity::FindingProduced { code: "RVD-VICTORY-001".into() },
    );
    let r = replay_session(&log);
    assert!(
        r.violations.iter().any(|v| v.constraint_name == "Response"),
        "Response violation must appear immediately after FindingProduced with no receipt"
    );
}
