use crate::lsif_indexer::{definition_tag, reference_tag, LsifContext};
use crate::lsif_types::{MonikerKind, UniquenessLevel};
use lsp_types_max::SymbolKind;
use std::collections::HashMap;
use std::io::Write;

pub fn index_typescript_source<W: Write>(
    source: &str,
    uri: &str,
    package_name: Option<&str>,
    builder: &mut crate::lsif_builder::LsifBuilder<W>,
) -> std::io::Result<()> {
    let mut parser = tree_sitter::Parser::new();
    parser
        .set_language(&tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into())
        .expect("tree-sitter-typescript grammar load failed");

    let tree = match parser.parse(source, None) {
        Some(t) => t,
        None => return Ok(()),
    };

    let doc_id = builder.emit_document(uri, "typescript")?;
    let source_bytes = source.as_bytes();

    let module_stem = uri
        .rsplit('/')
        .next()
        .and_then(|f| f.strip_suffix(".tsx").or_else(|| f.strip_suffix(".ts")))
        .unwrap_or("unknown")
        .to_string();

    // Pre-pass: collect import statements → local name → (scheme, identifier).
    let import_map = collect_import_map(tree.root_node(), source_bytes, uri);

    let mut ctx = LsifContext::new(builder, doc_id.clone(), module_stem.clone());
    ctx.package_name = package_name.map(str::to_string);

    walk(
        tree.root_node(),
        source_bytes,
        &mut ctx,
        &module_stem,
        &import_map,
    )?;
    ctx.builder.end_document(doc_id)?;
    Ok(())
}

// ── Import pre-pass ───────────────────────────────────────────────────────────

fn collect_import_map<'a>(
    root: tree_sitter::Node<'a>,
    source: &'a [u8],
    _importer_uri: &str,
) -> HashMap<String, (String, String)> {
    let mut map = HashMap::new();
    let mut cursor = root.walk();
    for child in root.children(&mut cursor) {
        if child.kind() == "import_statement" {
            process_import_statement(child, source, &mut map);
        }
    }
    map
}

fn process_import_statement<'a>(
    node: tree_sitter::Node<'a>,
    source: &'a [u8],
    map: &mut HashMap<String, (String, String)>,
) {
    let source_node = node.child_by_field_name("source");
    let module_source = source_node
        .map(|n| {
            node_text(n, source)
                .trim_matches('"')
                .trim_matches('\'')
                .to_string()
        })
        .unwrap_or_default();
    if module_source.is_empty() {
        return;
    }

    let (scheme, mod_prefix) = if module_source.starts_with('.') {
        let stem = module_source
            .rsplit('/')
            .next()
            .unwrap_or(&module_source)
            .to_string();
        ("typescript".to_string(), stem)
    } else {
        ("npm".to_string(), module_source.clone())
    };

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "import_clause" {
            let mut ic = child.walk();
            for sub in child.children(&mut ic) {
                if sub.kind() == "named_imports" {
                    let mut ni = sub.walk();
                    for spec in sub.children(&mut ni) {
                        if spec.kind() == "import_specifier" {
                            let export_name_node = spec.child_by_field_name("name").or_else(|| {
                                let mut c = spec.walk();
                                let cs: Vec<_> = spec.children(&mut c).collect();
                                cs.into_iter().find(|n| n.kind() == "identifier")
                            });
                            let local_name_node =
                                spec.child_by_field_name("alias").or(export_name_node);

                            if let (Some(exp_node), Some(local_node)) =
                                (export_name_node, local_name_node)
                            {
                                let export_name = node_text(exp_node, source);
                                let local_name = node_text(local_node, source);
                                if !export_name.is_empty() && !local_name.is_empty() {
                                    let identifier = format!("{mod_prefix}::{export_name}");
                                    map.insert(
                                        local_name.to_string(),
                                        (scheme.clone(), identifier),
                                    );
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

// ── Main walk ─────────────────────────────────────────────────────────────────

fn walk<W: Write>(
    node: tree_sitter::Node<'_>,
    source: &[u8],
    ctx: &mut LsifContext<'_, W>,
    module_stem: &str,
    import_map: &HashMap<String, (String, String)>,
) -> std::io::Result<()> {
    match node.kind() {
        "function_declaration" => {
            emit_named_def(
                node,
                source,
                ctx,
                SymbolKind::FUNCTION,
                "function",
                module_stem,
                true,
            )?;
        }
        "class_declaration" => {
            emit_named_def(
                node,
                source,
                ctx,
                SymbolKind::CLASS,
                "class",
                module_stem,
                true,
            )?;
        }
        "interface_declaration" => {
            emit_named_def(
                node,
                source,
                ctx,
                SymbolKind::INTERFACE,
                "interface",
                module_stem,
                true,
            )?;
        }
        "type_alias_declaration" => {
            emit_named_def(
                node,
                source,
                ctx,
                SymbolKind::TYPE_PARAMETER,
                "type",
                module_stem,
                true,
            )?;
        }
        "lexical_declaration" => {
            emit_lexical_declaration(node, source, ctx, module_stem)?;
        }
        "call_expression" => {
            emit_call_expression(node, source, ctx, import_map)?;
        }
        "import_statement" => {
            emit_import_monikers(node, source, ctx, import_map)?;
            return Ok(());
        }
        _ => {}
    }

    let has_scope = match node.kind() {
        "statement_block"
        | "function_declaration"
        | "class_declaration"
        | "interface_declaration" => {
            ctx.push_scope();
            true
        }
        _ => false,
    };

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        walk(child, source, ctx, module_stem, import_map)?;
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

fn ts_range_to_lsp(range: tree_sitter::Range) -> lsp_types_max::Range {
    lsp_types_max::Range {
        start: lsp_types_max::Position {
            line: range.start_point.row as u32,
            character: range.start_point.column as u32,
        },
        end: lsp_types_max::Position {
            line: range.end_point.row as u32,
            character: range.end_point.column as u32,
        },
    }
}

fn is_exported(node: tree_sitter::Node<'_>) -> bool {
    node.parent()
        .map(|p| p.kind() == "export_statement")
        .unwrap_or(false)
}

/// A leading doc comment attaches before the outermost statement, which for
/// an exported declaration is the wrapping `export_statement`, not the inner
/// `function_declaration`/`class_declaration`/etc. Walk up to that wrapper
/// first so `extract_doc_comments`'s `prev_sibling()` walk actually finds it.
fn doc_comment_anchor(node: tree_sitter::Node<'_>) -> tree_sitter::Node<'_> {
    match node.parent() {
        Some(p) if p.kind() == "export_statement" => p,
        _ => node,
    }
}

fn emit_symbol_moniker<W: Write>(
    ctx: &mut LsifContext<'_, W>,
    rs_id: crate::lsif_types::Id,
    name: &str,
    module_stem: &str,
    exported: bool,
) -> std::io::Result<()> {
    if !exported {
        return Ok(());
    }
    let identifier = format!("{module_stem}::{name}");
    ctx.emit_moniker(
        rs_id.clone(),
        "typescript",
        identifier.clone(),
        MonikerKind::Export,
        UniquenessLevel::Project,
    )?;
    if let Some(pkg) = ctx.package_name.clone() {
        let npm_id = format!("{pkg}::{name}");
        ctx.emit_moniker(
            rs_id,
            "npm",
            npm_id,
            MonikerKind::Export,
            UniquenessLevel::Scheme,
        )?;
    }
    Ok(())
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
            if trimmed.starts_with("/**") || trimmed.ends_with("*/") {
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
            } else if let Some(rest) = trimmed.strip_prefix("///") {
                doc_lines.push(rest.trim().to_string());
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

fn emit_named_def<W: Write>(
    node: tree_sitter::Node<'_>,
    source: &[u8],
    ctx: &mut LsifContext<'_, W>,
    kind: SymbolKind,
    _kw: &str,
    module_stem: &str,
    can_be_exported: bool,
) -> std::io::Result<()> {
    let name_node = match node.child_by_field_name("name") {
        Some(n) => n,
        None => return Ok(()),
    };
    let name = node_text(name_node, source);
    if name.is_empty() {
        return Ok(());
    }

    let exported = can_be_exported && is_exported(node);

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
    let docs = extract_doc_comments(doc_comment_anchor(node), source);
    let hover_md = if docs.is_empty() {
        format!("```typescript\n{}\n```", sig)
    } else {
        format!("```typescript\n{}\n```\n\n{}", sig, docs)
    };
    ctx.emit_hover(rs_id.clone(), hover_md)?;
    ctx.emit_definition(rs_id.clone(), range_id)?;
    emit_symbol_moniker(ctx, rs_id, name, module_stem, exported)?;
    Ok(())
}

fn emit_lexical_declaration<W: Write>(
    node: tree_sitter::Node<'_>,
    source: &[u8],
    ctx: &mut LsifContext<'_, W>,
    module_stem: &str,
) -> std::io::Result<()> {
    let exported = is_exported(node);
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "variable_declarator" {
            if let Some(name_node) = child.child_by_field_name("name") {
                let name = node_text(name_node, source);
                if name.is_empty() {
                    continue;
                }
                let rs_id = ctx.new_result_set()?;
                ctx.insert_symbol(name.to_string(), rs_id.clone());

                let name_range = ts_range_to_lsp(name_node.range());
                let full_range = ts_range_to_lsp(node.range());

                let range_id = ctx.link_range(
                    name_range.start,
                    name_range.end,
                    Some(definition_tag(name, SymbolKind::VARIABLE, full_range, None)),
                )?;
                ctx.builder.bind_next(range_id.clone(), rs_id.clone())?;

                let sig = extract_signature(child, source);
                let docs = extract_doc_comments(doc_comment_anchor(node), source);
                let hover_md = if docs.is_empty() {
                    format!("```typescript\n{}\n```", sig)
                } else {
                    format!("```typescript\n{}\n```\n\n{}", sig, docs)
                };
                ctx.emit_hover(rs_id.clone(), hover_md)?;
                ctx.emit_definition(rs_id.clone(), range_id)?;
                emit_symbol_moniker(ctx, rs_id, name, module_stem, exported)?;
            }
        }
    }
    Ok(())
}

fn emit_call_expression<W: Write>(
    node: tree_sitter::Node<'_>,
    source: &[u8],
    ctx: &mut LsifContext<'_, W>,
    import_map: &HashMap<String, (String, String)>,
) -> std::io::Result<()> {
    let callee_node = match node.child_by_field_name("function") {
        Some(n) => n,
        None => return Ok(()),
    };

    let (name, name_range) = match extract_callee_name(callee_node, source) {
        Some(p) => p,
        None => return Ok(()),
    };

    let lsp_range = ts_range_to_lsp(name_range);
    let range_id = ctx.link_range(
        lsp_range.start,
        lsp_range.end,
        Some(reference_tag(name, SymbolKind::FUNCTION, lsp_range)),
    )?;

    let mut obj_name: Option<String> = None;
    if callee_node.kind() == "member_expression" {
        if let Some(obj_node) = callee_node.child_by_field_name("object") {
            obj_name = Some(node_text(obj_node, source).to_string());
        }
    }

    if let Some(rs_id) = ctx.lookup_symbol(name) {
        ctx.builder.bind_next(range_id.clone(), rs_id.clone())?;
        ctx.emit_reference(rs_id, range_id)?;
    } else if let Some((scheme, identifier)) = import_map.get(name) {
        let rs_id = ctx.new_result_set()?;
        ctx.builder.bind_next(range_id.clone(), rs_id.clone())?;
        ctx.emit_moniker(
            rs_id.clone(),
            scheme.clone(),
            identifier.clone(),
            MonikerKind::Import,
            UniquenessLevel::Project,
        )?;
        ctx.emit_reference(rs_id, range_id)?;
    } else if let Some(ref obj) = obj_name {
        if let Some((scheme, identifier)) = import_map.get(obj) {
            let rs_id = ctx.new_result_set()?;
            ctx.builder.bind_next(range_id.clone(), rs_id.clone())?;
            ctx.emit_moniker(
                rs_id.clone(),
                scheme.clone(),
                format!("{}::{}", identifier, name),
                MonikerKind::Import,
                UniquenessLevel::Project,
            )?;
            ctx.emit_reference(rs_id, range_id)?;
        }
    }

    Ok(())
}

fn emit_import_monikers<W: Write>(
    node: tree_sitter::Node<'_>,
    source: &[u8],
    ctx: &mut LsifContext<'_, W>,
    import_map: &HashMap<String, (String, String)>,
) -> std::io::Result<()> {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "import_clause" {
            let mut ic = child.walk();
            for sub in child.children(&mut ic) {
                if sub.kind() == "named_imports" {
                    let mut ni = sub.walk();
                    for spec in sub.children(&mut ni) {
                        if spec.kind() == "import_specifier" {
                            let name_node = spec
                                .child_by_field_name("alias")
                                .or_else(|| spec.child_by_field_name("name"))
                                .or_else(|| {
                                    let mut c = spec.walk();
                                    let cs: Vec<_> = spec.children(&mut c).collect();
                                    cs.into_iter().find(|n| n.kind() == "identifier")
                                });
                            if let Some(nn) = name_node {
                                let local = node_text(nn, source);
                                if let (Some(rs_id), Some((scheme, ident))) =
                                    (ctx.lookup_symbol(local), import_map.get(local))
                                {
                                    ctx.emit_moniker(
                                        rs_id,
                                        scheme.clone(),
                                        ident.clone(),
                                        MonikerKind::Import,
                                        UniquenessLevel::Project,
                                    )?;
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    Ok(())
}

fn extract_callee_name<'a>(
    node: tree_sitter::Node<'_>,
    source: &'a [u8],
) -> Option<(&'a str, tree_sitter::Range)> {
    match node.kind() {
        "identifier" => {
            let t = node_text(node, source);
            if t.is_empty() {
                None
            } else {
                Some((t, node.range()))
            }
        }
        "member_expression" => node
            .child_by_field_name("property")
            .map(|n| (node_text(n, source), n.range()))
            .filter(|(t, _)| !t.is_empty()),
        _ => None,
    }
}
