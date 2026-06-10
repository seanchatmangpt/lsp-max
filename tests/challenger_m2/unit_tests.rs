/// Unit tests for completion and diagnostic helpers (no server required).
use std::str::FromStr;
use tower_lsp_max::lsp_types::*;
use tower_lsp_max_playground::handlers::completions::get_completions;
use tower_lsp_max_playground::handlers::diagnostics::get_diagnostics;

#[test]
fn test_completion_empty_file_no_panic() {
    let pos = Position::new(0, 0);
    let result = std::panic::catch_unwind(|| get_completions(pos, ""));
    assert!(
        result.is_ok(),
        "Expected get_completions on empty file to not panic"
    );
    let items = result.unwrap();
    assert!(items.is_empty());
}

#[test]
fn test_completion_out_of_bounds_no_panic() {
    let pos = Position::new(100, 100);
    let result = std::panic::catch_unwind(|| get_completions(pos, "fn main() {}"));
    assert!(
        result.is_ok(),
        "Expected get_completions with out of bounds line to not panic"
    );
    let items = result.unwrap();
    assert!(items.is_empty());
}

#[test]
fn test_completion_multiple_rpc_arguments_success() {
    let text = r#"
#[rpc]
pub trait TestServer {
    async fn foo(&self);
    async fn bar(&self);
    async fn baz(&self);
    #[rpc(flag, name = "textDoc
}
"#;
    let pos = Position::new(6, 29);
    let items = get_completions(pos, text);

    let has_completion = items
        .iter()
        .any(|item| item.label == "textDocument/completion");

    assert!(
        has_completion,
        "Should suggest completion when name is in a list of parameters"
    );
}

#[test]
fn test_completion_server_capabilities_nested_brace_success() {
    let text = r#"
fn get_caps() -> ServerCapabilities {
    ServerCapabilities {
        workspace: Some(WorkspaceServerCapabilities {
            workspace_folders: Some(WorkspaceFoldersServerCapabilities {
                supported: Some(true),
            })
        }),
        comp
    }
}
"#;
    let pos = Position::new(8, 12);
    let items = get_completions(pos, text);

    let has_completion = items.iter().any(|item| item.label == "completion_provider");

    assert!(has_completion, "Should find completion_provider inside ServerCapabilities structure even with nested blocks");

    let text_exact_brace = r#"
fn get_caps() -> ServerCapabilities {
    ServerCapabilities {
        workspace: Some(WorkspaceServerCapabilities {
            workspace_folders: Some(WorkspaceFoldersServerCapabilities {
                supported: Some(true),
            }
            )
        }
        ),
        comp
    }
}
"#;
    let pos_exact = Position::new(11, 12);
    let items_exact = get_completions(pos_exact, text_exact_brace);
    let has_completion_exact = items_exact
        .iter()
        .any(|item| item.label == "completion_provider");
    assert!(
        has_completion_exact,
        "Should find completions even with formatting closing braces on separate lines"
    );
}

#[test]
fn test_completion_outside_impl_block_no_false_positive() {
    let text = r#"
impl LanguageServer for Backend {
    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }
}

fn helper_function() {
    textDoc
}
"#;
    let pos = Position::new(8, 11);
    let items = get_completions(pos, text);

    let has_completion = items
        .iter()
        .any(|item| item.label == "textDocument/completion");

    assert!(
        !has_completion,
        "Should not suggest trait methods when cursor is outside the impl block"
    );
}

#[test]
fn test_nested_capabilities_no_false_positive_warning() {
    let url = Uri::from_str("file:///Users/sac/test.rs").unwrap();
    let text = r#"
struct Dummy;
impl LanguageServer for Dummy {
    async fn initialize(&self, params: InitializeParams) -> Result<InitializeResult> {
        Ok(InitializeResult {
            capabilities: ServerCapabilities {
                workspace: Some(WorkspaceOptions {
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
    async fn shutdown(&self) -> Result<()> { Ok(()) }
    async fn did_change_workspace_folders(&self, params: DidChangeWorkspaceFoldersParams) {}
}
"#;
    let diags = get_diagnostics(text, &url);

    let false_positive = diags.iter().any(|d| {
        d.code == Some(NumberOrString::String("TLM002".to_string()))
            && d.message.contains("did_change_workspace_folders")
    });

    assert!(
        !false_positive,
        "Should not trigger false positive warning for did_change_workspace_folders when nested capability is declared"
    );
}

#[test]
fn test_completion_extreme_bounds_safety() {
    let pos_large = Position::new(u32::MAX, u32::MAX);
    let result_large = std::panic::catch_unwind(|| {
        get_completions(pos_large, "impl LanguageServer for Backend {}")
    });
    assert!(
        result_large.is_ok(),
        "Should not panic on large position bounds"
    );

    let text_unicode = "impl LanguageServer for Backend {\n    // 👨‍👩‍👧‍👦 unicode helper\n    sh\n}";
    let pos_unicode = Position::new(2, 6);
    let result_unicode = std::panic::catch_unwind(|| get_completions(pos_unicode, text_unicode));
    assert!(
        result_unicode.is_ok(),
        "Should not panic on unicode content"
    );
    let items = result_unicode.unwrap();
    assert!(
        items.iter().any(|item| item.label == "shutdown"),
        "Should suggest shutdown completion"
    );
}

#[test]
fn test_rpc_attribute_parsing_adversarial() {
    let text_spaces = r#"
#[rpc]
pub trait TestServer {
    #[rpc(  name   =   "textDocument/comp
"#;
    let pos_spaces = Position::new(3, 37);
    let completions_spaces = get_completions(pos_spaces, text_spaces);
    assert!(
        completions_spaces
            .iter()
            .any(|item| item.label == "textDocument/completion"),
        "Should suggest completion with spaces in rpc attribute name parameter"
    );

    let text_no_quote = r#"
#[rpc]
pub trait TestServer {
    #[rpc(name = "textDocument/comp
"#;
    let pos_no_quote = Position::new(3, 32);
    let completions_no_quote = get_completions(pos_no_quote, text_no_quote);
    assert!(
        completions_no_quote
            .iter()
            .any(|item| item.label == "textDocument/completion"),
        "Should suggest completion without trailing quote"
    );

    let text_comment = r#"
#[rpc]
pub trait TestServer {
    #[rpc(
        // name = "ignore",
        name = "textDocument/comp
"#;
    let pos_comment = Position::new(5, 33);
    let completions_comment = get_completions(pos_comment, text_comment);
    assert!(
        completions_comment
            .iter()
            .any(|item| item.label == "textDocument/completion"),
        "Should suggest completion when previous name is commented out"
    );
}

#[test]
fn test_brace_parsing_stack_mismatches() {
    let text_unclosed = r#"
impl LanguageServer for Backend {
    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }
    sh
"#;
    let pos_unclosed = Position::new(5, 6);
    let completions_unclosed = get_completions(pos_unclosed, text_unclosed);
    assert!(
        completions_unclosed
            .iter()
            .any(|item| item.label == "shutdown"),
        "Should suggest shutdown even if impl block is not closed"
    );

    let text_comments = r#"
impl LanguageServer for Backend {
    // } <- this brace is in comment
    sh
}
"#;
    let pos_comments = Position::new(3, 6);
    let completions_comments = get_completions(pos_comments, text_comments);
    assert!(
        completions_comments
            .iter()
            .any(|item| item.label == "shutdown"),
        "Should suggest shutdown even with closing brace in comments"
    );

    let text_strings = r#"
impl LanguageServer for Backend {
    let x = "}"; // brace in string literal
    sh
}
"#;
    let pos_strings = Position::new(3, 6);
    let completions_strings = get_completions(pos_strings, text_strings);
    assert!(
        completions_strings
            .iter()
            .any(|item| item.label == "shutdown"),
        "Should suggest shutdown even with closing brace in string literal"
    );

    let text_impl_comment = r#"
// impl LanguageServer for Dummy {
fn test() {
    sh
}
"#;
    let pos_impl_comment = Position::new(3, 6);
    let completions_impl_comment = get_completions(pos_impl_comment, text_impl_comment);
    assert!(
        !completions_impl_comment
            .iter()
            .any(|item| item.label == "shutdown"),
        "Should not suggest shutdown when impl block is in comment"
    );
}
