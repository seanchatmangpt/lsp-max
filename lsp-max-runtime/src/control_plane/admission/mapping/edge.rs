use super::super::mapping_helpers::*;
use super::super::types::GraphAdmissionError;
use lsp_max_lsif::lsif::Edge;

pub(super) fn map_edge(
    edge: &Edge,
    graph_name: &oxigraph::model::GraphName,
    quads: &mut Vec<oxigraph::model::Quad>,
) -> Result<(), GraphAdmissionError> {
    let out_uri = format!("urn:project:local:lsif:{}", get_edge_out_v(edge));
    let subject = oxigraph::model::NamedOrBlankNode::NamedNode(
        oxigraph::model::NamedNode::new(&out_uri).unwrap(),
    );

    let predicate_uri = match edge {
        Edge::Contains { .. } => "https://microsoft.github.io/language-server-protocol/specifications/lsif/0.6.0/specification/contains",
        Edge::Next { .. } => "https://microsoft.github.io/language-server-protocol/specifications/lsif/0.6.0/specification/next",
        Edge::Moniker { .. } => "https://microsoft.github.io/language-server-protocol/specifications/lsif/0.6.0/specification/moniker",
        Edge::Attach { .. } => "https://microsoft.github.io/language-server-protocol/specifications/lsif/0.6.0/specification/attach",
        Edge::PackageInformation { .. } => "https://microsoft.github.io/language-server-protocol/specifications/lsif/0.6.0/specification/packageInformation",
        Edge::Item { .. } => "https://microsoft.github.io/language-server-protocol/specifications/lsif/0.6.0/specification/item",
        Edge::TextDocumentDefinition { .. } => "https://microsoft.github.io/language-server-protocol/specifications/lsif/0.6.0/specification/textDocument_definition",
        Edge::TextDocumentReferences { .. } => "https://microsoft.github.io/language-server-protocol/specifications/lsif/0.6.0/specification/textDocument_references",
        Edge::TextDocumentHover { .. } => "https://microsoft.github.io/language-server-protocol/specifications/lsif/0.6.0/specification/textDocument_hover",
        Edge::TextDocumentDeclaration { .. } => "https://microsoft.github.io/language-server-protocol/specifications/lsif/0.6.0/specification/textDocument_declaration",
        Edge::TextDocumentImplementation { .. } => "https://microsoft.github.io/language-server-protocol/specifications/lsif/0.6.0/specification/textDocument_implementation",
        Edge::TextDocumentTypeDefinition { .. } => "https://microsoft.github.io/language-server-protocol/specifications/lsif/0.6.0/specification/textDocument_typeDefinition",
        Edge::TextDocumentCallHierarchy { .. } => "https://microsoft.github.io/language-server-protocol/specifications/lsif/0.6.0/specification/textDocument_callHierarchy",
        Edge::TextDocumentTypeHierarchy { .. } => "https://microsoft.github.io/language-server-protocol/specifications/lsif/0.6.0/specification/textDocument_typeHierarchy",
        Edge::TextDocumentFoldingRange { .. } => "https://microsoft.github.io/language-server-protocol/specifications/lsif/0.6.0/specification/textDocument_foldingRange",
        Edge::TextDocumentDocumentLink { .. } => "https://microsoft.github.io/language-server-protocol/specifications/lsif/0.6.0/specification/textDocument_documentLink",
        Edge::TextDocumentDocumentSymbol { .. } => "https://microsoft.github.io/language-server-protocol/specifications/lsif/0.6.0/specification/textDocument_documentSymbol",
        Edge::TextDocumentDiagnostic { .. } => "https://microsoft.github.io/language-server-protocol/specifications/lsif/0.6.0/specification/textDocument_diagnostic",
        Edge::TextDocumentSemanticTokens { .. } => "https://microsoft.github.io/language-server-protocol/specifications/lsif/0.6.0/specification/textDocument_semanticTokens_full",
    };

    let whitelist = [
        "contains",
        "next",
        "moniker",
        "attach",
        "packageInformation",
        "item",
        "document",
        "property",
        "resultSet",
        "textDocument_definition",
        "textDocument_references",
        "textDocument_hover",
        "textDocument_declaration",
        "textDocument_implementation",
        "textDocument_typeDefinition",
        "textDocument_callHierarchy",
        "textDocument_typeHierarchy",
        "textDocument_foldingRange",
        "textDocument_documentLink",
        "textDocument_documentSymbol",
        "textDocument_diagnostic",
        "textDocument_semanticTokens_full",
    ];

    let suffix = predicate_uri.rsplit('/').next().unwrap_or("");
    if !whitelist.contains(&suffix) {
        return Err(GraphAdmissionError::NamespaceViolation(format!(
            "Ontology laundering: predicate {} not in whitelist",
            predicate_uri
        )));
    }

    let predicate = oxigraph::model::NamedNode::new(predicate_uri).unwrap();

    for in_v in get_edge_in_vs(edge) {
        let in_uri = format!("urn:project:local:lsif:{}", in_v);
        let object =
            oxigraph::model::Term::NamedNode(oxigraph::model::NamedNode::new(&in_uri).unwrap());
        quads.push(oxigraph::model::Quad::new(
            subject.clone(),
            predicate.clone(),
            object,
            graph_name.clone(),
        ));
    }

    // Edge::Item mapping properties on the target node
    if let Edge::Item {
        in_vs,
        document,
        property,
        ..
    } = edge
    {
        let doc_uri = format!("urn:project:local:lsif:{}", nos_to_string(document));
        let doc_node =
            oxigraph::model::Term::NamedNode(oxigraph::model::NamedNode::new(&doc_uri).unwrap());

        for in_v in in_vs {
            let target_uri = format!("urn:project:local:lsif:{}", nos_to_string(in_v));
            let target_subj = oxigraph::model::NamedOrBlankNode::NamedNode(
                oxigraph::model::NamedNode::new(&target_uri).unwrap(),
            );

            quads.push(oxigraph::model::Quad::new(
                target_subj.clone(),
                oxigraph::model::NamedNode::new("https://microsoft.github.io/language-server-protocol/specifications/lsif/0.6.0/specification/document").unwrap(),
                doc_node.clone(),
                graph_name.clone(),
            ));

            if let Some(prop) = property {
                let prop_str = match prop {
                    lsp_max_lsif::lsif::ItemEdgeProperty::Definitions => "definitions",
                    lsp_max_lsif::lsif::ItemEdgeProperty::Declarations => "declarations",
                    lsp_max_lsif::lsif::ItemEdgeProperty::References => "references",
                    lsp_max_lsif::lsif::ItemEdgeProperty::ReferenceResults => "referenceResults",
                    lsp_max_lsif::lsif::ItemEdgeProperty::ImplementationResults => {
                        "implementationResults"
                    }
                    lsp_max_lsif::lsif::ItemEdgeProperty::TypeDefinitions => {
                        "typeDefinitionResults"
                    }
                    lsp_max_lsif::lsif::ItemEdgeProperty::ReferenceLinks => "referenceLinks",
                };

                quads.push(oxigraph::model::Quad::new(
                    target_subj,
                    oxigraph::model::NamedNode::new("https://microsoft.github.io/language-server-protocol/specifications/lsif/0.6.0/specification/property").unwrap(),
                    oxigraph::model::Term::Literal(oxigraph::model::Literal::new_simple_literal(prop_str)),
                    graph_name.clone(),
                ));
            }
        }
    }

    Ok(())
}
