use lsp_max::max_protocol::{AnalysisBundle, SnapshotId};
/// max/snapshot and max/exportAnalysisBundle tests.
use std::time::Duration;

use super::common::{boot_server, cleanup_receipts, expect_result, wait_for_response, write_msg};

#[tokio::test(flavor = "current_thread")]
async fn test_max_snapshot_returns_snap_id() {
    let (tx, rx, _h, _guard) = boot_server().await;

    write_msg(
        &tx,
        serde_json::json!({"jsonrpc":"2.0","id":1,"method":"max/snapshot"}),
    )
    .await;
    let resp = wait_for_response(rx, 1, Duration::from_millis(300)).await;

    let snap_id: SnapshotId = serde_json::from_value(expect_result(&resp).clone()).unwrap();
    assert!(
        snap_id.0.starts_with("snap-"),
        "snapshot id must start with 'snap-', got: {}",
        snap_id.0
    );
    cleanup_receipts();
}

#[tokio::test(flavor = "current_thread")]
async fn test_max_export_analysis_bundle() {
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
            "method":"max/exportAnalysisBundle",
            "params": serde_json::to_value(snap_id.clone()).unwrap()
        }),
    )
    .await;
    let resp = wait_for_response(rx, 2, Duration::from_millis(300)).await;
    let bundle: AnalysisBundle = serde_json::from_value(expect_result(&resp).clone()).unwrap();
    assert_eq!(
        bundle.snapshot_id.0, snap_id.0,
        "bundle snapshot_id must match"
    );
    let named_count = bundle
        .diagnostics
        .iter()
        .filter(
            |d| !matches!(&d.law_axis, lsp_max::max_protocol::LawAxis::Custom(s) if s.is_empty()),
        )
        .count();
    assert_eq!(
        bundle.capability_vector.gaps.len(),
        named_count,
        "gaps must equal diagnostics with a named law_axis"
    );
    let expected_actions: usize = bundle
        .diagnostics
        .iter()
        .map(|d| d.repair_actions.len())
        .sum();
    assert_eq!(
        bundle.actions.len(),
        expected_actions,
        "actions must equal total repair_actions across all diagnostics"
    );
    let _ = &bundle.conformance_vector;
    let _ = &bundle.receipts;
    cleanup_receipts();
}

#[tokio::test(flavor = "current_thread")]
async fn test_max_export_analysis_bundle_gap_paths_non_empty() {
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
            "method":"max/exportAnalysisBundle",
            "params": serde_json::to_value(snap_id).unwrap()
        }),
    )
    .await;
    let resp = wait_for_response(rx.clone(), 2, Duration::from_millis(300)).await;
    let bundle: AnalysisBundle = serde_json::from_value(expect_result(&resp).clone()).unwrap();

    for gap in &bundle.capability_vector.gaps {
        assert!(
            !gap.capability_path.is_empty(),
            "CapabilityGap capability_path must not be empty"
        );
        assert!(
            !gap.reason.is_empty(),
            "CapabilityGap reason must not be empty"
        );
    }

    let all_repair_descs: Vec<String> = bundle
        .diagnostics
        .iter()
        .flat_map(|d| d.repair_actions.iter().map(|ra| ra.description.clone()))
        .collect();
    for action in &bundle.actions {
        assert!(
            !action.action.title.is_empty(),
            "bundle action title must not be empty"
        );
        assert!(
            all_repair_descs.contains(&action.action.title),
            "bundle action '{}' must match a diagnostic repair description",
            action.action.title
        );
    }
    cleanup_receipts();
}

#[tokio::test(flavor = "current_thread")]
async fn test_max_export_analysis_bundle_actions_match_repair_actions() {
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
            "method":"max/exportAnalysisBundle",
            "params": serde_json::to_value(snap_id).unwrap()
        }),
    )
    .await;
    let resp = wait_for_response(rx.clone(), 2, Duration::from_millis(300)).await;
    let bundle: AnalysisBundle = serde_json::from_value(expect_result(&resp).clone()).unwrap();

    let expected: usize = bundle
        .diagnostics
        .iter()
        .map(|d| d.repair_actions.len())
        .sum();
    assert_eq!(
        bundle.actions.len(),
        expected,
        "bundle.actions count ({}) must equal total repair_actions ({})",
        bundle.actions.len(),
        expected
    );
    cleanup_receipts();
}
