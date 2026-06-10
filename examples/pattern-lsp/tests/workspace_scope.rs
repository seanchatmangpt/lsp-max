use assert_cmd::Command;
use std::fs;
use std::path::PathBuf;

#[test]
fn test_workspace_scope_proof() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf();

    let f1 = root.join("crates/fake_subcrate/src/lib.rs");
    let f2 = root.join("examples/fake_example/src/main.rs");
    let f3 = root.join("playground/fake.rs");

    fs::create_dir_all(f1.parent().unwrap()).unwrap();
    fs::create_dir_all(f2.parent().unwrap()).unwrap();
    fs::create_dir_all(f3.parent().unwrap()).unwrap();

    fs::write(&f1, "let x = serde_json::Value::Null;").unwrap();
    fs::write(&f2, "fn main() { unimplemented!() }").unwrap();
    fs::write(&f3, "fn foo() { todo!() }").unwrap();

    let mut cmd = Command::cargo_bin("pattern-lsp").unwrap();
    cmd.arg("scan").arg("workspace").arg("--format").arg("json");

    let output = cmd.unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);

    let _ = fs::remove_dir_all(root.join("crates/fake_subcrate"));
    let _ = fs::remove_dir_all(root.join("examples/fake_example"));
    let _ = fs::remove_file(&f3);

    assert!(stdout.contains("crates/fake_subcrate/src/lib.rs"));
    assert!(stdout.contains("examples/fake_example/src/main.rs"));
    assert!(stdout.contains("playground/fake.rs"));
}
