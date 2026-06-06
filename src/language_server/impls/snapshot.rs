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
