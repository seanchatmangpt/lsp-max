use lsp_max_lsif::lsif::{Edge, Vertex};
use lsp_max_protocol::MaxDiagnostic;

pub fn nos_to_string(nos: &lsp_types_max::NumberOrString) -> String {
    match nos {
        lsp_types_max::NumberOrString::Number(n) => n.to_string(),
        lsp_types_max::NumberOrString::String(s) => s.clone(),
    }
}

pub fn get_vertex_id(vertex: &Vertex) -> String {
    match vertex {
        Vertex::MetaData { id, .. } => nos_to_string(id),
        Vertex::Source { id, .. } => nos_to_string(id),
        Vertex::Project { id, .. } => nos_to_string(id),
        Vertex::Document { id, .. } => nos_to_string(id),
        Vertex::ResultSet { id, .. } => nos_to_string(id),
        Vertex::Range { id, .. } => nos_to_string(id),
        Vertex::ResultRange { id, .. } => nos_to_string(id),
        Vertex::Moniker { id, .. } => nos_to_string(id),
        Vertex::PackageInformation { id, .. } => nos_to_string(id),
        Vertex::HoverResult { id, .. } => nos_to_string(id),
        Vertex::ReferenceResult { id, .. } => nos_to_string(id),
        Vertex::DeclarationResult { id, .. } => nos_to_string(id),
        Vertex::DefinitionResult { id, .. } => nos_to_string(id),
        Vertex::ImplementationResult { id, .. } => nos_to_string(id),
        Vertex::TypeDefinitionResult { id, .. } => nos_to_string(id),
        Vertex::CallHierarchyResult { id, .. } => nos_to_string(id),
        Vertex::TypeHierarchyResult { id, .. } => nos_to_string(id),
        Vertex::FoldingRangeResult { id, .. } => nos_to_string(id),
        Vertex::DocumentLinkResult { id, .. } => nos_to_string(id),
        Vertex::DocumentSymbolResult { id, .. } => nos_to_string(id),
        Vertex::DiagnosticResult { id, .. } => nos_to_string(id),
        Vertex::SemanticTokensResult { id, .. } => nos_to_string(id),
        Vertex::Event { id, .. } => nos_to_string(id),
    }
}

pub fn get_edge_out_v(edge: &Edge) -> String {
    match edge {
        Edge::Contains { out_v, .. } => nos_to_string(out_v),
        Edge::Next { out_v, .. } => nos_to_string(out_v),
        Edge::Moniker { out_v, .. } => nos_to_string(out_v),
        Edge::Attach { out_v, .. } => nos_to_string(out_v),
        Edge::PackageInformation { out_v, .. } => nos_to_string(out_v),
        Edge::Item { out_v, .. } => nos_to_string(out_v),
        Edge::TextDocumentHover { out_v, .. } => nos_to_string(out_v),
        Edge::TextDocumentDefinition { out_v, .. } => nos_to_string(out_v),
        Edge::TextDocumentDeclaration { out_v, .. } => nos_to_string(out_v),
        Edge::TextDocumentReferences { out_v, .. } => nos_to_string(out_v),
        Edge::TextDocumentImplementation { out_v, .. } => nos_to_string(out_v),
        Edge::TextDocumentTypeDefinition { out_v, .. } => nos_to_string(out_v),
        Edge::TextDocumentCallHierarchy { out_v, .. } => nos_to_string(out_v),
        Edge::TextDocumentTypeHierarchy { out_v, .. } => nos_to_string(out_v),
        Edge::TextDocumentFoldingRange { out_v, .. } => nos_to_string(out_v),
        Edge::TextDocumentDocumentLink { out_v, .. } => nos_to_string(out_v),
        Edge::TextDocumentDocumentSymbol { out_v, .. } => nos_to_string(out_v),
        Edge::TextDocumentDiagnostic { out_v, .. } => nos_to_string(out_v),
        Edge::TextDocumentSemanticTokens { out_v, .. } => nos_to_string(out_v),
    }
}

pub fn get_edge_in_vs(edge: &Edge) -> Vec<String> {
    match edge {
        Edge::Contains { in_vs, .. } => in_vs.iter().map(nos_to_string).collect(),
        Edge::Next { in_v, .. } => vec![nos_to_string(in_v)],
        Edge::Moniker { in_v, .. } => vec![nos_to_string(in_v)],
        Edge::Attach { in_v, .. } => vec![nos_to_string(in_v)],
        Edge::PackageInformation { in_v, .. } => vec![nos_to_string(in_v)],
        Edge::Item { in_vs, .. } => in_vs.iter().map(nos_to_string).collect(),
        Edge::TextDocumentHover { in_v, .. } => vec![nos_to_string(in_v)],
        Edge::TextDocumentDefinition { in_v, .. } => vec![nos_to_string(in_v)],
        Edge::TextDocumentDeclaration { in_v, .. } => vec![nos_to_string(in_v)],
        Edge::TextDocumentReferences { in_v, .. } => vec![nos_to_string(in_v)],
        Edge::TextDocumentImplementation { in_v, .. } => vec![nos_to_string(in_v)],
        Edge::TextDocumentTypeDefinition { in_v, .. } => vec![nos_to_string(in_v)],
        Edge::TextDocumentCallHierarchy { in_v, .. } => vec![nos_to_string(in_v)],
        Edge::TextDocumentTypeHierarchy { in_v, .. } => vec![nos_to_string(in_v)],
        Edge::TextDocumentFoldingRange { in_v, .. } => vec![nos_to_string(in_v)],
        Edge::TextDocumentDocumentLink { in_v, .. } => vec![nos_to_string(in_v)],
        Edge::TextDocumentDocumentSymbol { in_v, .. } => vec![nos_to_string(in_v)],
        Edge::TextDocumentDiagnostic { in_v, .. } => vec![nos_to_string(in_v)],
        Edge::TextDocumentSemanticTokens { in_v, .. } => vec![nos_to_string(in_v)],
    }
}

pub fn map_diagnostic_to_quads(
    diag: &MaxDiagnostic,
    graph_name: &oxigraph::model::GraphName,
) -> Vec<oxigraph::model::Quad> {
    let mut quads = Vec::new();
    let diag_id = if diag.diagnostic_id.is_empty() {
        uuid::Uuid::new_v4().to_string()
    } else {
        diag.diagnostic_id.clone()
    };
    let diag_uri = format!("urn:project:local:diagnostic:{}", diag_id);
    let subject = oxigraph::model::NamedOrBlankNode::NamedNode(
        oxigraph::model::NamedNode::new(&diag_uri).unwrap(),
    );

    quads.push(oxigraph::model::Quad::new(
        subject.clone(),
        oxigraph::model::NamedNode::new("http://www.w3.org/1999/02/22-rdf-syntax-ns#type").unwrap(),
        oxigraph::model::Term::NamedNode(
            oxigraph::model::NamedNode::new("urn:lsp-max:core:Diagnostic").unwrap(),
        ),
        graph_name.clone(),
    ));
    quads.push(oxigraph::model::Quad::new(
        subject.clone(),
        oxigraph::model::NamedNode::new("http://www.w3.org/1999/02/22-rdf-syntax-ns#type").unwrap(),
        oxigraph::model::Term::NamedNode(
            oxigraph::model::NamedNode::new("urn:lsp-max:core:Artifact").unwrap(),
        ),
        graph_name.clone(),
    ));

    quads.push(oxigraph::model::Quad::new(
        subject.clone(),
        oxigraph::model::NamedNode::new("urn:lsp-max:core:lawId").unwrap(),
        oxigraph::model::Term::Literal(oxigraph::model::Literal::new_simple_literal(&diag.law_id)),
        graph_name.clone(),
    ));

    quads.push(oxigraph::model::Quad::new(
        subject.clone(),
        oxigraph::model::NamedNode::new("urn:lsp-max:core:message").unwrap(),
        oxigraph::model::Term::Literal(oxigraph::model::Literal::new_simple_literal(
            &diag.lsp.message,
        )),
        graph_name.clone(),
    ));

    for route in &diag.doc_routes {
        quads.push(oxigraph::model::Quad::new(
            subject.clone(),
            oxigraph::model::NamedNode::new("urn:lsp-max:core:docUri").unwrap(),
            oxigraph::model::Term::Literal(oxigraph::model::Literal::new_simple_literal(
                &route.path,
            )),
            graph_name.clone(),
        ));
    }

    if let Some(ref obligation) = diag.receipt_obligation {
        for rcpt_id in &obligation.required_receipts {
            let rcpt_uri = format!("urn:lsp-max:receipt:{}", rcpt_id);
            quads.push(oxigraph::model::Quad::new(
                subject.clone(),
                oxigraph::model::NamedNode::new("http://www.w3.org/ns/prov#wasGeneratedBy")
                    .unwrap(),
                oxigraph::model::Term::NamedNode(
                    oxigraph::model::NamedNode::new(&rcpt_uri).unwrap(),
                ),
                graph_name.clone(),
            ));
        }
    }

    quads
}
