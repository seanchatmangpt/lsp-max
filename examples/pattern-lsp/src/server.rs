use clap_noun_verb::Result;
use clap_noun_verb_macros::verb;
use lsp_max::{Client, LanguageServer, LspService, Server};
use lsp_types_max::*;
use serde::Serialize;

struct PatternLsp {
    client: Client,
    index: lsp_max::rule_pack_server::WorkspaceIndex,
    adapter: lsp_max::ast::AutoLspAdapter,
    packs: lsp_max::rule_pack_server::ValidatedRulePackSet,
}

impl PatternLsp {
    fn new(client: Client) -> Self {
        let mut rules = Vec::new();
        let rules_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("rules");

        if rules_dir.exists() {
            if let Ok(entries) = std::fs::read_dir(rules_dir) {
                for entry in entries.flatten() {
                    if entry.path().extension().and_then(|e| e.to_str()) == Some("toml") {
                        if let Ok(content) = std::fs::read_to_string(entry.path()) {
                            if let Ok(pack) =
                                toml::from_str::<lsp_max::rule_pack_server::RulePack>(&content)
                            {
                                rules.extend(pack.rules);
                            }
                        }
                    }
                }
            }
        }

        let rule_pack = lsp_max::rule_pack_server::RulePack {
            id: "pattern-pack".to_string(),
            version: "1.0.0".to_string(),
            rules,
            depends_on: Vec::new(),
        };

        let packs =
            lsp_max::rule_pack_server::ValidatedRulePackSet::new(&[rule_pack]).unwrap_or_default();

        Self {
            client,
            index: lsp_max::rule_pack_server::WorkspaceIndex::new(),
            adapter: lsp_max::ast::AutoLspAdapter::new_default(),
            packs,
        }
    }
}

impl lsp_max::rule_pack_server::RulePackServer for PatternLsp {
    fn rule_packs(&self) -> &lsp_max::rule_pack_server::ValidatedRulePackSet {
        &self.packs
    }
    fn grammar(&self) -> tree_sitter::Language {
        tree_sitter_rust::LANGUAGE.into()
    }
    fn server_name(&self) -> &'static str {
        "pattern-lsp"
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
}

#[lsp_max::async_trait]
impl LanguageServer for PatternLsp {
    async fn initialize(&self, _: InitializeParams) -> lsp_max::jsonrpc::Result<InitializeResult> {
        use lsp_max::rule_pack_server::RulePackServer;
        Ok(self.build_initialize_result())
    }

    async fn shutdown(&self) -> lsp_max::jsonrpc::Result<()> {
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
}

// ── CLI verb ──────────────────────────────────────────────────────────────────

#[derive(Serialize)]
pub struct ServerResult {
    pub success: bool,
}

/// Start the pattern LSP server
#[verb("serve")]
pub fn cmd_serve(stdio: bool) -> Result<ServerResult> {
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        if stdio {
            let stdin = tokio::io::stdin();
            let stdout = tokio::io::stdout();
            let (service, socket) = LspService::new(PatternLsp::new);
            Server::new(stdin, stdout, socket)
                .serve(service)
                .await
                .unwrap();
        }
    });
    Ok(ServerResult { success: true })
}
