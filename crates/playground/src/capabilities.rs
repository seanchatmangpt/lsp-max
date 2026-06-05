use tower_lsp_max::lsp_types::*;

/// Build the `ServerCapabilities` advertised during `initialize`.
///
/// Only capabilities with handler implementations are declared.
/// Declaring a capability without a handler causes the client to send requests
/// that return `method_not_found` — silent breakage in the downstream server.
pub fn server_capabilities() -> ServerCapabilities {
    ServerCapabilities {
        // Incremental sync: the Rope document store applies range patches in O(log n).
        text_document_sync: Some(TextDocumentSyncCapability::Options(
            TextDocumentSyncOptions {
                open_close: Some(true),
                change: Some(TextDocumentSyncKind::INCREMENTAL),
                save: Some(TextDocumentSyncSaveOptions::Supported(true)),
                will_save: None,
                will_save_wait_until: None,
            },
        )),

        // Completions triggered by `.`, `:`, space, and `"` to cover all
        // tower-lsp-max completion contexts.
        completion_provider: Some(CompletionOptions {
            resolve_provider: Some(false),
            trigger_characters: Some(vec![
                ".".to_string(),
                ":".to_string(),
                " ".to_string(),
                "\"".to_string(),
            ]),
            all_commit_characters: None,
            work_done_progress_options: Default::default(),
            completion_item: None,
        }),

        // Hover over method names, capability fields, and protocol types.
        hover_provider: Some(HoverProviderCapability::Simple(true)),

        // Code actions: quickfix stubs + scaffold generation.
        code_action_provider: Some(CodeActionProviderCapability::Options(CodeActionOptions {
            code_action_kinds: Some(vec![CodeActionKind::QUICKFIX, CodeActionKind::SOURCE]),
            resolve_provider: Some(false),
            work_done_progress_options: Default::default(),
        })),

        ..Default::default()
    }
}
