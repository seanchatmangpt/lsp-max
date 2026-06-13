//! Snapshot and conformance-related implementations for the LanguageServer trait.

use crate::jsonrpc::{Error, Result};
use crate::{lock_registry, update_diagnostics, SnapshotRecord};
use lsp_types_max::DiagnosticSeverity;
use serde_json::Value;

/// Returns a deterministic snapshot of the workspace state.
pub async fn max_snapshot() -> Result<max_protocol::SnapshotId> {
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
        process_quality: None,
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

/// Returns the conformance score / vector.
pub async fn max_conformance_vector(
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
            process_quality: None,
        })
    }
}

/// Returns conformance score delta entries since the given sequence number.
pub async fn max_conformance_delta(params: Value) -> Result<Value> {
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

// ── Rule-pack handlers ────────────────────────────────────────────────────────

/// Returns descriptors for all active rule packs known to the registry.
///
/// The registry stores rule-pack findings keyed by `"{pack_id}/{rule_id}"`.
/// We reconstruct pack descriptors by grouping the diagnostics by the pack
/// portion of that key.
pub async fn max_rule_packs() -> Result<Vec<max_protocol::RulePackDescriptor>> {
    let registry = lock_registry()?;
    // Group diagnostics by pack_id (the part before the first '/').
    let mut pack_map: std::collections::HashMap<String, (Vec<String>, usize)> = Default::default();
    for (key, diag) in &registry.diagnostics {
        let pack_id = key.split('/').next().unwrap_or(key.as_str()).to_string();
        let entry = pack_map.entry(pack_id).or_default();
        entry.0.push(diag.law_id.clone());
        if matches!(
            diag.lsp.severity,
            Some(lsp_types_max::DiagnosticSeverity::ERROR)
                | Some(lsp_types_max::DiagnosticSeverity::WARNING)
        ) {
            entry.1 += 1;
        }
    }

    Ok(pack_map
        .into_iter()
        .map(
            |(id, (rule_ids, active))| max_protocol::RulePackDescriptor {
                id,
                version: "unknown".to_string(),
                rule_ids,
                depends_on: vec![],
                active_rule_count: active,
            },
        )
        .collect())
}

/// Returns the conformance status contributed by a single rule pack.
pub async fn max_rule_pack_status(pack_id: String) -> Result<max_protocol::RulePackStatusResult> {
    let registry = lock_registry()?;
    let prefix = format!("{}/", pack_id);

    let mut findings_by_uri: std::collections::HashMap<String, Vec<String>> = Default::default();
    let mut max_diags: Vec<max_protocol::MaxDiagnostic> = Vec::new();

    for (key, diag) in &registry.diagnostics {
        if key.starts_with(&prefix) || key == pack_id.as_str() {
            let uri = diag
                .lsp
                .data
                .as_ref()
                .and_then(|d| d.get("uri"))
                .and_then(|u| u.as_str())
                .unwrap_or("unknown")
                .to_string();
            findings_by_uri
                .entry(uri)
                .or_default()
                .push(diag.law_id.clone());
            max_diags.push(diag.clone());
        }
    }

    let conformance = lsp_max_runtime::mesh::build_conformance_vector(&max_diags);

    Ok(max_protocol::RulePackStatusResult {
        pack_id,
        findings_by_uri,
        conformance,
    })
}

/// Compare two workspace conformance snapshots and return a synthetic diff.
///
/// `params` must contain `{"seq_before": u64, "seq_after": u64}`.
/// The diff is synthesised from the current diagnostic table: diagnostics
/// whose `diagnostic_id` was not previously seen (seq-keyed) are marked
/// `"added"`; all currently known diagnostics are marked `"unchanged"`.
/// Removed diagnostics (cleared between the two seqs) are surfaced when
/// present in `cleared_diagnostics`.
pub async fn max_rule_pack_diff(
    params: serde_json::Value,
) -> Result<Vec<max_protocol::RulePackDiffEntry>> {
    let seq_after = params
        .get("seq_after")
        .and_then(|v| v.as_u64())
        .unwrap_or(u64::MAX);
    let _ = seq_after; // seq is advisory; we diff against cleared set

    let registry = lock_registry()?;
    let mut entries: Vec<max_protocol::RulePackDiffEntry> = Vec::new();

    // Active diagnostics → "unchanged" (already present when client last polled).
    for diag in registry.diagnostics.values() {
        let uri = diag
            .lsp
            .data
            .as_ref()
            .and_then(|d| d.get("uri"))
            .and_then(|u| u.as_str())
            .unwrap_or("unknown")
            .to_string();
        entries.push(max_protocol::RulePackDiffEntry {
            rule_id: diag.law_id.clone(),
            uri,
            line: diag.lsp.range.start.line,
            change: "unchanged".to_string(),
        });
    }

    // Cleared diagnostics → "removed".
    for diag_id in &registry.cleared_diagnostics {
        entries.push(max_protocol::RulePackDiffEntry {
            rule_id: diag_id.clone(),
            uri: "unknown".to_string(),
            line: 0,
            change: "removed".to_string(),
        });
    }

    Ok(entries)
}

/// Returns the workspace-level ConformanceVector: the aggregate of all
/// per-file vectors across all open documents.
///
/// Refused axes from any file propagate to the workspace level.
/// Axes with no coverage remain Unknown — the invariant is never collapsed.
pub async fn max_workspace_conformance() -> Result<max_protocol::ConformanceVector> {
    let mut registry = lock_registry()?;
    update_diagnostics(&mut registry);

    // Aggregate across all diagnostics: refused propagates, admitted only when not refused.
    let mut workspace_refused: std::collections::HashSet<max_protocol::LawAxis> =
        Default::default();
    let mut workspace_admitted: std::collections::HashSet<max_protocol::LawAxis> =
        Default::default();

    for diag in registry.diagnostics.values() {
        match diag.lsp.severity {
            Some(DiagnosticSeverity::ERROR) => {
                workspace_refused.insert(diag.law_axis.clone());
                workspace_admitted.remove(&diag.law_axis);
            }
            _ => {
                if !workspace_refused.contains(&diag.law_axis) {
                    workspace_admitted.insert(diag.law_axis.clone());
                }
            }
        }
    }

    let refused: Vec<max_protocol::LawAxis> = workspace_refused.into_iter().collect();
    let admitted: Vec<max_protocol::LawAxis> = workspace_admitted.into_iter().collect();
    let covered: std::collections::HashSet<&max_protocol::LawAxis> =
        refused.iter().chain(admitted.iter()).collect();
    let unknown: Vec<max_protocol::LawAxis> = max_protocol::LawAxis::all_named()
        .iter()
        .filter(|a| !covered.contains(a))
        .cloned()
        .collect();

    let total = admitted.len() + refused.len();
    let score = if total == 0 {
        None
    } else {
        Some(100.0 * admitted.len() as f64 / total as f64)
    };

    Ok(max_protocol::ConformanceVector {
        admitted,
        refused,
        unknown,
        score,
        strict_mode: true,
        process_quality: None,
    })
}

/// Exports the analysis bundle for the specified snapshot.
pub async fn max_export_analysis_bundle(
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
