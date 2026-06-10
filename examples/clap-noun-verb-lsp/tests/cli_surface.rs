use assert_cmd::Command;

#[test]
fn test_cli_surface_scan() {
    let mut cmd = Command::cargo_bin("clap-noun-verb-lsp").unwrap();
    cmd.arg("scan").arg("workspace").arg("--format").arg("json");
    cmd.assert().success();
}

#[test]
fn test_cli_surface_graph() {
    let mut cmd = Command::cargo_bin("clap-noun-verb-lsp").unwrap();
    cmd.arg("graph").arg("export").arg("--format").arg("json");
    cmd.assert().success();
}

#[test]
fn test_cli_surface_command() {
    let mut cmd = Command::cargo_bin("clap-noun-verb-lsp").unwrap();
    cmd.arg("command")
        .arg("inspect")
        .arg("--noun")
        .arg("noun")
        .arg("--verb")
        .arg("verb");
    cmd.assert().success();
}

#[test]
fn test_cli_surface_layout() {
    let mut cmd = Command::cargo_bin("clap-noun-verb-lsp").unwrap();
    cmd.arg("layout").arg("check");
    cmd.assert().success();
}

#[test]
fn test_cli_surface_rules_list() {
    let mut cmd = Command::cargo_bin("clap-noun-verb-lsp").unwrap();
    cmd.arg("rules").arg("list");
    cmd.assert().success();
}

#[test]
fn test_cli_surface_doctor() {
    let mut cmd = Command::cargo_bin("clap-noun-verb-lsp").unwrap();
    cmd.arg("doctor").arg("check");
    cmd.assert().success();
}

#[test]
fn test_cli_surface_receipt() {
    let mut cmd = Command::cargo_bin("clap-noun-verb-lsp").unwrap();
    cmd.arg("receipt").arg("show").arg("--latest");
    cmd.assert().success();
}
