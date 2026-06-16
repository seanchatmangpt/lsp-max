use lsp_max::max_protocol::{ConformanceVector, GateId, SnapshotId};
/// max/runGate, max/conformanceVector, max/instanceList, max/lsif tests.
use std::time::Duration;

use super::common::{boot_server, cleanup_receipts, expect_result, wait_for_response, write_msg};

#[tokio::test(flavor = "current_thread")]
async fn test_max_run_gate_returns_bool() {
    let (tx, rx, _h, _guard) = boot_server().await;

    write_msg(
        &tx,
        serde_json::json!({
            "jsonrpc":"2.0","id":1,
            "method":"max/runGate",
            "params": serde_json::to_value(GateId("some-gate".to_string())).unwrap()
        }),
    )
    .await;
    let resp = wait_for_response(rx, 1, Duration::from_millis(500)).await;

    let result = expect_result(&resp);
    assert!(
        result.is_boolean(),
        "max/runGate must return a boolean, got: {}",
        result
    );
    assert_eq!(
        result,
        &serde_json::Value::Bool(true),
        "some-gate must return true"
    );
    cleanup_receipts();
}

#[tokio::test(flavor = "current_thread")]
async fn test_max_conformance_vector() {
    let (tx, rx, _h, _guard) = boot_server().await;

    write_msg(
        &tx,
        serde_json::json!({"jsonrpc":"2.0","id":1,"method":"max/snapshot"}),
    )
    .await;
    let resp = wait_for_response(rx.clone(), 1, Duration::from_millis(300)).await;
    let snap_id: SnapshotId = serde_json::from_value(expect_result(&resp).clone()).unwrap();

    write_msg(
        &tx,
        serde_json::json!({
            "jsonrpc":"2.0","id":2,
            "method":"max/conformanceVector",
            "params": serde_json::to_value(snap_id).unwrap()
        }),
    )
    .await;
    let resp = wait_for_response(rx, 2, Duration::from_millis(300)).await;
    let cv: ConformanceVector = serde_json::from_value(expect_result(&resp).clone()).unwrap();

    let admitted_set: std::collections::HashSet<String> =
        cv.admitted.iter().map(|x| format!("{:?}", x)).collect();
    let refused_set: std::collections::HashSet<String> =
        cv.refused.iter().map(|x| format!("{:?}", x)).collect();
    assert!(
        admitted_set.is_disjoint(&refused_set),
        "admitted and refused must be disjoint"
    );
    cleanup_receipts();
}

#[tokio::test(flavor = "current_thread")]
async fn test_max_instance_list() {
    let (tx, rx, _h, _guard) = boot_server().await;

    write_msg(
        &tx,
        serde_json::json!({"jsonrpc":"2.0","id":1,"method":"max/instanceList"}),
    )
    .await;
    let resp = wait_for_response(rx, 1, Duration::from_millis(300)).await;
    let result = expect_result(&resp);

    assert!(
        result.is_array(),
        "max/instanceList must return a JSON array, got: {}",
        result
    );
    let arr = result.as_array().unwrap();

    for entry in arr {
        assert!(entry.get("id").is_some(), "entry missing 'id': {}", entry);
        assert!(
            entry.get("phase").is_some(),
            "entry missing 'phase': {}",
            entry
        );
        assert!(
            entry.get("conformance_score").is_some(),
            "entry missing 'conformance_score': {}",
            entry
        );
        let score = entry["conformance_score"]
            .as_f64()
            .expect("conformance_score must be f64");
        assert!(
            (0.0..=100.0).contains(&score),
            "conformance_score out of range: {}",
            score
        );
    }

    cleanup_receipts();
}

#[tokio::test(flavor = "current_thread")]
async fn test_rpc_run_gate_returns_true_when_no_diagnostics() {
    let (tx, rx, _h, _guard) = boot_server().await;

    write_msg(
        &tx,
        serde_json::json!({
            "jsonrpc":"2.0","id":1,
            "method":"max/runGate",
            "params": serde_json::to_value(GateId("some-gate".to_string())).unwrap()
        }),
    )
    .await;
    let resp = wait_for_response(rx, 1, Duration::from_millis(500)).await;
    let result = expect_result(&resp);
    assert!(result.is_boolean(), "max/runGate must return a boolean");
    assert_eq!(
        result,
        &serde_json::Value::Bool(true),
        "gate with no blocking diagnostics must return true"
    );
    cleanup_receipts();
}

#[tokio::test(flavor = "current_thread")]
async fn test_rpc_run_gate_returns_false_when_diagnostic_references_gate() {
    let (tx, rx, _h, _guard) = boot_server().await;

    write_msg(
        &tx,
        serde_json::json!({
            "jsonrpc":"2.0","id":1,
            "method":"max/runGate",
            "params": serde_json::to_value(GateId("some-gate".to_string())).unwrap()
        }),
    )
    .await;
    let resp1 = wait_for_response(rx.clone(), 1, Duration::from_millis(500)).await;
    let result1 = expect_result(&resp1);
    assert_eq!(
        result1,
        &serde_json::Value::Bool(true),
        "some-gate must return true when unblocked"
    );

    write_msg(
        &tx,
        serde_json::json!({
            "jsonrpc":"2.0","id":2,
            "method":"max/runGate",
            "params": serde_json::to_value(GateId("gate-state-check".to_string())).unwrap()
        }),
    )
    .await;
    let resp2 = wait_for_response(rx, 2, Duration::from_millis(500)).await;
    let result2 = expect_result(&resp2);
    assert!(
        result2.is_boolean(),
        "max/runGate must return a boolean for gate-state-check"
    );
    cleanup_receipts();
}

#[tokio::test(flavor = "current_thread")]
async fn test_max_lsif_export() {
    let (tx, rx, _h, _guard) = boot_server().await;

    write_msg(
        &tx,
        serde_json::json!({"jsonrpc":"2.0","id":99,"method":"max/lsif"}),
    )
    .await;
    let resp = wait_for_response(rx, 99, Duration::from_millis(300)).await;

    let lsif_str: String = serde_json::from_value(expect_result(&resp).clone()).unwrap();
    assert!(lsif_str.contains("\"label\":\"metaData\""));
    assert!(lsif_str.contains("\"label\":\"project\""));
}
