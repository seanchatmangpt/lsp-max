use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use thiserror::Error;
use tree_sitter::{Node, Parser};

use crate::identity::{digest_parts, ProjectAdmission};
use crate::semantic::{SemanticSnapshot, SourceRange, Symbol};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Occurrence {
    pub path: String,
    pub name: String,
    pub range: SourceRange,
    pub is_definition: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct HoverInfo {
    pub symbol: Symbol,
    pub occurrence: Occurrence,
    pub definition_count: usize,
    pub lexical_only: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompletionCandidate {
    pub label: String,
    pub kind: String,
    pub detail: String,
    pub definition_path: Option<String>,
    pub lexical_only: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TextReplacement {
    pub range: SourceRange,
    pub new_text: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RenamePlan {
    pub subject_revision: String,
    pub old_name: String,
    pub new_name: String,
    pub edits: BTreeMap<String, Vec<TextReplacement>>,
    pub plan_hash: String,
    pub lexical_only: bool,
}

#[derive(Clone, Debug, Error, PartialEq, Eq, Serialize, Deserialize)]
pub enum IntelligenceError {
    #[error("document is outside the admitted workspace: {0}")]
    MissingDocument(String),
    #[error("no Rust identifier at {path}:{line}:{column}")]
    NoIdentifier {
        path: String,
        line: u32,
        column: u32,
    },
    #[error("symbol is not defined in the admitted workspace: {0}")]
    Undefined(String),
    #[error("symbol `{name}` has {definitions} admitted definitions")]
    Ambiguous { name: String, definitions: usize },
    #[error("rename target is not an admitted Rust identifier: {0}")]
    InvalidIdentifier(String),
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkspaceIndex {
    pub admission_hash: String,
    pub semantic_revision: String,
    pub index_hash: String,
    pub symbols: Vec<Symbol>,
    pub occurrences: Vec<Occurrence>,
    #[serde(skip)]
    files: BTreeMap<String, String>,
}

impl WorkspaceIndex {
    pub fn build(
        admission: &ProjectAdmission,
        snapshot: &SemanticSnapshot,
        files: &BTreeMap<String, String>,
    ) -> Self {
        let mut occurrences = Vec::new();
        let definition_ranges: BTreeSet<_> = snapshot
            .symbols
            .iter()
            .map(|symbol| {
                (
                    symbol.path.clone(),
                    symbol.selection_range.start_byte,
                    symbol.selection_range.end_byte,
                )
            })
            .collect();

        for (path, source) in files {
            if path.ends_with(".rs") {
                scan_occurrences(path, source, &definition_ranges, &mut occurrences);
            }
        }

        occurrences.sort_by(|left, right| {
            (
                &left.path,
                left.range.start_byte,
                left.range.end_byte,
                &left.name,
            )
                .cmp(&(
                    &right.path,
                    right.range.start_byte,
                    right.range.end_byte,
                    &right.name,
                ))
        });

        let symbols_bytes = serde_json::to_vec(&snapshot.symbols).unwrap_or_default();
        let occurrences_bytes = serde_json::to_vec(&occurrences).unwrap_or_default();
        let index_hash = digest_parts([
            b"ra-max/workspace-index/v1".as_slice(),
            admission.project_hash.as_bytes(),
            snapshot.revision_hash.as_bytes(),
            symbols_bytes.as_slice(),
            occurrences_bytes.as_slice(),
        ]);

        Self {
            admission_hash: admission.project_hash.clone(),
            semantic_revision: snapshot.revision_hash.clone(),
            index_hash,
            symbols: snapshot.symbols.clone(),
            occurrences,
            files: files.clone(),
        }
    }

    pub fn verify(&self) -> bool {
        let symbols_bytes = serde_json::to_vec(&self.symbols).unwrap_or_default();
        let occurrences_bytes = serde_json::to_vec(&self.occurrences).unwrap_or_default();
        self.index_hash
            == digest_parts([
                b"ra-max/workspace-index/v1".as_slice(),
                self.admission_hash.as_bytes(),
                self.semantic_revision.as_bytes(),
                symbols_bytes.as_slice(),
                occurrences_bytes.as_slice(),
            ])
    }

    pub fn identifier_at(&self, path: &str, line: u32, column: u32) -> Option<Occurrence> {
        self.occurrences
            .iter()
            .filter(|occurrence| occurrence.path == path)
            .filter(|occurrence| occurrence.range.contains_cursor(line, column))
            .min_by_key(|occurrence| occurrence.range.end_byte - occurrence.range.start_byte)
            .cloned()
    }

    pub fn definition_at(
        &self,
        path: &str,
        line: u32,
        column: u32,
    ) -> Result<Symbol, IntelligenceError> {
        let occurrence = self.occurrence_or_error(path, line, column)?;
        self.unique_definition(&occurrence.name)
    }

    pub fn references_at(
        &self,
        path: &str,
        line: u32,
        column: u32,
        include_definition: bool,
    ) -> Result<Vec<Occurrence>, IntelligenceError> {
        let occurrence = self.occurrence_or_error(path, line, column)?;
        self.unique_definition(&occurrence.name)?;
        Ok(self
            .occurrences
            .iter()
            .filter(|candidate| candidate.name == occurrence.name)
            .filter(|candidate| include_definition || !candidate.is_definition)
            .cloned()
            .collect())
    }

    pub fn hover_at(
        &self,
        path: &str,
        line: u32,
        column: u32,
    ) -> Result<HoverInfo, IntelligenceError> {
        let occurrence = self.occurrence_or_error(path, line, column)?;
        let definition_count = self
            .symbols
            .iter()
            .filter(|symbol| symbol.name == occurrence.name)
            .count();
        let symbol = self.unique_definition(&occurrence.name)?;
        Ok(HoverInfo {
            symbol,
            occurrence,
            definition_count,
            lexical_only: true,
        })
    }

    pub fn completions(
        &self,
        path: &str,
        line: u32,
        column: u32,
        limit: usize,
    ) -> Result<Vec<CompletionCandidate>, IntelligenceError> {
        let source = self
            .files
            .get(path)
            .ok_or_else(|| IntelligenceError::MissingDocument(path.to_owned()))?;
        let prefix = completion_prefix(source, line, column).unwrap_or_default();
        let mut seen = BTreeSet::new();
        let mut candidates = Vec::new();

        for symbol in &self.symbols {
            if symbol.name.starts_with(&prefix)
                && seen.insert((symbol.name.clone(), symbol.kind.clone()))
            {
                candidates.push(CompletionCandidate {
                    label: symbol.name.clone(),
                    kind: symbol.kind.clone(),
                    detail: symbol.signature.clone(),
                    definition_path: Some(symbol.path.clone()),
                    lexical_only: true,
                });
            }
        }

        for keyword in RUST_KEYWORDS {
            if keyword.starts_with(&prefix)
                && seen.insert(((*keyword).to_owned(), "keyword".to_owned()))
            {
                candidates.push(CompletionCandidate {
                    label: (*keyword).to_owned(),
                    kind: "keyword".to_owned(),
                    detail: "Rust keyword".to_owned(),
                    definition_path: None,
                    lexical_only: true,
                });
            }
        }

        candidates.sort_by(|left, right| {
            (&left.label, &left.kind, &left.definition_path).cmp(&(
                &right.label,
                &right.kind,
                &right.definition_path,
            ))
        });
        candidates.truncate(limit);
        Ok(candidates)
    }

    pub fn document_symbols(&self, path: &str) -> Vec<Symbol> {
        self.symbols
            .iter()
            .filter(|symbol| symbol.path == path)
            .cloned()
            .collect()
    }

    pub fn workspace_symbols(&self, query: &str, limit: usize) -> Vec<Symbol> {
        let query = query.to_lowercase();
        let mut symbols: Vec<_> = self
            .symbols
            .iter()
            .filter(|symbol| query.is_empty() || symbol.name.to_lowercase().contains(&query))
            .cloned()
            .collect();
        symbols.truncate(limit);
        symbols
    }

    pub fn prepare_rename_at(
        &self,
        path: &str,
        line: u32,
        column: u32,
    ) -> Result<Occurrence, IntelligenceError> {
        let occurrence = self.occurrence_or_error(path, line, column)?;
        self.unique_definition(&occurrence.name)?;
        Ok(occurrence)
    }

    pub fn rename_plan(
        &self,
        path: &str,
        line: u32,
        column: u32,
        new_name: &str,
    ) -> Result<RenamePlan, IntelligenceError> {
        if !is_admitted_identifier(new_name) {
            return Err(IntelligenceError::InvalidIdentifier(new_name.to_owned()));
        }

        let occurrence = self.occurrence_or_error(path, line, column)?;
        self.unique_definition(&occurrence.name)?;
        let mut edits: BTreeMap<String, Vec<TextReplacement>> = BTreeMap::new();

        for reference in self
            .occurrences
            .iter()
            .filter(|candidate| candidate.name == occurrence.name)
        {
            edits
                .entry(reference.path.clone())
                .or_default()
                .push(TextReplacement {
                    range: reference.range.clone(),
                    new_text: new_name.to_owned(),
                });
        }

        for replacements in edits.values_mut() {
            replacements.sort_by_key(|replacement| replacement.range.start_byte);
        }

        let edits_bytes = serde_json::to_vec(&edits).unwrap_or_default();
        let plan_hash = digest_parts([
            b"ra-max/rename-plan/v1".as_slice(),
            self.semantic_revision.as_bytes(),
            occurrence.name.as_bytes(),
            new_name.as_bytes(),
            edits_bytes.as_slice(),
        ]);

        Ok(RenamePlan {
            subject_revision: self.semantic_revision.clone(),
            old_name: occurrence.name,
            new_name: new_name.to_owned(),
            edits,
            plan_hash,
            lexical_only: true,
        })
    }

    fn occurrence_or_error(
        &self,
        path: &str,
        line: u32,
        column: u32,
    ) -> Result<Occurrence, IntelligenceError> {
        if !self.files.contains_key(path) {
            return Err(IntelligenceError::MissingDocument(path.to_owned()));
        }
        self.identifier_at(path, line, column)
            .ok_or_else(|| IntelligenceError::NoIdentifier {
                path: path.to_owned(),
                line,
                column,
            })
    }

    fn unique_definition(&self, name: &str) -> Result<Symbol, IntelligenceError> {
        let definitions: Vec<_> = self
            .symbols
            .iter()
            .filter(|symbol| symbol.name == name)
            .cloned()
            .collect();
        match definitions.len() {
            0 => Err(IntelligenceError::Undefined(name.to_owned())),
            1 => Ok(definitions.into_iter().next().expect("length checked")),
            definitions => Err(IntelligenceError::Ambiguous {
                name: name.to_owned(),
                definitions,
            }),
        }
    }
}

fn scan_occurrences(
    path: &str,
    source: &str,
    definition_ranges: &BTreeSet<(String, usize, usize)>,
    output: &mut Vec<Occurrence>,
) {
    let mut parser = Parser::new();
    let language = tree_sitter_rust::LANGUAGE.into();
    if parser.set_language(&language).is_err() {
        return;
    }
    let Some(tree) = parser.parse(source, None) else {
        return;
    };

    let mut stack = vec![tree.root_node()];
    while let Some(node) = stack.pop() {
        if is_identifier_node(node) {
            if let Ok(name) = node.utf8_text(source.as_bytes()) {
                let range = SourceRange::from_node(node);
                let is_definition = definition_ranges.contains(&(
                    path.to_owned(),
                    range.start_byte,
                    range.end_byte,
                ));
                output.push(Occurrence {
                    path: path.to_owned(),
                    name: name.to_owned(),
                    range,
                    is_definition,
                });
            }
        }

        for index in (0..node.child_count() as u32).rev() {
            if let Some(child) = node.child(index) {
                stack.push(child);
            }
        }
    }
}

fn is_identifier_node(node: Node<'_>) -> bool {
    matches!(
        node.kind(),
        "identifier" | "type_identifier" | "field_identifier" | "shorthand_field_identifier"
    )
}

fn completion_prefix(source: &str, line: u32, column: u32) -> Option<String> {
    let byte = position_to_byte(source, line, column)?;
    let before = &source[..byte];
    let start = before
        .char_indices()
        .rev()
        .find_map(|(index, character)| {
            (!is_identifier_continue(character)).then_some(index + character.len_utf8())
        })
        .unwrap_or(0);
    Some(before[start..].to_owned())
}

fn position_to_byte(source: &str, line: u32, column: u32) -> Option<usize> {
    let target_line = line as usize;
    let target_column = column as usize;
    let mut current_line = 0usize;
    let mut offset = 0usize;

    for segment in source.split_inclusive('\n') {
        let line_text = segment.strip_suffix('\n').unwrap_or(segment);
        if current_line == target_line {
            if target_column <= line_text.len() && line_text.is_char_boundary(target_column) {
                return Some(offset + target_column);
            }
            return None;
        }
        offset += segment.len();
        current_line += 1;
    }

    (current_line == target_line && target_column == 0).then_some(offset)
}

fn is_admitted_identifier(value: &str) -> bool {
    let mut characters = value.chars();
    let Some(first) = characters.next() else {
        return false;
    };
    (first == '_' || first.is_alphabetic())
        && characters.all(is_identifier_continue)
        && !RUST_KEYWORDS.contains(&value)
}

fn is_identifier_continue(character: char) -> bool {
    character == '_' || character.is_alphanumeric()
}

const RUST_KEYWORDS: &[&str] = &[
    "as", "async", "await", "break", "const", "continue", "crate", "dyn", "else",
    "enum", "extern", "false", "fn", "for", "if", "impl", "in", "let", "loop",
    "match", "mod", "move", "mut", "pub", "ref", "return", "self", "Self", "static",
    "struct", "super", "trait", "true", "type", "unsafe", "use", "where", "while",
];

#[cfg(test)]
mod tests {
    use super::*;
    use crate::identity::SemanticSubject;
    use crate::semantic::{SemanticEngine, TreeSitterRustEngine};

    fn fixture() -> (ProjectAdmission, SemanticSnapshot, BTreeMap<String, String>) {
        let files = BTreeMap::from([
            (
                "src/lib.rs".to_owned(),
                "pub fn meaning() -> u32 { 42 }\n".to_owned(),
            ),
            (
                "src/main.rs".to_owned(),
                "fn main() { let answer = meaning(); println!(\"meaning\"); }\n".to_owned(),
            ),
        ]);
        let admission = ProjectAdmission::from_files(
            SemanticSubject::tree_sitter_vertical_slice(),
            "/workspace",
            &files,
        );
        let snapshot = TreeSitterRustEngine.analyze(&admission, &files);
        (admission, snapshot, files)
    }

    fn position(source: &str, needle: &str) -> (u32, u32) {
        let byte = source.find(needle).expect("needle must exist");
        let before = &source[..byte];
        let line = before.bytes().filter(|byte| *byte == b'\n').count() as u32;
        let column = before
            .rsplit_once('\n')
            .map_or(before.len(), |(_, tail)| tail.len()) as u32;
        (line, column)
    }

    #[test]
    fn cross_file_navigation_hover_and_references_are_deterministic() {
        let (admission, snapshot, files) = fixture();
        let index = WorkspaceIndex::build(&admission, &snapshot, &files);
        let main = files.get("src/main.rs").expect("fixture");
        let (line, column) = position(main, "meaning");

        let definition = index
            .definition_at("src/main.rs", line, column)
            .expect("unique item definition");
        let hover = index
            .hover_at("src/main.rs", line, column)
            .expect("hover");
        let references = index
            .references_at("src/main.rs", line, column, true)
            .expect("references");

        assert_eq!(definition.path, "src/lib.rs");
        assert!(hover.symbol.signature.starts_with("pub fn meaning"));
        assert_eq!(references.len(), 2);
        assert!(index.verify());
    }

    #[test]
    fn comments_and_strings_do_not_become_references() {
        let (admission, snapshot, files) = fixture();
        let index = WorkspaceIndex::build(&admission, &snapshot, &files);
        let meaning_occurrences = index
            .occurrences
            .iter()
            .filter(|occurrence| occurrence.name == "meaning")
            .count();
        assert_eq!(meaning_occurrences, 2);
    }

    #[test]
    fn completion_and_rename_cover_the_unique_item_surface() {
        let (admission, snapshot, mut files) = fixture();
        files.insert(
            "src/scratch.rs".to_owned(),
            "fn probe() { mea }\n".to_owned(),
        );
        let admission = ProjectAdmission::from_files(admission.subject, "/workspace", &files);
        let snapshot = TreeSitterRustEngine.analyze(&admission, &files);
        let index = WorkspaceIndex::build(&admission, &snapshot, &files);
        let scratch = files.get("src/scratch.rs").expect("fixture");
        let (line, column) = position(scratch, "mea");
        let completions = index
            .completions("src/scratch.rs", line, column + 3, 20)
            .expect("completion");
        assert!(completions.iter().any(|item| item.label == "meaning"));

        let main = files.get("src/main.rs").expect("fixture");
        let (line, column) = position(main, "meaning");
        let rename = index
            .rename_plan("src/main.rs", line, column, "answer_to_everything")
            .expect("rename plan");
        assert_eq!(rename.edits.values().map(Vec::len).sum::<usize>(), 2);
        assert!(rename.lexical_only);
    }

    #[test]
    fn duplicate_item_names_are_typed_ambiguity_not_guessed_scope() {
        let files = BTreeMap::from([
            ("src/a.rs".to_owned(), "pub fn duplicate() {}\n".to_owned()),
            ("src/b.rs".to_owned(), "pub fn duplicate() {}\n".to_owned()),
        ]);
        let admission = ProjectAdmission::from_files(
            SemanticSubject::tree_sitter_vertical_slice(),
            "/workspace",
            &files,
        );
        let snapshot = TreeSitterRustEngine.analyze(&admission, &files);
        let index = WorkspaceIndex::build(&admission, &snapshot, &files);
        let (line, column) = position(files.get("src/a.rs").expect("fixture"), "duplicate");

        assert!(matches!(
            index.definition_at("src/a.rs", line, column),
            Err(IntelligenceError::Ambiguous { definitions: 2, .. })
        ));
    }
}
