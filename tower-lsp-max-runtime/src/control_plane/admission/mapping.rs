use super::mapping_helpers::*;
use super::types::GraphAdmissionError;
use tower_lsp_max_lsif::lsif::{Edge, Element, Vertex};

pub fn map_element_to_quads(
    element: &Element,
    graph_name: &oxigraph::model::GraphName,
    quads: &mut Vec<oxigraph::model::Quad>,
) -> Result<(), GraphAdmissionError> {
    match element {
        Element::Vertex(vertex) => map_vertex(vertex, graph_name, quads),
        Element::Edge(edge) => map_edge(edge, graph_name, quads),
    }
}

fn map_vertex(
    vertex: &Vertex,
    graph_name: &oxigraph::model::GraphName,
    quads: &mut Vec<oxigraph::model::Quad>,
) -> Result<(), GraphAdmissionError> {
    let sub_uri = format!("urn:project:local:lsif:{}", get_vertex_id(vertex));
    let subject = oxigraph::model::NamedOrBlankNode::NamedNode(
        oxigraph::model::NamedNode::new(&sub_uri)
            .map_err(|e| GraphAdmissionError::ParsingFailed(e.to_string()))?,
    );

    let class_uri = match vertex {
        Vertex::MetaData { .. } => "urn:tower-lsp-max:core:Metadata",
        Vertex::Project { .. } => "https://microsoft.github.io/language-server-protocol/specifications/lsif/0.6.0/specification/Project",
        Vertex::Document { .. } => "https://microsoft.github.io/language-server-protocol/specifications/lsif/0.6.0/specification/Document",
        Vertex::ResultSet { .. } => "https://microsoft.github.io/language-server-protocol/specifications/lsif/0.6.0/specification/ResultSet",
        Vertex::Range { .. } => "https://microsoft.github.io/language-server-protocol/specifications/lsif/0.6.0/specification/Range",
        Vertex::Moniker { .. } => "https://microsoft.github.io/language-server-protocol/specifications/lsif/0.6.0/specification/Moniker",
        Vertex::PackageInformation { .. } => "https://microsoft.github.io/language-server-protocol/specifications/lsif/0.6.0/specification/PackageInformation",
        Vertex::HoverResult { .. } => "https://microsoft.github.io/language-server-protocol/specifications/lsif/0.6.0/specification/HoverResult",
        Vertex::DefinitionResult { .. } => "https://microsoft.github.io/language-server-protocol/specifications/lsif/0.6.0/specification/DefinitionResult",
        Vertex::ReferenceResult { .. } => "https://microsoft.github.io/language-server-protocol/specifications/lsif/0.6.0/specification/ReferenceResult",
        Vertex::DeclarationResult { .. } => "https://microsoft.github.io/language-server-protocol/specifications/lsif/0.6.0/specification/DeclarationResult",
        Vertex::ImplementationResult { .. } => "https://microsoft.github.io/language-server-protocol/specifications/lsif/0.6.0/specification/ImplementationResult",
        Vertex::TypeDefinitionResult { .. } => "https://microsoft.github.io/language-server-protocol/specifications/lsif/0.6.0/specification/TypeDefinitionResult",
        Vertex::CallHierarchyResult { .. } => "https://microsoft.github.io/language-server-protocol/specifications/lsif/0.6.0/specification/CallHierarchyResult",
        Vertex::TypeHierarchyResult { .. } => "https://microsoft.github.io/language-server-protocol/specifications/lsif/0.6.0/specification/TypeHierarchyResult",
        _ => "urn:tower-lsp-max:core:GenericVertex",
    };

    quads.push(oxigraph::model::Quad::new(
        subject.clone(),
        oxigraph::model::NamedNode::new("http://www.w3.org/1999/02/22-rdf-syntax-ns#type").unwrap(),
        oxigraph::model::Term::NamedNode(oxigraph::model::NamedNode::new(class_uri).unwrap()),
        graph_name.clone(),
    ));

    match vertex {
        Vertex::MetaData {
            version,
            project_root,
            position_encoding,
            ..
        } => {
            quads.push(oxigraph::model::Quad::new(
                subject.clone(),
                oxigraph::model::NamedNode::new("urn:tower-lsp-max:core:version").unwrap(),
                oxigraph::model::Term::Literal(oxigraph::model::Literal::new_simple_literal(
                    version,
                )),
                graph_name.clone(),
            ));
            quads.push(oxigraph::model::Quad::new(
                subject.clone(),
                oxigraph::model::NamedNode::new("urn:tower-lsp-max:core:projectRoot").unwrap(),
                oxigraph::model::Term::Literal(oxigraph::model::Literal::new_simple_literal(
                    project_root,
                )),
                graph_name.clone(),
            ));
            let pos_enc_str = match position_encoding {
                tower_lsp_max_lsif::lsif::PositionEncoding::Utf8 => "utf-8",
                tower_lsp_max_lsif::lsif::PositionEncoding::Utf16 => "utf-16",
                tower_lsp_max_lsif::lsif::PositionEncoding::Utf32 => "utf-32",
            };
            quads.push(oxigraph::model::Quad::new(
                subject.clone(),
                oxigraph::model::NamedNode::new("urn:tower-lsp-max:core:positionEncoding").unwrap(),
                oxigraph::model::Term::Literal(oxigraph::model::Literal::new_simple_literal(
                    pos_enc_str,
                )),
                graph_name.clone(),
            ));
        }
        Vertex::Project {
            kind,
            resource,
            contents,
            ..
        } => {
            if let Some(k) = kind {
                quads.push(oxigraph::model::Quad::new(
                    subject.clone(),
                    oxigraph::model::NamedNode::new("urn:tower-lsp-max:core:kind").unwrap(),
                    oxigraph::model::Term::Literal(oxigraph::model::Literal::new_simple_literal(k)),
                    graph_name.clone(),
                ));
            }
            if let Some(res) = resource {
                quads.push(oxigraph::model::Quad::new(
                    subject.clone(),
                    oxigraph::model::NamedNode::new("urn:tower-lsp-max:core:resource").unwrap(),
                    oxigraph::model::Term::Literal(oxigraph::model::Literal::new_simple_literal(
                        res,
                    )),
                    graph_name.clone(),
                ));
            }
            if let Some(cont) = contents {
                quads.push(oxigraph::model::Quad::new(
                    subject.clone(),
                    oxigraph::model::NamedNode::new("urn:tower-lsp-max:core:contents").unwrap(),
                    oxigraph::model::Term::Literal(oxigraph::model::Literal::new_simple_literal(
                        cont,
                    )),
                    graph_name.clone(),
                ));
            }
        }
        Vertex::Document {
            uri, language_id, ..
        } => {
            quads.push(oxigraph::model::Quad::new(
                subject.clone(),
                oxigraph::model::NamedNode::new("urn:tower-lsp-max:core:uri").unwrap(),
                oxigraph::model::Term::Literal(oxigraph::model::Literal::new_simple_literal(uri)),
                graph_name.clone(),
            ));
            quads.push(oxigraph::model::Quad::new(
                subject.clone(),
                oxigraph::model::NamedNode::new("urn:tower-lsp-max:core:languageId").unwrap(),
                oxigraph::model::Term::Literal(oxigraph::model::Literal::new_simple_literal(
                    language_id,
                )),
                graph_name.clone(),
            ));
        }
        Vertex::Range {
            start, end, tag, ..
        } => {
            quads.push(oxigraph::model::Quad::new(
                subject.clone(),
                oxigraph::model::NamedNode::new("urn:tower-lsp-max:core:startLine").unwrap(),
                oxigraph::model::Term::Literal(oxigraph::model::Literal::new_simple_literal(
                    start.line.to_string(),
                )),
                graph_name.clone(),
            ));
            quads.push(oxigraph::model::Quad::new(
                subject.clone(),
                oxigraph::model::NamedNode::new("urn:tower-lsp-max:core:startCharacter").unwrap(),
                oxigraph::model::Term::Literal(oxigraph::model::Literal::new_simple_literal(
                    start.character.to_string(),
                )),
                graph_name.clone(),
            ));
            quads.push(oxigraph::model::Quad::new(
                subject.clone(),
                oxigraph::model::NamedNode::new("urn:tower-lsp-max:core:endLine").unwrap(),
                oxigraph::model::Term::Literal(oxigraph::model::Literal::new_simple_literal(
                    end.line.to_string(),
                )),
                graph_name.clone(),
            ));
            quads.push(oxigraph::model::Quad::new(
                subject.clone(),
                oxigraph::model::NamedNode::new("urn:tower-lsp-max:core:endCharacter").unwrap(),
                oxigraph::model::Term::Literal(oxigraph::model::Literal::new_simple_literal(
                    end.character.to_string(),
                )),
                graph_name.clone(),
            ));

            if let Some(range_tag) = tag {
                let (tag_type, text, symbol_kind, full_range, detail) = match range_tag {
                    tower_lsp_max_lsif::lsif::RangeTag::Declaration {
                        text,
                        kind,
                        full_range,
                        detail,
                    } => (
                        Some("declaration"),
                        Some(text),
                        Some(format!("{:?}", kind)),
                        Some(full_range),
                        detail.as_ref(),
                    ),
                    tower_lsp_max_lsif::lsif::RangeTag::Definition {
                        text,
                        kind,
                        full_range,
                        detail,
                    } => (
                        Some("definition"),
                        Some(text),
                        Some(format!("{:?}", kind)),
                        Some(full_range),
                        detail.as_ref(),
                    ),
                    tower_lsp_max_lsif::lsif::RangeTag::Reference { text } => {
                        (Some("reference"), Some(text), None, None, None)
                    }
                    tower_lsp_max_lsif::lsif::RangeTag::Unknown { text } => {
                        (Some("unknown"), Some(text), None, None, None)
                    }
                };

                if let Some(tt) = tag_type {
                    quads.push(oxigraph::model::Quad::new(
                        subject.clone(),
                        oxigraph::model::NamedNode::new("urn:tower-lsp-max:core:tagType").unwrap(),
                        oxigraph::model::Term::Literal(
                            oxigraph::model::Literal::new_simple_literal(tt),
                        ),
                        graph_name.clone(),
                    ));
                }
                if let Some(t) = text {
                    quads.push(oxigraph::model::Quad::new(
                        subject.clone(),
                        oxigraph::model::NamedNode::new("urn:tower-lsp-max:core:text").unwrap(),
                        oxigraph::model::Term::Literal(
                            oxigraph::model::Literal::new_simple_literal(t),
                        ),
                        graph_name.clone(),
                    ));
                }
                if let Some(sk) = symbol_kind {
                    quads.push(oxigraph::model::Quad::new(
                        subject.clone(),
                        oxigraph::model::NamedNode::new("urn:tower-lsp-max:core:symbolKind")
                            .unwrap(),
                        oxigraph::model::Term::Literal(
                            oxigraph::model::Literal::new_simple_literal(sk),
                        ),
                        graph_name.clone(),
                    ));
                }
                if let Some(det) = detail {
                    quads.push(oxigraph::model::Quad::new(
                        subject.clone(),
                        oxigraph::model::NamedNode::new("urn:tower-lsp-max:core:detail").unwrap(),
                        oxigraph::model::Term::Literal(
                            oxigraph::model::Literal::new_simple_literal(det),
                        ),
                        graph_name.clone(),
                    ));
                }
                if let Some(fr) = full_range {
                    quads.push(oxigraph::model::Quad::new(
                        subject.clone(),
                        oxigraph::model::NamedNode::new("urn:tower-lsp-max:core:fullStartLine")
                            .unwrap(),
                        oxigraph::model::Term::Literal(
                            oxigraph::model::Literal::new_simple_literal(fr.start.line.to_string()),
                        ),
                        graph_name.clone(),
                    ));
                    quads.push(oxigraph::model::Quad::new(
                        subject.clone(),
                        oxigraph::model::NamedNode::new(
                            "urn:tower-lsp-max:core:fullStartCharacter",
                        )
                        .unwrap(),
                        oxigraph::model::Term::Literal(
                            oxigraph::model::Literal::new_simple_literal(
                                fr.start.character.to_string(),
                            ),
                        ),
                        graph_name.clone(),
                    ));
                    quads.push(oxigraph::model::Quad::new(
                        subject.clone(),
                        oxigraph::model::NamedNode::new("urn:tower-lsp-max:core:fullEndLine")
                            .unwrap(),
                        oxigraph::model::Term::Literal(
                            oxigraph::model::Literal::new_simple_literal(fr.end.line.to_string()),
                        ),
                        graph_name.clone(),
                    ));
                    quads.push(oxigraph::model::Quad::new(
                        subject.clone(),
                        oxigraph::model::NamedNode::new("urn:tower-lsp-max:core:fullEndCharacter")
                            .unwrap(),
                        oxigraph::model::Term::Literal(
                            oxigraph::model::Literal::new_simple_literal(
                                fr.end.character.to_string(),
                            ),
                        ),
                        graph_name.clone(),
                    ));
                }
            }
        }
        Vertex::Moniker {
            scheme,
            identifier,
            kind,
            unique,
            ..
        } => {
            quads.push(oxigraph::model::Quad::new(
                subject.clone(),
                oxigraph::model::NamedNode::new("urn:tower-lsp-max:core:scheme").unwrap(),
                oxigraph::model::Term::Literal(oxigraph::model::Literal::new_simple_literal(
                    scheme,
                )),
                graph_name.clone(),
            ));
            quads.push(oxigraph::model::Quad::new(
                subject.clone(),
                oxigraph::model::NamedNode::new("urn:tower-lsp-max:core:identifier").unwrap(),
                oxigraph::model::Term::Literal(oxigraph::model::Literal::new_simple_literal(
                    identifier,
                )),
                graph_name.clone(),
            ));
            let kind_str = match kind {
                tower_lsp_max_lsif::lsif_types::MonikerKind::Import => "import",
                tower_lsp_max_lsif::lsif_types::MonikerKind::Export => "export",
                tower_lsp_max_lsif::lsif_types::MonikerKind::Local => "local",
            };
            quads.push(oxigraph::model::Quad::new(
                subject.clone(),
                oxigraph::model::NamedNode::new("urn:tower-lsp-max:core:kind").unwrap(),
                oxigraph::model::Term::Literal(oxigraph::model::Literal::new_simple_literal(
                    kind_str,
                )),
                graph_name.clone(),
            ));
            let unique_str = match unique {
                tower_lsp_max_lsif::lsif_types::UniquenessLevel::Document => "document",
                tower_lsp_max_lsif::lsif_types::UniquenessLevel::Project => "project",
                tower_lsp_max_lsif::lsif_types::UniquenessLevel::Group => "group",
                tower_lsp_max_lsif::lsif_types::UniquenessLevel::Scheme => "scheme",
                tower_lsp_max_lsif::lsif_types::UniquenessLevel::Global => "global",
            };
            quads.push(oxigraph::model::Quad::new(
                subject.clone(),
                oxigraph::model::NamedNode::new("urn:tower-lsp-max:core:unique").unwrap(),
                oxigraph::model::Term::Literal(oxigraph::model::Literal::new_simple_literal(
                    unique_str,
                )),
                graph_name.clone(),
            ));
        }
        Vertex::PackageInformation {
            name,
            manager,
            version,
            repository,
            ..
        } => {
            quads.push(oxigraph::model::Quad::new(
                subject.clone(),
                oxigraph::model::NamedNode::new("urn:tower-lsp-max:core:name").unwrap(),
                oxigraph::model::Term::Literal(oxigraph::model::Literal::new_simple_literal(name)),
                graph_name.clone(),
            ));
            quads.push(oxigraph::model::Quad::new(
                subject.clone(),
                oxigraph::model::NamedNode::new("urn:tower-lsp-max:core:manager").unwrap(),
                oxigraph::model::Term::Literal(oxigraph::model::Literal::new_simple_literal(
                    manager,
                )),
                graph_name.clone(),
            ));
            quads.push(oxigraph::model::Quad::new(
                subject.clone(),
                oxigraph::model::NamedNode::new("urn:tower-lsp-max:core:version").unwrap(),
                oxigraph::model::Term::Literal(oxigraph::model::Literal::new_simple_literal(
                    version,
                )),
                graph_name.clone(),
            ));
            if let Some(repo) = repository {
                quads.push(oxigraph::model::Quad::new(
                    subject.clone(),
                    oxigraph::model::NamedNode::new("urn:tower-lsp-max:core:repositoryUrl")
                        .unwrap(),
                    oxigraph::model::Term::Literal(oxigraph::model::Literal::new_simple_literal(
                        &repo.url,
                    )),
                    graph_name.clone(),
                ));
                quads.push(oxigraph::model::Quad::new(
                    subject.clone(),
                    oxigraph::model::NamedNode::new("urn:tower-lsp-max:core:repositoryType")
                        .unwrap(),
                    oxigraph::model::Term::Literal(oxigraph::model::Literal::new_simple_literal(
                        &repo.type_,
                    )),
                    graph_name.clone(),
                ));
            }
        }
        Vertex::HoverResult { result, .. } => {
            if let Ok(json_str) = serde_json::to_string(result) {
                quads.push(oxigraph::model::Quad::new(
                    subject.clone(),
                    oxigraph::model::NamedNode::new("urn:tower-lsp-max:core:hoverData").unwrap(),
                    oxigraph::model::Term::Literal(oxigraph::model::Literal::new_simple_literal(
                        &json_str,
                    )),
                    graph_name.clone(),
                ));
            }
        }
        Vertex::DiagnosticResult { result, .. } => {
            if let Ok(json_str) = serde_json::to_string(result) {
                quads.push(oxigraph::model::Quad::new(
                    subject.clone(),
                    oxigraph::model::NamedNode::new("urn:tower-lsp-max:core:diagnosticData")
                        .unwrap(),
                    oxigraph::model::Term::Literal(oxigraph::model::Literal::new_simple_literal(
                        &json_str,
                    )),
                    graph_name.clone(),
                ));
            }
        }
        _ => {}
    }

    Ok(())
}

fn map_edge(
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
                    tower_lsp_max_lsif::lsif::ItemEdgeProperty::Definitions => "definitions",
                    tower_lsp_max_lsif::lsif::ItemEdgeProperty::Declarations => "declarations",
                    tower_lsp_max_lsif::lsif::ItemEdgeProperty::References => "references",
                    tower_lsp_max_lsif::lsif::ItemEdgeProperty::ReferenceResults => {
                        "referenceResults"
                    }
                    tower_lsp_max_lsif::lsif::ItemEdgeProperty::ImplementationResults => {
                        "implementationResults"
                    }
                    tower_lsp_max_lsif::lsif::ItemEdgeProperty::TypeDefinitions => {
                        "typeDefinitionResults"
                    }
                    tower_lsp_max_lsif::lsif::ItemEdgeProperty::ReferenceLinks => "referenceLinks",
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
