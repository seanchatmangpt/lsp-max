use clap_noun_verb_macros::verb;
use lsp_max::{Client, LanguageServer, LspService, Server};
use lsp_types_max::*;
use regex::Regex;
use std::sync::OnceLock;

// ── Axum-specific rules (inline — no external TOML files) ─────────────────────

struct AxumRule {
    id: &'static str,
    severity: DiagnosticSeverity,
    re: fn() -> &'static Regex,
    message: &'static str,
}

fn unwrap_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"\.unwrap\(\)").unwrap())
}

fn blocking_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"std::thread::sleep|std::fs::read|std::fs::write").unwrap())
}

const AXUM_RULES: &[AxumRule] = &[
    AxumRule {
        id: "AXUM-001",
        severity: DiagnosticSeverity::ERROR,
        re: unwrap_re,
        message: "unwrap() in handler — use ? or map_err instead",
    },
    AxumRule {
        id: "AXUM-002",
        severity: DiagnosticSeverity::WARNING,
        re: blocking_re,
        message: "blocking call in async context — use tokio equivalents",
    },
];

// ── Backend ───────────────────────────────────────────────────────────────────

struct AxumBackend {
    client: Client,
    index: lsp_max::rule_pack_server::WorkspaceIndex,
    adapter: lsp_max::ast::AutoLspAdapter,
    packs: lsp_max::rule_pack_server::ValidatedRulePackSet,
}

impl AxumBackend {
    fn new(client: Client) -> Self {
        let rules: Vec<lsp_max::rule_pack_server::Rule> = AXUM_RULES
            .iter()
            .map(|r| {
                let severity = match r.severity {
                    DiagnosticSeverity::ERROR => "error",
                    DiagnosticSeverity::WARNING => "warning",
                    DiagnosticSeverity::INFORMATION => "info",
                    DiagnosticSeverity::HINT => "hint",
                    _ => "warning",
                };
                let pattern = (r.re)().as_str().to_string();

                lsp_max::rule_pack_server::Rule {
                    id: r.id.to_string(),
                    name: r.message.to_string(),
                    severity: severity.to_string(),
                    pattern,
                    path_globs: Vec::new(),
                    exclude_globs: Vec::new(),
                    rationale: r.message.to_string(),
                    message: r.message.to_string(),
                    eval_budget: lsp_max::rule_pack_server::EvalBudget::Sync,
                }
            })
            .collect();

        let rule_pack = lsp_max::rule_pack_server::RulePack {
            id: "axum-pack".to_string(),
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

impl lsp_max::rule_pack_server::RulePackServer for AxumBackend {
    fn rule_packs(&self) -> &lsp_max::rule_pack_server::ValidatedRulePackSet {
        &self.packs
    }
    fn grammar(&self) -> tree_sitter::Language {
        tree_sitter_rust::LANGUAGE.into()
    }
    fn server_name(&self) -> &'static str {
        "axum-lsp"
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
impl LanguageServer for AxumBackend {
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
}

// ── CLI entry point ───────────────────────────────────────────────────────────

#[verb("start")]
fn start_server(_stdio: bool) -> clap_noun_verb::Result<()> {
    tokio::runtime::Runtime::new().unwrap().block_on(async {
        let stdin = tokio::io::stdin();
        let stdout = tokio::io::stdout();
        let (service, socket) = LspService::new(AxumBackend::new);
        let _ = Server::new(stdin, stdout, socket).serve(service).await;
    });
    Ok(())
}

fn main() -> clap_noun_verb::Result<()> {
    clap_noun_verb::run()
}
