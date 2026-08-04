pub mod core;
pub mod db;
pub mod semantic;

#[cfg(feature = "codegen")]
pub mod codegen;

use crate::core::document::Document;
use crate::db::SalsaLspAdapter;
use lsp_types_max::{
    Diagnostic, DidChangeTextDocumentParams, DidCloseTextDocumentParams, DidOpenTextDocumentParams,
    DocumentSymbol, DocumentUri, FoldingRange, FoldingRangeKind, Hover, HoverContents,
    MarkedString, Position, SelectionRange, SymbolKind,
};
use parking_lot::Mutex;
use semantic::{SemanticIndex, SemanticQueryPack, SemanticRename};

/// Admission failures produced before a document reaches the incremental AST
/// database.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum AutoLspError {
    /// A server attempted to reuse one adapter with a second grammar.
    #[error("LSPMAX_GRAMMAR_SESSION_MISMATCH: one AutoLspAdapter cannot host multiple grammars")]
    GrammarSessionMismatch,
    /// A change arrived before the corresponding document-open admission.
    #[error("LSPMAX_DOCUMENT_NOT_OPEN: didChange requires an admitted didOpen session")]
    DocumentNotOpen,
    /// An edit was constructed against a document revision that is no longer current.
    #[error("LSPMAX_STALE_DOCUMENT_VERSION: expected version {expected}, actual version {actual}")]
    StaleDocumentVersion { expected: i32, actual: i32 },
}

/// The `AutoLspAdapter` acts as the formal bridge between the `lsp-max`
/// execution engine and the incremental AST generation from `lsp-max-ast-core`.
///
/// It strictly adheres to the architectural mandate by cleanly separating the
/// transport/JSON-RPC layer (`lsp-max`) from the formal grammar
/// parsing layer (`lsp-max-ast-core`).
pub struct AutoLspAdapter {
    /// Lazily initialized because the historical public API supplies the
    /// grammar on `didOpen` rather than at construction time.
    inner: Mutex<Option<SalsaLspAdapter>>,
}

impl Default for AutoLspAdapter {
    fn default() -> Self {
        Self::new_default()
    }
}

impl AutoLspAdapter {
    /// Creates a new `AutoLspAdapter`.
    pub fn new_default() -> Self {
        Self {
            inner: Mutex::new(None),
        }
    }

    /// Handles a document open event, injecting the initial state into the incremental database.
    pub fn handle_did_open(
        &self,
        params: DidOpenTextDocumentParams,
        language: tree_sitter::Language,
    ) {
        let _ = self.try_handle_did_open(params, language);
    }

    /// Admits a document into the incremental database, returning a typed
    /// refusal when the adapter is reused with a different grammar.
    pub fn try_handle_did_open(
        &self,
        params: DidOpenTextDocumentParams,
        language: tree_sitter::Language,
    ) -> Result<(), AutoLspError> {
        let mut inner = self.inner.lock();
        let adapter = inner.get_or_insert_with(|| SalsaLspAdapter::new(language.clone()));
        if !adapter.accepts_language(&language) {
            return Err(AutoLspError::GrammarSessionMismatch);
        }
        adapter.handle_did_open(params);
        Ok(())
    }

    /// Handles a document change event, applying incremental diffs to the AST database.
    pub fn handle_did_change(
        &self,
        params: DidChangeTextDocumentParams,
        language: tree_sitter::Language,
    ) {
        let _ = self.try_handle_did_change(params, language);
    }

    /// Applies an incremental edit through Salsa, refusing changes that have
    /// no admitted document session or use the wrong grammar.
    pub fn try_handle_did_change(
        &self,
        params: DidChangeTextDocumentParams,
        language: tree_sitter::Language,
    ) -> Result<(), AutoLspError> {
        let inner = self.inner.lock();
        let Some(adapter) = inner.as_ref() else {
            return Err(AutoLspError::DocumentNotOpen);
        };
        if !adapter.accepts_language(&language) {
            return Err(AutoLspError::GrammarSessionMismatch);
        }
        if !adapter.contains_uri(&params.text_document.uri) {
            return Err(AutoLspError::DocumentNotOpen);
        }
        adapter.handle_did_change(params);
        Ok(())
    }

    /// Handles a document close event, cleaning up memory.
    pub fn handle_did_close(&self, params: DidCloseTextDocumentParams) {
        if let Some(adapter) = self.inner.lock().as_ref() {
            adapter.handle_did_close(params);
        }
    }

    /// Analyzes the document and returns a set of diagnostics derived from the AST.
    pub fn pull_diagnostics(&self, uri: &DocumentUri) -> Vec<Diagnostic> {
        self.inner
            .lock()
            .as_ref()
            .map_or_else(Vec::new, |adapter| adapter.pull_diagnostics(uri))
    }

    /// Provides read-access to a managed document for semantic token and symbol generation.
    pub fn get_document<F, R>(&self, uri: &DocumentUri, f: F) -> Option<R>
    where
        F: FnOnce(&Document) -> R,
    {
        self.inner
            .lock()
            .as_ref()
            .and_then(|adapter| adapter.get_document(uri, f))
    }

    /// Derives hierarchical symbols from named Tree-sitter declarations.
    ///
    /// The grammar-neutral rule is deliberately narrow: a named node is a
    /// symbol only when the grammar exposes a `name` field. This avoids
    /// presenting every syntax node as a user-facing declaration while
    /// allowing generated adapters to work without handwritten traversal.
    pub fn document_symbols(&self, uri: &DocumentUri) -> Vec<DocumentSymbol> {
        self.get_document(uri, |document| {
            collect_document_symbols(document, document.tree.root_node())
        })
        .unwrap_or_default()
    }

    /// Returns structural hover evidence for the smallest named node at a
    /// document position.
    pub fn hover(&self, uri: &DocumentUri, position: Position) -> Option<Hover> {
        self.get_document(uri, |document| {
            let position = document.normalize_position(&position).ok()?;
            let point =
                tree_sitter::Point::new(position.line as usize, position.character as usize);
            let node = smallest_named_node(document.tree.root_node(), point)?;
            let range = document.denormalize_range(&node.range()).ok()?;
            Some(Hover {
                contents: HoverContents::Scalar(MarkedString::String(format!(
                    "Tree-sitter node: `{}`",
                    node.kind()
                ))),
                range: Some(range),
            })
        })?
    }

    /// Derives folding ranges for multi-line named nodes.
    pub fn folding_ranges(&self, uri: &DocumentUri) -> Vec<FoldingRange> {
        self.get_document(uri, |document| {
            let mut ranges = Vec::new();
            collect_folding_ranges(document.tree.root_node(), &mut ranges);
            ranges
        })
        .unwrap_or_default()
    }

    /// Builds the LSP selection-range parent chain from the smallest named
    /// syntax node through each named ancestor.
    pub fn selection_range(&self, uri: &DocumentUri, position: Position) -> Option<SelectionRange> {
        self.get_document(uri, |document| {
            let normalized = document.normalize_position(&position).ok()?;
            let point =
                tree_sitter::Point::new(normalized.line as usize, normalized.character as usize);
            let mut node = smallest_named_node(document.tree.root_node(), point)?;
            let mut ranges = Vec::new();
            loop {
                ranges.push(document.denormalize_range(&node.range()).ok()?);
                node = match node.parent().and_then(|parent| {
                    if parent.is_named() {
                        Some(parent)
                    } else {
                        parent.parent()
                    }
                }) {
                    Some(parent) => parent,
                    None => break,
                };
            }
            ranges.into_iter().rev().fold(None, |parent, range| {
                Some(SelectionRange {
                    range,
                    parent: parent.map(Box::new),
                })
            })
        })?
    }

    /// Compiles an admitted query pack into a semantic snapshot for one open
    /// document. The snapshot is rebuilt from the incrementally maintained
    /// tree, so it never outlives its source revision.
    pub fn semantic_index(
        &self,
        uri: &DocumentUri,
        pack: &SemanticQueryPack,
    ) -> Option<SemanticIndex> {
        let inner = self.inner.lock();
        let adapter = inner.as_ref()?;
        if !pack.accepts_language(&adapter.language()) {
            return None;
        }
        adapter.get_document(uri, |document| pack.index(uri, document))
    }

    /// Constructs a rename only when the caller's document version matches
    /// the admitted incremental source.
    pub fn semantic_rename(
        &self,
        uri: &DocumentUri,
        position: Position,
        new_name: &str,
        expected_version: i32,
        pack: &SemanticQueryPack,
    ) -> Result<SemanticRename, AutoLspError> {
        let inner = self.inner.lock();
        let adapter = inner.as_ref().ok_or(AutoLspError::DocumentNotOpen)?;
        if !pack.accepts_language(&adapter.language()) {
            return Err(AutoLspError::GrammarSessionMismatch);
        }
        let actual_version = adapter
            .document_version(uri)
            .ok_or(AutoLspError::DocumentNotOpen)?;
        if actual_version != expected_version {
            return Err(AutoLspError::StaleDocumentVersion {
                expected: expected_version,
                actual: actual_version,
            });
        }
        let edits = adapter
            .get_document(uri, |document| {
                pack.index(uri, document).rename_edits(position, new_name)
            })
            .ok_or(AutoLspError::DocumentNotOpen)?;
        Ok(SemanticRename {
            uri: uri.clone(),
            version: actual_version,
            edits,
        })
    }
}

fn collect_document_symbols(
    document: &Document,
    parent: tree_sitter::Node<'_>,
) -> Vec<DocumentSymbol> {
    let mut symbols = Vec::new();
    let mut cursor = parent.walk();
    for node in parent.named_children(&mut cursor) {
        if let Some(name_node) = node.child_by_field_name("name") {
            if let (Ok(range), Ok(selection_range), Ok(name)) = (
                document.denormalize_range(&node.range()),
                document.denormalize_range(&name_node.range()),
                name_node.utf8_text(document.as_bytes()),
            ) {
                let children = collect_document_symbols(document, node);
                #[allow(deprecated)]
                symbols.push(DocumentSymbol {
                    name: name.to_string(),
                    detail: Some(node.kind().to_string()),
                    kind: symbol_kind(node.kind()),
                    tags: None,
                    deprecated: None,
                    range,
                    selection_range,
                    children: (!children.is_empty()).then_some(children),
                });
                continue;
            }
        }
        symbols.extend(collect_document_symbols(document, node));
    }
    symbols
}

fn symbol_kind(kind: &str) -> SymbolKind {
    if kind.contains("class") || kind.contains("struct") || kind.contains("interface") {
        SymbolKind::CLASS
    } else if kind.contains("function") || kind.contains("method") {
        SymbolKind::FUNCTION
    } else if kind.contains("module") || kind.contains("namespace") {
        SymbolKind::MODULE
    } else if kind.contains("enum") {
        SymbolKind::ENUM
    } else if kind.contains("constant") {
        SymbolKind::CONSTANT
    } else if kind.contains("variable") || kind.contains("declarator") {
        SymbolKind::VARIABLE
    } else {
        SymbolKind::OBJECT
    }
}

fn collect_folding_ranges(node: tree_sitter::Node<'_>, ranges: &mut Vec<FoldingRange>) {
    let range = node.range();
    if node.is_named() && range.start_point.row < range.end_point.row {
        ranges.push(FoldingRange {
            start_line: range.start_point.row as u32,
            start_character: None,
            end_line: range.end_point.row as u32,
            end_character: None,
            kind: Some(FoldingRangeKind::Region),
            collapsed_text: None,
        });
    }
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        collect_folding_ranges(child, ranges);
    }
}

fn smallest_named_node(
    root: tree_sitter::Node<'_>,
    point: tree_sitter::Point,
) -> Option<tree_sitter::Node<'_>> {
    let node = root.named_descendant_for_point_range(point, point)?;
    if node.is_named() {
        Some(node)
    } else {
        node.parent()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lsp_types_max::{
        TextDocumentContentChangeEvent, TextDocumentItem, VersionedTextDocumentIdentifier,
    };

    #[test]
    fn test_adapter_initialization() {
        let adapter = AutoLspAdapter::new_default();
        assert!(adapter.inner.lock().is_none());
    }

    #[test]
    fn public_adapter_routes_through_incremental_database() {
        let adapter = AutoLspAdapter::new_default();
        let language: tree_sitter::Language = tree_sitter_html::LANGUAGE.into();
        let uri: DocumentUri = "file:///facade.html".parse().unwrap();

        adapter
            .try_handle_did_open(
                DidOpenTextDocumentParams {
                    text_document: TextDocumentItem {
                        uri: uri.clone(),
                        language_id: "html".to_string(),
                        version: 1,
                        text: "<p>valid</p>".to_string(),
                    },
                },
                language.clone(),
            )
            .unwrap();
        assert!(adapter.pull_diagnostics(&uri).is_empty());

        adapter
            .try_handle_did_change(
                DidChangeTextDocumentParams {
                    text_document: VersionedTextDocumentIdentifier {
                        uri: uri.clone(),
                        version: 2,
                    },
                    content_changes: vec![TextDocumentContentChangeEvent {
                        range: None,
                        range_length: None,
                        text: "<<<INVALID>>>".to_string(),
                    }],
                },
                language,
            )
            .unwrap();
        assert!(!adapter.pull_diagnostics(&uri).is_empty());
    }

    #[test]
    fn change_before_open_is_typed_refusal() {
        let adapter = AutoLspAdapter::new_default();
        let uri: DocumentUri = "file:///missing.html".parse().unwrap();
        let result = adapter.try_handle_did_change(
            DidChangeTextDocumentParams {
                text_document: VersionedTextDocumentIdentifier { uri, version: 1 },
                content_changes: Vec::new(),
            },
            tree_sitter_html::LANGUAGE.into(),
        );
        assert_eq!(result, Err(AutoLspError::DocumentNotOpen));
    }

    #[test]
    fn second_grammar_is_typed_refusal() {
        let adapter = AutoLspAdapter::new_default();
        let first_uri: DocumentUri = "file:///first.html".parse().unwrap();
        adapter
            .try_handle_did_open(
                DidOpenTextDocumentParams {
                    text_document: TextDocumentItem {
                        uri: first_uri,
                        language_id: "html".to_string(),
                        version: 1,
                        text: "<p>valid</p>".to_string(),
                    },
                },
                tree_sitter_html::LANGUAGE.into(),
            )
            .unwrap();

        let second_uri: DocumentUri = "file:///second.py".parse().unwrap();
        let result = adapter.try_handle_did_open(
            DidOpenTextDocumentParams {
                text_document: TextDocumentItem {
                    uri: second_uri,
                    language_id: "python".to_string(),
                    version: 1,
                    text: "pass".to_string(),
                },
            },
            tree_sitter_python::LANGUAGE.into(),
        );
        assert_eq!(result, Err(AutoLspError::GrammarSessionMismatch));
    }

    #[test]
    fn structural_index_manufactures_symbols_hover_folds_and_selection() {
        let adapter = AutoLspAdapter::new_default();
        let uri: DocumentUri = "file:///structural.py".parse().unwrap();
        let source = "class Factory:\n    def manufacture(self):\n        return 1\n";
        adapter
            .try_handle_did_open(
                DidOpenTextDocumentParams {
                    text_document: TextDocumentItem {
                        uri: uri.clone(),
                        language_id: "python".to_string(),
                        version: 1,
                        text: source.to_string(),
                    },
                },
                tree_sitter_python::LANGUAGE.into(),
            )
            .unwrap();

        let symbols = adapter.document_symbols(&uri);
        assert_eq!(symbols.len(), 1);
        assert_eq!(symbols[0].name, "Factory");
        assert_eq!(symbols[0].kind, SymbolKind::CLASS);
        let children = symbols[0].children.as_ref().unwrap();
        assert_eq!(children.len(), 1);
        assert_eq!(children[0].name, "manufacture");
        assert_eq!(children[0].kind, SymbolKind::FUNCTION);

        let hover = adapter.hover(&uri, Position::new(1, 9)).unwrap();
        assert!(format!("{:?}", hover.contents).contains("identifier"));

        let folds = adapter.folding_ranges(&uri);
        assert!(folds
            .iter()
            .any(|range| range.start_line == 0 && range.end_line >= 2));

        let selection = adapter.selection_range(&uri, Position::new(1, 9)).unwrap();
        assert_eq!(selection.range.start.line, 1);
        assert!(selection.parent.is_some());
    }

    #[test]
    fn python_query_pack_manufactures_definition_references_and_rename() {
        let adapter = AutoLspAdapter::new_default();
        let language: tree_sitter::Language = tree_sitter_python::LANGUAGE.into();
        let pack = SemanticQueryPack::admit(
            &language,
            "(function_definition name: (identifier) @definition.function)\n\
             (call function: (identifier) @reference.function)",
        )
        .unwrap();
        let uri: DocumentUri = "file:///semantic.py".parse().unwrap();
        adapter
            .try_handle_did_open(
                DidOpenTextDocumentParams {
                    text_document: TextDocumentItem {
                        uri: uri.clone(),
                        language_id: "python".to_string(),
                        version: 7,
                        text: "def build():\n    pass\n\nbuild()\nbuild()\n".to_string(),
                    },
                },
                language,
            )
            .unwrap();

        let index = adapter.semantic_index(&uri, &pack).unwrap();
        let definition = index.definition(Position::new(3, 1)).unwrap();
        assert_eq!(definition.range.start, Position::new(0, 4));
        assert_eq!(index.references(Position::new(0, 5), false).len(), 2);

        let rename = adapter
            .semantic_rename(&uri, Position::new(3, 1), "manufacture", 7, &pack)
            .unwrap();
        assert_eq!(rename.version, 7);
        assert_eq!(rename.edits.len(), 3);
        assert!(rename
            .edits
            .iter()
            .all(|edit| edit.new_text == "manufacture"));
    }

    #[test]
    fn javascript_pack_proves_grammar_neutral_semantic_projection() {
        let adapter = AutoLspAdapter::new_default();
        let language: tree_sitter::Language = tree_sitter_javascript::LANGUAGE.into();
        let pack = SemanticQueryPack::admit(
            &language,
            "(function_declaration name: (identifier) @definition.function)\n\
             (call_expression function: (identifier) @reference.function)",
        )
        .unwrap();
        let uri: DocumentUri = "file:///semantic.js".parse().unwrap();
        adapter
            .try_handle_did_open(
                DidOpenTextDocumentParams {
                    text_document: TextDocumentItem {
                        uri: uri.clone(),
                        language_id: "javascript".to_string(),
                        version: 1,
                        text: "function build() {}\nbuild();\n".to_string(),
                    },
                },
                language,
            )
            .unwrap();
        let index = adapter.semantic_index(&uri, &pack).unwrap();
        assert_eq!(index.references(Position::new(0, 10), false).len(), 1);
        assert_eq!(
            index.definition(Position::new(1, 1)).unwrap().range.start,
            Position::new(0, 9)
        );
    }

    #[test]
    fn query_pack_and_stale_rename_are_typed_refusals() {
        let language: tree_sitter::Language = tree_sitter_python::LANGUAGE.into();
        let orphan = SemanticQueryPack::admit(
            &language,
            "(call function: (identifier) @reference.function)",
        );
        assert!(matches!(
            orphan,
            Err(semantic::QueryPackError::MissingDefinitions)
        ));

        let pack = SemanticQueryPack::admit(
            &language,
            "(function_definition name: (identifier) @definition.function)",
        )
        .unwrap();
        let adapter = AutoLspAdapter::new_default();
        let uri: DocumentUri = "file:///stale.py".parse().unwrap();
        adapter
            .try_handle_did_open(
                DidOpenTextDocumentParams {
                    text_document: TextDocumentItem {
                        uri: uri.clone(),
                        language_id: "python".to_string(),
                        version: 4,
                        text: "def build():\n    pass\n".to_string(),
                    },
                },
                language,
            )
            .unwrap();
        assert_eq!(
            adapter.semantic_rename(&uri, Position::new(0, 5), "x", 3, &pack),
            Err(AutoLspError::StaleDocumentVersion {
                expected: 3,
                actual: 4
            })
        );
    }
}
