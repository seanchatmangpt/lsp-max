use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use tree_sitter::{Node, Parser};

use crate::identity::{digest_parts, ProjectAdmission};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceRange {
    pub start_byte: usize,
    pub end_byte: usize,
    pub start_line: u32,
    pub start_column: u32,
    pub end_line: u32,
    pub end_column: u32,
}

impl SourceRange {
    pub fn from_node(node: Node<'_>) -> Self {
        let start = node.start_position();
        let end = node.end_position();
        Self {
            start_byte: node.start_byte(),
            end_byte: node.end_byte(),
            start_line: saturating_u32(start.row),
            start_column: saturating_u32(start.column),
            end_line: saturating_u32(end.row),
            end_column: saturating_u32(end.column),
        }
    }

    pub fn contains(&self, line: u32, column: u32) -> bool {
        let after_start =
            line > self.start_line || (line == self.start_line && column >= self.start_column);
        let before_end =
            line < self.end_line || (line == self.end_line && column < self.end_column);
        after_start && before_end
    }

    pub fn contains_cursor(&self, line: u32, column: u32) -> bool {
        self.contains(line, column)
            || (line == self.end_line
                && column == self.end_column
                && self.end_byte > self.start_byte)
    }
}

fn saturating_u32(value: usize) -> u32 {
    value.min(u32::MAX as usize) as u32
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SemanticDiagnostic {
    pub path: String,
    pub code: String,
    pub message: String,
    pub range: SourceRange,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Symbol {
    pub path: String,
    pub kind: String,
    pub name: String,
    pub signature: String,
    pub range: SourceRange,
    pub selection_range: SourceRange,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SemanticSnapshot {
    pub admission_hash: String,
    pub engine: String,
    pub revision_hash: String,
    pub diagnostics: Vec<SemanticDiagnostic>,
    pub symbols: Vec<Symbol>,
}

pub trait SemanticEngine: Send + Sync {
    fn name(&self) -> &'static str;

    fn analyze(
        &self,
        admission: &ProjectAdmission,
        files: &BTreeMap<String, String>,
    ) -> SemanticSnapshot;
}

/// Bounded first engine for RFC 0006.
///
/// This engine proves the runtime/authority architecture using genuine Rust
/// parsing and structural symbols. It intentionally does not claim HIR,
/// name-resolution, macro-expansion, or type-inference equivalence.
#[derive(Clone, Copy, Debug, Default)]
pub struct TreeSitterRustEngine;

impl TreeSitterRustEngine {
    fn parse_file(
        &self,
        path: &str,
        source: &str,
        diagnostics: &mut Vec<SemanticDiagnostic>,
        symbols: &mut Vec<Symbol>,
    ) {
        let mut parser = Parser::new();
        let language = tree_sitter_rust::LANGUAGE.into();
        if parser.set_language(&language).is_err() {
            diagnostics.push(SemanticDiagnostic {
                path: path.to_owned(),
                code: "RA_MAX_GRAMMAR_REFUSED".to_owned(),
                message: "Rust grammar could not be admitted by Tree-sitter".to_owned(),
                range: zero_range(),
            });
            return;
        }

        let Some(tree) = parser.parse(source, None) else {
            diagnostics.push(SemanticDiagnostic {
                path: path.to_owned(),
                code: "RA_MAX_PARSE_UNSUPPORTED".to_owned(),
                message: "Tree-sitter returned no syntax tree".to_owned(),
                range: zero_range(),
            });
            return;
        };

        let mut stack = vec![tree.root_node()];
        while let Some(node) = stack.pop() {
            if node.is_error() || node.is_missing() {
                diagnostics.push(SemanticDiagnostic {
                    path: path.to_owned(),
                    code: if node.is_missing() {
                        "RA_MAX_SYNTAX_MISSING".to_owned()
                    } else {
                        "RA_MAX_SYNTAX_ERROR".to_owned()
                    },
                    message: format!("Rust syntax node `{}` is not well formed", node.kind()),
                    range: SourceRange::from_node(node),
                });
            }

            if let Some(kind) = symbol_kind(node.kind()) {
                if let Some(name_node) = node.child_by_field_name("name") {
                    if let Ok(name) = name_node.utf8_text(source.as_bytes()) {
                        symbols.push(Symbol {
                            path: path.to_owned(),
                            kind: kind.to_owned(),
                            name: name.to_owned(),
                            signature: item_signature(node, source),
                            range: SourceRange::from_node(node),
                            selection_range: SourceRange::from_node(name_node),
                        });
                    }
                }
            }

            for index in (0..node.child_count() as u32).rev() {
                if let Some(child) = node.child(index) {
                    stack.push(child);
                }
            }
        }
    }
}

impl SemanticEngine for TreeSitterRustEngine {
    fn name(&self) -> &'static str {
        "tree-sitter-rust-structural-v2"
    }

    fn analyze(
        &self,
        admission: &ProjectAdmission,
        files: &BTreeMap<String, String>,
    ) -> SemanticSnapshot {
        let mut diagnostics = Vec::new();
        let mut symbols = Vec::new();

        if !admission.verify(files) {
            diagnostics.push(SemanticDiagnostic {
                path: admission.root.clone(),
                code: "RA_MAX_SUBJECT_DRIFT_REFUSED".to_owned(),
                message: "Observed files do not match the admitted project identity".to_owned(),
                range: zero_range(),
            });
        } else {
            for (path, source) in files {
                if path.ends_with(".rs") {
                    self.parse_file(path, source, &mut diagnostics, &mut symbols);
                }
            }
        }

        diagnostics.sort_by(|left, right| {
            (&left.path, left.range.start_byte, &left.code).cmp(&(
                &right.path,
                right.range.start_byte,
                &right.code,
            ))
        });
        symbols.sort_by(|left, right| {
            (&left.path, left.range.start_byte, &left.kind, &left.name).cmp(&(
                &right.path,
                right.range.start_byte,
                &right.kind,
                &right.name,
            ))
        });

        let diagnostics_bytes = serde_json::to_vec(&diagnostics).unwrap_or_default();
        let symbols_bytes = serde_json::to_vec(&symbols).unwrap_or_default();
        let revision_hash = digest_parts([
            b"ra-max/semantic-revision/v2".as_slice(),
            admission.project_hash.as_bytes(),
            self.name().as_bytes(),
            diagnostics_bytes.as_slice(),
            symbols_bytes.as_slice(),
        ]);

        SemanticSnapshot {
            admission_hash: admission.project_hash.clone(),
            engine: self.name().to_owned(),
            revision_hash,
            diagnostics,
            symbols,
        }
    }
}

fn symbol_kind(node_kind: &str) -> Option<&'static str> {
    match node_kind {
        "function_item" => Some("function"),
        "struct_item" => Some("struct"),
        "enum_item" => Some("enum"),
        "trait_item" => Some("trait"),
        "type_item" => Some("type_alias"),
        "const_item" => Some("const"),
        "static_item" => Some("static"),
        "mod_item" => Some("module"),
        _ => None,
    }
}

fn item_signature(node: Node<'_>, source: &str) -> String {
    let text = node
        .utf8_text(source.as_bytes())
        .unwrap_or_default()
        .trim();
    let boundary = text
        .find('{')
        .or_else(|| text.find(';').map(|index| index + 1))
        .unwrap_or(text.len());
    let mut signature = text[..boundary]
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    if signature.len() > 240 {
        signature.truncate(237);
        signature.push_str("...");
    }
    signature
}

fn zero_range() -> SourceRange {
    SourceRange {
        start_byte: 0,
        end_byte: 0,
        start_line: 0,
        start_column: 0,
        end_line: 0,
        end_column: 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::identity::SemanticSubject;

    fn admission(files: &BTreeMap<String, String>) -> ProjectAdmission {
        ProjectAdmission::from_files(
            SemanticSubject::tree_sitter_vertical_slice(),
            "/workspace",
            files,
        )
    }

    #[test]
    fn extracts_rust_symbols_from_real_parse_tree() {
        let files = BTreeMap::from([(
            "src/lib.rs".to_owned(),
            "pub struct Receipt;\npub fn verify() -> bool { true }\n".to_owned(),
        )]);
        let snapshot = TreeSitterRustEngine.analyze(&admission(&files), &files);

        assert!(snapshot.diagnostics.is_empty());
        let receipt = snapshot
            .symbols
            .iter()
            .find(|symbol| symbol.kind == "struct" && symbol.name == "Receipt")
            .expect("struct symbol");
        let verify = snapshot
            .symbols
            .iter()
            .find(|symbol| symbol.kind == "function" && symbol.name == "verify")
            .expect("function symbol");
        assert_eq!(receipt.signature, "pub struct Receipt;");
        assert_eq!(verify.signature, "pub fn verify() -> bool");
        assert!(verify.selection_range.start_byte > verify.range.start_byte);
    }

    #[test]
    fn reports_invalid_rust_without_refusing_the_whole_snapshot() {
        let files = BTreeMap::from([(
            "src/main.rs".to_owned(),
            "fn main( { let x = ; }".to_owned(),
        )]);
        let snapshot = TreeSitterRustEngine.analyze(&admission(&files), &files);

        assert!(!snapshot.diagnostics.is_empty());
        assert!(snapshot
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code.starts_with("RA_MAX_SYNTAX_")));
    }

    #[test]
    fn subject_drift_is_typed_refusal() {
        let admitted_files =
            BTreeMap::from([("src/main.rs".to_owned(), "fn main() {}".to_owned())]);
        let changed_files = BTreeMap::from([(
            "src/main.rs".to_owned(),
            "fn main() { println!(\"changed\"); }".to_owned(),
        )]);

        let snapshot = TreeSitterRustEngine.analyze(&admission(&admitted_files), &changed_files);
        assert_eq!(snapshot.diagnostics[0].code, "RA_MAX_SUBJECT_DRIFT_REFUSED");
    }
}
