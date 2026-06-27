use crate::lsif::*;
use crate::lsif::{Edge, ItemEdgeProperty, Vertex};
use crate::lsif_builder::LsifBuilder;
use crate::lsif_types::{EdgeType, HoverContents, MonikerKind, UniquenessLevel, VertexType};
use lsp_types_max::{MarkupContent, MarkupKind, Position, SymbolKind};
use std::collections::HashMap;
use std::io::Write;

/// Per-document state threaded through `LsifEmit` implementations.
pub struct LsifContext<'b, W: Write> {
    pub builder: &'b mut LsifBuilder<W>,
    /// The LSIF document vertex id for the file currently being indexed.
    pub doc_id: Id,
    /// Crate/package path prefix used when constructing monikers (e.g. `"my_crate"`).
    pub module_path: String,
    /// Optional package name for npm-style monikers.
    pub package_name: Option<String>,
    /// Lexical scope stack mapping a symbol name to the resultSet vertex id created for its definition.
    pub result_sets: Vec<HashMap<String, Id>>,
    /// Maps a resultSet ID to its associated ReferenceResult ID.
    pub reference_results: HashMap<Id, Id>,
}

impl<'b, W: Write> LsifContext<'b, W> {
    pub fn new(
        builder: &'b mut LsifBuilder<W>,
        doc_id: Id,
        module_path: impl Into<String>,
    ) -> Self {
        Self {
            builder,
            doc_id,
            module_path: module_path.into(),
            package_name: None,
            result_sets: vec![HashMap::new()],
            reference_results: HashMap::new(),
        }
    }

    pub fn push_scope(&mut self) {
        self.result_sets.push(HashMap::new());
    }

    pub fn pop_scope(&mut self) {
        self.result_sets.pop();
    }

    pub fn insert_symbol(&mut self, name: String, rs_id: Id) {
        if let Some(map) = self.result_sets.last_mut() {
            map.insert(name, rs_id);
        }
    }

    pub fn lookup_symbol(&self, name: &str) -> Option<Id> {
        for map in self.result_sets.iter().rev() {
            if let Some(id) = map.get(name) {
                return Some(id.clone());
            }
        }
        None
    }

    /// Emit a fresh resultSet vertex and return its id.
    pub fn new_result_set(&mut self) -> std::io::Result<Id> {
        self.builder.emit_result_set()
    }

    /// Emit a range vertex, wire a `contains` edge from the document, and return the range id.
    pub fn link_range(
        &mut self,
        start: Position,
        end: Position,
        tag: Option<RangeTag>,
    ) -> std::io::Result<Id> {
        let range_id = self.builder.emit_range(start, end, tag)?;
        self.builder
            .contains(self.doc_id.clone(), vec![range_id.clone()])?;
        Ok(range_id)
    }

    /// Emit a hover result and wire `textDocument/hover` from `target_id`.
    pub fn emit_hover(
        &mut self,
        target_id: Id,
        markdown: impl Into<String>,
    ) -> std::io::Result<Id> {
        self.builder.bind_hover(
            target_id,
            HoverContents::Markup(MarkupContent {
                kind: MarkupKind::Markdown,
                value: markdown.into(),
            }),
        )
    }

    /// Emit a definitionResult and Item edge linking `result_set_id` → `range_id`.
    pub fn emit_definition(&mut self, result_set_id: Id, range_id: Id) -> std::io::Result<Id> {
        let def_res = self.builder.bind_definition(
            result_set_id.clone(),
            vec![range_id.clone()],
            self.doc_id.clone(),
        )?;

        let ref_res_id = self.builder.next_id();
        self.builder
            .emit(crate::lsif::Element::Vertex(Vertex::ReferenceResult {
                id: ref_res_id.clone(),
                type_: VertexType::Vertex,
            }))?;

        let edge_id = self.builder.next_id();
        self.builder
            .emit(crate::lsif::Element::Edge(Edge::TextDocumentReferences {
                id: edge_id,
                type_: EdgeType::Edge,
                out_v: result_set_id.clone(),
                in_v: ref_res_id.clone(),
            }))?;

        let item_edge_id = self.builder.next_id();
        self.builder.emit(crate::lsif::Element::Edge(Edge::Item {
            id: item_edge_id,
            type_: EdgeType::Edge,
            out_v: ref_res_id.clone(),
            in_vs: vec![range_id],
            document: self.doc_id.clone(),
            property: Some(ItemEdgeProperty::Definitions),
        }))?;

        self.reference_results.insert(result_set_id, ref_res_id);
        Ok(def_res)
    }

    pub fn emit_reference(&mut self, result_set_id: Id, range_id: Id) -> std::io::Result<()> {
        let ref_res_id = if let Some(id) = self.reference_results.get(&result_set_id) {
            id.clone()
        } else {
            let id = self.builder.next_id();
            self.builder
                .emit(crate::lsif::Element::Vertex(Vertex::ReferenceResult {
                    id: id.clone(),
                    type_: VertexType::Vertex,
                }))?;

            let edge_id = self.builder.next_id();
            self.builder
                .emit(crate::lsif::Element::Edge(Edge::TextDocumentReferences {
                    id: edge_id,
                    type_: EdgeType::Edge,
                    out_v: result_set_id.clone(),
                    in_v: id.clone(),
                }))?;

            self.reference_results
                .insert(result_set_id.clone(), id.clone());
            id
        };

        let item_edge_id = self.builder.next_id();
        self.builder.emit(crate::lsif::Element::Edge(Edge::Item {
            id: item_edge_id,
            type_: EdgeType::Edge,
            out_v: ref_res_id,
            in_vs: vec![range_id],
            document: self.doc_id.clone(),
            property: Some(ItemEdgeProperty::References),
        }))?;

        Ok(())
    }

    /// Emit a moniker vertex and wire it to `result_set_id`.
    pub fn emit_moniker(
        &mut self,
        result_set_id: Id,
        scheme: impl Into<String>,
        identifier: impl Into<String>,
        kind: MonikerKind,
        unique: UniquenessLevel,
    ) -> std::io::Result<Id> {
        let moniker_id = self.builder.next_id();
        self.builder
            .emit(crate::lsif::Element::Vertex(Vertex::Moniker {
                id: moniker_id.clone(),
                type_: VertexType::Vertex,
                scheme: scheme.into(),
                identifier: identifier.into(),
                kind,
                unique,
            }))?;
        let edge_id = self.builder.next_id();
        self.builder
            .emit(crate::lsif::Element::Edge(Edge::Moniker {
                id: edge_id,
                type_: EdgeType::Edge,
                out_v: result_set_id,
                in_v: moniker_id.clone(),
            }))?;
        Ok(moniker_id)
    }
}

/// Implemented by auto-lsp generated AST node types to contribute LSIF vertices and edges.
///
/// The return value is the resultSet id if one was created (definitions), or `None`
/// (references, leaf nodes that do not introduce a new symbol identity).
pub trait LsifEmit {
    fn emit<W: Write>(&self, ctx: &mut LsifContext<'_, W>) -> std::io::Result<Option<Id>>;
}

/// Helper: build a `Definition` range tag.
pub fn definition_tag(
    text: impl Into<String>,
    kind: SymbolKind,
    full_range: lsp_types_max::Range,
    detail: Option<String>,
) -> RangeTag {
    RangeTag::Definition {
        text: text.into(),
        kind,
        full_range,
        detail,
    }
}

/// Helper: build a `Reference` range tag.
pub fn reference_tag(
    text: impl Into<String>,
    kind: SymbolKind,
    full_range: lsp_types_max::Range,
) -> RangeTag {
    RangeTag::Reference {
        text: text.into(),
        kind,
        full_range,
    }
}

/// Helper: build an `Unknown` range tag.
pub fn unknown_tag(
    text: impl Into<String>,
    kind: SymbolKind,
    full_range: lsp_types_max::Range,
) -> RangeTag {
    RangeTag::Unknown {
        text: text.into(),
        kind,
        full_range,
    }
}
