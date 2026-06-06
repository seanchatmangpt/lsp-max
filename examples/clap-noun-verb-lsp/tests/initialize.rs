use assert_cmd::Command;
use std::fs;
use std::path::PathBuf;

#[test]
fn test_initialize_capabilities() {
    let fixture_path = PathBuf::from("tests/fixtures/initialize.json");
    let input = fs::read_to_string(fixture_path).unwrap();

    let mut cmd = Command::cargo_bin("clap-noun-verb-lsp").unwrap();
    cmd.arg("server")
       .arg("serve")
       .arg("--stdio")
       .write_stdin(format!("Content-Length: {}\r\n\r\n{}", input.len(), input));

    let output = cmd.output().unwrap();
    let stdout = String::from_utf8(output.stdout).unwrap();
    let stderr = String::from_utf8(output.stderr).unwrap();

    println!("STDOUT: {}", stdout);
    println!("STDERR: {}", stderr);

    assert!(stdout.contains("diagnosticProvider"));
    assert!(stdout.contains("semanticTokensProvider"));
    assert!(stdout.contains("inlayHintProvider"));
    assert!(stdout.contains("inlineValueProvider"));
    assert!(stdout.contains("codeActionProvider"));
    assert!(stdout.contains("codeLensProvider"));
    assert!(stdout.contains("documentSymbolProvider"));
    assert!(stdout.contains("workspaceSymbolProvider"));
    assert!(stdout.contains("definitionProvider"));
    assert!(stdout.contains("referencesProvider"));
    assert!(stdout.contains("callHierarchyProvider"));
    assert!(stdout.contains("monikerProvider"));
    assert!(stdout.contains("executeCommandProvider"));
}
