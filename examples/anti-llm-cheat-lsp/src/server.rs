use lsp_max::jsonrpc::{Error, Result};
use lsp_max::lsp_types::*;
use lsp_max::{Client, LanguageServer};
use std::sync::{Arc, Mutex};

use crate::engine;
use crate::virtual_docs::{
    checkpoint_status, failset, forbidden_implications, ggen_render, lsp318_matrix, receipt_ledger,
};

pub struct AntiLlmServer {
    pub client: Client,
    pub workspace_root: Arc<Mutex<Option<String>>>,
    pub index: lsp_max::rule_pack_server::WorkspaceIndex,
    pub adapter: lsp_max::ast::AutoLspAdapter,
    pub packs: lsp_max::rule_pack_server::ValidatedRulePackSet,
}

impl AntiLlmServer {
    pub fn new(client: Client) -> Self {
        Self {
            client,
            workspace_root: Arc::new(Mutex::new(None)),
            index: lsp_max::rule_pack_server::WorkspaceIndex::new(),
            adapter: lsp_max::ast::AutoLspAdapter::new_default(),
            packs: lsp_max::rule_pack_server::ValidatedRulePackSet::empty(),
        }
    }
}

impl lsp_max::rule_pack_server::RulePackServer for AntiLlmServer {
    fn rule_packs(&self) -> &lsp_max::rule_pack_server::ValidatedRulePackSet {
        &self.packs
    }
    fn grammar(&self) -> tree_sitter::Language {
        tree_sitter_rust::LANGUAGE.into()
    }
    fn server_name(&self) -> &'static str {
        "anti-llm-cheat-lsp"
    }
    fn client(&self) -> &Client {
        &self.client
    }
    fn adapter(&self) -> &lsp_max::ast::AutoLspAdapter {
        &self.adapter
    }
    fn workspace_index(&self) -> Option<&lsp_max::rule_pack_server::WorkspaceIndex> {
        Some(&self.index)
    }
    fn scan_uri_classified(
        &self,
        uri: &Url,
        _content: &str,
    ) -> lsp_max::rule_pack_server::ClassifiedFindings {
        if uri.as_str().starts_with("anti-llm://") || uri.as_str().starts_with("ggen://") {
            (Vec::new(), Vec::new())
        } else {
            let root_dir = {
                let guard = self.workspace_root.lock().unwrap();
                guard.clone().unwrap_or_else(|| ".".to_string())
            };
            let obs = engine::scan_directory(&root_dir);
            let diags = engine::evaluate_diagnostics(&obs);
            let sync: Vec<lsp_max::rule_pack_server::Finding> = diags
                .into_iter()
                .filter(|d| {
                    let norm_path = d.file_path.replace("\\", "/");
                    let norm_uri = uri.to_string().replace("\\", "/");
                    norm_uri.ends_with(&norm_path)
                })
                .map(|d| {
                    let lsp_diag = d.to_lsp();
                    let law_axis = lsp_max::rule_pack_server::severity_to_law_axis(if d.blocking {
                        "error"
                    } else {
                        "warning"
                    });
                    let max_diag = lsp_max::max_protocol::MaxDiagnostic {
                        lsp: lsp_diag.clone(),
                        diagnostic_id: format!("{}-{}:{}", d.code, d.line, d.column),
                        law_id: d.code.clone(),
                        law_axis,
                        violated_invariant: d.forbidden_implication.clone(),
                        ..Default::default()
                    };
                    (max_diag, lsp_diag)
                })
                .collect();
            (sync, Vec::new())
        }
    }
}

#[lsp_max::async_trait]
impl LanguageServer for AntiLlmServer {
    #[allow(deprecated, clippy::field_reassign_with_default)]
    async fn initialize(&self, params: InitializeParams) -> Result<InitializeResult> {
        if let Some(uri) = params.root_uri {
            if let Ok(url) = url::Url::parse(uri.as_str()) {
                if let Ok(path) = url.to_file_path() {
                    let mut root = self.workspace_root.lock().unwrap();
                    *root = Some(path.to_string_lossy().to_string());
                }
            }
        }

        let mut caps = ServerCapabilities::default();
        caps.text_document_sync =
            Some(TextDocumentSyncCapability::Kind(TextDocumentSyncKind::FULL));
        caps.inline_completion_provider = Some(OneOf::Left(true));
        caps.workspace = Some(WorkspaceServerCapabilities {
            text_document_content: Some(OneOf::Right(TextDocumentContentRegistrationOptions {
                text_document_content_options: TextDocumentContentOptions {
                    schemes: vec!["anti-llm".to_string(), "ggen".to_string()],
                },
                text_document_registration_options: TextDocumentRegistrationOptions {
                    document_selector: None,
                },
                static_registration_options: StaticRegistrationOptions { id: None },
            })),
            ..Default::default()
        });
        caps.folding_range_provider = Some(FoldingRangeProviderCapability::Simple(true));
        caps.document_range_formatting_provider = Some(OneOf::Left(true));
        caps.code_action_provider = Some(CodeActionProviderCapability::Simple(true));
        caps.completion_provider = Some(CompletionOptions {
            resolve_provider: Some(true),
            trigger_characters: Some(vec!["#".to_string()]),
            all_commit_characters: None,
            work_done_progress_options: WorkDoneProgressOptions::default(),
            completion_item: None,
        });
        caps.signature_help_provider = Some(SignatureHelpOptions::default());
        caps.code_lens_provider = Some(CodeLensOptions {
            resolve_provider: Some(true),
        });

        Ok(InitializeResult {
            capabilities: caps,
            server_info: Some(ServerInfo {
                name: "anti-llm-cheat-lsp".to_string(),
                version: Some("26.6.5".to_string()),
            }),
            offset_encoding: None,
        })
    }

    async fn initialized(&self, _: InitializedParams) {
        self.client
            .log_message(MessageType::INFO, "anti-llm-cheat-lsp server initialized")
            .await;
    }

    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        use lsp_max::rule_pack_server::RulePackServer;
        self.handle_did_open(params).await;
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        use lsp_max::rule_pack_server::RulePackServer;
        self.handle_did_change(params).await;
    }

    async fn did_close(&self, params: DidCloseTextDocumentParams) {
        use lsp_max::rule_pack_server::RulePackServer;
        self.handle_did_close(params);
    }

    async fn did_save(&self, params: DidSaveTextDocumentParams) {
        use lsp_max::rule_pack_server::RulePackServer;
        let uri = &params.text_document.uri;
        let content = if let Some(index) = self.workspace_index() {
            index
                .get(uri.as_str())
                .map(|doc| doc.content.clone())
                .unwrap_or_default()
        } else {
            String::new()
        };
        self.publish_findings_classified(uri.clone(), &content)
            .await;
    }

    async fn inline_completion(
        &self,
        params: InlineCompletionParams,
    ) -> Result<Option<InlineCompletionResponse>> {
        // Query the document content to check for victory-language phrases
        let _text = params.text_document_position.text_document.uri.as_str();

        // Return inline completions if victory language terms are typed
        let items = vec![
            InlineCompletionItem {
                insert_text: StringOrStringValue::String("FAILSET_NONEMPTY".to_string()),
                filter_text: Some("Victory confirmed".to_string()),
                range: None,
                command: None,
                insert_text_format: None,
            },
            InlineCompletionItem {
                insert_text: StringOrStringValue::String("CANDIDATE".to_string()),
                filter_text: Some("fully admitted".to_string()),
                range: None,
                command: None,
                insert_text_format: None,
            },
        ];

        Ok(Some(InlineCompletionResponse::List(InlineCompletionList {
            items,
        })))
    }

    async fn text_document_content(
        &self,
        params: lsp_max::max_protocol::lsp_3_18::TextDocumentContentParams,
    ) -> Result<lsp_max::max_protocol::lsp_3_18::TextDocumentContentResult> {
        let uri = params.text_document.uri.as_str();
        let content = match uri {
            "anti-llm://failset" => {
                let root_dir = {
                    let guard = self.workspace_root.lock().unwrap();
                    guard.clone().unwrap_or_else(|| ".".to_string())
                };
                let obs = engine::scan_directory(&root_dir);
                let diags = engine::evaluate_diagnostics(&obs);
                failset::generate_failset_markdown(&diags)
            }
            "anti-llm://lsp318-matrix" => lsp318_matrix::generate_matrix_markdown(),
            "anti-llm://receipt-ledger" => {
                let root_dir = {
                    let guard = self.workspace_root.lock().unwrap();
                    guard.clone().unwrap_or_else(|| ".".to_string())
                };
                receipt_ledger::generate_ledger_markdown(&format!("{}/receipts", root_dir))
            }
            "anti-llm://forbidden-implications" => {
                forbidden_implications::generate_implications_markdown()
            }
            "anti-llm://checkpoint-status" => checkpoint_status::generate_checkpoint_markdown(),
            // ggen:// virtual document — render a ggen artifact for the ontology
            // URI embedded in the `ggen://` path; never written to disk. The
            // ontology symbol is whatever follows `ggen://`.
            _ if uri.starts_with("ggen://") => ggen_render::generate_ggen_markdown(uri),
            _ => "".to_string(),
        };

        Ok(lsp_max::max_protocol::lsp_3_18::TextDocumentContentResult { text: content })
    }

    async fn folding_range(&self, params: FoldingRangeParams) -> Result<Option<Vec<FoldingRange>>> {
        let _uri = params.text_document.uri;
        // Mock folding range for markdown virtual documents (header section folding)
        let folds = vec![FoldingRange {
            start_line: 0,
            start_character: None,
            end_line: 5,
            end_character: None,
            kind: Some(FoldingRangeKind::Comment),
            collapsed_text: Some("Metadata".to_string()),
        }];
        Ok(Some(folds))
    }

    async fn range_formatting(
        &self,
        _params: DocumentRangeFormattingParams,
    ) -> Result<Option<Vec<TextEdit>>> {
        // Explicitly refuse range formatting by law over non-virtual paths
        Err(Error::invalid_request())
    }

    async fn ranges_formatting(
        &self,
        params: lsp_max::max_protocol::lsp_3_18::DocumentRangesFormattingParams,
    ) -> Result<Option<Vec<lsp_max::max_protocol::lsp_3_18::TextEdit>>> {
        let uri = &params.text_document.uri;
        if uri.starts_with("anti-llm://") {
            Ok(Some(vec![]))
        } else {
            Err(Error::invalid_request())
        }
    }

    async fn signature_help(&self, params: SignatureHelpParams) -> Result<Option<SignatureHelp>> {
        // Nullable activeParameter test support
        let _active_param = params
            .context
            .and_then(|c| c.active_signature_help.and_then(|h| h.active_parameter));
        Ok(Some(SignatureHelp {
            signatures: vec![SignatureInformation {
                label: "anti_llm_rule_verify()".to_string(),
                documentation: Some(Documentation::MarkupContent(MarkupContent {
                    kind: MarkupKind::Markdown,
                    value: "Verifies admissibility rules".to_string(),
                })),
                parameters: None,
                active_parameter: None,
            }],
            active_signature: None,
            active_parameter: None,
        }))
    }

    async fn completion(&self, _params: CompletionParams) -> Result<Option<CompletionResponse>> {
        let items = vec![
            CompletionItem {
                label: "FAILSET_NONEMPTY".to_string(),
                kind: Some(CompletionItemKind::TEXT),
                detail: Some("Active blocking failset exists".to_string()),
                ..Default::default()
            },
            CompletionItem {
                label: "CANDIDATE".to_string(),
                kind: Some(CompletionItemKind::TEXT),
                detail: Some("All requirements met, awaiting audit".to_string()),
                ..Default::default()
            },
            // Snippets in text document edits (LSP318-012)
            CompletionItem {
                label: "todo_snippet".to_string(),
                kind: Some(CompletionItemKind::SNIPPET),
                detail: Some("Todo snippet".to_string()),
                insert_text: Some("println!(\"$1\");".to_string()),
                insert_text_format: Some(InsertTextFormat::SNIPPET),
                ..Default::default()
            },
        ];
        Ok(Some(CompletionResponse::List(CompletionList {
            is_incomplete: false,
            items,
            item_defaults: None,
        })))
    }

    async fn code_action(&self, params: CodeActionParams) -> Result<Option<CodeActionResponse>> {
        let _uri = params.text_document.uri;
        let actions = vec![
            CodeActionOrCommand::CodeAction(CodeAction {
                title: "Open anti-llm://failset".to_string(),
                kind: Some(CodeActionKind::QUICKFIX),
                command: Some(Command {
                    title: "Open Failset Document".to_string(),
                    command: "anti-llm.openFailset".to_string(),
                    arguments: None,
                }),
                ..Default::default()
            }),
            CodeActionOrCommand::CodeAction(CodeAction {
                title: "Open anti-llm://lsp318-matrix".to_string(),
                kind: Some(CodeActionKind::QUICKFIX),
                command: Some(Command {
                    title: "Open LSP 3.18 Matrix".to_string(),
                    command: "anti-llm.openMatrix".to_string(),
                    arguments: None,
                }),
                ..Default::default()
            }),
        ];
        Ok(Some(actions))
    }

    async fn code_lens(&self, params: CodeLensParams) -> Result<Option<Vec<CodeLens>>> {
        let uri = params.text_document.uri;
        let lens = vec![CodeLens {
            range: Range::new(Position::new(0, 0), Position::new(0, 5)),
            command: Some(Command {
                title: "Admissibility Check Active".to_string(),
                command: "anti-llm.check".to_string(),
                arguments: None,
            }),
            data: Some(serde_json::json!({ "uri": uri.as_str() })),
        }];
        Ok(Some(lens))
    }

    async fn code_lens_resolve(&self, mut code_lens: CodeLens) -> Result<CodeLens> {
        if let Some(data) = &code_lens.data {
            if let Some(uri) = data.get("uri").and_then(|u| u.as_str()) {
                code_lens.command = Some(Command {
                    title: format!("Check Admissibility for {}", uri),
                    command: "anti-llm.check".to_string(),
                    arguments: None,
                });
            }
        }
        Ok(code_lens)
    }

    async fn did_open_notebook_document(&self, _params: DidOpenNotebookDocumentParams) {}

    async fn did_change_notebook_document(&self, _params: DidChangeNotebookDocumentParams) {}

    async fn did_save_notebook_document(&self, _params: DidSaveNotebookDocumentParams) {}

    async fn did_close_notebook_document(&self, _params: DidCloseNotebookDocumentParams) {}
}
