//! LSIF generation, state dump, restore, and reset implementations.

use crate::jsonrpc::{Error, Result};
use crate::{lock_registry, update_diagnostics, ServerRegistry};
use serde_json::Value;
use tower_lsp_max_lsif::lsif::*;
use tower_lsp_max_lsif::lsif_builder::LsifBuilder;
use url::Url;

/// Dumps the current server registry state.
pub async fn max_dump_state() -> Result<serde_json::Value> {
    let registry = lock_registry()?;
    serde_json::to_value(&*registry).map_err(|e| {
        tracing::error!("registry serialization failed: {}", e);
        Error::internal_error()
    })
}

/// Restores the server registry state.
pub async fn max_restore_state(params: serde_json::Value) -> Result<()> {
    let mut registry = lock_registry()?;
    if let Ok(restored) = serde_json::from_value::<ServerRegistry>(params) {
        *registry = restored;
        Ok(())
    } else {
        Err(Error::invalid_params("Failed to parse ServerRegistry JSON"))
    }
}

/// Returns a lightweight summary of all instances.
pub async fn max_instance_list() -> Result<Value> {
    let registry = lock_registry()?;
    // In this implementation, we only have one instance "LSP_1".
    Ok(serde_json::json!([{
        "id": "LSP_1",
        "phase": format!("{:?}", registry.current_state),
        "conformance_score": 100.0,
    }]))
}

/// Resets the server registry to its initial state.
pub async fn max_reset() -> Result<()> {
    let mut registry = lock_registry()?;
    registry.diagnostics.clear();
    registry.receipts.clear();
    registry.snapshots.clear();
    registry.current_state = crate::service::State::Uninitialized;
    Ok(())
}

/// Streams the current registry state as an LSIF NDJSON graph.
pub async fn max_lsif() -> Result<String> {
    let mut registry = lock_registry()?;
    update_diagnostics(&mut registry);

    let mut buffer = Vec::new();
    let mut builder = LsifBuilder::new(&mut buffer);

    builder
        .emit_metadata(
            "0.6.0",
            "file:///",
            ToolInfo {
                name: "tower-lsp-max".to_string(),
                version: Some("26.6.5".to_string()),
                args: Some(vec![]),
            },
        )
        .map_err(|_| Error::internal_error())?;

    let project_id = builder
        .emit_project(Some("rust"), Some("file:///".to_string()))
        .map_err(|_| Error::internal_error())?;

    // Export documents and diagnostics
    for (uri_str, _version) in &registry.document_versions {
        let doc_id = builder
            .emit_document(uri_str.as_str(), "rust")
            .map_err(|_| Error::internal_error())?;
        
        builder
            .contains(project_id.clone(), vec![doc_id.clone()])
            .map_err(|_| Error::internal_error())?;

        // Map diagnostics related to this document
        let mut diags = Vec::new();
        for max_d in registry.diagnostics.values() {
            if max_d.doc_routes.iter().any(|r| r.path == uri_str.as_str()) {
                diags.push(max_d.lsp.clone());
            }
        }

        if !diags.is_empty() {
            let diag_result_id = builder
                .diagnostic_result(diags)
                .map_err(|_| Error::internal_error())?;

            builder
                .diagnostic_edge(doc_id.clone(), diag_result_id)
                .map_err(|_| Error::internal_error())?;
        }

        builder.end_document(doc_id).map_err(|_| Error::internal_error())?;
    }

    builder.end_project(project_id).map_err(|_| Error::internal_error())?;

    Ok(String::from_utf8(buffer).map_err(|_| Error::internal_error())?)
}
