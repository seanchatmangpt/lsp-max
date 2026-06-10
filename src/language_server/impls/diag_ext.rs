//! Advanced Diagnostics (DiagnosticServerCapabilities) implementation.
//!
//! This module implements the pull-based diagnostic model introduced in LSP 3.17,
//! providing both document-scoped and workspace-scoped diagnostic reports.
//! It integrates with the `tower-lsp-max` registry and autonomic loop.

use crate::jsonrpc::Result;
use crate::{lock_registry, update_diagnostics};
use lsp_types_max::*;
use std::collections::HashMap;
use std::str::FromStr;

/// Handler for the `textDocument/diagnostic` endpoint.
/// Implements pull-based diagnostics for a single document.
pub async fn diagnostic(
    _params: DocumentDiagnosticParams,
) -> Result<DocumentDiagnosticReportResult> {
    let mut registry = lock_registry()?;
    update_diagnostics(&mut registry);

    let requested_uri = _params.text_document.uri;
    let requested_path = requested_uri.path();

    let mut items = Vec::new();

    for diag in registry.diagnostics.values() {
        // If the diagnostic has doc_routes, check if it matches the requested URI.
        // If it has no doc_routes, we treat it as a global diagnostic that might
        // be relevant to multiple documents or just the main ones.
        let is_relevant = if diag.doc_routes.is_empty() {
            true // Global diagnostics are returned for all pull requests in this impl
        } else {
            diag.doc_routes.iter().any(|route| {
                requested_path.as_str().ends_with(&route.path)
                    || route.path == requested_path.as_str()
            })
        };

        if is_relevant {
            items.push(diag.lsp.clone());
        }
    }

    Ok(DocumentDiagnosticReportResult::Report(
        DocumentDiagnosticReport::Full(RelatedFullDocumentDiagnosticReport {
            related_documents: None,
            full_document_diagnostic_report: FullDocumentDiagnosticReport {
                result_id: None, // We don't implement result-id caching in this version
                items,
            },
        }),
    ))
}

/// Handler for the `workspace/diagnostic` endpoint.
/// Implements pull-based diagnostics for the entire workspace.
pub async fn workspace_diagnostic(
    _params: WorkspaceDiagnosticParams,
) -> Result<WorkspaceDiagnosticReportResult> {
    let mut registry = lock_registry()?;
    update_diagnostics(&mut registry);

    // Collect all diagnostics and group them by URI if possible.
    // Since MaxDiagnostic doesn't strictly own a URI but has doc_routes,
    // we use doc_routes to attribute them.

    let mut reports = Vec::new();
    let mut global_items = Vec::new();
    let mut uri_to_items: HashMap<Uri, Vec<Diagnostic>> = HashMap::new();

    for diag in registry.diagnostics.values() {
        if diag.doc_routes.is_empty() {
            global_items.push(diag.lsp.clone());
        } else {
            for route in &diag.doc_routes {
                // Reconstruct the URI directly from the absolute route path mapped in the registry.
                if let Ok(uri) = Uri::from_str(&format!("file://{}", route.path)) {
                    uri_to_items.entry(uri).or_default().push(diag.lsp.clone());
                } else {
                    global_items.push(diag.lsp.clone());
                }
            }
        }
    }

    // Add document-specific reports
    for (uri, items) in uri_to_items {
        reports.push(WorkspaceDocumentDiagnosticReport::Full(
            WorkspaceFullDocumentDiagnosticReport {
                uri,
                version: None,
                full_document_diagnostic_report: FullDocumentDiagnosticReport {
                    result_id: None,
                    items,
                },
            },
        ));
    }

    // If uri_to_items was empty but we have global items, we attach them to the root registry URI.
    if reports.is_empty() && !global_items.is_empty() {
        // Fallback: use root path
        if let Ok(root_uri) = Uri::from_str(&format!("file://{}", registry.root_path.display())) {
            reports.push(WorkspaceDocumentDiagnosticReport::Full(
                WorkspaceFullDocumentDiagnosticReport {
                    uri: root_uri,
                    version: None,
                    full_document_diagnostic_report: FullDocumentDiagnosticReport {
                        result_id: None,
                        items: global_items,
                    },
                },
            ));
        }
    }

    Ok(WorkspaceDiagnosticReportResult::Report(
        WorkspaceDiagnosticReport { items: reports },
    ))
}

pub async fn work_done_progress_cancel(_params: WorkDoneProgressCancelParams) {}

pub async fn set_trace(_params: SetTraceParams) {}

pub async fn progress(_params: ProgressParams) {}
