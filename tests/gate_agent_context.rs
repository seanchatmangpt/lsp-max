use serde_json::Value;
use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use tempfile::TempDir;

// Copy fnv1a to derive the exact path expected
fn fnv1a(bytes: &[u8]) -> u64 {
    let mut hash: u64 = 0xcbf29ce484222325;
    for &b in bytes {
        hash ^= b as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

fn derive_test_gate_path(agent_id: &str, temp_dir: &str) -> PathBuf {
    let workspace = env::current_dir().unwrap();
    let hash = fnv1a(workspace.to_string_lossy().as_bytes());
    let dir = PathBuf::from(temp_dir);
    dir.join(format!("lsp-max-gate-{:016x}-agent-{}", hash, agent_id))
}

fn get_cli_command() -> Command {
    let mut cmd = Command::new("cargo");
    cmd.args([
        "run",
        "-q",
        "-p",
        "lsp-max-cli",
        "--bin",
        "lsp-max-cli",
        "--",
    ]);
    cmd
}

#[test]
fn gate_context_emitted_when_blocked() {
    let tmp = TempDir::new().unwrap();
    let agent_id = "test-agent-blocked";

    let expected_path = derive_test_gate_path(agent_id, tmp.path().to_str().unwrap());

    let payload = br#"{"blocked":true,"codes":["WASM4PM-007"],"seq":3}"#;
    fs::write(&expected_path, payload).unwrap();

    let mut cmd = get_cli_command();
    cmd.env("XDG_RUNTIME_DIR", tmp.path().to_str().unwrap());
    cmd.env("LSP_MAX_AGENT_ID", agent_id);
    cmd.args(["gate", "check", "--format=agent-context"]);

    let output = cmd.output().unwrap();

    // Expected: exit code = 1
    assert_eq!(output.status.code(), Some(1));

    let stdout = String::from_utf8(output.stdout).unwrap();

    // stdout contains <gate-context>
    assert!(stdout.contains("<gate-context>"));

    // Extract JSON between <gate-context> tags
    let start = stdout.find("<gate-context>").unwrap() + "<gate-context>\n".len();
    let end = stdout.find("</gate-context>").unwrap();
    let json_str = &stdout[start..end];

    let ctx: Value = serde_json::from_str(json_str).unwrap();

    // admission_allowed = false
    assert_eq!(ctx["admission_allowed"], false);

    // active_andon_codes non-empty
    let codes = ctx["active_andon_codes"].as_array().unwrap();
    assert!(!codes.is_empty());
    assert_eq!(codes[0], "WASM4PM-007");

    // available_repairs non-empty
    let repairs = ctx["available_repairs"].as_array().unwrap();
    assert!(!repairs.is_empty());

    // since_seq present
    assert_eq!(ctx["since_seq"].as_u64(), Some(3));
}

#[test]
fn clear_gate_does_not_emit_false_block() {
    let tmp = TempDir::new().unwrap();
    let agent_id = "test-agent-clear";

    let expected_path = derive_test_gate_path(agent_id, tmp.path().to_str().unwrap());
    fs::write(&expected_path, b"0").unwrap();

    let mut cmd = get_cli_command();
    cmd.env("XDG_RUNTIME_DIR", tmp.path().to_str().unwrap());
    cmd.env("LSP_MAX_AGENT_ID", agent_id);
    cmd.args(["gate", "check", "--format=agent-context"]);

    let output = cmd.output().unwrap();

    // Expected: gate check exits 0
    assert_eq!(output.status.code(), Some(0));

    let stdout = String::from_utf8(output.stdout).unwrap();
    let ctx: Value = serde_json::from_str(&stdout).unwrap();

    // admission_allowed true
    assert_eq!(ctx["admission_allowed"], true);

    // active_andon_codes empty
    assert!(ctx["active_andon_codes"].as_array().unwrap().is_empty());
}
