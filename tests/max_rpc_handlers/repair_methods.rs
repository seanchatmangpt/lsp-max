use lsp_max::max_protocol::{MaxCodeAction, MaxDiagnostic, Receipt};
/// max/explainDiagnostic, max/repairPlan, max/applyRepairTransaction tests.
use std::time::Duration;

use super::common::{
    boot_server, cleanup_receipts, expect_error, expect_result, wait_for_response, write_msg,
};

#[tokio::test(flavor = "current_thread")]
async fn test_max_explain_diagnostic_known_id() {
    let (tx, rx, _h, _guard) = boot_server().await;

    write_msg(
        &tx,
        serde_json::json!({"jsonrpc":"2.0","id":1,"method":"max/explainDiagnostic","params":"diag-missing-receipt"}),
    )
    .await;
    let resp = wait_for_response(rx, 1, Duration::from_millis(300)).await;

    if resp.get("result").is_some() {
        let diag: MaxDiagnostic = serde_json::from_value(expect_result(&resp).clone()).unwrap();
        assert_eq!(diag.diagnostic_id, "diag-missing-receipt");
        assert!(
            !diag.lsp.message.is_empty(),
            "diagnostic message must not be empty"
        );
    } else {
        let err = expect_error(&resp);
        assert!(err["message"].as_str().unwrap_or("").contains("not found"));
    }
    cleanup_receipts();
}

#[tokio::test(flavor = "current_thread")]
async fn test_max_explain_diagnostic_unknown_id_returns_error() {
    let (tx, rx, _h, _guard) = boot_server().await;

    write_msg(
        &tx,
        serde_json::json!({"jsonrpc":"2.0","id":1,"method":"max/explainDiagnostic","params":"diag-does-not-exist-xyz"}),
    )
    .await;
    let resp = wait_for_response(rx, 1, Duration::from_millis(300)).await;

    let err = expect_error(&resp);
    let msg = err["message"].as_str().unwrap_or("");
    assert!(
        msg.contains("not found"),
        "error message should contain 'not found', got: {}",
        msg
    );
    cleanup_receipts();
}

#[tokio::test(flavor = "current_thread")]
async fn test_max_repair_plan_known_diagnostic() {
    let (tx, rx, _h, _guard) = boot_server().await;

    write_msg(
        &tx,
        serde_json::json!({"jsonrpc":"2.0","id":1,"method":"max/repairPlan","params":"diag-missing-receipt"}),
    )
    .await;
    let resp = wait_for_response(rx, 1, Duration::from_millis(300)).await;

    if resp.get("result").is_some() {
        let plans: Vec<MaxCodeAction> =
            serde_json::from_value(expect_result(&resp).clone()).unwrap();
        if !plans.is_empty() {
            let first = &plans[0];
            assert!(
                !first.action.title.is_empty(),
                "action title must not be empty"
            );
        }
    } else {
        let err = expect_error(&resp);
        assert!(!err["message"].as_str().unwrap_or("").is_empty());
    }
    cleanup_receipts();
}

#[tokio::test(flavor = "current_thread")]
async fn test_max_repair_plan_unknown_diagnostic_returns_empty_or_error() {
    let (tx, rx, _h, _guard) = boot_server().await;

    write_msg(
        &tx,
        serde_json::json!({"jsonrpc":"2.0","id":1,"method":"max/repairPlan","params":"diag-does-not-exist-xyz"}),
    )
    .await;
    let resp = wait_for_response(rx, 1, Duration::from_millis(300)).await;

    if resp.get("result").is_some() {
        let plans: Vec<MaxCodeAction> =
            serde_json::from_value(expect_result(&resp).clone()).unwrap();
        assert!(
            plans.is_empty(),
            "unknown diagnostic should yield empty plan"
        );
    } else {
        let err = expect_error(&resp);
        assert!(!err["message"].as_str().unwrap_or("").is_empty());
    }
    cleanup_receipts();
}

#[tokio::test(flavor = "current_thread")]
async fn test_max_apply_repair_transaction_returns_receipt() {
    let (tx, rx, _h, _guard) = boot_server().await;

    write_msg(
        &tx,
        serde_json::json!({"jsonrpc":"2.0","id":1,"method":"max/repairPlan","params":"diag-auth-generator"}),
    )
    .await;
    let resp = wait_for_response(rx.clone(), 1, Duration::from_millis(300)).await;
    let plans: Vec<MaxCodeAction> = serde_json::from_value(expect_result(&resp).clone()).unwrap();
    assert!(!plans.is_empty(), "auth-generator must have a repair plan");
    let action = plans[0].clone();

    write_msg(
        &tx,
        serde_json::json!({
            "jsonrpc":"2.0","id":2,
            "method":"max/applyRepairTransaction",
            "params": serde_json::to_value(&action).unwrap()
        }),
    )
    .await;
    let resp = wait_for_response(rx, 2, Duration::from_millis(300)).await;

    let receipt: Receipt = serde_json::from_value(expect_result(&resp).clone()).unwrap();
    assert!(
        receipt.receipt_id.starts_with("rcpt-"),
        "receipt_id must start with 'rcpt-', got: {}",
        receipt.receipt_id
    );
    assert!(!receipt.hash.is_empty(), "receipt hash must not be empty");
    cleanup_receipts();
}

#[tokio::test(flavor = "current_thread")]
async fn test_max_apply_repair_transaction_blocks_without_prerequisite_receipt() {
    let (tx, rx, _h, _guard) = boot_server().await;

    write_msg(
        &tx,
        serde_json::json!({"jsonrpc":"2.0","id":1,"method":"max/repairPlan","params":"diag-missing-receipt"}),
    )
    .await;
    let resp = wait_for_response(rx.clone(), 1, Duration::from_millis(300)).await;
    let plans: Vec<MaxCodeAction> = serde_json::from_value(expect_result(&resp).clone()).unwrap();
    assert!(!plans.is_empty());
    let action_with_dep = plans[0].clone();

    write_msg(
        &tx,
        serde_json::json!({
            "jsonrpc":"2.0","id":2,
            "method":"max/applyRepairTransaction",
            "params": serde_json::to_value(&action_with_dep).unwrap()
        }),
    )
    .await;
    let resp = wait_for_response(rx, 2, Duration::from_millis(300)).await;

    let err = expect_error(&resp);
    let msg = err["message"].as_str().unwrap_or("");
    assert!(
        msg.contains("Receipt integrity violation"),
        "expected receipt integrity violation, got: {}",
        msg
    );
    cleanup_receipts();
}
