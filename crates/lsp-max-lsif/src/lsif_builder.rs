use crate::lsif::*;
use lsp_types_max::Position;
use std::io::{self, Write};

pub struct LsifBuilder<W: Write> {
    writer: W,
    next_id: i64,
    open_documents: std::collections::HashSet<i64>,
    open_projects: std::collections::HashSet<i64>,
    has_emitted_metadata: bool,
}

impl<W: Write> LsifBuilder<W> {
    pub fn new(writer: W) -> Self {
        Self {
            writer,
            next_id: 1,
            open_documents: std::collections::HashSet::new(),
            open_projects: std::collections::HashSet::new(),
            has_emitted_metadata: false,
        }
    }

    pub fn next_id(&mut self) -> Id {
        let id = self.next_id;
        self.next_id += 1;
        Id::Number(id as i32)
    }

    pub fn emit(&mut self, element: Element) -> io::Result<()> {
        if !self.has_emitted_metadata {
            if let Element::Vertex(Vertex::MetaData { .. }) = &element {
                self.has_emitted_metadata = true;
            } else {
                panic!("MetaData MUST be the first element emitted in an LSIF graph.");
            }
        }
        let json = serde_json::to_string(&element)?;
        writeln!(self.writer, "{}", json)
    }

    pub fn emit_metadata(
        &mut self,
        version: &str,
        project_root: &str,
        tool: ToolInfo,
    ) -> io::Result<Id> {
        let id = self.next_id();
        self.emit(Element::Vertex(Vertex::MetaData {
            id: id.clone(),
            type_: VertexType::Vertex,
            version: version.to_string(),
            project_root: project_root.to_string(),
            position_encoding: PositionEncoding::Utf16,
            tool_info: Some(tool),
        }))?;
        Ok(id)
    }

    pub fn emit_project(&mut self, kind: Option<&str>, resource: Option<String>) -> io::Result<Id> {
        let project_id = self.next_id();
        self.emit(Element::Vertex(Vertex::Project {
            id: project_id.clone(),
            type_: VertexType::Vertex,
            kind: kind.map(|s| s.to_string()),
            resource,
            contents: None,
        }))?;
        self.begin_project(project_id.clone())?;
        Ok(project_id)
    }

    pub fn emit_document(&mut self, uri: &str, language_id: &str) -> io::Result<Id> {
        let doc_id = self.next_id();
        self.emit(Element::Vertex(Vertex::Document {
            id: doc_id.clone(),
            type_: VertexType::Vertex,
            uri: uri.to_string(),
            language_id: language_id.to_string(),
            contents: None,
        }))?;
        self.begin_document(doc_id.clone())?;
        Ok(doc_id)
    }

    pub fn emit_range(
        &mut self,
        start: Position,
        end: Position,
        tag: Option<RangeTag>,
    ) -> io::Result<Id> {
        let range_id = self.next_id();
        self.emit(Element::Vertex(Vertex::Range {
            id: range_id.clone(),
            type_: VertexType::Vertex,
            start,
            end,
            tag,
        }))?;
        Ok(range_id)
    }

    pub fn emit_result_set(&mut self) -> io::Result<Id> {
        let result_set_id = self.next_id();
        self.emit(Element::Vertex(Vertex::ResultSet {
            id: result_set_id.clone(),
            type_: VertexType::Vertex,
        }))?;
        Ok(result_set_id)
    }

    pub fn contains(&mut self, out_v: Id, in_vs: Vec<Id>) -> io::Result<Id> {
        let edge_id = self.next_id();
        self.emit(Element::Edge(Edge::Contains {
            id: edge_id.clone(),
            type_: EdgeType::Edge,
            out_v,
            in_vs,
        }))?;
        Ok(edge_id)
    }

    pub fn diagnostic_result(&mut self, result: Vec<lsp_types_max::Diagnostic>) -> io::Result<Id> {
        let diag_result_id = self.next_id();
        self.emit(Element::Vertex(Vertex::DiagnosticResult {
            id: diag_result_id.clone(),
            type_: VertexType::Vertex,
            result,
        }))?;
        Ok(diag_result_id)
    }

    pub fn diagnostic_edge(&mut self, out_v: Id, in_v: Id) -> io::Result<Id> {
        let edge_id = self.next_id();
        self.emit(Element::Edge(Edge::TextDocumentDiagnostic {
            id: edge_id.clone(),
            type_: EdgeType::Edge,
            out_v,
            in_v,
        }))?;
        Ok(edge_id)
    }

    pub fn bind_next(&mut self, out_v: Id, in_v: Id) -> io::Result<Id> {
        let edge_id = self.next_id();
        self.emit(Element::Edge(Edge::Next {
            id: edge_id.clone(),
            type_: EdgeType::Edge,
            out_v,
            in_v,
        }))?;
        Ok(edge_id)
    }

    pub fn bind_hover(&mut self, target_id: Id, contents: HoverContents) -> io::Result<Id> {
        let hover_result_id = self.next_id();
        self.emit(Element::Vertex(Vertex::HoverResult {
            id: hover_result_id.clone(),
            type_: VertexType::Vertex,
            result: HoverResultData {
                contents,
                range: None,
            },
        }))?;

        let hover_edge_id = self.next_id();
        self.emit(Element::Edge(Edge::TextDocumentHover {
            id: hover_edge_id,
            type_: EdgeType::Edge,
            out_v: target_id,
            in_v: hover_result_id.clone(),
        }))?;

        Ok(hover_result_id)
    }

    pub fn bind_definition(
        &mut self,
        result_set_id: Id,
        range_ids: Vec<Id>,
        document_id: Id,
    ) -> io::Result<Id> {
        let def_result_id = self.next_id();
        self.emit(Element::Vertex(Vertex::DefinitionResult {
            id: def_result_id.clone(),
            type_: VertexType::Vertex,
        }))?;

        let def_edge_id = self.next_id();
        self.emit(Element::Edge(Edge::TextDocumentDefinition {
            id: def_edge_id,
            type_: EdgeType::Edge,
            out_v: result_set_id,
            in_v: def_result_id.clone(),
        }))?;

        let item_edge_id = self.next_id();
        self.emit(Element::Edge(Edge::Item {
            id: item_edge_id,
            type_: EdgeType::Edge,
            out_v: def_result_id.clone(),
            in_vs: range_ids,
            document: document_id,
            property: Some(ItemEdgeProperty::Definitions),
        }))?;

        Ok(def_result_id)
    }

    pub fn bind_references(
        &mut self,
        result_set_id: Id,
        range_ids: Vec<Id>,
        document_id: Id,
    ) -> io::Result<Id> {
        let ref_result_id = self.next_id();
        self.emit(Element::Vertex(Vertex::ReferenceResult {
            id: ref_result_id.clone(),
            type_: VertexType::Vertex,
        }))?;

        let ref_edge_id = self.next_id();
        self.emit(Element::Edge(Edge::TextDocumentReferences {
            id: ref_edge_id,
            type_: EdgeType::Edge,
            out_v: result_set_id,
            in_v: ref_result_id.clone(),
        }))?;

        let item_edge_id = self.next_id();
        self.emit(Element::Edge(Edge::Item {
            id: item_edge_id,
            type_: EdgeType::Edge,
            out_v: ref_result_id.clone(),
            in_vs: range_ids,
            document: document_id,
            property: Some(ItemEdgeProperty::References),
        }))?;

        Ok(ref_result_id)
    }

    pub fn bind_declaration(
        &mut self,
        result_set_id: Id,
        range_ids: Vec<Id>,
        document_id: Id,
    ) -> io::Result<Id> {
        let decl_result_id = self.next_id();
        self.emit(Element::Vertex(Vertex::DeclarationResult {
            id: decl_result_id.clone(),
            type_: VertexType::Vertex,
        }))?;

        let decl_edge_id = self.next_id();
        self.emit(Element::Edge(Edge::TextDocumentDeclaration {
            id: decl_edge_id,
            type_: EdgeType::Edge,
            out_v: result_set_id,
            in_v: decl_result_id.clone(),
        }))?;

        let item_edge_id = self.next_id();
        self.emit(Element::Edge(Edge::Item {
            id: item_edge_id,
            type_: EdgeType::Edge,
            out_v: decl_result_id.clone(),
            in_vs: range_ids,
            document: document_id,
            property: Some(ItemEdgeProperty::Declarations),
        }))?;

        Ok(decl_result_id)
    }

    pub fn begin_document(&mut self, doc_id: Id) -> io::Result<()> {
        if let Id::Number(n) = &doc_id {
            self.open_documents.insert(*n as i64);
        }
        let event_id = self.next_id();
        self.emit(Element::Vertex(Vertex::Event {
            id: event_id,
            type_: VertexType::Vertex,
            kind: EventKind::Begin,
            scope: EventScope::Document,
            data: doc_id,
        }))
    }

    pub fn end_document(&mut self, doc_id: Id) -> io::Result<()> {
        if let Id::Number(n) = &doc_id {
            self.open_documents.remove(&(*n as i64));
        }
        let event_id = self.next_id();
        self.emit(Element::Vertex(Vertex::Event {
            id: event_id,
            type_: VertexType::Vertex,
            kind: EventKind::End,
            scope: EventScope::Document,
            data: doc_id,
        }))
    }

    pub fn begin_project(&mut self, project_id: Id) -> io::Result<()> {
        if let Id::Number(n) = &project_id {
            self.open_projects.insert(*n as i64);
        }
        let event_id = self.next_id();
        self.emit(Element::Vertex(Vertex::Event {
            id: event_id,
            type_: VertexType::Vertex,
            kind: EventKind::Begin,
            scope: EventScope::Project,
            data: project_id,
        }))
    }

    /// Convenience: emit `metaData` + `project` vertices and return the project id.
    /// The caller is responsible for calling `begin_project` / `end_project` around
    /// the document indexing loop.
    pub fn emit_meta_project(&mut self, root: &str, lang: &str) -> io::Result<Id> {
        self.emit_metadata(
            "0.6.0",
            root,
            ToolInfo {
                name: "lsp-max-lsif".into(),
                version: Some(env!("CARGO_PKG_VERSION").into()),
                args: None,
            },
        )?;
        self.emit_project(Some(lang), Some(root.to_string()))
    }

    pub fn end_project(&mut self, project_id: Id) -> io::Result<()> {
        if let Id::Number(n) = &project_id {
            self.open_projects.remove(&(*n as i64));
        }
        let event_id = self.next_id();
        self.emit(Element::Vertex(Vertex::Event {
            id: event_id,
            type_: VertexType::Vertex,
            kind: EventKind::End,
            scope: EventScope::Project,
            data: project_id,
        }))
    }
}
