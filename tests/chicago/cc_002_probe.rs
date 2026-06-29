//! Chicago acceptance test for CC-002: ServerEntry::probe()
//! Status: CANDIDATE — implement probe() to make this test pass.
//! Ticket: docs/jira/v26.6.30/CC-002-lsp-max-toml-auto-scan.md

use chicago_tdd_tools::chicago_test;

#[chicago_test(
    ticket = "docs/jira/v26.6.30/CC-002-lsp-max-toml-auto-scan.md",
    scaffold_fn = "lsp_max_compositor::config::ServerEntry::probe"
)]
fn probe_reachable_command_returns_ok() {
    // Given: a ServerEntry whose command is "echo" (always reachable)
    let mut entry = lsp_max_compositor::config::ServerEntry {
        id: "test-echo".to_string(),
        command: Some("echo".to_string()),
        args: Some(vec!["--version".to_string()]),
        priority: "primary".to_string(),
        primary_extensions: vec![".rs".to_string()],
        secondary_extensions: vec![],
        andon_code_prefixes: None,
    };
    // When: probe is called with a 500ms timeout
    let result = entry.probe(std::time::Duration::from_millis(500));
    // Then: probe succeeds (echo exits 0)
    assert!(
        result.is_ok(),
        "probe should succeed for reachable command, got: {result:?}"
    );
}

#[chicago_test(
    ticket = "docs/jira/v26.6.30/CC-002-lsp-max-toml-auto-scan.md",
    scaffold_fn = "lsp_max_compositor::config::ServerEntry::probe"
)]
fn probe_missing_command_returns_err() {
    // Given: a ServerEntry whose command does not exist
    let mut entry = lsp_max_compositor::config::ServerEntry {
        id: "test-missing".to_string(),
        command: Some("/tmp/no-such-binary-lsp-max-test".to_string()),
        args: None,
        priority: "primary".to_string(),
        primary_extensions: vec![".rs".to_string()],
        secondary_extensions: vec![],
        andon_code_prefixes: None,
    };
    // When: probe is called
    let result = entry.probe(std::time::Duration::from_millis(100));
    // Then: probe fails (binary not found)
    assert!(result.is_err(), "probe should fail for missing command");
}
