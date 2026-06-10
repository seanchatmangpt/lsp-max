use super::super::mapping_helpers::*;
use super::super::types::GraphAdmissionError;
use lsp_max_lsif::lsif::Vertex;

pub(super) fn map_vertex(
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
                lsp_max_lsif::lsif::PositionEncoding::Utf8 => "utf-8",
                lsp_max_lsif::lsif::PositionEncoding::Utf16 => "utf-16",
                lsp_max_lsif::lsif::PositionEncoding::Utf32 => "utf-32",
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
                    lsp_max_lsif::lsif::RangeTag::Declaration {
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
                    lsp_max_lsif::lsif::RangeTag::Definition {
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
                    lsp_max_lsif::lsif::RangeTag::Reference { text } => {
                        (Some("reference"), Some(text), None, None, None)
                    }
                    lsp_max_lsif::lsif::RangeTag::Unknown { text } => {
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
                lsp_max_lsif::lsif_types::MonikerKind::Import => "import",
                lsp_max_lsif::lsif_types::MonikerKind::Export => "export",
                lsp_max_lsif::lsif_types::MonikerKind::Local => "local",
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
                lsp_max_lsif::lsif_types::UniquenessLevel::Document => "document",
                lsp_max_lsif::lsif_types::UniquenessLevel::Project => "project",
                lsp_max_lsif::lsif_types::UniquenessLevel::Group => "group",
                lsp_max_lsif::lsif_types::UniquenessLevel::Scheme => "scheme",
                lsp_max_lsif::lsif_types::UniquenessLevel::Global => "global",
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
