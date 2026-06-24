// use crate::lsif::{Element, Vertex, VertexType, Edge, EdgeType, Id};
// use lsp_types_max::Position;

#[salsa::query_group(SemanticStorage)]
pub trait SemanticDb: salsa::Database {
    #[salsa::input]
    fn document_text(&self, uri: String) -> String;

    fn parse_rust_ast(&self, uri: String) -> RustAst;
    fn compute_semantic_quads(&self, uri: String) -> Vec<String>;
    fn cryptographic_standing(&self, uri: String) -> String;
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RustAstNode {
    pub symbol: String,
    pub start_line: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RustAst {
    pub nodes: Vec<RustAstNode>,
}

use tree_sitter::StreamingIterator;

fn parse_rust_ast(db: &dyn SemanticDb, uri: String) -> RustAst {
    let text = db.document_text(uri);
    let mut parser = tree_sitter::Parser::new();
    parser.set_language(&tree_sitter_rust::LANGUAGE.into()).unwrap();
    
    let mut nodes = Vec::new();
    if let Some(tree) = parser.parse(&text, None) {
        let query = tree_sitter::Query::new(
            &tree_sitter_rust::LANGUAGE.into(),
            "(function_item name: (identifier) @name)
             (struct_item name: (type_identifier) @name)",
        ).unwrap();
        
        let mut cursor = tree_sitter::QueryCursor::new();
        let mut matches = cursor.matches(&query, tree.root_node(), text.as_bytes());
        while let Some(m) = matches.next() {
            for cap in m.captures {
                let node = cap.node;
                if let Ok(symbol) = node.utf8_text(text.as_bytes()) {
                    nodes.push(RustAstNode {
                        symbol: symbol.to_string(),
                        start_line: node.start_position().row as u32,
                    });
                }
            }
        }
    }
    
    RustAst { nodes }
}

fn compute_semantic_quads(db: &dyn SemanticDb, uri: String) -> Vec<String> {
    let ast = db.parse_rust_ast(uri.clone());
    let mut semantic_facts = Vec::new();
    
    for node in ast.nodes {
        semantic_facts.push(format!("<urn:lsif:v:{}> <lsif:contains> <urn:lsif:symbol:{}> .", uri, node.symbol));
        semantic_facts.push(format!("<urn:lsif:symbol:{}> <lsif:line> \"{}\" .", node.symbol, node.start_line));
    }
    semantic_facts
}

fn cryptographic_standing(db: &dyn SemanticDb, uri: String) -> String {
    let facts = db.compute_semantic_quads(uri);
    let mut hasher = blake3::Hasher::new();
    for fact in &facts {
        hasher.update(fact.as_bytes());
        hasher.update(b"\n");
    }
    hasher.finalize().to_hex().to_string()
}

#[salsa::database(SemanticStorage)]
#[derive(Default)]
pub struct LsifSemanticDatabase {
    storage: salsa::Storage<Self>,
}

impl salsa::Database for LsifSemanticDatabase {}
