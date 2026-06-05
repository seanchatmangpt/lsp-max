use std::collections::HashMap;

use syn::spanned::Spanned;
use tower_lsp_max::lsp_types::*;

use super::completions::{CAPABILITY_FIELDS, METHODS};
use crate::Backend;

// Diagnostic source tag shown in the IDE
const SOURCE: &str = "tower-lsp-max-playground";

// Diagnostic codes
const CAPABILITY_WITHOUT_METHOD: &str = "TLM001";
const METHOD_WITHOUT_CAPABILITY: &str = "TLM002";
const MISSING_MANDATORY_METHOD: &str = "TLM003";
const MISSING_INITIALIZED_OVERRIDE: &str = "TLM004";
const TYPO_IN_METHOD_NAME: &str = "TLM005";
const INVALID_RPC_NAME: &str = "TLM006";

struct OverriddenMethod {
    name: String,
    span: proc_macro2::Span,
}

#[allow(dead_code)]
struct InvalidRpcName {
    name: String,
    span: proc_macro2::Span,
    message: String,
}

struct DeclaredCapability {
    name: String,
    span: proc_macro2::Span,
}

#[derive(Default)]
struct ImplAnalysis {
    overridden_methods: Vec<OverriddenMethod>,
    declared_capabilities: Vec<DeclaredCapability>,
    invalid_rpc_names: Vec<InvalidRpcName>,
    has_initialize: bool,
    has_shutdown: bool,
    has_initialized: bool,
    impl_trait_span: Option<proc_macro2::Span>,
    impl_close_brace_span: Option<proc_macro2::Span>,
    server_capabilities_open_brace_span: Option<proc_macro2::Span>,
}


/// Public entry point taking raw source text. Matches the spec signature so tests can call it directly.
pub fn get_diagnostics(text: &str, _uri: &Url) -> Vec<Diagnostic> {
    let ast = match syn::parse_file(text) {
        Ok(ast) => ast,
        Err(_) => return vec![],
    };
    let analysis = analyze_impl_block(&ast);
    build_diagnostics(analysis)
}

/// Diagnostic entry point called from `lib.rs`.
///
/// Parses the document with `syn`, locates the `impl LanguageServer` block,
/// and checks the capability-method contract.
pub async fn compute(backend: &Backend, uri: &Url) -> Vec<Diagnostic> {
    let doc = backend.docs.get(uri);
    let doc = match doc {
        Some(d) if d.language_id == "rust" => d,
        _ => return vec![],
    };
    let source = doc.text.to_string();
    drop(doc);

    // syn::parse_file fails on incomplete files (common during editing).
    // Return empty diagnostics rather than false positives.
    let ast = match syn::parse_file(&source) {
        Ok(ast) => ast,
        Err(_) => return vec![],
    };

    let analysis = analyze_impl_block(&ast);
    build_diagnostics(analysis)
}

fn analyze_impl_block(ast: &syn::File) -> ImplAnalysis {
    use syn::visit::Visit;

    struct Visitor {
        analysis: ImplAnalysis,
        in_ls_impl: bool,
        in_initialize_fn: bool,
    }

    impl<'ast> Visit<'ast> for Visitor {
        fn visit_item_trait(&mut self, node: &'ast syn::ItemTrait) {
            let has_rpc_attr = node.attrs.iter().any(|attr| attr.meta.path().is_ident("rpc"));
            if has_rpc_attr {
                for item in &node.items {
                    if let syn::TraitItem::Fn(method) = item {
                        if let Some(attr) = method.attrs.iter().find(|attr| attr.meta.path().is_ident("rpc")) {
                            let mut rpc_name = None;
                            let mut rpc_name_span = None;
                            let _ = attr.parse_nested_meta(|meta| {
                                if meta.path.is_ident("name") {
                                    let lit_str: syn::LitStr = meta.value()?.parse()?;
                                    rpc_name = Some(lit_str.value());
                                    rpc_name_span = Some(lit_str.span());
                                }
                                Ok(())
                            });
                            if let (Some(name), Some(span)) = (rpc_name, rpc_name_span) {
                                let is_valid = METHODS.iter().any(|m| m.lsp_method == name);
                                if !is_valid {
                                    let mut min_dist = usize::MAX;
                                    let mut closest_match: Option<&'static str> = None;
                                    for entry in METHODS {
                                        let dist = levenshtein_distance(&name, entry.lsp_method);
                                        if dist < min_dist {
                                            min_dist = dist;
                                            closest_match = Some(entry.lsp_method);
                                        }
                                    }
                                    let msg = if let Some(closest) = closest_match {
                                        if min_dist <= 4 {
                                            format!(
                                                "`{}` is not a valid RPC method. Did you mean '{}'?",
                                                name, closest
                                            )
                                        } else {
                                            format!("`{}` is not a valid RPC method.", name)
                                        }
                                    } else {
                                        format!("`{}` is not a valid RPC method.", name)
                                    };
                                    self.analysis.invalid_rpc_names.push(InvalidRpcName {
                                        name,
                                        span,
                                        message: msg,
                                    });
                                }
                            }
                        }
                    }
                }
            }
            syn::visit::visit_item_trait(self, node);
        }

        fn visit_item_impl(&mut self, node: &'ast syn::ItemImpl) {
            let is_ls_impl = node
                .trait_
                .as_ref()
                .and_then(|(_, path, _)| path.segments.last())
                .map(|s| s.ident == "LanguageServer")
                .unwrap_or(false);

            if is_ls_impl {
                self.in_ls_impl = true;
                if let Some((_, path, _)) = &node.trait_ {
                    self.analysis.impl_trait_span = Some(path.span());
                }
                self.analysis.impl_close_brace_span = Some(node.brace_token.span.close());
                syn::visit::visit_item_impl(self, node);
                self.in_ls_impl = false;
            }
        }

        fn visit_impl_item_fn(&mut self, node: &'ast syn::ImplItemFn) {
            if !self.in_ls_impl {
                return;
            }
            let name = node.sig.ident.to_string();
            self.analysis.overridden_methods.push(OverriddenMethod {
                name: name.clone(),
                span: node.sig.ident.span(),
            });
            match name.as_str() {
                "initialize" => self.analysis.has_initialize = true,
                "shutdown" => self.analysis.has_shutdown = true,
                "initialized" => self.analysis.has_initialized = true,
                _ => {}
            }
            if name == "initialize" {
                self.in_initialize_fn = true;
                syn::visit::visit_impl_item_fn(self, node);
                self.in_initialize_fn = false;
            }
        }

        fn visit_expr_struct(&mut self, node: &'ast syn::ExprStruct) {
            if !self.in_initialize_fn {
                return;
            }
            let is_sc = node
                .path
                .segments
                .last()
                .map(|s| s.ident == "ServerCapabilities")
                .unwrap_or(false);
            if is_sc {
                self.analysis.server_capabilities_open_brace_span = Some(node.brace_token.span.open());
                for field in &node.fields {
                    if let syn::Member::Named(ident) = &field.member {
                        let field_name = ident.to_string();
                        if !is_none_expr(&field.expr) {
                            self.analysis.declared_capabilities.push(DeclaredCapability {
                                name: field_name,
                                span: ident.span(),
                            });
                        }
                    }
                }
            }
        }
    }

    let mut v = Visitor {
        analysis: ImplAnalysis::default(),
        in_ls_impl: false,
        in_initialize_fn: false,
    };
    syn::visit::visit_file(&mut v, ast);
    v.analysis
}

fn is_none_expr(expr: &syn::Expr) -> bool {
    match expr {
        syn::Expr::Path(p) => p
            .path
            .segments
            .last()
            .map(|s| s.ident == "None")
            .unwrap_or(false),
        _ => false,
    }
}

fn build_diagnostics(analysis: ImplAnalysis) -> Vec<Diagnostic> {
    let mut diags = Vec::new();

    let trait_range = analysis
        .impl_trait_span
        .map(to_lsp_range)
        .unwrap_or_default();

    // Check for typos in method names
    for method in &analysis.overridden_methods {
        let is_valid = METHODS.iter().any(|m| m.fn_name == method.name.as_str());
        if !is_valid {
            let mut min_dist = usize::MAX;
            let mut closest_match: Option<&'static str> = None;
            for entry in METHODS {
                let dist = levenshtein_distance(&method.name, entry.fn_name);
                if dist < min_dist {
                    min_dist = dist;
                    closest_match = Some(entry.fn_name);
                }
            }

            let msg = if let Some(closest) = closest_match {
                if min_dist <= 4 {
                    format!(
                        "`{}` is not a valid method of the `LanguageServer` trait. Did you mean `{}`?",
                        method.name, closest
                    )
                } else {
                    format!("`{}` is not a valid method of the `LanguageServer` trait.", method.name)
                }
            } else {
                format!("`{}` is not a valid method of the `LanguageServer` trait.", method.name)
            };

            diags.push(make_diag(
                to_lsp_range(method.span),
                DiagnosticSeverity::ERROR,
                TYPO_IN_METHOD_NAME,
                &msg,
            ));
        }
    }

    // TLM003: missing mandatory initialize
    if !analysis.has_initialize {
        diags.push(make_diag(
            trait_range,
            DiagnosticSeverity::ERROR,
            MISSING_MANDATORY_METHOD,
            "`initialize` is not overridden. It is mandatory — the default returns \
             `Error::method_not_found()`. Add:\n```rust\nasync fn initialize(&self, \
             params: InitializeParams) -> Result<InitializeResult> {\n    \
             Ok(InitializeResult { capabilities: ServerCapabilities::default(), \
             ..Default::default() })\n}\n```",
        ));
    }

    // TLM003: missing mandatory shutdown
    if !analysis.has_shutdown {
        diags.push(make_diag(
            trait_range,
            DiagnosticSeverity::ERROR,
            MISSING_MANDATORY_METHOD,
            "`shutdown` is not overridden. It is mandatory. Add:\n```rust\n\
             async fn shutdown(&self) -> Result<()> { Ok(()) }\n```",
        ));
    }

    // TLM004: initialized not overridden (hint)
    if !analysis.has_initialized {
        diags.push(make_diag(
            trait_range,
            DiagnosticSeverity::HINT,
            MISSING_INITIALIZED_OVERRIDE,
            "`initialized` is not overridden. The default is a no-op. Most servers override \
             it to log a ready message or register dynamic capabilities.",
        ));
    }

    // TLM001: capability declared but handler not overridden
    for cap in &analysis.declared_capabilities {
        let required_methods = methods_for_capability(&cap.name);
        for method in &required_methods {
            if !analysis.overridden_methods.iter().any(|m| m.name == *method) {
                let lsp_method = METHODS
                    .iter()
                    .find(|m| m.fn_name == *method)
                    .map(|m| m.lsp_method)
                    .unwrap_or("?");
                diags.push(make_diag(
                    to_lsp_range(cap.span),
                    DiagnosticSeverity::WARNING,
                    CAPABILITY_WITHOUT_METHOD,
                    &format!(
                        "`ServerCapabilities::{}` is declared but `{}` is not overridden. \
                         The client will send `{}` requests and receive `method not found` errors.",
                        cap.name, method, lsp_method
                    ),
                ));
            }
        }
    }

    // TLM002: handler overridden but capability not declared
    for method in &analysis.overridden_methods {
        if let Some(entry) = METHODS.iter().find(|m| m.fn_name == method.name.as_str()) {
            if let Some(cap_field) = entry.capability_field {
                if !analysis.declared_capabilities.iter().any(|c| c.name == cap_field) {
                    diags.push(make_diag(
                        to_lsp_range(method.span),
                        DiagnosticSeverity::WARNING,
                        METHOD_WITHOUT_CAPABILITY,
                        &format!(
                            "`{}` is overridden but `ServerCapabilities::{}` is \
                             not set. The client will never send `{}` — the override is dead code.",
                            method.name, cap_field, entry.lsp_method
                        ),
                    ));
                }
            }
        }
    }

    // TLM006: invalid RPC name
    for invalid in &analysis.invalid_rpc_names {
        diags.push(make_diag(
            to_lsp_range(invalid.span),
            DiagnosticSeverity::ERROR,
            INVALID_RPC_NAME,
            &invalid.message,
        ));
    }

    diags
}

fn methods_for_capability(field: &str) -> Vec<&'static str> {
    METHODS
        .iter()
        .filter(|m| m.capability_field == Some(field))
        .map(|m| m.fn_name)
        .collect()
}

fn make_diag(
    range: Range,
    severity: DiagnosticSeverity,
    code: &str,
    message: &str,
) -> Diagnostic {
    Diagnostic {
        range,
        severity: Some(severity),
        code: Some(NumberOrString::String(code.to_string())),
        source: Some(SOURCE.to_string()),
        message: message.to_string(),
        ..Default::default()
    }
}

// ---------------------------------------------------------------------------
// Code Actions
// ---------------------------------------------------------------------------

/// Code action entry point called from `lib.rs`.
pub async fn code_actions(
    backend: &Backend,
    uri: &Url,
    diagnostics: &[Diagnostic],
) -> Option<CodeActionResponse> {
    let doc = backend.docs.get(uri);
    let doc = match doc {
        Some(d) if d.language_id == "rust" => d,
        _ => return None,
    };
    let source = doc.text.to_string();
    let rope_text = doc.text.clone();
    drop(doc);

    // Try to parse AST
    let ast = syn::parse_file(&source).ok();
    let analysis = ast.as_ref().map(analyze_impl_block).unwrap_or_default();

    let mut actions: Vec<CodeActionOrCommand> = Vec::new();

    for diag in diagnostics {
        match diag.code.as_ref() {
            Some(NumberOrString::String(code)) if code == TYPO_IN_METHOD_NAME => {
                // Get the misspelled name from the document using the diagnostic range
                let start_char = rope_offset(&rope_text, diag.range.start);
                let end_char = rope_offset(&rope_text, diag.range.end);
                if start_char <= end_char && end_char <= rope_text.len_chars() {
                    let misspelled_name = rope_text.slice(start_char..end_char).to_string();
                    // Find closest match by Levenshtein distance
                    let mut min_dist = usize::MAX;
                    let mut closest_match: Option<&super::completions::MethodEntry> = None;
                    for entry in METHODS {
                        let dist = levenshtein_distance(&misspelled_name, entry.fn_name);
                        if dist < min_dist {
                            min_dist = dist;
                            closest_match = Some(entry);
                        }
                    }
                    if let Some(closest) = closest_match {
                        if min_dist <= 4 {
                            actions.push(CodeActionOrCommand::CodeAction(CodeAction {
                                title: format!("Correct typo to `{}`", closest.fn_name),
                                kind: Some(CodeActionKind::QUICKFIX),
                                diagnostics: Some(vec![diag.clone()]),
                                edit: Some(WorkspaceEdit {
                                    changes: Some(HashMap::from([(
                                        uri.clone(),
                                        vec![TextEdit {
                                            range: diag.range,
                                            new_text: closest.fn_name.to_string(),
                                        }],
                                    )])),
                                    ..Default::default()
                                }),
                                is_preferred: Some(true),
                                ..Default::default()
                                }));
                        }
                    }
                }
            }
            Some(NumberOrString::String(code)) if code == INVALID_RPC_NAME => {
                let start_char = rope_offset(&rope_text, diag.range.start);
                let end_char = rope_offset(&rope_text, diag.range.end);
                if start_char <= end_char && end_char <= rope_text.len_chars() {
                    let misspelled_name = rope_text.slice(start_char..end_char).to_string();
                    let clean_name = misspelled_name.trim_matches('"');
                    let mut min_dist = usize::MAX;
                    let mut closest_match: Option<&super::completions::MethodEntry> = None;
                    for entry in METHODS {
                        let dist = levenshtein_distance(clean_name, entry.lsp_method);
                        if dist < min_dist {
                            min_dist = dist;
                            closest_match = Some(entry);
                        }
                    }
                    if let Some(closest) = closest_match {
                        if min_dist <= 4 {
                            let new_text = if misspelled_name.starts_with('"') && misspelled_name.ends_with('"') {
                                format!("\"{}\"", closest.lsp_method)
                            } else {
                                closest.lsp_method.to_string()
                            };
                            actions.push(CodeActionOrCommand::CodeAction(CodeAction {
                                title: format!("Correct RPC name to `{}`", closest.lsp_method),
                                kind: Some(CodeActionKind::QUICKFIX),
                                diagnostics: Some(vec![diag.clone()]),
                                edit: Some(WorkspaceEdit {
                                    changes: Some(HashMap::from([(
                                        uri.clone(),
                                        vec![TextEdit {
                                            range: diag.range,
                                            new_text,
                                        }],
                                    )])),
                                    ..Default::default()
                                }),
                                is_preferred: Some(true),
                                ..Default::default()
                            }));
                        }
                    }
                }
            }
            Some(NumberOrString::String(code)) if code == CAPABILITY_WITHOUT_METHOD => {
                if let Some(method_name) = extract_method_from_missing_override_msg(&diag.message) {
                    if let Some(entry) = METHODS.iter().find(|m| m.fn_name == method_name) {
                        let stub = generate_method_stub(entry);
                        let insert_pos = analysis
                            .impl_close_brace_span
                            .map(|s| to_lsp_position(s.start()))
                            .unwrap_or_else(|| find_fallback_insert_pos(&rope_text));
                        actions.push(CodeActionOrCommand::CodeAction(CodeAction {
                            title: format!("Generate stub for `{}`", entry.fn_name),
                            kind: Some(CodeActionKind::QUICKFIX),
                            diagnostics: Some(vec![diag.clone()]),
                            edit: Some(workspace_edit_insert(uri, insert_pos, &stub)),
                            is_preferred: Some(true),
                            ..Default::default()
                        }));
                    }
                }
            }
            Some(NumberOrString::String(code)) if code == METHOD_WITHOUT_CAPABILITY => {
                if let Some(cap_field) = extract_capability_from_dead_code_msg(&diag.message) {
                    if let Some((name, ty, _)) =
                        CAPABILITY_FIELDS.iter().find(|(n, _, _)| *n == cap_field)
                    {
                        let mut action = CodeAction {
                            title: format!("Add `{name}: Some(...)` to ServerCapabilities"),
                            kind: Some(CodeActionKind::QUICKFIX),
                            diagnostics: Some(vec![diag.clone()]),
                            ..Default::default()
                        };

                        if let Some(open_brace) = analysis.server_capabilities_open_brace_span {
                            let insert_pos = to_lsp_position(open_brace.end());
                            let default_val = default_val_for_cap(name);
                            let new_text = format!("\n            {name}: {default_val},");
                            action.edit = Some(workspace_edit_insert(uri, insert_pos, &new_text));
                            action.is_preferred = Some(true);
                        } else {
                            action.command = Some(Command {
                                title: format!("Add capability declaration for `{name}`"),
                                command: "tower-lsp-max-playground.addCapability".to_string(),
                                arguments: Some(vec![
                                    serde_json::to_value(uri.as_str()).unwrap(),
                                    serde_json::Value::String(name.to_string()),
                                    serde_json::Value::String(ty.to_string()),
                                ]),
                            });
                        }
                        actions.push(CodeActionOrCommand::CodeAction(action));
                    }
                }
            }
            Some(NumberOrString::String(code)) if code == MISSING_MANDATORY_METHOD => {
                let insert_pos = analysis
                    .impl_close_brace_span
                    .map(|s| to_lsp_position(s.start()))
                    .unwrap_or_else(|| find_fallback_insert_pos(&rope_text));
                
                // Add the specific missing method action
                if diag.message.contains("initialize") {
                    if let Some(entry) = METHODS.iter().find(|m| m.fn_name == "initialize") {
                        let stub = generate_method_stub(entry);
                        actions.push(CodeActionOrCommand::CodeAction(CodeAction {
                            title: "Generate stub for `initialize`".to_string(),
                            kind: Some(CodeActionKind::QUICKFIX),
                            diagnostics: Some(vec![diag.clone()]),
                            edit: Some(workspace_edit_insert(uri, insert_pos, &stub)),
                            is_preferred: Some(true),
                            ..Default::default()
                        }));
                    }
                } else if diag.message.contains("shutdown") {
                    if let Some(entry) = METHODS.iter().find(|m| m.fn_name == "shutdown") {
                        let stub = generate_method_stub(entry);
                        actions.push(CodeActionOrCommand::CodeAction(CodeAction {
                            title: "Generate stub for `shutdown`".to_string(),
                            kind: Some(CodeActionKind::QUICKFIX),
                            diagnostics: Some(vec![diag.clone()]),
                            edit: Some(workspace_edit_insert(uri, insert_pos, &stub)),
                            is_preferred: Some(true),
                            ..Default::default()
                        }));
                    }
                }

                // Also keep the scaffold option
                actions.push(CodeActionOrCommand::CodeAction(CodeAction {
                    title: "Generate minimal server scaffold (initialize + initialized + shutdown)"
                        .to_string(),
                    kind: Some(CodeActionKind::SOURCE),
                    diagnostics: Some(vec![diag.clone()]),
                    edit: Some(workspace_edit_insert(uri, insert_pos, MINIMAL_SCAFFOLD)),
                    is_preferred: Some(false),
                    ..Default::default()
                }));
            }
            _ => {}
        }
    }

    if actions.is_empty() {
        None
    } else {
        Some(actions)
    }
}

fn generate_method_stub(entry: &super::completions::MethodEntry) -> String {
    if entry.fn_name == "initialize" {
        r#"
    async fn initialize(&self, _params: InitializeParams) -> Result<InitializeResult> {
        Ok(InitializeResult {
            capabilities: ServerCapabilities::default(),
            ..Default::default()
        })
    }
"#
        .to_string()
    } else if entry.fn_name == "shutdown" {
        r#"
    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }
"#
        .to_string()
    } else {
        let params = if entry.params_type == "()" {
            "_: ()".to_string()
        } else {
            format!("_params: {}", entry.params_type)
        };
        let body = if entry.return_type.starts_with("Result<") {
            format!("Err(tower_lsp::jsonrpc::Error::method_not_found()) // TODO: implement {}", entry.fn_name)
        } else {
            format!("// TODO: implement {}", entry.fn_name)
        };
        format!(
            "\n    async fn {}(&self, {}) -> {} {{\n        {}\n    }}\n",
            entry.fn_name, params, entry.return_type, body
        )
    }
}

fn workspace_edit_insert(uri: &Url, pos: Position, text: &str) -> WorkspaceEdit {
    let edit = TextEdit {
        range: Range {
            start: pos,
            end: pos,
        },
        new_text: text.to_string(),
    };
    let mut changes = HashMap::new();
    changes.insert(uri.clone(), vec![edit]);
    WorkspaceEdit {
        changes: Some(changes),
        ..Default::default()
    }
}

fn extract_method_from_missing_override_msg(msg: &str) -> Option<&'static str> {
    // "`ServerCapabilities::X` is declared but `Y` is not overridden..."
    let start = msg.find("but `")? + 5;
    let end = msg[start..].find('`')? + start;
    let name = &msg[start..end];
    METHODS.iter().find(|m| m.fn_name == name).map(|m| m.fn_name)
}

fn extract_capability_from_dead_code_msg(msg: &str) -> Option<&'static str> {
    // "`Y` is overridden but `ServerCapabilities::X` is not set..."
    let marker = "ServerCapabilities::";
    let start = msg.find(marker)? + marker.len();
    let end = msg[start..].find('`')? + start;
    let name = &msg[start..end];
    CAPABILITY_FIELDS
        .iter()
        .find(|(n, _, _)| *n == name)
        .map(|(n, _, _)| *n)
}

fn to_lsp_position(lc: proc_macro2::LineColumn) -> Position {
    Position {
        line: (lc.line - 1) as u32,
        character: lc.column as u32,
    }
}

fn to_lsp_range(span: proc_macro2::Span) -> Range {
    Range {
        start: to_lsp_position(span.start()),
        end: to_lsp_position(span.end()),
    }
}

fn levenshtein_distance(a: &str, b: &str) -> usize {
    let a_chars: Vec<char> = a.chars().collect();
    let b_chars: Vec<char> = b.chars().collect();
    let num_a = a_chars.len();
    let num_b = b_chars.len();

    let mut dp = vec![vec![0; num_b + 1]; num_a + 1];

    for (i, row) in dp.iter_mut().enumerate() {
        row[0] = i;
    }
    for (j, cell) in dp[0].iter_mut().enumerate() {
        *cell = j;
    }

    for i in 1..=num_a {
        for j in 1..=num_b {
            if a_chars[i - 1] == b_chars[j - 1] {
                dp[i][j] = dp[i - 1][j - 1];
            } else {
                dp[i][j] = 1 + std::cmp::min(
                    dp[i - 1][j - 1], // substitution
                    std::cmp::min(
                        dp[i - 1][j], // deletion
                        dp[i][j - 1], // insertion
                    ),
                );
            }
        }
    }

    dp[num_a][num_b]
}

fn default_val_for_cap(name: &str) -> &'static str {
    match name {
        "text_document_sync" => "Some(TextDocumentSyncCapability::Kind(TextDocumentSyncKind::INCREMENTAL))",
        "completion_provider" => "Some(CompletionOptions::default())",
        "hover_provider" => "Some(HoverProviderCapability::Simple(true))",
        "definition_provider" => "Some(OneOf::Left(true))",
        "declaration_provider" => "Some(DeclarationCapability::Simple(true))",
        "type_definition_provider" => "Some(TypeDefinitionProviderCapability::Simple(true))",
        "implementation_provider" => "Some(ImplementationProviderCapability::Simple(true))",
        "references_provider" => "Some(OneOf::Left(true))",
        "document_highlight_provider" => "Some(OneOf::Left(true))",
        "document_symbol_provider" => "Some(OneOf::Left(true))",
        "workspace_symbol_provider" => "Some(OneOf::Left(true))",
        "code_action_provider" => "Some(CodeActionProviderCapability::Simple(true))",
        "code_lens_provider" => "Some(CodeLensOptions::default())",
        "document_link_provider" => "Some(DocumentLinkOptions::default())",
        "document_formatting_provider" => "Some(OneOf::Left(true))",
        "document_range_formatting_provider" => "Some(OneOf::Left(true))",
        "document_on_type_formatting_provider" => "Some(DocumentOnTypeFormattingOptions::default())",
        "rename_provider" => "Some(OneOf::Left(true))",
        _ => "Some(Default::default())",
    }
}

/// Convert an LSP Position (line/character) to a char offset in a Rope.
fn rope_offset(rope: &ropey::Rope, pos: Position) -> usize {
    let line_start = rope.line_to_char(pos.line as usize);
    line_start + pos.character as usize
}

fn char_to_lsp_position(rope: &ropey::Rope, char_idx: usize) -> Position {
    let line = rope.char_to_line(char_idx);
    let line_start_char = rope.line_to_char(line);
    let character = char_idx - line_start_char;
    Position::new(line as u32, character as u32)
}

fn find_fallback_insert_pos(text: &ropey::Rope) -> Position {
    let mut char_idx = text.len_chars();
    while char_idx > 0 {
        char_idx -= 1;
        if text.char(char_idx) == '}' {
            return char_to_lsp_position(text, char_idx);
        }
    }
    let end_line = text.len_lines().saturating_sub(1);
    let end_char = text.line(end_line).len_chars();
    Position::new(end_line as u32, end_char as u32)
}

const MINIMAL_SCAFFOLD: &str = r#"
    async fn initialize(&self, _params: InitializeParams) -> Result<InitializeResult> {
        Ok(InitializeResult {
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Kind(
                    TextDocumentSyncKind::INCREMENTAL,
                )),
                ..Default::default()
            },
            ..Default::default()
        })
    }

    async fn initialized(&self, _: InitializedParams) {
        self.client
            .log_message(MessageType::INFO, "server initialized!")
            .await;
    }

    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }
"#;
