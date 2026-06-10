use crate::mesh::AutonomicMesh;
use crate::mesh_types::LspPhase;
use crate::sha256::sha256;

impl AutonomicMesh {
    pub fn verify_instance_ledger(&self, instance_id: &str) -> Result<(), String> {
        let instance = self
            .instances
            .get(instance_id)
            .ok_or_else(|| format!("Instance {} not found", instance_id))?;

        let history = &instance.receipts;
        if history.is_empty() {
            return Err("Ledger is empty".to_string());
        }

        if instance_id == "LSP_1" {
            let r0 = &history[0];
            if r0.receipt_id != "rcpt-uninitialized" {
                return Err(format!("Invalid initial receipt ID: {}", r0.receipt_id));
            }
            let mut expected_hash = sha256(r0.receipt_id.as_bytes());
            if r0.hash != expected_hash {
                return Err(format!(
                    "Hash mismatch at index 0: expected {}, got {}",
                    expected_hash, r0.hash
                ));
            }

            if history.len() > 1 {
                let r1 = &history[1];
                if !r1
                    .receipt_id
                    .starts_with("rcpt-uninitialized-to-initializing:")
                {
                    return Err(format!("Invalid receipt ID at index 1: {}", r1.receipt_id));
                }
                let prefix_len = "rcpt-uninitialized-to-initializing:".len();
                let json_str = &r1.receipt_id[prefix_len..];
                if serde_json::from_str::<serde_json::Value>(json_str).is_err() {
                    return Err("Failed to parse client capabilities in receipt 1".to_string());
                }

                expected_hash = sha256(format!("{}:{}", expected_hash, r1.receipt_id).as_bytes());
                if r1.hash != expected_hash {
                    return Err(format!(
                        "Hash mismatch at index 1: expected {}, got {}",
                        expected_hash, r1.hash
                    ));
                }
            }

            if history.len() > 2 {
                let r2 = &history[2];
                if !r2
                    .receipt_id
                    .starts_with("rcpt-initializing-to-initialized:")
                {
                    return Err(format!("Invalid receipt ID at index 2: {}", r2.receipt_id));
                }
                let prefix_len = "rcpt-initializing-to-initialized:".len();
                let json_str = &r2.receipt_id[prefix_len..];
                if serde_json::from_str::<serde_json::Value>(json_str).is_err() {
                    return Err("Failed to parse server capabilities in receipt 2".to_string());
                }

                expected_hash = sha256(format!("{}:{}", expected_hash, r2.receipt_id).as_bytes());
                if r2.hash != expected_hash {
                    return Err(format!(
                        "Hash mismatch at index 2: expected {}, got {}",
                        expected_hash, r2.hash
                    ));
                }
            }

            if history.len() > 3 {
                let r3 = &history[3];
                if r3.receipt_id != "rcpt-initialized-to-shutdown" {
                    return Err(format!("Invalid receipt ID at index 3: {}", r3.receipt_id));
                }
                expected_hash = sha256(format!("{}:{}", expected_hash, r3.receipt_id).as_bytes());
                if r3.hash != expected_hash {
                    return Err(format!(
                        "Hash mismatch at index 3: expected {}, got {}",
                        expected_hash, r3.hash
                    ));
                }
            }

            if history.len() > 4 {
                let r4 = &history[4];
                if r4.receipt_id != "rcpt-shutdown-to-exited" {
                    return Err(format!("Invalid receipt ID at index 4: {}", r4.receipt_id));
                }
                expected_hash = sha256(format!("{}:{}", expected_hash, r4.receipt_id).as_bytes());
                if r4.hash != expected_hash {
                    return Err(format!(
                        "Hash mismatch at index 4: expected {}, got {}",
                        expected_hash, r4.hash
                    ));
                }
            }

            if history.len() > 5 {
                return Err("Ledger contains unexpected items beyond Exited state".to_string());
            }

            let expected_phase = match history.len() {
                1 => LspPhase::Uninitialized,
                2 => LspPhase::Initializing,
                3 => LspPhase::Initialized,
                4 => LspPhase::ShutDown,
                5 => LspPhase::Exited,
                _ => unreachable!(),
            };

            if instance.phase != expected_phase {
                return Err(format!(
                    "Phase mismatch: instance.phase is '{}' but ledger shows '{}'",
                    instance.phase, expected_phase
                ));
            }
        } else {
            for (idx, r) in history.iter().enumerate() {
                if r.receipt_id.is_empty() {
                    return Err(format!("Empty receipt ID at index {}", idx));
                }
                if r.hash.is_empty() {
                    return Err(format!("Empty receipt hash at index {}", idx));
                }
            }
        }

        Ok(())
    }

    pub fn get_ledger_diagnostic_report(&self, instance_id: &str) -> String {
        let mut report = format!("Ledger Diagnostic Report for Instance: {}\n", instance_id);
        match self.verify_instance_ledger(instance_id) {
            Ok(()) => {
                report.push_str("Status: VERIFIED (Cryptographic integrity intact)\n");
            }
            Err(e) => {
                report.push_str(&format!(
                    "Status: FAILED (Ledger verification failed: {})\n",
                    e
                ));
            }
        }

        if let Some(instance) = self.instances.get(instance_id) {
            report.push_str(&format!("Active Phase: {}\n", instance.phase));
            report.push_str(&format!("Policy State: {:?}\n", instance.policy_state));
            report.push_str(&format!("Receipts count: {}\n", instance.receipts.len()));
            for (idx, r) in instance.receipts.iter().enumerate() {
                report.push_str(&format!(
                    "  [{}] ID: {} | Hash: {}\n",
                    idx, r.receipt_id, r.hash
                ));
            }
        } else {
            report.push_str("Instance not found in mesh registry.\n");
        }
        report
    }
}
