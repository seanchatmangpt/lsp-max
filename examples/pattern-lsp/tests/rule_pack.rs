use std::process::Command;
use assert_cmd::prelude::*;

#[test]
fn test_rules_list() {
    let mut cmd = Command::cargo_bin("pattern-lsp").unwrap();
    cmd.arg("rules").arg("list");
    
    cmd.assert().success();
}
