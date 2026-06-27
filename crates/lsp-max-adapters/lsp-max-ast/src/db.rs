//! Salsa 0.26 incremental computation database for `lsp-max-ast`.
//!
//! This module wires the Salsa incremental engine so that
//! `ast_diagnostics` is only re-executed when the underlying
//! `SourceFile` text actually changes.

use salsa::Setter;

use dashmap::DashMap;
use lsp_types_max::{
    Diagnostic, DiagnosticSeverity, DidChangeTextDocumentParams, DidCloseTextDocumentParams,
    DidOpenTextDocumentParams, DocumentUri, NumberOrString, PositionEncodingKind,
};
use parking_lot::Mutex;
use tree_sitter::Parser;

use crate::core::document::Document;

// ---------------------------------------------------------------------------
// Input
// ---------------------------------------------------------------------------

/// A source file tracked by the Salsa incremental engine.
///
/// Each field change bumps the revision, causing dependent tracked
/// functions to be re-evaluated on next access.
#[salsa::input]
pub struct SourceFile {
    pub uri: String,
    pub text: String,
    pub encoding: PositionEncodingKind,
}

// ---------------------------------------------------------------------------
// Trait database
// ---------------------------------------------------------------------------

/// Trait implemented by all Salsa databases that host the AST layer.
///
/// `language()` returns the tree-sitter grammar to be used for parsing.
/// It is an untracked field on the database; callers must not call this
/// inside a tracked query if they need deterministic re-execution.
#[salsa::db]
pub trait LspDb: salsa::Database {
    fn language(&self) -> tree_sitter::Language;
}

// ---------------------------------------------------------------------------
// Serialisable diagnostic used inside Salsa tracked queries
// ---------------------------------------------------------------------------

/// A lightweight diagnostic record that Salsa can compare and memoize.
///
/// `Vec<SalsaDiag>` is the return type of `ast_diagnostics`, which is the
/// tracked function. This is a separate type from `lsp_types_max::Diagnostic`
/// because `Diagnostic` does not implement `salsa::Update`.
#[derive(Debug, Clone, PartialEq, salsa::Update)]
pub struct SalsaDiag {
    pub start_line: u32,
    pub start_char: u32,
    pub end_line: u32,
    pub end_char: u32,
    pub message: String,
}

impl From<&SalsaDiag> for Diagnostic {
    fn from(d: &SalsaDiag) -> Diagnostic {
        Diagnostic {
            range: lsp_types_max::Range {
                start: lsp_types_max::Position {
                    line: d.start_line,
                    character: d.start_char,
                },
                end: lsp_types_max::Position {
                    line: d.end_line,
                    character: d.end_char,
                },
            },
            severity: Some(DiagnosticSeverity::ERROR),
            code: Some(NumberOrString::String("AST_ERROR".to_string())),
            source: Some("lsp-max-ast".to_string()),
            message: d.message.clone(),
            ..Default::default()
        }
    }
}

// ---------------------------------------------------------------------------
// Plain (non-tracked) parse helper
// ---------------------------------------------------------------------------

/// Parse a source text and return a `Document`.  Not tracked by Salsa directly;
/// called from within tracked functions.
fn parse_doc_for_language(
    language: &tree_sitter::Language,
    text: &str,
    encoding: &PositionEncodingKind,
) -> Option<Document> {
    let mut parser = Parser::new();
    parser.set_language(language).ok()?;
    let tree = parser.parse(text.as_bytes(), None)?;
    let enc_ref: Option<&PositionEncodingKind> = match encoding.as_str() {
        "utf-8" => Some(&PositionEncodingKind::UTF8),
        "utf-32" => Some(&PositionEncodingKind::UTF32),
        _ => None,
    };
    Some(Document::new(text.to_owned(), tree, enc_ref))
}

// ---------------------------------------------------------------------------
// Tracked queries
// ---------------------------------------------------------------------------

/// Parse the document text for a `SourceFile` and collect all AST errors as
/// `SalsaDiag` records.
///
/// This is the only `#[salsa::tracked]` query in this crate.  It is memoised:
/// Salsa only re-executes it when `source_file.text` or `source_file.encoding`
/// changes.  `Vec<SalsaDiag>` satisfies `salsa::Update` via the blanket `Vec`
/// impl (which requires `T: Update`; `SalsaDiag` derives `Update`).
#[salsa::tracked]
pub fn ast_diagnostics(db: &dyn LspDb, source_file: SourceFile) -> Vec<SalsaDiag> {
    let text = source_file.text(db);
    let encoding = source_file.encoding(db);
    let language = db.language();

    let Some(doc) = parse_doc_for_language(&language, &text, &encoding) else {
        return Vec::new();
    };

    let mut diags = Vec::new();
    let mut queue = vec![doc.tree.root_node()];
    while let Some(node) = queue.pop() {
        if node.is_error() || node.is_missing() {
            let range = doc.denormalize_range(&node.range()).unwrap_or_default();
            diags.push(SalsaDiag {
                start_line: range.start.line,
                start_char: range.start.character,
                end_line: range.end.line,
                end_char: range.end.character,
                message: "Syntax error detected by formal parser.".to_string(),
            });
        }
        for i in 0..node.child_count() as u32 {
            if let Some(child) = node.child(i) {
                queue.push(child);
            }
        }
    }
    diags
}

/// Parse a `SourceFile` and return the document.  Not tracked by Salsa;
/// callers that need memoized access should use `ast_diagnostics`.
pub fn parse_document(db: &dyn LspDb, source_file: SourceFile) -> Option<Document> {
    let text = source_file.text(db);
    let encoding = source_file.encoding(db);
    parse_doc_for_language(&db.language(), &text, &encoding)
}

// ---------------------------------------------------------------------------
// Concrete database
// ---------------------------------------------------------------------------

/// Concrete Salsa database for the `lsp-max-ast` crate.
///
/// The `language` field is stored as plain data (untracked in Salsa terms)
/// because the grammar is fixed for the lifetime of a server session.
#[salsa::db]
#[derive(Clone)]
pub struct LspMaxDb {
    storage: salsa::Storage<Self>,
    language: tree_sitter::Language,
}

impl LspMaxDb {
    pub fn new(language: tree_sitter::Language) -> Self {
        Self {
            storage: salsa::Storage::default(),
            language,
        }
    }
}

#[salsa::db]
impl salsa::Database for LspMaxDb {}

#[salsa::db]
impl LspDb for LspMaxDb {
    fn language(&self) -> tree_sitter::Language {
        self.language.clone()
    }
}

// ---------------------------------------------------------------------------
// SalsaLspAdapter
// ---------------------------------------------------------------------------

/// An LSP adapter backed by the Salsa incremental engine.
///
/// Wraps a `LspMaxDb` and a `DashMap` that maps each open URI to its
/// `SourceFile` input handle.  On `handle_did_change`, only the `text`
/// field is updated; Salsa automatically invalidates the memoised
/// `ast_diagnostics` result.
pub struct SalsaLspAdapter {
    db: Mutex<LspMaxDb>,
    inputs: DashMap<DocumentUri, SourceFile>,
}

impl SalsaLspAdapter {
    pub fn new(language: tree_sitter::Language) -> Self {
        Self {
            db: Mutex::new(LspMaxDb::new(language)),
            inputs: DashMap::new(),
        }
    }

    /// Register a newly-opened document in the incremental database.
    pub fn handle_did_open(&self, params: DidOpenTextDocumentParams) {
        let uri = params.text_document.uri;
        let text = params.text_document.text;
        let db = self.db.lock();
        let encoding = PositionEncodingKind::UTF16; // LSP default
        let source = SourceFile::new(&*db, uri.to_string(), text, encoding);
        drop(db);
        self.inputs.insert(uri, source);
    }

    /// Apply incremental edits and bump the `text` input in Salsa.
    ///
    /// Applies the LSP content changes via a `Document` update (which
    /// handles tree-sitter incremental edit bookkeeping), then calls
    /// `set_text` to inform Salsa that the text has changed.
    pub fn handle_did_change(&self, params: DidChangeTextDocumentParams) {
        let uri = params.text_document.uri;

        let source = match self.inputs.get(&uri) {
            Some(r) => *r,
            None => return,
        };

        let mut db = self.db.lock();
        let current_text: String = source.text(&*db).clone();
        let encoding: PositionEncodingKind = source.encoding(&*db);
        let language = db.language();

        let enc_ref: Option<&PositionEncodingKind> = match encoding.as_str() {
            "utf-8" => Some(&PositionEncodingKind::UTF8),
            "utf-32" => Some(&PositionEncodingKind::UTF32),
            _ => None,
        };

        let mut parser = Parser::new();
        if parser.set_language(&language).is_err() {
            return;
        }

        if let Some(tree) = parser.parse(current_text.as_bytes(), None) {
            let mut doc = Document::new(current_text, tree, enc_ref);
            let _ = doc.update(&mut parser, &params.content_changes);
            source.set_text(&mut *db).to(doc.as_str().to_string());
        }
    }

    /// Remove a closed document from the incremental database.
    pub fn handle_did_close(&self, params: DidCloseTextDocumentParams) {
        self.inputs.remove(&params.text_document.uri);
    }

    /// Pull diagnostics for a URI as `lsp_types_max::Diagnostic`, using
    /// the Salsa cache where possible.
    pub fn pull_diagnostics(&self, uri: &DocumentUri) -> Vec<Diagnostic> {
        let source = match self.inputs.get(uri) {
            Some(r) => *r,
            None => return Vec::new(),
        };
        let db = self.db.lock();
        ast_diagnostics(&*db, source)
            .iter()
            .map(Diagnostic::from)
            .collect()
    }

    /// Read-only access to the parsed `Document` for a URI.
    pub fn get_document<F, R>(&self, uri: &DocumentUri, f: F) -> Option<R>
    where
        F: FnOnce(&Document) -> R,
    {
        let source = match self.inputs.get(uri) {
            Some(r) => *r,
            None => return None,
        };
        let db = self.db.lock();
        parse_document(&*db, source).map(|doc| f(&doc))
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use lsp_types_max::{TextDocumentContentChangeEvent, VersionedTextDocumentIdentifier};

    fn html_language() -> tree_sitter::Language {
        tree_sitter_html::LANGUAGE.into()
    }

    // ------------------------------------------------------------------
    // TRUE: did_open_sets_salsa_source_file
    // ------------------------------------------------------------------
    #[test]
    fn did_open_sets_salsa_source_file() {
        let adapter = SalsaLspAdapter::new(html_language());
        let uri: DocumentUri = "file:///test.html".parse().unwrap();

        adapter.handle_did_open(DidOpenTextDocumentParams {
            text_document: lsp_types_max::TextDocumentItem {
                uri: uri.clone(),
                language_id: "html".to_string(),
                version: 1,
                text: "<p>hello</p>".to_string(),
            },
        });

        // After open, the URI must be tracked.
        assert!(
            adapter.inputs.contains_key(&uri),
            "SourceFile not registered after did_open"
        );

        // Text is readable from the db.
        let source = *adapter.inputs.get(&uri).unwrap();
        let db = adapter.db.lock();
        assert_eq!(source.text(&*db), "<p>hello</p>");
    }

    // ------------------------------------------------------------------
    // TRUE: diagnostics_cached_when_file_unchanged
    //       Call pull_diagnostics twice; expect same result.
    // ------------------------------------------------------------------
    #[test]
    fn diagnostics_cached_when_file_unchanged() {
        let adapter = SalsaLspAdapter::new(html_language());
        let uri: DocumentUri = "file:///stable.html".parse().unwrap();

        adapter.handle_did_open(DidOpenTextDocumentParams {
            text_document: lsp_types_max::TextDocumentItem {
                uri: uri.clone(),
                language_id: "html".to_string(),
                version: 1,
                text: "<p>ok</p>".to_string(),
            },
        });

        let d1 = adapter.pull_diagnostics(&uri);
        let d2 = adapter.pull_diagnostics(&uri);
        assert_eq!(d1, d2, "cached result must equal first result");
        assert!(d1.is_empty(), "valid HTML should have no diagnostics");
    }

    // ------------------------------------------------------------------
    // TRUE: diagnostics_recomputed_when_file_changed
    //       Introduce a syntax error; verify a new diagnostic appears.
    // ------------------------------------------------------------------
    #[test]
    fn diagnostics_recomputed_when_file_changed() {
        let adapter = SalsaLspAdapter::new(html_language());
        let uri: DocumentUri = "file:///change.html".parse().unwrap();

        adapter.handle_did_open(DidOpenTextDocumentParams {
            text_document: lsp_types_max::TextDocumentItem {
                uri: uri.clone(),
                language_id: "html".to_string(),
                version: 1,
                text: "<p>hello</p>".to_string(),
            },
        });
        assert!(
            adapter.pull_diagnostics(&uri).is_empty(),
            "should be clean initially"
        );

        // Replace entire document with broken HTML.
        adapter.handle_did_change(DidChangeTextDocumentParams {
            text_document: VersionedTextDocumentIdentifier {
                uri: uri.clone(),
                version: 2,
            },
            content_changes: vec![TextDocumentContentChangeEvent {
                range: None,
                range_length: None,
                text: "<<<INVALID>>>".to_string(),
            }],
        });

        let diags = adapter.pull_diagnostics(&uri);
        assert!(
            !diags.is_empty(),
            "broken HTML must produce AST_ERROR diagnostics; got none"
        );
    }

    // ------------------------------------------------------------------
    // TRUE + COUNTERFACTUAL: did_change_invalidates_only_changed_uri
    //       Change file A; verify file B diagnostics are unaffected.
    // ------------------------------------------------------------------
    #[test]
    fn did_change_invalidates_only_changed_uri() {
        let adapter = SalsaLspAdapter::new(html_language());

        let uri_a: DocumentUri = "file:///a.html".parse().unwrap();
        let uri_b: DocumentUri = "file:///b.html".parse().unwrap();

        adapter.handle_did_open(DidOpenTextDocumentParams {
            text_document: lsp_types_max::TextDocumentItem {
                uri: uri_a.clone(),
                language_id: "html".to_string(),
                version: 1,
                text: "<p>A</p>".to_string(),
            },
        });
        adapter.handle_did_open(DidOpenTextDocumentParams {
            text_document: lsp_types_max::TextDocumentItem {
                uri: uri_b.clone(),
                language_id: "html".to_string(),
                version: 1,
                text: "<p>B</p>".to_string(),
            },
        });

        let b_diags_before = adapter.pull_diagnostics(&uri_b);

        // Corrupt file A only.
        adapter.handle_did_change(DidChangeTextDocumentParams {
            text_document: VersionedTextDocumentIdentifier {
                uri: uri_a.clone(),
                version: 2,
            },
            content_changes: vec![TextDocumentContentChangeEvent {
                range: None,
                range_length: None,
                text: "<<<BAD>>>".to_string(),
            }],
        });

        let a_diags = adapter.pull_diagnostics(&uri_a);
        let b_diags_after = adapter.pull_diagnostics(&uri_b);

        assert!(
            !a_diags.is_empty(),
            "file A must have errors after corrupt change"
        );
        assert_eq!(b_diags_before, b_diags_after, "file B must be unaffected");
    }

    // ------------------------------------------------------------------
    // FALSE (stale diagnostics are refused):
    //   If the URI is not tracked (was never opened or was closed),
    //   pull_diagnostics must return empty — the old stale result must
    //   NOT be returned.
    // ------------------------------------------------------------------
    #[test]
    fn stale_diagnostics_are_refused() {
        let adapter = SalsaLspAdapter::new(html_language());
        let uri: DocumentUri = "file:///ghost.html".parse().unwrap();

        // Never opened — pull must return empty.
        let diags = adapter.pull_diagnostics(&uri);
        assert!(
            diags.is_empty(),
            "unknown URI must return no diagnostics; got {diags:?}"
        );

        // Open, then close.
        adapter.handle_did_open(DidOpenTextDocumentParams {
            text_document: lsp_types_max::TextDocumentItem {
                uri: uri.clone(),
                language_id: "html".to_string(),
                version: 1,
                text: "<p>ok</p>".to_string(),
            },
        });
        adapter.handle_did_close(DidCloseTextDocumentParams {
            text_document: lsp_types_max::TextDocumentIdentifier { uri: uri.clone() },
        });

        let diags_after_close = adapter.pull_diagnostics(&uri);
        assert!(
            diags_after_close.is_empty(),
            "closed URI must return no diagnostics; got {diags_after_close:?}"
        );
    }
}
