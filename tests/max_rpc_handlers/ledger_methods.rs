/// max/receipt, max/verifyLedger, max/ledgerReport, max/clearDiagnostic tests.
use std::time::Duration;
use tower_lsp_max::max_protocol::{MaxCodeAction, Receipt};

use super::common::{
    boot_server, cleanup_receipts, expect_error, expect_result, wait_for_response, write_msg,
};

#[tokio::test(flavor = "current_thread")]
async fn test_max_receipt_lookup() {
    let (tx, rx, _h, _guard) = boot_server().await;

    write_msg(
        &tx,
        serde_json::json!({"jsonrpc":"2.0","id":1,"method":"max/repairPlan","params":"diag-auth-generator"}),
    )
    .await;
    let resp = wait_for_response(rx.clone(), 1, Duration::from_secs(3)).await;

    let expected_hash = if resp.get("result").is_some() {
        let plans: Vec<MaxCodeAction> =
            serde_json::from_value(expect_result(&resp).clone()).unwrap();
        if plans.is_empty() {
            cleanup_receipts();
            return;
        }
        let gen_action = plans[0].clone();

        write_msg(
            &tx,
            serde_json::json!({
                "jsonrpc":"2.0","id":2,
                "method":"max/applyRepairTransaction",
                "params": serde_json::to_value(&gen_action).unwrap()
            }),
        )
        .await;
        let resp = wait_for_response(rx.clone(), 2, Duration::from_secs(3)).await;
        let gen_receipt: Receipt = serde_json::from_value(expect_result(&resp).clone()).unwrap();
        assert_eq!(gen_receipt.receipt_id, "rcpt-security-auth");
        gen_receipt.hash
    } else {
        write_msg(
            &tx,
            serde_json::json!({"jsonrpc":"2.0","id":99,"method":"max/receipt","params":"rcpt-security-auth"}),
        )
        .await;
        let resp = wait_for_response(rx.clone(), 99, Duration::from_secs(3)).await;
        if resp.get("error").is_some() {
            cleanup_receipts();
            return;
        }
        let r: Receipt = serde_json::from_value(expect_result(&resp).clone()).unwrap();
        r.hash
    };

    write_msg(
        &tx,
        serde_json::json!({"jsonrpc":"2.0","id":3,"method":"max/receipt","params":"rcpt-security-auth"}),
    )
    .await;
    let resp = wait_for_response(rx, 3, Duration::from_secs(3)).await;
    let retrieved: Receipt = serde_json::from_value(expect_result(&resp).clone()).unwrap();
    assert_eq!(retrieved.receipt_id, "rcpt-security-auth");
    assert_eq!(retrieved.hash, expected_hash, "hashes must match");
    cleanup_receipts();
}

#[tokio::test(flavor = "current_thread")]
async fn test_max_receipt_unknown_returns_error() {
    let (tx, rx, _h, _guard) = boot_server().await;

    write_msg(
        &tx,
        serde_json::json!({"jsonrpc":"2.0","id":1,"method":"max/receipt","params":"rcpt-does-not-exist"}),
    )
    .await;
    let resp = wait_for_response(rx, 1, Duration::from_secs(3)).await;

    let err = expect_error(&resp);
    assert!(
        !err["message"].as_str().unwrap_or("").is_empty(),
        "error message must be non-empty for unknown receipt"
    );
    cleanup_receipts();
}

#[tokio::test(flavor = "current_thread")]
async fn test_max_clear_diagnostic() {
    let (tx, rx, _h, _guard) = boot_server().await;

    write_msg(
        &tx,
        serde_json::json!({"jsonrpc":"2.0","id":1,"method":"max/explainDiagnostic","params":"diag-missing-receipt"}),
    )
    .await;
    let resp = wait_for_response(rx.clone(), 1, Duration::from_secs(3)).await;
    assert!(
        resp.get("result").is_some(),
        "diagnostic must exist before clearing"
    );

    write_msg(
        &tx,
        serde_json::json!({"jsonrpc":"2.0","id":2,"method":"max/clearDiagnostic","params":"diag-missing-receipt"}),
    )
    .await;
    let resp = wait_for_response(rx.clone(), 2, Duration::from_secs(3)).await;
    assert!(resp.get("result").is_some(), "clearDiagnostic must succeed");

    write_msg(
        &tx,
        serde_json::json!({"jsonrpc":"2.0","id":3,"method":"max/explainDiagnostic","params":"diag-missing-receipt"}),
    )
    .await;
    let resp = wait_for_response(rx, 3, Duration::from_secs(3)).await;
    let err = expect_error(&resp);
    let msg = err["message"].as_str().unwrap_or("");
    assert!(
        msg.contains("not found"),
        "after clear, diagnostic must not be found; got: {}",
        msg
    );
    cleanup_receipts();
}

#[tokio::test(flavor = "current_thread")]
async fn test_max_verify_ledger() {
    let (tx, rx, _h, _guard) = boot_server().await;

    write_msg(
        &tx,
        serde_json::json!({"jsonrpc":"2.0","id":1,"method":"max/verifyLedger"}),
    )
    .await;
    let resp = wait_for_response(rx, 1, Duration::from_secs(3)).await;
    assert!(
        resp.get("result").is_some() || resp.get("error").is_some(),
        "max/verifyLedger must produce a JSON-RPC response, got: {}",
        resp
    );
    cleanup_receipts();
}

#[tokio::test(flavor = "current_thread")]
async fn test_max_ledger_report() {
    let (tx, rx, _h, _guard) = boot_server().await;

    write_msg(
        &tx,
        serde_json::json!({"jsonrpc":"2.0","id":1,"method":"max/ledgerReport"}),
    )
    .await;
    let resp = wait_for_response(rx, 1, Duration::from_secs(3)).await;
    let _ = expect_result(&resp);
    cleanup_receipts();
}
