use crate::observations::Observation;
use tree_sitter::{Node, Parser};

pub fn parse_rust_ast(filepath: &str, content: &str) -> Vec<Observation> {
    let mut observations = Vec::new();
    let mut parser = Parser::new();
    let lang = tree_sitter_rust::language();
    if parser.set_language(&lang).is_err() {
        return observations;
    }

    if let Some(tree) = parser.parse(content, None) {
        let root = tree.root_node();
        traverse_node(root, content.as_bytes(), filepath, &mut observations);
    }
    observations
}

fn traverse_node(node: Node, source: &[u8], filepath: &str, obs: &mut Vec<Observation>) {
    let kind = node.kind();
    let range = node.range();
    let text = node.utf8_text(source).unwrap_or_default().to_string();

    // Check for use of tower_lsp
    if kind == "use_declaration"
        && (text.contains("tower_lsp") || text.contains("tower-lsp"))
        && !(text.contains("tower_lsp_max") || text.contains("tower-lsp-max"))
    {
        obs.push(Observation {
            file_path: filepath.to_string(),
            start_byte: range.start_byte,
            end_byte: range.end_byte,
            line: range.start_point.row + 1,
            column: range.start_point.column + 1,
            kind: "ast_node".to_string(),
            construct: "use tower_lsp".to_string(),
            context: text.clone(),
            message: "Import of plain tower_lsp found in AST".to_string(),
        });
    }

    // Check for tower_lsp:: namespace references
    if kind == "scoped_identifier"
        && text.starts_with("tower_lsp::")
        && !text.starts_with("tower_lsp_max::")
    {
        obs.push(Observation {
            file_path: filepath.to_string(),
            start_byte: range.start_byte,
            end_byte: range.end_byte,
            line: range.start_point.row + 1,
            column: range.start_point.column + 1,
            kind: "ast_node".to_string(),
            construct: "tower_lsp::".to_string(),
            context: text.clone(),
            message: "Namespace path tower_lsp:: found in AST".to_string(),
        });
    }

    // Check for forbidden mutations in authority paths
    let forbidden_constructs = [
        ("std::fs::write", "std::fs::write"),
        ("tokio::fs::write", "tokio::fs::write"),
        ("File::create", "File::create"),
        ("OpenOptions", "OpenOptions"),
        ("write_all", "write_all"),
        ("WorkspaceEdit", "WorkspaceEdit"),
        ("execute_command", "execute_command"),
        ("workspace/applyEdit", "workspace/applyEdit"),
    ];

    for &(construct, name) in &forbidden_constructs {
        if text.contains(construct)
            && (kind == "call_expression"
                || kind == "identifier"
                || kind == "path_expression"
                || kind == "type_identifier")
        {
            obs.push(Observation {
                file_path: filepath.to_string(),
                start_byte: range.start_byte,
                end_byte: range.end_byte,
                line: range.start_point.row + 1,
                column: range.start_point.column + 1,
                kind: "ast_node".to_string(),
                construct: name.to_string(),
                context: text.clone(),
                message: format!("Forbidden construct '{}' found in AST", name),
            });
        }
    }

    // Check for panics and unwraps
    let panic_constructs = [
        ("unwrap", "unwrap()"),
        ("expect", "expect()"),
        ("panic!", "panic!()"),
        ("todo!", "todo!()"),
        ("unimplemented!", "unimplemented!()"),
        ("dbg!", "dbg!()"),
    ];

    for &(construct, name) in &panic_constructs {
        if text.contains(construct)
            && (kind == "call_expression"
                || kind == "macro_invocation"
                || kind == "field_expression")
        {
            obs.push(Observation {
                file_path: filepath.to_string(),
                start_byte: range.start_byte,
                end_byte: range.end_byte,
                line: range.start_point.row + 1,
                column: range.start_point.column + 1,
                kind: "ast_node".to_string(),
                construct: name.to_string(),
                context: text.clone(),
                message: format!("Smell/panic construct '{}' found in AST", name),
            });
        }
    }

    // Check for serde_json::Value
    if text.contains("serde_json::Value")
        && (kind == "type_identifier" || kind == "path_expression")
    {
        obs.push(Observation {
            file_path: filepath.to_string(),
            start_byte: range.start_byte,
            end_byte: range.end_byte,
            line: range.start_point.row + 1,
            column: range.start_point.column + 1,
            kind: "ast_node".to_string(),
            construct: "serde_json::Value".to_string(),
            context: text.clone(),
            message: "serde_json::Value used instead of typed structure".to_string(),
        });
    }

    // Check for substring check over TODO, customization-map.json, etc.
    let substring_checks = [
        ("content.contains", "content.contains"),
        ("path.ends_with", "path.ends_with"),
        ("path_str.contains", "path_str.contains"),
        ("make_diag", "make_diag"),
    ];

    for &(construct, name) in &substring_checks {
        if text.contains(construct) && kind == "call_expression" {
            obs.push(Observation {
                file_path: filepath.to_string(),
                start_byte: range.start_byte,
                end_byte: range.end_byte,
                line: range.start_point.row + 1,
                column: range.start_point.column + 1,
                kind: "ast_node".to_string(),
                construct: name.to_string(),
                context: text.clone(),
                message: format!("Call to '{}' found (substring check smell)", name),
            });
        }
    }

    // Recurse to children
    let mut cursor = node.walk();
    if cursor.goto_first_child() {
        loop {
            traverse_node(cursor.node(), source, filepath, obs);
            if !cursor.goto_next_sibling() {
                break;
            }
        }
    }
}
