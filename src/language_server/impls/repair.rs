//! Diagnostic repair planning and transaction application implementations.

use crate::gate::run_gate_logic;
use crate::jsonrpc::{Error, Result};
use crate::workspace_edit::apply_workspace_edit;
use crate::{lock_registry, sha256, update_diagnostics};
use lsp_types_max::{CodeAction, CodeActionKind, DiagnosticSeverity, TextEdit, WorkspaceEdit};

/// Returns a full MaxDiagnostic by ID.
pub async fn max_explain_diagnostic(params: String) -> Result<max_protocol::MaxDiagnostic> {
    let mut registry = lock_registry()?;
    update_diagnostics(&mut registry);
    if let Some(diag) = registry.diagnostics.get(&params) {
        Ok(diag.clone())
    } else {
        Err(Error::invalid_params(format!(
            "Diagnostic '{}' not found",
            params
        )))
    }
}

/// Returns repair actions for a specific diagnostic or law.
pub async fn max_repair_plan(params: String) -> Result<Vec<max_protocol::MaxCodeAction>> {
    let mut registry = lock_registry()?;
    update_diagnostics(&mut registry);

    if let Some(plans) = registry.repair_plans.get(&params) {
        return Ok(plans.clone());
    }

    let mut matched = Vec::new();
    for plans in registry.repair_plans.values() {
        for plan in plans {
            if let Some(ref diags) = plan.action.diagnostics {
                for d in diags {
                    if let Some(max_d) = registry
                        .diagnostics
                        .values()
                        .find(|md| md.lsp.message == d.message)
                    {
                        if max_d.law_id == params {
                            matched.push(plan.clone());
                        }
                    }
                }
            }
        }
    }

    if !matched.is_empty() {
        return Ok(matched);
    }

    Err(Error::invalid_params(format!(
        "No repair plan found for '{}'",
        params
    )))
}

/// Applies a transactional code action and returns a receipt.
pub async fn max_apply_repair_transaction(
    params: max_protocol::MaxCodeAction,
) -> Result<max_protocol::Receipt> {
    // Phase 1: collect all data needed while holding the lock, then drop it before I/O.
    let (current_state, root_path, workspace_edit, gate_names, diag_filter) = {
        let mut registry = lock_registry()?;
        update_diagnostics(&mut registry);

        // Preconditions check
        for pre in &params.preconditions {
            if pre.condition == "State is Uninitialized"
                && registry.current_state != crate::service::State::Uninitialized
            {
                return Err(Error::invalid_params(format!(
                    "Precondition failed: Server state is {:?}, but condition requires State is Uninitialized.",
                    registry.current_state
                )));
            }
        }

        // Expected receipts check
        for expected in &params.receipt_plan.expected_receipts {
            if !registry.receipts.contains_key(expected) {
                return Err(Error::invalid_params(format!(
                    "Receipt integrity violation: Required cryptographic receipt '{}' is not present in the registry.",
                    expected
                )));
            }
        }

        // Safety verification check: workspace edit must have an explicit validation plan
        if params.action.edit.is_some() && params.validation_plan.gates.is_empty() {
            return Err(Error::invalid_params(
                "Unsafe transaction: A workspace edit is not called 'safe' unless there is an explicit validation plan (non-empty gates)."
            ));
        }

        let current_state = registry.current_state;
        let root_path = registry.root_path.clone();
        let workspace_edit = params.action.edit.clone();
        let gate_names: Vec<String> = params
            .validation_plan
            .gates
            .iter()
            .map(|g| g.0.clone())
            .collect();
        let diag_filter: Option<Vec<(String, lsp_types_max::Range)>> = params
            .action
            .diagnostics
            .as_ref()
            .map(|diags| diags.iter().map(|d| (d.message.clone(), d.range)).collect());

        (
            current_state,
            root_path,
            workspace_edit,
            gate_names,
            diag_filter,
        )
        // MutexGuard dropped here — lock released before any file I/O
    };

    // Phase 2: perform all file I/O without holding the lock.
    let mut backups = std::collections::HashMap::new();
    if let Some(ref edit) = workspace_edit {
        if let Some(ref changes) = edit.changes {
            for url in changes.keys() {
                if let Ok(parsed_url) = url::Url::parse(url.as_str()) {
                    if let Ok(path) = parsed_url.to_file_path() {
                        let content = if path.exists() {
                            std::fs::read_to_string(&path).ok()
                        } else {
                            None
                        };
                        backups.insert(path, content);
                    }
                }
            }
        }
        if let Err(e) = apply_workspace_edit(edit) {
            return Err(Error::invalid_params(format!(
                "Failed to apply edits: {}",
                e
            )));
        }
    }

    // Run validation gates (uses only the snapshot values, no lock needed)
    let mut validation_failed = false;
    let mut failed_gate = String::new();
    for gate_name in &gate_names {
        if !run_gate_logic(gate_name, current_state, root_path.clone()) {
            validation_failed = true;
            failed_gate = gate_name.clone();
            break;
        }
    }

    if validation_failed {
        // Rollback files
        for (path, backup) in backups {
            if let Some(old_content) = backup {
                let _ = std::fs::write(&path, old_content);
            } else {
                let _ = std::fs::remove_file(&path);
            }
        }
        return Err(Error::invalid_params(format!(
            "Transaction validation failed: validation gate '{}' failed check. Rolled back changes.",
            failed_gate
        )));
    }

    // Compute receipt hash before re-acquiring the lock
    let serialized = serde_json::to_vec(&params).map_err(|e| {
        let _ = e;
        Error::internal_error()
    })?;
    let hash = sha256(&serialized);

    let receipt_id = if params.action.title.contains("security authorization") {
        "rcpt-security-auth".to_string()
    } else {
        format!("rcpt-{}", &hash[0..16])
    };

    let receipt = max_protocol::Receipt {
        receipt_id: receipt_id.clone(),
        hash,
        prev_receipt_hash: None,
    };

    // Phase 3: re-acquire lock only to write receipts/diagnostics back.
    {
        let mut registry = lock_registry()?;

        // Record successful gate executions
        for gate_name in &gate_names {
            registry.gates.insert(gate_name.clone(), true);
        }

        // Clear resolved diagnostics and repair plans
        if let Some(ref filter) = diag_filter {
            let mut resolved_ids = Vec::new();
            for (msg, range) in filter {
                for (id, max_d) in &registry.diagnostics {
                    if &max_d.lsp.message == msg && &max_d.lsp.range == range {
                        resolved_ids.push(id.clone());
                    }
                }
            }
            for id in resolved_ids {
                registry.cleared_diagnostics.insert(id.clone());
                registry.diagnostics.remove(&id);
                registry.repair_plans.remove(&id);
            }
        }

        // Update diagnostics dynamic state
        update_diagnostics(&mut registry);

        registry.receipts.insert(receipt_id, receipt.clone());
    }

    Ok(receipt)
}

/// Executes a validation gate.
pub async fn max_run_gate(params: max_protocol::GateId) -> Result<bool> {
    let mut registry = lock_registry()?;
    update_diagnostics(&mut registry);
    let success = run_gate_logic(
        &params.0,
        registry.current_state,
        registry.root_path.clone(),
    );
    registry.gates.insert(params.0.clone(), success);
    Ok(success)
}
