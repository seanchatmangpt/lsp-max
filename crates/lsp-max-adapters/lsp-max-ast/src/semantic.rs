//! Declarative Tree-sitter query packs projected into LSP semantics.

use std::collections::HashSet;

use lsp_types_max::{DocumentUri, Location, Position, Range, TextEdit};
use streaming_iterator::StreamingIterator;
use tree_sitter::{Language, Query, QueryCursor};

use crate::core::document::Document;

const DEFINITION_PREFIX: &str = "definition.";
const REFERENCE_PREFIX: &str = "reference.";

/// A grammar-specific, declarative description of definitions and references.
///
/// Capture names are the protocol between the pack and the generic adapter:
/// `@definition.<class>` declares a symbol and `@reference.<class>` uses one.
/// A reference class is admitted only when the pack declares the corresponding
/// definition class.
#[derive(Debug, Clone)]
pub struct SemanticQueryPack {
    source: String,
    language: Language,
}

impl SemanticQueryPack {
    /// Admits a query pack against its exact grammar.
    pub fn admit(language: &Language, source: impl Into<String>) -> Result<Self, QueryPackError> {
        let source = source.into();
        let query = Query::new(language, &source)
            .map_err(|error| QueryPackError::InvalidQuery(error.to_string()))?;
        let mut definitions = HashSet::new();
        let mut references = HashSet::new();
        for capture in query.capture_names() {
            if let Some(class) = capture.strip_prefix(DEFINITION_PREFIX) {
                definitions.insert(class.to_string());
            } else if let Some(class) = capture.strip_prefix(REFERENCE_PREFIX) {
                references.insert(class.to_string());
            } else {
                return Err(QueryPackError::UnsupportedCapture(capture.to_string()));
            }
        }
        if definitions.is_empty() {
            return Err(QueryPackError::MissingDefinitions);
        }
        for class in references {
            if !definitions.contains(&class) {
                return Err(QueryPackError::ReferenceWithoutDefinition(class));
            }
        }
        Ok(Self {
            source,
            language: language.clone(),
        })
    }

    pub(crate) fn accepts_language(&self, language: &Language) -> bool {
        self.language == language.clone()
    }

    pub(crate) fn index(&self, uri: &DocumentUri, document: &Document) -> SemanticIndex {
        let query = Query::new(&self.language, &self.source)
            .expect("an admitted query remains valid for its bound grammar");
        let capture_names = query.capture_names();
        let mut cursor = QueryCursor::new();
        let mut matches = cursor.matches(&query, document.tree.root_node(), document.as_bytes());
        let mut entries = Vec::new();
        while let Some(query_match) = matches.next() {
            for capture in query_match.captures {
                let capture_name = capture_names[capture.index as usize];
                let (role, class) =
                    if let Some(class) = capture_name.strip_prefix(DEFINITION_PREFIX) {
                        (SemanticRole::Definition, class)
                    } else if let Some(class) = capture_name.strip_prefix(REFERENCE_PREFIX) {
                        (SemanticRole::Reference, class)
                    } else {
                        continue;
                    };
                if let (Ok(name), Ok(range)) = (
                    capture.node.utf8_text(document.as_bytes()),
                    document.denormalize_range(&capture.node.range()),
                ) {
                    entries.push(SemanticEntry {
                        name: name.to_string(),
                        class: class.to_string(),
                        role,
                        range,
                    });
                }
            }
        }
        SemanticIndex {
            uri: uri.clone(),
            entries,
        }
    }
}

/// Typed query-pack admission failures.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum QueryPackError {
    #[error("LSPMAX_SEMANTIC_QUERY_INVALID: {0}")]
    InvalidQuery(String),
    #[error("LSPMAX_SEMANTIC_DEFINITION_MISSING: query pack declares no definitions")]
    MissingDefinitions,
    #[error("LSPMAX_SEMANTIC_CAPTURE_UNSUPPORTED: {0}")]
    UnsupportedCapture(String),
    #[error("LSPMAX_SEMANTIC_REFERENCE_ORPHANED: reference class `{0}` has no definition")]
    ReferenceWithoutDefinition(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SemanticRole {
    Definition,
    Reference,
}

#[derive(Debug, Clone)]
struct SemanticEntry {
    name: String,
    class: String,
    role: SemanticRole,
    range: Range,
}

/// A semantic snapshot derived exclusively from an admitted query pack.
#[derive(Debug, Clone)]
pub struct SemanticIndex {
    uri: DocumentUri,
    entries: Vec<SemanticEntry>,
}

impl SemanticIndex {
    fn subject_at(&self, position: Position) -> Option<&SemanticEntry> {
        self.entries
            .iter()
            .filter(|entry| contains(entry.range, position))
            .min_by_key(|entry| range_size(entry.range))
    }

    /// Resolves the selected capture to its first matching declaration.
    pub fn definition(&self, position: Position) -> Option<Location> {
        let subject = self.subject_at(position)?;
        self.entries
            .iter()
            .find(|entry| {
                entry.role == SemanticRole::Definition
                    && entry.class == subject.class
                    && entry.name == subject.name
            })
            .map(|entry| Location::new(self.uri.clone(), entry.range))
    }

    /// Finds all same-class uses, optionally including the declaration.
    pub fn references(&self, position: Position, include_declaration: bool) -> Vec<Location> {
        let Some(subject) = self.subject_at(position) else {
            return Vec::new();
        };
        self.entries
            .iter()
            .filter(|entry| {
                entry.class == subject.class
                    && entry.name == subject.name
                    && (include_declaration || entry.role == SemanticRole::Reference)
            })
            .map(|entry| Location::new(self.uri.clone(), entry.range))
            .collect()
    }

    /// Constructs edits for every same-class capture of the selected symbol.
    pub fn rename_edits(&self, position: Position, new_name: &str) -> Vec<TextEdit> {
        self.references(position, true)
            .into_iter()
            .map(|location| TextEdit::new(location.range, new_name.to_string()))
            .collect()
    }
}

/// A version-bound rename construction. Callers must preserve `version` when
/// converting this into a `TextDocumentEdit` for actuation.
#[derive(Debug, Clone, PartialEq)]
pub struct SemanticRename {
    pub uri: DocumentUri,
    pub version: i32,
    pub edits: Vec<TextEdit>,
}

fn contains(range: Range, position: Position) -> bool {
    range.start <= position && position <= range.end
}

fn range_size(range: Range) -> (u32, u32) {
    (
        range.end.line.saturating_sub(range.start.line),
        range.end.character.saturating_sub(range.start.character),
    )
}
