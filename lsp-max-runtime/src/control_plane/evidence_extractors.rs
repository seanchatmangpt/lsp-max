//! Oxigraph store extractors for evidence payloads.

use super::evidence_types::{
    CryptographicReceiptEvidencePayload, DiagnosticEvidencePayload, RangeEvidencePayload,
    WorkspaceEvidencePayload,
};

/// Helper to extract all admitted workspaces from an oxigraph Store.
pub fn extract_workspaces_from_store(
    store: &oxigraph::store::Store,
    graph_name: &oxigraph::model::GraphName,
) -> Result<Vec<WorkspaceEvidencePayload>, String> {
    let rdf_type =
        oxigraph::model::NamedNode::new("http://www.w3.org/1999/02/22-rdf-syntax-ns#type").unwrap();
    let project_type = oxigraph::model::Term::NamedNode(
        oxigraph::model::NamedNode::new("https://microsoft.github.io/language-server-protocol/specifications/lsif/0.6.0/specification/Project").unwrap()
    );
    let doc_type = oxigraph::model::Term::NamedNode(
        oxigraph::model::NamedNode::new("https://microsoft.github.io/language-server-protocol/specifications/lsif/0.6.0/specification/Document").unwrap()
    );
    let uri_pred = oxigraph::model::NamedNode::new("urn:lsp-max:core:uri").unwrap();

    let mut document_uris = Vec::new();
    for quad in store.quads_for_pattern(
        None,
        Some(rdf_type.as_ref()),
        Some(doc_type.as_ref()),
        Some(graph_name.as_ref()),
    ) {
        let quad = quad.map_err(|e| e.to_string())?;
        for uri_quad in store.quads_for_pattern(
            Some(quad.subject.as_ref()),
            Some(uri_pred.as_ref()),
            None,
            Some(graph_name.as_ref()),
        ) {
            let uri_quad = uri_quad.map_err(|e| e.to_string())?;
            if let oxigraph::model::Term::Literal(lit) = uri_quad.object {
                document_uris.push(lit.value().to_string());
            }
        }
    }

    let mut workspaces = Vec::new();
    for quad in store.quads_for_pattern(
        None,
        Some(rdf_type.as_ref()),
        Some(project_type.as_ref()),
        Some(graph_name.as_ref()),
    ) {
        let quad = quad.map_err(|e| e.to_string())?;
        workspaces.push(WorkspaceEvidencePayload {
            id: quad.subject.to_string(),
            kind: "Project".to_string(),
            document_uris: document_uris.clone(),
        });
    }

    if workspaces.is_empty() && !document_uris.is_empty() {
        workspaces.push(WorkspaceEvidencePayload {
            id: "urn:lsp-max:workspace:default".to_string(),
            kind: "Workspace".to_string(),
            document_uris,
        });
    }
    Ok(workspaces)
}

/// Helper to extract all admitted ranges from an oxigraph Store.
pub fn extract_ranges_from_store(
    store: &oxigraph::store::Store,
    graph_name: &oxigraph::model::GraphName,
) -> Result<Vec<RangeEvidencePayload>, String> {
    let rdf_type =
        oxigraph::model::NamedNode::new("http://www.w3.org/1999/02/22-rdf-syntax-ns#type").unwrap();
    let range_type = oxigraph::model::Term::NamedNode(
        oxigraph::model::NamedNode::new("https://microsoft.github.io/language-server-protocol/specifications/lsif/0.6.0/specification/Range").unwrap()
    );

    fn lit_u32(
        store: &oxigraph::store::Store,
        subject: oxigraph::model::NamedOrBlankNodeRef,
        pred: &oxigraph::model::NamedNode,
        graph: &oxigraph::model::GraphName,
    ) -> u32 {
        store
            .quads_for_pattern(
                Some(subject),
                Some(pred.as_ref()),
                None,
                Some(graph.as_ref()),
            )
            .flatten()
            .find_map(|q| {
                if let oxigraph::model::Term::Literal(l) = q.object {
                    l.value().parse().ok()
                } else {
                    None
                }
            })
            .unwrap_or(0)
    }

    let sl = oxigraph::model::NamedNode::new("urn:lsp-max:range:startLine").unwrap();
    let sc = oxigraph::model::NamedNode::new("urn:lsp-max:range:startCharacter").unwrap();
    let el = oxigraph::model::NamedNode::new("urn:lsp-max:range:endLine").unwrap();
    let ec = oxigraph::model::NamedNode::new("urn:lsp-max:range:endCharacter").unwrap();

    let mut ranges = Vec::new();
    for quad in store.quads_for_pattern(
        None,
        Some(rdf_type.as_ref()),
        Some(range_type.as_ref()),
        Some(graph_name.as_ref()),
    ) {
        let quad = quad.map_err(|e| e.to_string())?;
        let subj = quad.subject.as_ref();
        ranges.push(RangeEvidencePayload {
            start_line: lit_u32(store, subj, &sl, graph_name),
            start_character: lit_u32(store, subj, &sc, graph_name),
            end_line: lit_u32(store, subj, &el, graph_name),
            end_character: lit_u32(store, subj, &ec, graph_name),
        });
    }
    Ok(ranges)
}

/// Helper to extract all admitted diagnostics from an oxigraph Store.
pub fn extract_diagnostics_from_store(
    store: &oxigraph::store::Store,
    graph_name: &oxigraph::model::GraphName,
) -> Result<Vec<DiagnosticEvidencePayload>, String> {
    let rdf_type =
        oxigraph::model::NamedNode::new("http://www.w3.org/1999/02/22-rdf-syntax-ns#type").unwrap();
    let diag_type = oxigraph::model::Term::NamedNode(
        oxigraph::model::NamedNode::new("urn:lsp-max:core:Diagnostic").unwrap(),
    );
    let msg_pred = oxigraph::model::NamedNode::new("urn:lsp-max:core:message").unwrap();
    let sev_pred = oxigraph::model::NamedNode::new("urn:lsp-max:core:severity").unwrap();
    let code_pred = oxigraph::model::NamedNode::new("urn:lsp-max:core:code").unwrap();
    let source_pred = oxigraph::model::NamedNode::new("urn:lsp-max:core:source").unwrap();

    fn lit_str(
        store: &oxigraph::store::Store,
        subject: oxigraph::model::NamedOrBlankNodeRef,
        pred: &oxigraph::model::NamedNode,
        graph: &oxigraph::model::GraphName,
    ) -> Option<String> {
        store
            .quads_for_pattern(
                Some(subject),
                Some(pred.as_ref()),
                None,
                Some(graph.as_ref()),
            )
            .flatten()
            .find_map(|q| {
                if let oxigraph::model::Term::Literal(l) = q.object {
                    Some(l.value().to_string())
                } else {
                    None
                }
            })
    }

    let mut diagnostics = Vec::new();
    for quad in store.quads_for_pattern(
        None,
        Some(rdf_type.as_ref()),
        Some(diag_type.as_ref()),
        Some(graph_name.as_ref()),
    ) {
        let quad = quad.map_err(|e| e.to_string())?;
        let subj = quad.subject.as_ref();
        diagnostics.push(DiagnosticEvidencePayload {
            message: lit_str(store, subj, &msg_pred, graph_name).unwrap_or_default(),
            severity: lit_str(store, subj, &sev_pred, graph_name),
            code: lit_str(store, subj, &code_pred, graph_name),
            source: lit_str(store, subj, &source_pred, graph_name),
            range: None,
        });
    }
    Ok(diagnostics)
}

/// Helper to extract all admitted cryptographic receipts from an oxigraph Store.
pub fn extract_receipts_from_store(
    store: &oxigraph::store::Store,
    graph_name: &oxigraph::model::GraphName,
) -> Result<Vec<CryptographicReceiptEvidencePayload>, String> {
    let rdf_type =
        oxigraph::model::NamedNode::new("http://www.w3.org/1999/02/22-rdf-syntax-ns#type").unwrap();
    let rcpt_type = oxigraph::model::Term::NamedNode(
        oxigraph::model::NamedNode::new("urn:lsp-max:core:Receipt").unwrap(),
    );
    let prev_hash_pred =
        oxigraph::model::NamedNode::new("urn:lsp-max:core:prevHash").unwrap();
    let result_hash_pred =
        oxigraph::model::NamedNode::new("urn:lsp-max:core:resultHash").unwrap();
    let query_hash_pred =
        oxigraph::model::NamedNode::new("urn:lsp-max:core:queryHash").unwrap();
    let graph_hash_pred =
        oxigraph::model::NamedNode::new("urn:lsp-max:core:graphHash").unwrap();

    fn lit_str(
        store: &oxigraph::store::Store,
        subject: oxigraph::model::NamedOrBlankNodeRef,
        pred: &oxigraph::model::NamedNode,
        graph: &oxigraph::model::GraphName,
    ) -> String {
        store
            .quads_for_pattern(
                Some(subject),
                Some(pred.as_ref()),
                None,
                Some(graph.as_ref()),
            )
            .flatten()
            .find_map(|q| {
                if let oxigraph::model::Term::Literal(l) = q.object {
                    Some(l.value().to_string())
                } else {
                    None
                }
            })
            .unwrap_or_default()
    }

    let mut receipts = Vec::new();
    for quad in store.quads_for_pattern(
        None,
        Some(rdf_type.as_ref()),
        Some(rcpt_type.as_ref()),
        Some(graph_name.as_ref()),
    ) {
        let quad = quad.map_err(|e| e.to_string())?;
        let subj = quad.subject.as_ref();
        receipts.push(CryptographicReceiptEvidencePayload {
            prev_hash: lit_str(store, subj, &prev_hash_pred, graph_name),
            consequence_hash: lit_str(store, subj, &result_hash_pred, graph_name),
            law_id: lit_str(store, subj, &query_hash_pred, graph_name),
            discipline_id: lit_str(store, subj, &graph_hash_pred, graph_name),
            sequence: 0,
            signature: String::new(),
        });
    }
    Ok(receipts)
}
