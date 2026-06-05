#![allow(clippy::new_without_default)]
#![allow(clippy::io_other_error)]

pub mod nouns;

fn main() -> clap_noun_verb::Result<()> {
    clap_noun_verb::run()
}

#[cfg(test)]
mod tests {
    use std::env;
    use std::fs;
    use std::path::Path;

    #[test]
    fn test_cli_nouns_integration() {
        let _guard = crate::nouns::TEST_ENV_LOCK
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        // Setup database file path — use tempfile for unique path to avoid cross-test state pollution
        let tmp_file = tempfile::NamedTempFile::new().expect("tempfile creation failed");
        let test_db_path = tmp_file.path().to_str().unwrap().to_string();
        // Remove the temp file so the mesh creates it fresh
        let _ = fs::remove_file(&test_db_path);
        env::set_var("TOWER_LSP_MAX_STATE_PATH", &test_db_path);

        // Also clean up refund receipt path if it exists
        let refund_receipt_path = std::env::temp_dir()
            .join("tower_lsp_max_refund_receipt.txt")
            .to_string_lossy()
            .into_owned();
        if Path::new(&refund_receipt_path).exists() {
            let _ = fs::remove_file(&refund_receipt_path);
        }

        // Get command registry
        let registry = clap_noun_verb::cli::CommandRegistry::get().lock().unwrap();

        // 1. Query initial state for LSP_1 using state verb
        let args = vec![
            "tower-lsp-max-cli".to_string(),
            "state".to_string(),
            "state".to_string(),
            "--instance-id".to_string(),
            "LSP_1".to_string(),
        ];
        let output = registry.execute_single_step(args).unwrap();
        let val = output.data;
        assert_eq!(val["instance_id"], "LSP_1");
        assert_eq!(val["phase"], "Initialized");
        assert_eq!(val["conformance_score"], 100.0);
        assert_eq!(val["diagnostics_count"], 0);

        // 2. Query initial state for LSP_2
        let args = vec![
            "tower-lsp-max-cli".to_string(),
            "state".to_string(),
            "state".to_string(),
            "--instance-id".to_string(),
            "LSP_2".to_string(),
        ];
        let output = registry.execute_single_step(args).unwrap();
        let val = output.data;
        assert_eq!(val["instance_id"], "LSP_2");
        assert_eq!(val["policy_state"], "Operational");

        // 3. Register diagnostic on LSP_1 under law-intake-validation
        let args = vec![
            "tower-lsp-max-cli".to_string(),
            "diagnostics".to_string(),
            "diagnose".to_string(),
            "--instance-id".to_string(),
            "LSP_1".to_string(),
            "--diagnostic-id".to_string(),
            "diag-invalid-input".to_string(),
            "--law-id".to_string(),
            "law-intake-validation".to_string(),
            "--severity".to_string(),
            "error".to_string(),
            "--message".to_string(),
            "Invalid input provided".to_string(),
        ];
        let output = registry.execute_single_step(args).unwrap();
        assert_eq!(output.data["success"], true);

        // 4. Verify gate logic transitioned LSP_2 to ClarificationRequested
        let args = vec![
            "tower-lsp-max-cli".to_string(),
            "state".to_string(),
            "state".to_string(),
            "--instance-id".to_string(),
            "LSP_2".to_string(),
        ];
        let output = registry.execute_single_step(args).unwrap();
        assert_eq!(output.data["policy_state"], "ClarificationRequested");

        // Verify LSP_1 conformance score is reduced
        let args = vec![
            "tower-lsp-max-cli".to_string(),
            "state".to_string(),
            "state".to_string(),
            "--instance-id".to_string(),
            "LSP_1".to_string(),
        ];
        let output = registry.execute_single_step(args).unwrap();
        assert_eq!(output.data["diagnostics_count"], 1);
        assert_eq!(output.data["conformance_score"], 70.0);

        // 5. Query diagnostics run
        let args = vec![
            "tower-lsp-max-cli".to_string(),
            "diagnostics".to_string(),
            "run".to_string(),
            "--target".to_string(),
            "LSP_1".to_string(),
        ];
        let output = registry.execute_single_step(args).unwrap();
        assert_eq!(output.data["count"], 1);
        assert_eq!(
            output.data["issues"][0]["message"],
            "Invalid input provided"
        );

        // 6. Clear diagnostic on LSP_1 to trigger the next stage of gate logic
        let args = vec![
            "tower-lsp-max-cli".to_string(),
            "diagnostics".to_string(),
            "clear".to_string(),
            "--instance-id".to_string(),
            "LSP_1".to_string(),
            "--diagnostic-id".to_string(),
            "diag-invalid-input".to_string(),
        ];
        let output = registry.execute_single_step(args).unwrap();
        assert_eq!(output.data["success"], true);

        // 7. Verify gate logic cleared diagnostics, transitioned LSP_2, and emitted receipts/bounded action
        let args = vec![
            "tower-lsp-max-cli".to_string(),
            "state".to_string(),
            "state".to_string(),
            "--instance-id".to_string(),
            "LSP_1".to_string(),
        ];
        let output = registry.execute_single_step(args).unwrap();
        assert_eq!(output.data["diagnostics_count"], 0);
        assert_eq!(output.data["conformance_score"], 100.0);

        let args = vec![
            "tower-lsp-max-cli".to_string(),
            "state".to_string(),
            "state".to_string(),
            "--instance-id".to_string(),
            "LSP_2".to_string(),
        ];
        let output = registry.execute_single_step(args).unwrap();
        assert_eq!(output.data["policy_state"], "RefundAuthorized");

        // Verify receipt list on LSP_1
        let args = vec![
            "tower-lsp-max-cli".to_string(),
            "receipt".to_string(),
            "list".to_string(),
            "--instance-id".to_string(),
            "LSP_1".to_string(),
        ];
        let output = registry.execute_single_step(args).unwrap();
        assert_eq!(output.data["count"], 1);
        assert_eq!(
            output.data["receipts"][0]["receipt_id"],
            "rcpt-intake-validated"
        );

        // Verify receipt list on LSP_2
        let args = vec![
            "tower-lsp-max-cli".to_string(),
            "receipt".to_string(),
            "list".to_string(),
            "--instance-id".to_string(),
            "LSP_2".to_string(),
        ];
        let output = registry.execute_single_step(args).unwrap();
        assert_eq!(output.data["count"], 1);
        assert_eq!(
            output.data["receipts"][0]["receipt_id"],
            "rcpt-refund-executed"
        );

        // Verify event log list
        let args = vec![
            "tower-lsp-max-cli".to_string(),
            "event".to_string(),
            "list".to_string(),
        ];
        let output = registry.execute_single_step(args).unwrap();
        assert!(output.data["count"].as_u64().unwrap() > 0);

        // 8. Test Client noun (verifies #[serde(flatten)] extra field works!)
        let args = vec![
            "tower-lsp-max-cli".to_string(),
            "client".to_string(),
            "connect".to_string(),
            "--id".to_string(),
            "client_abc".to_string(),
        ];
        let output = registry.execute_single_step(args).unwrap();
        assert_eq!(output.data["client"]["id"], "client_abc");

        // Send a message
        let args = vec![
            "tower-lsp-max-cli".to_string(),
            "client".to_string(),
            "send".to_string(),
            "--id".to_string(),
            "client_abc".to_string(),
            "--message".to_string(),
            "hello mesh".to_string(),
        ];
        let output = registry.execute_single_step(args).unwrap();
        assert_eq!(output.data["success"], true);

        // Verify that running another state command does NOT wipe out the client details
        let args = vec![
            "tower-lsp-max-cli".to_string(),
            "state".to_string(),
            "state".to_string(),
            "--instance-id".to_string(),
            "LSP_1".to_string(),
        ];
        let _ = registry.execute_single_step(args).unwrap();

        // Receive the message
        let args = vec![
            "tower-lsp-max-cli".to_string(),
            "client".to_string(),
            "receive".to_string(),
            "--id".to_string(),
            "client_abc".to_string(),
        ];
        let output = registry.execute_single_step(args).unwrap();
        assert_eq!(output.data["message"]["body"], "hello mesh");

        // Cleanup — temp file removed when tmp_file handle drops; clean receipt file too
        if Path::new(&refund_receipt_path).exists() {
            let _ = fs::remove_file(&refund_receipt_path);
        }
        env::remove_var("TOWER_LSP_MAX_STATE_PATH");
        drop(tmp_file);
    }
}
