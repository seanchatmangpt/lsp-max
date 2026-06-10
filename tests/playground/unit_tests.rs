/// Synchronous unit/challenger tests — no server required.
use tower_lsp_max::lsp_types::Position;
use tower_lsp_max_playground::handlers::completions::get_completions;

#[test]
fn test_challenger_edge_cases_completions() {
    // 1. Check impl LanguageServer false positive (cursor outside impl but within 200 lines)
    let code_outside_impl = r#"
impl LanguageServer for Backend {
    async fn shutdown(&self) -> Result<()> { Ok(()) }
}

struct AnotherStruct;
impl AnotherStruct {
    f
}
"#;
    let completions = get_completions(Position::new(7, 5), code_outside_impl);
    let has_formatting = completions.iter().any(|item| item.label == "formatting");
    println!(
        "CHALLENGE RESULT: outside impl has_formatting = {}",
        has_formatting
    );

    // 2. Check ServerCapabilities false positive
    let code_outside_sc = r#"
fn init() {
    let capabilities = ServerCapabilities {
        text_document_sync: Some(TextDocumentSyncCapability::Kind(
            TextDocumentSyncKind::FULL,
        )),
    };

    let another = AnotherStruct {
        t
    };
}
"#;
    let completions_sc = get_completions(Position::new(9, 9), code_outside_sc);
    let has_sync_cap = completions_sc
        .iter()
        .any(|item| item.label == "text_document_sync");
    println!(
        "CHALLENGE RESULT: outside sc has_sync_cap = {}",
        has_sync_cap
    );

    // 3. Check rpc attribute completion when another parameter is first
    let code_rpc_param = r#"
#[rpc]
pub trait TestServer {
    async fn method_a(&self);
    async fn method_b(&self);
    async fn method_c(&self);
    #[rpc(kind = "request", name = "textDoc
}
"#;
    let completions_rpc = get_completions(Position::new(6, 42), code_rpc_param);
    let has_completion_rpc = completions_rpc
        .iter()
        .any(|item| item.label == "textDocument/completion");
    println!(
        "CHALLENGE RESULT: rpc with kind first and remote parent has completion = {}",
        has_completion_rpc
    );
}

#[test]
fn test_methods_and_capabilities_table_consistency() {
    use tower_lsp_max_playground::handlers::completions::{CAPABILITY_FIELDS, METHODS};
    let mut errors = Vec::new();

    for method in METHODS {
        if let Some(cap_field) = method.capability_field {
            let cap_entry = CAPABILITY_FIELDS
                .iter()
                .find(|(name, _, _)| *name == cap_field);
            if let Some(entry) = cap_entry {
                let (_, _, methods_list) = entry;
                if !methods_list.contains(&method.fn_name) {
                    errors.push(format!(
                        "Method '{}' references capability_field '{}', but is not listed in its methods list in CAPABILITY_FIELDS",
                        method.fn_name, cap_field
                    ));
                }
            } else {
                errors.push(format!(
                    "Method '{}' references capability_field '{}', which is missing in CAPABILITY_FIELDS",
                    method.fn_name, cap_field
                ));
            }
        }
    }

    for (cap_name, _, methods_list) in CAPABILITY_FIELDS {
        for fn_name in *methods_list {
            let method_entry = METHODS.iter().find(|m| m.fn_name == *fn_name);
            if let Some(method) = method_entry {
                if method.capability_field != Some(*cap_name) {
                    errors.push(format!(
                        "Method '{}' is listed under '{}' in CAPABILITY_FIELDS, but its capability_field in METHODS is {:?}",
                        fn_name, cap_name, method.capability_field
                    ));
                }
            } else {
                errors.push(format!(
                    "CAPABILITY_FIELDS references method '{}' under '{}', but it is missing in METHODS",
                    fn_name, cap_name
                ));
            }
        }
    }

    if !errors.is_empty() {
        println!(
            "CHALLENGE RESULT: found {} table inconsistencies:",
            errors.len()
        );
        for err in &errors {
            println!("  - {}", err);
        }
    } else {
        println!("CHALLENGE RESULT: table consistency verified successfully!");
    }
}

#[test]
fn test_challenger_diagnostics_false_positives() {
    use tower_lsp_max::lsp_types::Uri;
    use tower_lsp_max_playground::handlers::diagnostics::rules::get_diagnostics;

    let code = r#"
impl LanguageServer for Backend {
    async fn initialize(&self, params: InitializeParams) -> Result<InitializeResult> {
        Ok(InitializeResult {
            capabilities: ServerCapabilities {
                workspace: Some(WorkspaceServerCapabilities {
                    workspace_folders: Some(WorkspaceFoldersServerCapabilities {
                        supported: Some(true),
                        change_notifications: Some(OneOf::Left(true)),
                    }),
                    ..Default::default()
                }),
                ..Default::default()
            },
            ..Default::default()
        })
    }

    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }

    async fn did_change_workspace_folders(&self, params: DidChangeWorkspaceFoldersParams) {
    }
}
"#;
    let uri = <Uri as std::str::FromStr>::from_str("file:///dummy.rs").unwrap();
    let diags = get_diagnostics(code, &uri);

    let has_dead_code_warning = diags.iter().any(|d| {
        d.code
            == Some(tower_lsp_max::lsp_types::NumberOrString::String(
                "TLM002".to_string(),
            ))
            && d.message.contains("did_change_workspace_folders")
    });
    println!(
        "CHALLENGE RESULT: did_change_workspace_folders triggers false dead code warning = {}",
        has_dead_code_warning
    );
}
