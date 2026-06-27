use crate::lsif_indexer::{definition_tag, reference_tag, LsifContext};
use crate::lsif_types::{MonikerKind, UniquenessLevel};
use lsp_types_max::SymbolKind;
use std::collections::HashMap;
use std::io::Write;

pub fn index_rust_source<W: Write>(
    source: &str,
    uri: &str,
    builder: &mut crate::lsif_builder::LsifBuilder<W>,
) -> std::io::Result<()> {
    let mut parser = tree_sitter::Parser::new();
    parser
        .set_language(&tree_sitter_rust::LANGUAGE.into())
        .expect("tree-sitter-rust grammar load failed");

    let tree = match parser.parse(source, None) {
        Some(t) => t,
        None => return Ok(()),
    };

    let doc_id = builder.emit_document(uri, "rust")?;
    let source_bytes = source.as_bytes();

    let module_path = uri
        .rsplit('/')
        .next()
        .and_then(|f| f.strip_suffix(".rs"))
        .unwrap_or("unknown")
        .to_string();

    // Pre-pass: collect `use` declarations
    let use_map = collect_use_map(tree.root_node(), source_bytes);

    let mut ctx = LsifContext::new(builder, doc_id.clone(), module_path);
    walk(tree.root_node(), source_bytes, &mut ctx, &use_map)?;
    ctx.builder.end_document(doc_id)?;
    Ok(())
}

// ── Use declaration pre-pass ──────────────────────────────────────────────────

fn collect_use_map<'a>(node: tree_sitter::Node<'a>, source: &'a [u8]) -> HashMap<String, String> {
    let mut map = HashMap::new();
    collect_use_map_node(node, source, &mut map);
    map
}

fn collect_use_map_node<'a>(
    node: tree_sitter::Node<'a>,
    source: &'a [u8],
    map: &mut HashMap<String, String>,
) {
    if node.kind() == "use_declaration" {
        if let Some(arg) = node.child_by_field_name("argument") {
            visit_use_tree(arg, source, "", map);
        }
        return;
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        collect_use_map_node(child, source, map);
    }
}

fn visit_use_tree<'a>(
    node: tree_sitter::Node<'a>,
    source: &'a [u8],
    prefix: &str,
    map: &mut HashMap<String, String>,
) {
    match node.kind() {
        "scoped_identifier" => {
            let path = node_text(node, source);
            let path = strip_crate_prefix(path);
            let local = path.rsplit("::").next().unwrap_or(path);
            let identifier = canonical_identifier(path);
            map.insert(local.to_string(), identifier);
        }
        "scoped_use_list" => {
            let path_node = node.child_by_field_name("path");
            let path_prefix = path_node
                .map(|n| strip_crate_prefix(node_text(n, source)).to_string())
                .unwrap_or_default();
            let combined = if prefix.is_empty() {
                path_prefix
            } else {
                format!("{prefix}::{path_prefix}")
            };
            let list = node.child_by_field_name("list");
            if let Some(list) = list {
                let mut c = list.walk();
                for child in list.children(&mut c) {
                    visit_use_tree(child, source, &combined, map);
                }
            }
        }
        "use_list" => {
            let mut c = node.walk();
            for child in node.children(&mut c) {
                visit_use_tree(child, source, prefix, map);
            }
        }
        "identifier" => {
            let name = node_text(node, source);
            if !name.is_empty() && name != "{" && name != "}" && name != "," {
                let identifier = if prefix.is_empty() {
                    name.to_string()
                } else {
                    format!("{prefix}::{name}")
                };
                let identifier = canonical_identifier(&identifier);
                map.insert(name.to_string(), identifier);
            }
        }
        "use_as_clause" => {
            let mut c = node.walk();
            let children: Vec<_> = node.children(&mut c).collect();
            if let (Some(path_node), Some(alias_node)) = (children.first(), children.last()) {
                if path_node.id() != alias_node.id() {
                    let path = node_text(*path_node, source);
                    let path = strip_crate_prefix(path);
                    let alias = node_text(*alias_node, source);
                    let identifier = canonical_identifier(path);
                    map.insert(alias.to_string(), identifier);
                }
            }
        }
        _ => {}
    }
}

fn strip_crate_prefix(path: &str) -> &str {
    for prefix in &["crate::", "self::", "super::"] {
        if let Some(rest) = path.strip_prefix(prefix) {
            return rest;
        }
    }
    path
}

fn canonical_identifier(path: &str) -> String {
    let parts: Vec<&str> = path.split("::").collect();
    if parts.len() >= 2 {
        format!("{}::{}", parts[parts.len() - 2], parts[parts.len() - 1])
    } else {
        path.to_string()
    }
}

// ── Main walk ─────────────────────────────────────────────────────────────────

fn walk<W: Write>(
    node: tree_sitter::Node<'_>,
    source: &[u8],
    ctx: &mut LsifContext<'_, W>,
    use_map: &HashMap<String, String>,
) -> std::io::Result<()> {
    match node.kind() {
        "function_item" => emit_function_item(node, source, ctx)?,
        "struct_item" => emit_named_def(node, source, ctx, SymbolKind::STRUCT, "struct")?,
        "enum_item" => emit_named_def(node, source, ctx, SymbolKind::ENUM, "enum")?,
        "trait_item" => emit_named_def(node, source, ctx, SymbolKind::INTERFACE, "trait")?,
        "type_item" => emit_named_def(node, source, ctx, SymbolKind::TYPE_PARAMETER, "type")?,
        "const_item" => emit_named_def(node, source, ctx, SymbolKind::CONSTANT, "const")?,
        "static_item" => emit_named_def(node, source, ctx, SymbolKind::VARIABLE, "static")?,
        "call_expression" => emit_call_expression(node, source, ctx, use_map)?,
        "use_declaration" => return Ok(()),
        _ => {}
    }

    let has_scope = match node.kind() {
        "block" | "function_item" | "impl_item" | "trait_item" => {
            ctx.push_scope();
            true
        }
        _ => false,
    };

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        walk(child, source, ctx, use_map)?;
    }

    if has_scope {
        ctx.pop_scope();
    }
    Ok(())
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn node_text<'a>(node: tree_sitter::Node<'_>, source: &'a [u8]) -> &'a str {
    node.utf8_text(source).unwrap_or("")
}

fn ts_point_to_lsp(point: tree_sitter::Point) -> lsp_types_max::Position {
    lsp_types_max::Position {
        line: point.row as u32,
        character: point.column as u32,
    }
}

fn ts_range_to_lsp(range: tree_sitter::Range) -> lsp_types_max::Range {
    lsp_types_max::Range {
        start: ts_point_to_lsp(range.start_point),
        end: ts_point_to_lsp(range.end_point),
    }
}

fn is_pub(node: tree_sitter::Node<'_>, source: &[u8]) -> bool {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "visibility_modifier" {
            return node_text(child, source).starts_with("pub");
        }
    }
    false
}

fn callee_name<'a>(
    callee_node: tree_sitter::Node<'_>,
    source: &'a [u8],
) -> Option<(&'a str, tree_sitter::Range)> {
    match callee_node.kind() {
        "identifier" => {
            let text = node_text(callee_node, source);
            if text.is_empty() {
                None
            } else {
                Some((text, callee_node.range()))
            }
        }
        "scoped_identifier" => callee_node
            .child_by_field_name("name")
            .map(|n| (node_text(n, source), n.range()))
            .filter(|(t, _)| !t.is_empty()),
        "field_expression" => callee_node
            .child_by_field_name("field")
            .map(|n| (node_text(n, source), n.range()))
            .filter(|(t, _)| !t.is_empty()),
        _ => None,
    }
}

fn extract_signature(node: tree_sitter::Node<'_>, source: &[u8]) -> String {
    let text = node.utf8_text(source).unwrap_or("").trim();
    if let Some(idx) = text.find('{') {
        text[..idx].trim().to_string()
    } else if let Some(idx) = text.find(';') {
        text[..idx].trim().to_string()
    } else {
        text.to_string()
    }
}

fn extract_doc_comments(node: tree_sitter::Node<'_>, source: &[u8]) -> String {
    let mut comments = Vec::new();
    let mut current = node.prev_sibling();
    while let Some(prev) = current {
        let kind = prev.kind();
        if kind == "line_comment" || kind == "block_comment" || kind == "comment" {
            let text = prev.utf8_text(source).unwrap_or("").trim();
            comments.push(text);
            current = prev.prev_sibling();
        } else {
            break;
        }
    }
    comments.reverse();

    let mut doc_lines = Vec::new();
    for comment in comments {
        for line in comment.lines() {
            let trimmed = line.trim();
            if let Some(rest) = trimmed.strip_prefix("///") {
                doc_lines.push(rest.trim().to_string());
            } else if let Some(rest) = trimmed.strip_prefix("//!") {
                doc_lines.push(rest.trim().to_string());
            } else if trimmed.starts_with("/**") || trimmed.ends_with("*/") {
                let clean = trimmed
                    .strip_prefix("/**")
                    .unwrap_or(trimmed)
                    .strip_suffix("*/")
                    .unwrap_or(trimmed)
                    .trim_start_matches('*')
                    .trim();
                if !clean.is_empty() {
                    doc_lines.push(clean.to_string());
                }
            } else if let Some(rest) = trimmed.strip_prefix("//") {
                doc_lines.push(rest.trim().to_string());
            } else if trimmed.starts_with('*') {
                let clean = trimmed.trim_start_matches('*').trim();
                doc_lines.push(clean.to_string());
            } else {
                doc_lines.push(trimmed.to_string());
            }
        }
    }
    doc_lines.join("\n")
}

// ── Emitters ──────────────────────────────────────────────────────────────────

fn emit_function_item<W: Write>(
    node: tree_sitter::Node<'_>,
    source: &[u8],
    ctx: &mut LsifContext<'_, W>,
) -> std::io::Result<()> {
    let name_node = match node.child_by_field_name("name") {
        Some(n) => n,
        None => return Ok(()),
    };
    let name = node_text(name_node, source);
    if name.is_empty() {
        return Ok(());
    }

    let rs_id = ctx.new_result_set()?;
    ctx.insert_symbol(name.to_string(), rs_id.clone());

    let name_range = ts_range_to_lsp(name_node.range());
    let full_range = ts_range_to_lsp(node.range());

    let range_id = ctx.link_range(
        name_range.start,
        name_range.end,
        Some(definition_tag(name, SymbolKind::FUNCTION, full_range, None)),
    )?;
    ctx.builder.bind_next(range_id.clone(), rs_id.clone())?;

    let sig = extract_signature(node, source);
    let docs = extract_doc_comments(node, source);
    let hover_md = if docs.is_empty() {
        format!("```rust\n{}\n```", sig)
    } else {
        format!("```rust\n{}\n```\n\n{}", sig, docs)
    };
    ctx.emit_hover(rs_id.clone(), hover_md)?;
    ctx.emit_definition(rs_id.clone(), range_id)?;

    if is_pub(node, source) {
        let identifier = format!("{}::{name}", ctx.module_path);
        ctx.emit_moniker(
            rs_id,
            "rust",
            identifier,
            MonikerKind::Export,
            UniquenessLevel::Project,
        )?;
    }
    Ok(())
}

fn emit_named_def<W: Write>(
    node: tree_sitter::Node<'_>,
    source: &[u8],
    ctx: &mut LsifContext<'_, W>,
    kind: SymbolKind,
    _kw: &str,
) -> std::io::Result<()> {
    let name_node = match node.child_by_field_name("name") {
        Some(n) => n,
        None => return Ok(()),
    };
    let name = node_text(name_node, source);
    if name.is_empty() {
        return Ok(());
    }

    let rs_id = ctx.new_result_set()?;
    ctx.insert_symbol(name.to_string(), rs_id.clone());

    let name_range = ts_range_to_lsp(name_node.range());
    let full_range = ts_range_to_lsp(node.range());

    let range_id = ctx.link_range(
        name_range.start,
        name_range.end,
        Some(definition_tag(name, kind, full_range, None)),
    )?;
    ctx.builder.bind_next(range_id.clone(), rs_id.clone())?;

    let sig = extract_signature(node, source);
    let docs = extract_doc_comments(node, source);
    let hover_md = if docs.is_empty() {
        format!("```rust\n{}\n```", sig)
    } else {
        format!("```rust\n{}\n```\n\n{}", sig, docs)
    };
    ctx.emit_hover(rs_id.clone(), hover_md)?;
    ctx.emit_definition(rs_id.clone(), range_id)?;

    if is_pub(node, source) {
        let identifier = format!("{}::{name}", ctx.module_path);
        ctx.emit_moniker(
            rs_id,
            "rust",
            identifier,
            MonikerKind::Export,
            UniquenessLevel::Project,
        )?;
    }
    Ok(())
}

fn emit_call_expression<W: Write>(
    node: tree_sitter::Node<'_>,
    source: &[u8],
    ctx: &mut LsifContext<'_, W>,
    use_map: &HashMap<String, String>,
) -> std::io::Result<()> {
    let callee_node = match node.child_by_field_name("function") {
        Some(n) => n,
        None => return Ok(()),
    };

    let (name, name_range) = match callee_name(callee_node, source) {
        Some(pair) => pair,
        None => return Ok(()),
    };

    let lsp_range = ts_range_to_lsp(name_range);
    let range_id = ctx.link_range(
        lsp_range.start,
        lsp_range.end,
        Some(reference_tag(name, SymbolKind::FUNCTION, lsp_range)),
    )?;

    let mut obj_name: Option<String> = None;
    if callee_node.kind() == "field_expression" {
        if let Some(val_node) = callee_node.child_by_field_name("value") {
            obj_name = Some(node_text(val_node, source).to_string());
        }
    }

    if let Some(rs_id) = ctx.lookup_symbol(name) {
        ctx.builder.bind_next(range_id.clone(), rs_id.clone())?;
        ctx.emit_reference(rs_id, range_id)?;
    } else if let Some(import_ident) = use_map.get(name) {
        let rs_id = ctx.new_result_set()?;
        ctx.builder.bind_next(range_id.clone(), rs_id.clone())?;
        ctx.emit_moniker(
            rs_id.clone(),
            "rust",
            import_ident.clone(),
            MonikerKind::Import,
            UniquenessLevel::Project,
        )?;
        ctx.emit_reference(rs_id, range_id)?;
    } else if let Some(ref obj) = obj_name {
        if let Some(import_ident) = use_map.get(obj) {
            let rs_id = ctx.new_result_set()?;
            ctx.builder.bind_next(range_id.clone(), rs_id.clone())?;
            ctx.emit_moniker(
                rs_id.clone(),
                "rust",
                format!("{}::{}", import_ident, name),
                MonikerKind::Import,
                UniquenessLevel::Project,
            )?;
            ctx.emit_reference(rs_id, range_id)?;
        }
    }

    Ok(())
}
