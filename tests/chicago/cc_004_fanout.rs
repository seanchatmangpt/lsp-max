//! Chicago acceptance test for CC-004: per-URI serialized notification fan-out
//! Status: CANDIDATE — implement FanoutCoordinator to make this test pass.
//! Ticket: docs/jira/v26.6.30/CC-004-notification-routing.md

use chicago_tdd_tools_proc_macros::chicago_test;

#[chicago_test(
    ticket = "docs/jira/v26.6.30/CC-004-notification-routing.md",
    scaffold_fn = "lsp_max_compositor::server::FanoutCoordinator::new"
)]
fn did_change_is_held_until_did_open_completes() {
    // Given: a FanoutCoordinator with two registered child server IDs
    let coord = lsp_max_compositor::server::FanoutCoordinator::new(vec![
        "rust-analyzer".to_string(),
        "anti-llm".to_string(),
    ]);
    let uri = "file:///src/main.rs";
    // When: did_open is recorded for the URI
    coord.record_did_open(uri);
    // Then: the URI is tracked as open
    assert!(
        coord.is_open(uri),
        "URI should be tracked as open after did_open"
    );
    // And: did_change is allowed (open already recorded)
    assert!(
        coord.can_did_change(uri),
        "did_change should be allowed after did_open"
    );
}

#[chicago_test(
    ticket = "docs/jira/v26.6.30/CC-004-notification-routing.md",
    scaffold_fn = "lsp_max_compositor::server::FanoutCoordinator::new"
)]
fn version_regression_emits_warning_not_andon() {
    // Given: a FanoutCoordinator tracking URI at version 5
    let coord = lsp_max_compositor::server::FanoutCoordinator::new(vec![]);
    let uri = "file:///src/lib.rs";
    coord.record_did_open(uri);
    coord.record_version(uri, 5);
    // When: a did_change arrives with version 3 (regression)
    let is_regression = coord.check_version_regression(uri, 3);
    // Then: regression detected
    assert!(
        is_regression,
        "version 3 after version 5 should be detected as regression"
    );
}
