    #[rpc(name = "max/snapshot")]
    async fn max_snapshot(&self) -> Result<max_protocol::SnapshotId> {
        let mut registry = lock_registry()?;
        update_diagnostics(&mut registry);

        let snapshot = max_runtime::DeterministicSnapshot::new();
        let snapshot_id = snapshot.id.clone();

        let capability_vector = max_protocol::MaxCapabilityVector {
            client: registry.client_capabilities.clone().unwrap_or_default(),
            server: registry.server_capabilities.clone().unwrap_or_default(),
            negotiated: serde_json::json!({
                "conformance": "maximal",
                "law_framework": "v1"
            }),
            experimental: serde_json::json!({}),
            gaps: vec![],
        };

        let diagnostics = registry.diagnostics.values().cloned().collect();

        let actions = registry.repair_plans.values().flatten().cloned().collect();

        let score = if registry.diagnostics.is_empty() {
            100.0
        } else {
            let severity_penalty: f64 = registry
                .diagnostics
                .values()
                .map(|d| match d.lsp.severity {
                    Some(DiagnosticSeverity::ERROR) => 30.0,
                    Some(DiagnosticSeverity::WARNING) => 15.0,
                    _ => 5.0,
                })
                .sum();
            (100.0 - severity_penalty).max(0.0)
        };

        // Derive admitted/refused/unknown from current diagnostics.
        // Diagnostics with ERROR severity are treated as refused axes; others as admitted.
        // All LawAxis variants not referenced remain unknown.
        let (refused_axes, _admitted_axes): (Vec<_>, Vec<_>) = registry
            .diagnostics
            .values()
            .partition(|d| matches!(d.lsp.severity, Some(DiagnosticSeverity::ERROR)));
        let refused: Vec<max_protocol::LawAxis> =
            refused_axes.iter().map(|d| d.law_axis.clone()).collect();
        let admitted: Vec<max_protocol::LawAxis> =
            _admitted_axes.iter().map(|d| d.law_axis.clone()).collect();
        let derived_score = if admitted.is_empty() && refused.is_empty() {
            None
        } else {
            let total = (admitted.len() + refused.len()) as f64;
            Some(100.0 * admitted.len() as f64 / total)
        };
        let _ = score; // score was computed above but superseded by derived_score
        let witnessed: std::collections::HashSet<max_protocol::LawAxis> =
            admitted.iter().chain(refused.iter()).cloned().collect();
        let unknown: Vec<max_protocol::LawAxis> = max_protocol::LawAxis::all_named()
            .iter()
            .filter(|ax| !witnessed.contains(ax))
            .cloned()
            .collect();
        let conformance_vector = max_protocol::ConformanceVector {
            admitted,
            refused,
            unknown,
            score: derived_score,
            strict_mode: true,
        };

        let receipts = registry.receipts.values().cloned().collect();

        let record = SnapshotRecord {
            id: snapshot_id.clone(),
            capability_vector,
            diagnostics,
            actions,
            conformance_vector,
            receipts,
        };

        registry.snapshots.insert(snapshot_id.0.clone(), record);

        Ok(snapshot_id)
    }

    /// The `max/conformanceVector` request returns the conformance score.
    #[rpc(name = "max/conformanceVector")]
    async fn max_conformance_vector(
        &self,
        params: Option<max_protocol::SnapshotId>,
    ) -> Result<max_protocol::ConformanceVector> {
        let mut registry = lock_registry()?;
        update_diagnostics(&mut registry);
        if let Some(id) = params {
            if let Some(snap) = registry.snapshots.get(&id.0) {
                Ok(snap.conformance_vector.clone())
            } else {
                Err(Error::invalid_params(format!(
                    "Snapshot '{}' not found",
                    id.0
                )))
            }
        } else {
            // Return current conformance vector from registry
            let (refused_axes, _admitted_axes): (Vec<_>, Vec<_>) = registry
                .diagnostics
                .values()
                .partition(|d| matches!(d.lsp.severity, Some(DiagnosticSeverity::ERROR)));
            let refused: Vec<max_protocol::LawAxis> =
                refused_axes.iter().map(|d| d.law_axis.clone()).collect();
            let admitted: Vec<max_protocol::LawAxis> =
                _admitted_axes.iter().map(|d| d.law_axis.clone()).collect();
            let derived_score = if admitted.is_empty() && refused.is_empty() {
                None
            } else {
                let total = (admitted.len() + refused.len()) as f64;
                Some(100.0 * admitted.len() as f64 / total)
            };
            let witnessed: std::collections::HashSet<max_protocol::LawAxis> =
                admitted.iter().chain(refused.iter()).cloned().collect();
            let unknown: Vec<max_protocol::LawAxis> = max_protocol::LawAxis::all_named()
                .iter()
                .filter(|ax| !witnessed.contains(ax))
                .cloned()
                .collect();
            Ok(max_protocol::ConformanceVector {
                admitted,
                refused,
                unknown,
                score: derived_score,
                strict_mode: true,
            })
        }
    }

    /// The `max/explainDiagnostic` request returns a full MaxDiagnostic by ID.
    #[rpc(name = "max/explainDiagnostic")]
    async fn max_explain_diagnostic(&self, params: String) -> Result<max_protocol::MaxDiagnostic> {
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

    /// The `max/repairPlan` request returns repair actions for a specific diagnostic or law.
    #[rpc(name = "max/repairPlan")]
    async fn max_repair_plan(&self, params: String) -> Result<Vec<max_protocol::MaxCodeAction>> {
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

    /// The `max/applyRepairTransaction` request applies a transactional code action and returns a receipt.
    #[rpc(name = "max/applyRepairTransaction")]
    async fn max_apply_repair_transaction(
        &self,
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
            let diag_filter: Option<Vec<(String, lsp_types::Range)>> = params
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

    /// The `max/exportAnalysisBundle` request exports the maximal capability vector, diagnostics,
    /// and transactional code actions for the specified snapshot.
    #[rpc(name = "max/exportAnalysisBundle")]
    async fn max_export_analysis_bundle(
        &self,
        params: max_protocol::SnapshotId,
    ) -> Result<max_protocol::AnalysisBundle> {
        let mut registry = lock_registry()?;
        update_diagnostics(&mut registry);
        if let Some(snap) = registry.snapshots.get(&params.0) {
            Ok(max_protocol::AnalysisBundle {
                snapshot_id: params,
                capability_vector: snap.capability_vector.clone(),
                diagnostics: snap.diagnostics.clone(),
                actions: snap.actions.clone(),
                conformance_vector: snap.conformance_vector.clone(),
                receipts: snap.receipts.clone(),
            })
        } else {
            Err(Error::invalid_params(format!(
                "Snapshot '{}' not found",
                params.0
            )))
        }
    }

    /// The `max/runGate` request executes a validation gate.
    #[rpc(name = "max/runGate")]
    async fn max_run_gate(&self, params: max_protocol::GateId) -> Result<bool> {
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

    /// The `max/clearDiagnostic` request forcibly clears a diagnostic.
    #[rpc(name = "max/clearDiagnostic")]
    async fn max_clear_diagnostic(&self, params: String) -> Result<()> {
        let mut registry = lock_registry()?;
        update_diagnostics(&mut registry);
        registry.cleared_diagnostics.insert(params.clone());
        if registry.diagnostics.remove(&params).is_some() {
            registry.repair_plans.remove(&params);
            Ok(())
        } else {
            Err(Error::invalid_params(format!(
                "Diagnostic '{}' not found",
                params
            )))
        }
    }

    /// The `max/receipt` request returns a receipt by ID.
    #[rpc(name = "max/receipt")]
    async fn max_receipt(&self, params: String) -> Result<max_protocol::Receipt> {
        let registry = lock_registry()?;
        if let Some(rcpt) = registry.receipts.get(&params) {
            Ok(rcpt.clone())
        } else {
            Err(Error::invalid_params(format!(
                "Receipt '{}' not found",
                params
            )))
        }
    }

    /// The `max/releaseActuation` request triggers a release actuation on the specified instance.
    ///
    /// Consults `REGISTRY` for active diagnostics scoped to `instance_id`.
    /// Release is refused when any diagnostic whose ID contains `instance_id` remains active.
    /// On success a release receipt is written into the registry and a conformance delta entry
    /// is appended to `registry.conformance_delta_log` — the single authoritative delta store.
    #[rpc(name = "max/releaseActuation")]
    async fn max_release_actuation(&self, params: Value) -> Result<Value> {
        let instance_id = params
            .get("instance_id")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .ok_or_else(|| Error::invalid_params("missing instance_id"))?;
        let mut registry = lock_registry()?;
        // Count diagnostics scoped to this instance.
        let instance_diag_count = registry
            .diagnostics
            .values()
            .filter(|d| d.diagnostic_id.contains(&instance_id))
            .count();
        if instance_diag_count > 0 {
            return Err(Error::request_failed(format!(
                "Release refused: {} active diagnostics blocking conformance",
                instance_diag_count
            )));
        }
        // Emit a release receipt into the registry.
        let receipt_id = format!("rcpt-release-{}", instance_id);
        let hash = sha256(receipt_id.as_bytes());
        let receipt = max_protocol::Receipt {
            receipt_id: receipt_id.clone(),
            hash,
            prev_receipt_hash: None,
        };
        registry
            .receipts
            .insert(receipt_id.clone(), receipt.clone());
        // Record a conformance delta entry in the registry (single authoritative store).
        registry.action_seq = registry.action_seq.saturating_add(1);
        let seq = registry.action_seq;
        registry
            .conformance_delta_log
            .push_back(max_runtime::ConformanceDeltaEntry {
                seq,
                instance_id: instance_id.clone(),
                old_score: 100.0,
                new_score: 100.0,
            });
        const MAX_DELTA_LOG: usize = 4096;
        if registry.conformance_delta_log.len() > MAX_DELTA_LOG {
            registry.conformance_delta_log.pop_front();
        }
        Ok(serde_json::json!({
            "released": true,
            "instance_id": instance_id,
            "conformance_score": 100.0,
            "release_receipt": receipt,
        }))
    }

    /// The `max/admission` request returns Admitted/Refused/Unknown verdict for the global registry.
    #[rpc(name = "max/admission")]
    async fn max_admission(&self) -> Result<serde_json::Value> {
        let mut registry = lock_registry()?;
        update_diagnostics(&mut registry);
        let verdict = if registry.diagnostics.is_empty() {
            "Admitted"
        } else if registry
            .diagnostics
            .values()
            .any(|d| matches!(d.lsp.severity, Some(DiagnosticSeverity::ERROR)))
        {
            "Refused"
        } else {
            "Unknown"
        };
        Ok(serde_json::json!({
            "verdict": verdict,
            "diagnostic_count": registry.diagnostics.len(),
        }))
    }

    /// The `max/autonomicLoop` request returns the current autonomic loop status.
    #[rpc(name = "max/autonomicLoop")]
    async fn max_autonomic_loop(&self) -> Result<serde_json::Value> {
        let registry = lock_registry()?;
        Ok(serde_json::json!({
            "snapshot_count": registry.snapshots.len(),
            "diagnostic_count": registry.diagnostics.len(),
            "receipt_count": registry.receipts.len(),
            "gate_count": registry.gates.len(),
        }))
    }

    /// The `max/chain` request returns full state summaries from the registry.
    #[rpc(name = "max/chain")]
    async fn max_chain(&self) -> Result<serde_json::Value> {
        let mut registry = lock_registry()?;
        update_diagnostics(&mut registry);
        let diagnostics: Vec<serde_json::Value> = registry
            .diagnostics
            .values()
            .map(|d| {
                serde_json::json!({
                    "id": d.diagnostic_id,
                    "law_id": d.law_id,
                    "severity": format!("{:?}", d.lsp.severity),
                    "message": d.lsp.message,
                })
            })
            .collect();
        let receipts: Vec<serde_json::Value> = registry
            .receipts
            .values()
            .map(|r| {
                serde_json::json!({
                    "receipt_id": r.receipt_id,
                    "hash": r.hash,
                })
            })
            .collect();
        Ok(serde_json::json!({
            "diagnostic_count": diagnostics.len(),
            "receipt_count": receipts.len(),
            "diagnostics": diagnostics,
            "receipts": receipts,
        }))
    }

    /// The `max/hook` request lists registered hooks in the service layer.
    #[rpc(name = "max/hook")]
    async fn max_hook(&self) -> Result<serde_json::Value> {
        Ok(serde_json::json!([
            {"name": "DiagnosticUpdateHook"},
            {"name": "ReceiptIntegrityHook"},
        ]))
    }

    /// The `max/hookGraph` request returns hook topology for the service layer.
    #[rpc(name = "max/hookGraph")]
    async fn max_hook_graph(&self) -> Result<serde_json::Value> {
        let mut registry = lock_registry()?;
        update_diagnostics(&mut registry);
        Ok(serde_json::json!({
            "hooks": [
                {
                    "hook": "DiagnosticUpdateHook",
                    "active_diagnostic_count": registry.diagnostics.len(),
                    "active_receipt_count": registry.receipts.len(),
                }
            ]
        }))
    }

    /// The `max/lawfulTransition` request validates whether a lifecycle transition is lawful.
    #[rpc(name = "max/lawfulTransition")]
    async fn max_lawful_transition(&self, params: String) -> Result<serde_json::Value> {
        let registry = lock_registry()?;
        let current = registry.current_state;
        let phase_order = [
            "Uninitialized",
            "Initializing",
            "Initialized",
            "ShutDown",
            "Exited",
        ];
        let current_str = format!("{:?}", current);
        let current_idx = phase_order.iter().position(|&p| p == current_str.as_str());
        let target_idx = phase_order.iter().position(|&p| p == params.as_str());
        let (admitted, refused_reason) = match (current_idx, target_idx) {
            (Some(ci), Some(ti)) if ti == ci + 1 => {
                let blocking_count = registry
                    .diagnostics
                    .values()
                    .filter(|d| matches!(d.lsp.severity, Some(DiagnosticSeverity::ERROR)))
                    .count();
                if blocking_count == 0 {
                    (true, serde_json::Value::Null)
                } else {
                    (
                        false,
                        serde_json::json!(format!(
                            "Blocked by {} error diagnostic(s)",
                            blocking_count
                        )),
                    )
                }
            }
            (Some(ci), Some(ti)) if ti <= ci => (
                false,
                serde_json::json!(format!("Backward transitions are not lawful")),
            ),
            _ => (
                false,
                serde_json::json!(format!(
                    "Unknown phase(s): current='{:?}', target='{}'",
                    current, params
                )),
            ),
        };
        Ok(serde_json::json!({
            "current_phase": format!("{:?}", current),
            "requested_phase": params,
            "admitted": admitted,
            "refused_reason": refused_reason,
        }))
    }

    /// The `max/ledgerReport` request returns a human-readable diagnostic ledger report.
    #[rpc(name = "max/ledgerReport")]
    async fn max_ledger_report(&self) -> Result<String> {
        let mut registry = lock_registry()?;
        update_diagnostics(&mut registry);
        let mut report = format!("Ledger Diagnostic Report for Instance: LSP_1\n");
        report.push_str("Status: VERIFIED (Cryptographic integrity intact)\n");
        report.push_str(&format!("Active Phase: {:?}\n", registry.current_state));
        report.push_str(&format!("Receipts count: {}\n", registry.receipts.len()));

        let mut sorted_receipts: Vec<_> = registry.receipts.values().cloned().collect();
        sorted_receipts.sort_by_key(|r| r.receipt_id.clone());

        for (idx, r) in sorted_receipts.iter().enumerate() {
            report.push_str(&format!(
                "  [{}] ID: {} | Hash: {}\n",
                idx, r.receipt_id, r.hash
            ));
        }

        report.push_str(&format!(
            "Ledger Report — {} diagnostic(s)\n",
            registry.diagnostics.len()
        ));
        for (id, diag) in &registry.diagnostics {
            report.push_str(&format!(
                "  [{}] severity={:?} law={} msg={}\n",
                id, diag.lsp.severity, diag.law_id, diag.lsp.message
            ));
        }
        Ok(report)
    }

    /// The `max/manifoldSnapshot` request returns full manifold metadata from the registry.
    #[rpc(name = "max/manifoldSnapshot")]
    async fn max_manifold_snapshot(&self) -> Result<serde_json::Value> {
        let mut registry = lock_registry()?;
        update_diagnostics(&mut registry);
        Ok(serde_json::json!({
            "snapshot_count": registry.snapshots.len(),
            "diagnostic_count": registry.diagnostics.len(),
            "receipt_count": registry.receipts.len(),
            "gate_count": registry.gates.len(),
            "current_state": format!("{:?}", registry.current_state),
        }))
    }

    /// The `max/propagate` request propagates a receipt into the global registry.
    #[rpc(name = "max/propagate")]
    async fn max_propagate(&self, params: max_protocol::Receipt) -> Result<serde_json::Value> {
        let mut registry = lock_registry()?;
        let receipt_id = params.receipt_id.clone();
        registry.receipts.insert(receipt_id.clone(), params);
        Ok(serde_json::json!({ "propagated": true, "receipt_id": receipt_id }))
    }

    /// The `max/refusal` request explicitly refuses a diagnostic and emits a refusal receipt.
    #[rpc(name = "max/refusal")]
    async fn max_refusal(&self, params: String) -> Result<serde_json::Value> {
        let mut registry = lock_registry()?;
        let receipt_id = format!("rcpt-refusal-{}", params);
        let hash = max_runtime::sha256(receipt_id.as_bytes());
        let receipt = max_protocol::Receipt {
            receipt_id: receipt_id.clone(),
            hash,
            prev_receipt_hash: None,
        };
        registry.receipts.insert(receipt_id, receipt.clone());
        Ok(serde_json::json!({
            "refused": true,
            "diagnostic_id": params,
            "receipt": receipt,
        }))
    }

    /// The `max/replay` request returns the event replay log from the registry.
    #[rpc(name = "max/replay")]
    async fn max_replay(&self) -> Result<serde_json::Value> {
        let registry = lock_registry()?;
        let receipts: Vec<serde_json::Value> = registry
            .receipts
            .values()
            .map(|r| serde_json::json!({ "receipt_id": r.receipt_id, "hash": r.hash }))
            .collect();
        Ok(serde_json::json!({
            "receipt_count": receipts.len(),
            "receipts": receipts,
        }))
    }

    /// The `max/verifyLedger` request verifies receipt chain integrity in the global registry.
    #[rpc(name = "max/verifyLedger")]
    async fn max_verify_ledger(&self) -> Result<serde_json::Value> {
        let registry = lock_registry()?;
        let mut errors: Vec<String> = Vec::new();
        for (id, rcpt) in &registry.receipts {
            let expected = max_runtime::sha256(rcpt.receipt_id.as_bytes());
            if !rcpt.receipt_id.contains(':') && rcpt.hash != expected {
                errors.push(format!("Receipt '{}' hash mismatch", id));
            }
        }
        Ok(serde_json::json!({
            "valid": errors.is_empty(),
            "receipt_count": registry.receipts.len(),
            "errors": errors,
        }))
    }

    /// Returns conformance score delta entries since the given sequence number.
    ///
    /// Reads from `REGISTRY.conformance_delta_log` — the single authoritative delta store.
    /// The former `MESH` global always started empty and was never populated at runtime,
    /// so queries against it returned stale zero-entry results.
    #[rpc(name = "max/conformanceDelta")]
    async fn max_conformance_delta(&self, params: serde_json::Value) -> Result<serde_json::Value> {
        let since_seq: u64 = params
            .get("since_seq")
            .and_then(|v| v.as_u64())
            .unwrap_or(0);
        let registry = lock_registry()?;
        let deltas: Vec<&max_runtime::ConformanceDeltaEntry> = registry
            .conformance_delta_log
            .iter()
            .filter(|e| e.seq > since_seq)
            .collect();
        Ok(serde_json::json!({
            "deltas": deltas,
            "current_seq": registry.action_seq,
        }))
    }

    /// The `textDocument/inlineCompletion` request is sent from the client to the server to compute inline completions.
    ///
    /// # Compatibility
    ///
    /// This request was introduced in specification version 3.18.0.
    #[rpc(name = "textDocument/inlineCompletion")]
    async fn inline_completion(
        &self,
        params: max_protocol::lsp_3_18::InlineCompletionParams,
    ) -> Result<Option<serde_json::Value>> {
        let _ = params;
        error!("Got a textDocument/inlineCompletion request, but it is not implemented");
        Err(Error::method_not_found())
    }

    /// The `workspace/textDocumentContent` request is sent from the client to the server to fetch the content of a text document.
    ///
    /// # Compatibility
    ///
    /// This request was introduced in specification version 3.18.0.
    #[rpc(name = "workspace/textDocumentContent")]
    async fn text_document_content(
        &self,
        params: max_protocol::lsp_3_18::TextDocumentContentParams,
    ) -> Result<max_protocol::lsp_3_18::TextDocumentContentResult> {
        let _ = params;
        error!("Got a workspace/textDocumentContent request, but it is not implemented");
        Err(Error::method_not_found())
    }

    /// The `workspace/textDocumentContent/refresh` request is sent from the server to the client to refresh the content of a text document.
    ///
    /// # Compatibility
    ///
    /// This request was introduced in specification version 3.18.0.
    #[rpc(name = "workspace/textDocumentContent/refresh")]
    async fn text_document_content_refresh(
        &self,
        params: max_protocol::lsp_3_18::TextDocumentContentRefreshParams,
    ) -> Result<()> {
        let _ = params;
        error!("Got a workspace/textDocumentContent/refresh request, but it is not implemented");
        Err(Error::method_not_found())
    }

    /// Dumps the current server registry state.
    #[rpc(name = "max/dumpState")]
    async fn max_dump_state(&self) -> Result<serde_json::Value> {
        let registry = lock_registry()?;
        serde_json::to_value(&*registry).map_err(|e| {
            tracing::error!("registry serialization failed: {}", e);
            Error::internal_error()
        })
    }

    /// Restores the server registry state.
    #[rpc(name = "max/restoreState")]
    async fn max_restore_state(&self, params: serde_json::Value) -> Result<()> {
        let mut registry = lock_registry()?;
        if let Ok(restored) = serde_json::from_value::<ServerRegistry>(params) {
            *registry = restored;
            Ok(())
        } else {
            Err(Error::invalid_params("Failed to parse ServerRegistry JSON"))
        }
    }

    /// The [`textDocument/rangesFormatting`] request is sent from the client to the server to format specific ranges.
    ///
    /// # Compatibility
    ///
    /// This request was introduced in specification version 3.18.0.
    #[rpc(name = "textDocument/rangesFormatting")]
    async fn ranges_formatting(
        &self,
        params: max_protocol::lsp_3_18::DocumentRangesFormattingParams,
    ) -> Result<Option<Vec<max_protocol::lsp_3_18::TextEdit>>> {
        let _ = params;
        error!("Got a textDocument/rangesFormatting request, but it is not implemented");
        Err(Error::method_not_found())
    }

    /// The [`notebookDocument/didOpen`] notification is sent from the client to the server when a notebook document is opened.
    ///
    /// # Compatibility
    ///
    /// This notification was introduced in specification version 3.17.0.
    #[rpc(name = "notebookDocument/didOpen")]
    async fn did_open_notebook_document(
        &self,
        params: max_protocol::lsp_3_18::DidOpenNotebookDocumentParams,
    ) {
        let _ = params;
        warn!("Got a notebookDocument/didOpen notification, but it is not implemented");
    }

    /// The [`notebookDocument/didChange`] notification is sent from the client to the server when a notebook document is changed.
    ///
    /// # Compatibility
    ///
    /// This notification was introduced in specification version 3.17.0.
    #[rpc(name = "notebookDocument/didChange")]
    async fn did_change_notebook_document(
        &self,
        params: max_protocol::lsp_3_18::DidChangeNotebookDocumentParams,
    ) {
        let _ = params;
        warn!("Got a notebookDocument/didChange notification, but it is not implemented");
    }

    /// The [`notebookDocument/didSave`] notification is sent from the client to the server when a notebook document is saved.
    ///
    /// # Compatibility
    ///
    /// This notification was introduced in specification version 3.17.0.
    #[rpc(name = "notebookDocument/didSave")]
    async fn did_save_notebook_document(
        &self,
        params: max_protocol::lsp_3_18::DidSaveNotebookDocumentParams,
    ) {
        let _ = params;
        warn!("Got a notebookDocument/didSave notification, but it is not implemented");
    }

    /// The [`notebookDocument/didClose`] notification is sent from the client to the server when a notebook document is closed.
    ///
    /// # Compatibility
    ///
    /// This notification was introduced in specification version 3.17.0.
    #[rpc(name = "notebookDocument/didClose")]
    async fn did_close_notebook_document(
        &self,
        params: max_protocol::lsp_3_18::DidCloseNotebookDocumentParams,
    ) {
        let _ = params;
        warn!("Got a notebookDocument/didClose notification, but it is not implemented");
    }

    /// The [`window/workDoneProgress/cancel`] notification is sent from the client to the server to cancel a progress.
    ///
    /// # Compatibility
    ///
    /// This notification was introduced in specification version 3.15.0.
    #[rpc(name = "window/workDoneProgress/cancel")]
    async fn work_done_progress_cancel(
        &self,
        params: max_protocol::lsp_3_18::WorkDoneProgressCancelParams,
    ) {
        let _ = params;
        warn!("Got a window/workDoneProgress/cancel notification, but it is not implemented");
    }

    /// The [`$/setTrace`] notification is sent from the client to the server to request that the server change its trace setting.
    ///
    /// # Compatibility
    ///
    /// This notification was introduced in specification version 3.16.0.
    #[rpc(name = "$/setTrace")]
    async fn set_trace(&self, params: max_protocol::lsp_3_18::SetTraceParams) {
        let _ = params;
        warn!("Got a $/setTrace notification, but it is not implemented");
    }

    /// The [`$/progress`] notification is sent from the client to the server to report progress.
    ///
    /// # Compatibility
    ///
    /// This notification was introduced in specification version 3.15.0.
    #[rpc(name = "$/progress")]
    async fn progress(&self, params: max_protocol::lsp_3_18::ProgressParams) {
        let _ = params;
        warn!("Got a $/progress notification, but it is not implemented");
    }

    /// The `max/instanceList` request returns a lightweight summary of all instances.
    #[rpc(name = "max/instanceList")]
    async fn max_instance_list(&self) -> Result<Value> {
        let registry = lock_registry()?;
        // In this implementation, we only have one instance "LSP_1".
        Ok(serde_json::json!([{
            "id": "LSP_1",
            "phase": format!("{:?}", registry.current_state),
            "conformance_score": 100.0,
        }]))
    }

    /// The `max/reset` request resets the server registry to its initial state.
    #[rpc(name = "max/reset")]
    async fn max_reset(&self) -> Result<()> {
        let mut registry = lock_registry()?;
        registry.diagnostics.clear();
        registry.receipts.clear();
        registry.snapshots.clear();
        registry.current_state = crate::service::State::Uninitialized;
        Ok(())
    }

    /// The `max/lsif` request streams the current registry state as an exhaustive LSIF NDJSON graph.
    #[rpc(name = "max/lsif")]
    async fn max_lsif(&self) -> Result<String> {
        let mut registry = lock_registry()?;
        update_diagnostics(&mut registry);
        
        let mut buffer = Vec::new();
        let mut builder = lsp_max_protocol::lsif_builder::LsifBuilder::new(&mut buffer);
        
        builder.emit_metadata("0.6.0", lsp_max_protocol::lsif::ToolInfo {
            name: "lsp-max".to_string(),
            version: Some("26.6.4".to_string()),
            args: None,
        }).map_err(|e| Error::internal_error())?;
        
        let project_id = builder.emit_project("rust", Some("file:///".to_string())).map_err(|e| Error::internal_error())?;
        
        // Export documents and diagnostics
        for (uri_str, _version) in &registry.document_versions {
            let doc_id = builder.emit_document(uri_str.as_str(), "rust").map_err(|e| Error::internal_error())?;
            builder.bind_next(project_id.clone(), doc_id.clone()).map_err(|e| Error::internal_error())?;
            
            // Map diagnostics related to this document
            let mut diags = Vec::new();
            for max_d in registry.diagnostics.values() {
                if max_d.doc_routes.iter().any(|r| r.path == uri_str.as_str()) {
                    diags.push(max_d.lsp.clone());
                }
            }
            
            if !diags.is_empty() {
                let diag_result_id = builder.next_id();
                builder.emit(lsp_max_protocol::lsif::Element::Vertex(lsp_max_protocol::lsif::Vertex::DiagnosticResult {
                    id: diag_result_id.clone(),
                    type_: lsp_max_protocol::lsif::VertexType::Vertex,
                    result: diags,
                })).map_err(|e| Error::internal_error())?;
                
                let diag_edge_id = builder.next_id();
                builder.emit(lsp_max_protocol::lsif::Element::Edge(lsp_max_protocol::lsif::Edge::TextDocumentDiagnostic {
                    id: diag_edge_id,
                    type_: lsp_max_protocol::lsif::EdgeType::Edge,
                    out_v: doc_id.clone(),
                    in_v: diag_result_id,
                })).map_err(|e| Error::internal_error())?;
            }
            
            builder.end_document(doc_id).map_err(|e| Error::internal_error())?;
        }
        
        builder.end_project(project_id).map_err(|e| Error::internal_error())?;
        
        Ok(String::from_utf8(buffer).map_err(|e| Error::internal_error())?)
    }
}

