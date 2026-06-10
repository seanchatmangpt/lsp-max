//! Validation gate logic for verifying server authorization, state checks, and compliance keys.

/// Evaluates a security gate check given its unique identifier, the current server state,
/// and the workspace root path.
///
/// Returns `true` if the gate permits transition, `false` otherwise.
pub fn run_gate_logic(
    gate_id: &str,
    current_state: crate::service::State,
    root_path: std::path::PathBuf,
) -> bool {
    match gate_id {
        "some-gate" => true,
        "gate-state-check" => current_state != crate::service::State::Uninitialized,
        "gate-receipt-check" | "gate-auth-check" => {
            let spec = crate::diagnostics::law_table::law_table()
                .into_iter()
                .find(|s| s.gate_id == gate_id);
            let Some(spec) = spec else { return false };
            let path = root_path.join(spec.receipt_file);
            if path.exists() {
                std::fs::read_to_string(&path)
                    .map(|c| c.trim() == spec.receipt_token)
                    .unwrap_or(false)
            } else {
                false
            }
        }
        _ => {
            let output = std::process::Command::new("cargo")
                .arg("check")
                .current_dir(root_path)
                .output();
            match output {
                Ok(out) => {
                    if !out.status.success() {
                        eprintln!("cargo check failed!");
                        eprintln!("stdout: {}", String::from_utf8_lossy(&out.stdout));
                        eprintln!("stderr: {}", String::from_utf8_lossy(&out.stderr));
                    }
                    out.status.success()
                }
                Err(e) => {
                    eprintln!("failed to execute cargo check: {:?}", e);
                    false
                }
            }
        }
    }
}
