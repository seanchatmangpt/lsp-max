//! Extended LSIF 0.6 emission surface: implementation/type-definition/hierarchy
//! result pairs (Group A), data-bearing result+edge pairs (Group B), and direct
//! edges plus metadata vertices (Group C). Split out of `lsif_builder.rs` to keep
//! that file within the per-file LOC bound.

use crate::lsif::*;
use crate::lsif_builder::LsifBuilder;
use lsp_types_max::Position;
use std::io::{self, Write};

impl<W: Write> LsifBuilder<W> {
    // ---- Group A: result-only pairs (mirror bind_definition) ----

    pub fn bind_implementation(
        &mut self,
        result_set_id: Id,
        range_ids: Vec<Id>,
        document_id: Id,
    ) -> io::Result<Id> {
        let impl_result_id = self.next_id();
        self.emit(Element::Vertex(Vertex::ImplementationResult {
            id: impl_result_id.clone(),
            type_: VertexType::Vertex,
        }))?;

        let edge_id = self.next_id();
        self.emit(Element::Edge(Edge::TextDocumentImplementation {
            id: edge_id,
            type_: EdgeType::Edge,
            out_v: result_set_id,
            in_v: impl_result_id.clone(),
        }))?;

        let item_edge_id = self.next_id();
        self.emit(Element::Edge(Edge::Item {
            id: item_edge_id,
            type_: EdgeType::Edge,
            out_v: impl_result_id.clone(),
            in_vs: range_ids,
            document: document_id,
            property: Some(ItemEdgeProperty::ImplementationResults),
        }))?;

        Ok(impl_result_id)
    }

    pub fn bind_type_definition(
        &mut self,
        result_set_id: Id,
        range_ids: Vec<Id>,
        document_id: Id,
    ) -> io::Result<Id> {
        let type_def_result_id = self.next_id();
        self.emit(Element::Vertex(Vertex::TypeDefinitionResult {
            id: type_def_result_id.clone(),
            type_: VertexType::Vertex,
        }))?;

        let edge_id = self.next_id();
        self.emit(Element::Edge(Edge::TextDocumentTypeDefinition {
            id: edge_id,
            type_: EdgeType::Edge,
            out_v: result_set_id,
            in_v: type_def_result_id.clone(),
        }))?;

        let item_edge_id = self.next_id();
        self.emit(Element::Edge(Edge::Item {
            id: item_edge_id,
            type_: EdgeType::Edge,
            out_v: type_def_result_id.clone(),
            in_vs: range_ids,
            document: document_id,
            property: Some(ItemEdgeProperty::TypeDefinitions),
        }))?;

        Ok(type_def_result_id)
    }

    pub fn bind_call_hierarchy(&mut self, result_set_id: Id) -> io::Result<Id> {
        let call_result_id = self.next_id();
        self.emit(Element::Vertex(Vertex::CallHierarchyResult {
            id: call_result_id.clone(),
            type_: VertexType::Vertex,
        }))?;

        let edge_id = self.next_id();
        self.emit(Element::Edge(Edge::TextDocumentCallHierarchy {
            id: edge_id,
            type_: EdgeType::Edge,
            out_v: result_set_id,
            in_v: call_result_id.clone(),
        }))?;

        Ok(call_result_id)
    }

    pub fn bind_type_hierarchy(&mut self, result_set_id: Id) -> io::Result<Id> {
        let type_result_id = self.next_id();
        self.emit(Element::Vertex(Vertex::TypeHierarchyResult {
            id: type_result_id.clone(),
            type_: VertexType::Vertex,
        }))?;

        let edge_id = self.next_id();
        self.emit(Element::Edge(Edge::TextDocumentTypeHierarchy {
            id: edge_id,
            type_: EdgeType::Edge,
            out_v: result_set_id,
            in_v: type_result_id.clone(),
        }))?;

        Ok(type_result_id)
    }

    // ---- Group B: data-bearing result + edge pairs (mirror diagnostic_result) ----

    pub fn folding_range_result(
        &mut self,
        result: Vec<lsp_types_max::FoldingRange>,
    ) -> io::Result<Id> {
        let result_id = self.next_id();
        self.emit(Element::Vertex(Vertex::FoldingRangeResult {
            id: result_id.clone(),
            type_: VertexType::Vertex,
            result,
        }))?;
        Ok(result_id)
    }

    pub fn folding_range_edge(&mut self, out_v: Id, in_v: Id) -> io::Result<Id> {
        let edge_id = self.next_id();
        self.emit(Element::Edge(Edge::TextDocumentFoldingRange {
            id: edge_id.clone(),
            type_: EdgeType::Edge,
            out_v,
            in_v,
        }))?;
        Ok(edge_id)
    }

    pub fn document_link_result(
        &mut self,
        result: Vec<lsp_types_max::DocumentLink>,
    ) -> io::Result<Id> {
        let result_id = self.next_id();
        self.emit(Element::Vertex(Vertex::DocumentLinkResult {
            id: result_id.clone(),
            type_: VertexType::Vertex,
            result,
        }))?;
        Ok(result_id)
    }

    pub fn document_link_edge(&mut self, out_v: Id, in_v: Id) -> io::Result<Id> {
        let edge_id = self.next_id();
        self.emit(Element::Edge(Edge::TextDocumentDocumentLink {
            id: edge_id.clone(),
            type_: EdgeType::Edge,
            out_v,
            in_v,
        }))?;
        Ok(edge_id)
    }

    pub fn document_symbol_result(&mut self, result: DocumentSymbolResultData) -> io::Result<Id> {
        let result_id = self.next_id();
        self.emit(Element::Vertex(Vertex::DocumentSymbolResult {
            id: result_id.clone(),
            type_: VertexType::Vertex,
            result,
        }))?;
        Ok(result_id)
    }

    pub fn document_symbol_edge(&mut self, out_v: Id, in_v: Id) -> io::Result<Id> {
        let edge_id = self.next_id();
        self.emit(Element::Edge(Edge::TextDocumentDocumentSymbol {
            id: edge_id.clone(),
            type_: EdgeType::Edge,
            out_v,
            in_v,
        }))?;
        Ok(edge_id)
    }

    pub fn semantic_tokens_result(&mut self, result: SemanticTokensData) -> io::Result<Id> {
        let result_id = self.next_id();
        self.emit(Element::Vertex(Vertex::SemanticTokensResult {
            id: result_id.clone(),
            type_: VertexType::Vertex,
            result,
        }))?;
        Ok(result_id)
    }

    pub fn semantic_tokens_edge(&mut self, out_v: Id, in_v: Id) -> io::Result<Id> {
        let edge_id = self.next_id();
        self.emit(Element::Edge(Edge::TextDocumentSemanticTokens {
            id: edge_id.clone(),
            type_: EdgeType::Edge,
            out_v,
            in_v,
        }))?;
        Ok(edge_id)
    }

    // ---- Group C: direct edges + metadata vertices ----

    pub fn attach(&mut self, out_v: Id, in_v: Id) -> io::Result<Id> {
        let edge_id = self.next_id();
        self.emit(Element::Edge(Edge::Attach {
            id: edge_id.clone(),
            type_: EdgeType::Edge,
            out_v,
            in_v,
        }))?;
        Ok(edge_id)
    }

    pub fn package_information_edge(&mut self, out_v: Id, in_v: Id) -> io::Result<Id> {
        let edge_id = self.next_id();
        self.emit(Element::Edge(Edge::PackageInformation {
            id: edge_id.clone(),
            type_: EdgeType::Edge,
            out_v,
            in_v,
        }))?;
        Ok(edge_id)
    }

    pub fn emit_source(
        &mut self,
        workspace_root: &str,
        repository: Option<Repository>,
    ) -> io::Result<Id> {
        let source_id = self.next_id();
        self.emit(Element::Vertex(Vertex::Source {
            id: source_id.clone(),
            type_: VertexType::Vertex,
            workspace_root: workspace_root.to_string(),
            repository,
        }))?;
        Ok(source_id)
    }

    pub fn emit_result_range(&mut self, start: Position, end: Position) -> io::Result<Id> {
        let range_id = self.next_id();
        self.emit(Element::Vertex(Vertex::ResultRange {
            id: range_id.clone(),
            type_: VertexType::Vertex,
            start,
            end,
        }))?;
        Ok(range_id)
    }

    pub fn emit_package_information(
        &mut self,
        name: &str,
        manager: &str,
        version: &str,
        repository: Option<Repository>,
    ) -> io::Result<Id> {
        let pkg_id = self.next_id();
        self.emit(Element::Vertex(Vertex::PackageInformation {
            id: pkg_id.clone(),
            type_: VertexType::Vertex,
            name: name.to_string(),
            manager: manager.to_string(),
            version: version.to_string(),
            repository,
        }))?;
        Ok(pkg_id)
    }
}
