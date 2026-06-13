use lsp_max_lsif::lsif_indexer::{definition_tag, reference_tag, LsifContext};
use lsp_max_lsif::lsif_types::{MonikerKind, UniquenessLevel};
use lsp_types_max::SymbolKind;
use std::collections::HashMap;
use std::io::Write;

pub fn index_rust_source<W: Write>(
    source: &str,
    uri: &str,
    builder: &mut lsp_max_lsif::lsif_builder::LsifBuilder<W>,
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

    // Pre-pass: collect `use` declarations so call-site references can be
    // wired to import monikers without a second traversal of the full tree.
    // Maps local name → moniker identifier (e.g. "Foo" → "other_mod::Foo").
    let use_map = collect_use_map(tree.root_node(), source_bytes);

    let mut ctx = LsifContext::new(builder, doc_id.clone(), module_path);
    walk(tree.root_node(), source_bytes, &mut ctx, &use_map)?;
    ctx.builder.end_document(doc_id)?;
    Ok(())
}

// ── Use declaration pre-pass ──────────────────────────────────────────────────

/// Walk the tree collecting `use` declarations into a map from local name →
/// qualified identifier that matches our export moniker format.
///
/// `use a::Foo;`          → "Foo" → "a::Foo"
/// `use a::{Foo, Bar};`   → "Foo" → "a::Foo", "Bar" → "a::Bar"
/// `use crate::a::Foo;`   → "Foo" → "a::Foo"  (crate:: stripped)
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
        return; // don't recurse into use_declaration children
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        collect_use_map_node(child, source, map);
    }
}

/// Recursively process a use_tree node, building the map.
/// `prefix` is the path accumulated so far (e.g. "a::b").
fn visit_use_tree<'a>(
    node: tree_sitter::Node<'a>,
    source: &'a [u8],
    prefix: &str,
    map: &mut HashMap<String, String>,
) {
    match node.kind() {
        // use a::b::Foo  →  scoped_identifier
        "scoped_identifier" => {
            let path = node_text(node, source);
            let path = strip_crate_prefix(path);
            // last segment is the local name
            let local = path.rsplit("::").next().unwrap_or(path);
            // identifier = last two segments (module::name) to match export format
            let identifier = canonical_identifier(path);
            map.insert(local.to_string(), identifier);
        }
        // use a::b::{Foo, Bar}  →  scoped_use_list
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
        // use {Foo, Bar}  →  use_list
        "use_list" => {
            let mut c = node.walk();
            for child in node.children(&mut c) {
                visit_use_tree(child, source, prefix, map);
            }
        }
        // plain identifier within a list: e.g. `Foo` in `use a::{Foo, Bar}`
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
        // use Foo as Bar  →  use_as_clause
        "use_as_clause" => {
            // local name is the alias, identifier is the original path
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

/// Strip leading `crate::` or `self::` or `super::` from a path.
fn strip_crate_prefix(path: &str) -> &str {
    for prefix in &["crate::", "self::", "super::"] {
        if let Some(rest) = path.strip_prefix(prefix) {
            return rest;
        }
    }
    path
}

/// Return the last two `::` separated components as the canonical identifier,
/// matching the export moniker format `"<module>::<name>"`.
/// Falls back to the full path if there is only one component.
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
        // Skip into use_declaration children handled in the pre-pass
        "use_declaration" => return Ok(()),
        _ => {}
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        walk(child, source, ctx, use_map)?;
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

/// Extract the simple call name from the `function` field of a call_expression.
///
/// tree-sitter gives us the full callee expression, which for `a.method()` is
/// a `field_expression`, and for `Mod::foo()` is a `scoped_identifier`. We
/// want just the terminal identifier so reference ranges are tight and lookup
/// against result_sets / use_map works.
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
        "scoped_identifier" => {
            // `Mod::foo` → name field is `foo`
            callee_node
                .child_by_field_name("name")
                .map(|n| (node_text(n, source), n.range()))
                .filter(|(t, _)| !t.is_empty())
        }
        "field_expression" => {
            // `obj.method` → field child is `method`
            callee_node
                .child_by_field_name("field")
                .map(|n| (node_text(n, source), n.range()))
                .filter(|(t, _)| !t.is_empty())
        }
        _ => None,
    }
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
    ctx.result_sets.insert(name.to_string(), rs_id.clone());

    let name_range = ts_range_to_lsp(name_node.range());
    let full_range = ts_range_to_lsp(node.range());

    let range_id = ctx.link_range(
        name_range.start,
        name_range.end,
        Some(definition_tag(name, SymbolKind::FUNCTION, full_range, None)),
    )?;
    ctx.builder.bind_next(range_id.clone(), rs_id.clone())?;
    ctx.emit_hover(rs_id.clone(), format!("```rust\nfn {name}\n```"))?;
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
    kw: &str,
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
    ctx.result_sets.insert(name.to_string(), rs_id.clone());

    let name_range = ts_range_to_lsp(name_node.range());
    let full_range = ts_range_to_lsp(node.range());

    let range_id = ctx.link_range(
        name_range.start,
        name_range.end,
        Some(definition_tag(name, kind, full_range, None)),
    )?;
    ctx.builder.bind_next(range_id.clone(), rs_id.clone())?;
    ctx.emit_hover(rs_id.clone(), format!("```rust\n{kw} {name}\n```"))?;
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

    // Extract just the terminal identifier, not the whole expression text.
    let (name, name_range) = match callee_name(callee_node, source) {
        Some(pair) => pair,
        None => return Ok(()),
    };

    let lsp_range = ts_range_to_lsp(name_range);
    let range_id = ctx.link_range(lsp_range.start, lsp_range.end, Some(reference_tag(name)))?;

    if let Some(rs_id) = ctx.result_sets.get(name).cloned() {
        // Intra-file: definition is in the same file, already in result_sets.
        ctx.builder.bind_next(range_id, rs_id)?;
    } else if let Some(import_ident) = use_map.get(name) {
        // Cross-file: name was imported via `use`. Create a resultSet + import
        // moniker so the linker can wire an attach edge to the export moniker.
        let rs_id = ctx.new_result_set()?;
        ctx.builder.bind_next(range_id, rs_id.clone())?;
        ctx.emit_moniker(
            rs_id,
            "rust",
            import_ident.clone(),
            MonikerKind::Import,
            UniquenessLevel::Project,
        )?;
    }
    // Unknown call site: emit the reference range but leave it unlinked.
    // This is valid LSIF — not every reference can be resolved without a
    // type checker.

    Ok(())
}
