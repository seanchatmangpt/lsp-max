use assert_cmd::Command;
use predicates::prelude::*;

#[test]
fn test_cli_scan_workspace() {
    let mut cmd = Command::cargo_bin("pattern-lsp").unwrap();
    cmd.arg("scan").arg("workspace").arg("--format").arg("json");
    
    // We expect the scan to finish successfully and print findings
    let output = cmd.unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    
    assert!(stdout.contains("ANTI-FAKE-001") || stdout.contains("ANTI-FAKE-002"));
    assert!(stdout.contains("source\":\"pattern-lsp"));
}
