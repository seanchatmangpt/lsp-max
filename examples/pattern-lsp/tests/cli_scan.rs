use assert_cmd::Command;
use std::path::Path;

// BUILD PREREQUISITE LAW: this court runs the real `pattern-lsp` binary.
// The binary path comes from Cargo's `CARGO_BIN_EXE_pattern-lsp` — Cargo
// builds the package's bin targets before compiling its integration tests,
// so the binary is a build prerequisite, never an assumed artifact. If the
// env is absent (misconfigured invocation) the court fails LOUDLY, never a
// silent fake-pass skip.
#[test]
fn test_cli_scan_workspace() {
    let bin = std::env::var("CARGO_BIN_EXE_pattern-lsp").expect(
        "CARGO_BIN_EXE_pattern-lsp must be set by cargo for same-package integration tests",
    );
    let bin = Path::new(&bin);
    assert!(
        bin.exists(),
        "pattern-lsp binary missing at build-prerequisite path {}",
        bin.display()
    );

    let mut cmd = Command::new(bin);
    cmd.arg("scan").arg("workspace").arg("--format").arg("json");

    // We expect the scan to finish successfully and print findings
    let output = cmd.unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(stdout.contains("ANTI-FAKE-001") || stdout.contains("ANTI-FAKE-002"));
    assert!(stdout.contains("source\":\"pattern-lsp"));
}
