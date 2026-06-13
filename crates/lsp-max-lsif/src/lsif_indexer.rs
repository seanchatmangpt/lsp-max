use crate::lsif::*;
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
    /// Maps a symbol name to the resultSet vertex id created for its definition.
    /// Reference sites (`CallExpression`, `Identifier` usages) look up this map
    /// to wire the `next` edge without a second pass.
    pub result_sets: HashMap<String, Id>,
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
            result_sets: HashMap::new(),
        }
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
        self.builder
            .bind_definition(result_set_id, vec![range_id], self.doc_id.clone())
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
pub fn reference_tag(text: impl Into<String>) -> RangeTag {
    RangeTag::Reference { text: text.into() }
}
