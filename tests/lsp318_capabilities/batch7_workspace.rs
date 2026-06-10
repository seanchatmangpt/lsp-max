/// Batch 7 — workspace notification and request method dispatch tests.
use super::common::{assert_well_formed, roundtrip, roundtrip_notification_then_shutdown};

#[tokio::test(flavor = "current_thread")]
async fn test_did_change_workspace_folders_dispatch() {
    let params = serde_json::json!({
        "event": {
            "added": [{"uri": "file:///tmp/added", "name": "added"}],
            "removed": []
        }
    });
    roundtrip_notification_then_shutdown("workspace/didChangeWorkspaceFolders", params).await;
}

#[tokio::test(flavor = "current_thread")]
async fn test_did_change_configuration_dispatch() {
    let params = serde_json::json!({ "settings": {"editor": {"tabSize": 4}} });
    roundtrip_notification_then_shutdown("workspace/didChangeConfiguration", params).await;
}

#[tokio::test(flavor = "current_thread")]
async fn test_did_change_watched_files_dispatch() {
    let params = serde_json::json!({
        "changes": [{"uri": "file:///tmp/foo.rs", "type": 2}]
    });
    roundtrip_notification_then_shutdown("workspace/didChangeWatchedFiles", params).await;
}

#[tokio::test(flavor = "current_thread")]
async fn test_workspace_symbol_dispatch() {
    let params = serde_json::json!({ "query": "MyStruct" });
    let resp = roundtrip("workspace/symbol", params).await;
    assert_well_formed(&resp);
}

#[tokio::test(flavor = "current_thread")]
async fn test_workspace_symbol_resolve_dispatch() {
    let params = serde_json::json!({
        "name": "MyStruct",
        "kind": 23,
        "location": {
            "uri": "file:///tmp/foo.rs",
            "range": {
                "start": {"line": 0, "character": 0},
                "end":   {"line": 0, "character": 0}
            }
        }
    });
    let resp = roundtrip("workspaceSymbol/resolve", params).await;
    assert_well_formed(&resp);
}

#[tokio::test(flavor = "current_thread")]
async fn test_execute_command_dispatch() {
    let params = serde_json::json!({
        "command": "editor.action.formatDocument",
        "arguments": []
    });
    let resp = roundtrip("workspace/executeCommand", params).await;
    assert_well_formed(&resp);
}

#[tokio::test(flavor = "current_thread")]
async fn test_workspace_diagnostic_dispatch() {
    let params = serde_json::json!({ "previousResultIds": [] });
    let resp = roundtrip("workspace/diagnostic", params).await;
    assert_well_formed(&resp);
}

#[tokio::test(flavor = "current_thread")]
async fn test_will_create_files_dispatch() {
    let params = serde_json::json!({
        "files": [{"uri": "file:///tmp/new_file.rs"}]
    });
    let resp = roundtrip("workspace/willCreateFiles", params).await;
    assert_well_formed(&resp);
}
