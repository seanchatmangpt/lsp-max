//! branchless-lsp - two-tier branchless verification language server.
//!
//! Tier 1 (advisory): a `syn` AST visit pass flags branchy syntax
//! (`ExprIf`/`ExprMatch`/`ExprForLoop`/`ExprWhile`/`ExprLoop`/`ExprTry`/
//! `ExprIndex`/panic-unwrap-expect) and emits `ANDON_BRANCH_SYNTAX`
//! diagnostics. Advisory only - never blocks admission.
//!
//! Tier 2 (authoritative): the compiler worker drives `rustc --emit=obj`
//! into a `/tmp` staging dir; the disassembly court parses the object with
//! `object` + gimli-ready `.debug_line` plumbing + `capstone` (ARM64 /
//! RISC-V(32+C)) to decide, on the real machine code, whether admitted
//! compilations are branchless. Tier 2 verdicts override Tier 1.
//!
//! Residual accounting: `#[allow_isel_residual]` permits an ISel
//! residual (HINT-residual); branchy code without it is ERROR-refused.
//! Receipts are the 3-vector <N_admitted, N_refused, N_residual> hashed
//! with BLAKE3 into `.cgap/receipts/branchless/`.

pub mod ast_linter;
pub mod attributes;
pub mod code_actions;
pub mod compiler_worker;
pub mod conformance;
pub mod disasm_court;
pub mod hover;
pub mod receipts;
pub mod rule_pack;

pub use ast_linter::{lint_source, BranchHit, LintReport};
pub use attributes::{AttrKind, AttrParseError, ParsedAttrs};
pub use code_actions::quick_fixes;
pub use compiler_worker::{CompileOutcome, CompilerWorker, TargetKind};
pub use disasm_court::{CourtArch, DisasmCourt, DisasmReport, FnVerdict};
pub use hover::hover_card;
pub use receipts::{ReceiptVector, ReceiptWriter};

use lsp_max::ast::AutoLspAdapter;
use lsp_max::max_protocol::{ConformanceVector, LawAxis};
use lsp_max::{rule_pack_server as rps, Client};
use lsp_types_max::{
    DidChangeTextDocumentParams, DidOpenTextDocumentParams, DidSaveTextDocumentParams,
    InitializeParams, InitializeResult,
};

/// Live Tier 2 state: the last court verdict vector per the saved document.
#[derive(Default)]
pub struct Tier2State {
    /// The last `ReceiptVector` folded from a did_save court run.
    pub last_vector: Option<crate::receipts::ReceiptVector>,
    /// URI of the document the vector belongs to.
    pub last_uri: Option<String>,
    /// The 80-byte session conformance seal, minted at `shutdown()`.
    ///
    /// `None` until shutdown seals the session. Wire surface note: the
    /// `LanguageServer::shutdown` trait signature is fixed at
    /// `Result<()>` (lsp-max `src/language_server.rs:29`), so the seal
    /// cannot ride the shutdown response; the minimal honest surface is
    /// this state slot (read by court/tooling consumers) plus one typed
    /// `window/logMessage` line per shutdown.
    pub session_seal: Option<[u8; crate::conformance::SEAL_LEN]>,
}

/// The branchless verification language server.
///
/// Implements the real `lsp_max::rule_pack_server::RulePackServer`
/// surface (upstream `src/rule_pack_server.rs:691`); no local
/// re-definition of the trait.
pub struct BranchlessLspServer {
    client: Client,
    packs: rps::ValidatedRulePackSet,
    adapter: AutoLspAdapter,
    index: rps::WorkspaceIndex,
    /// Live Tier 2 court state, served via `max/conformanceVector`.
    tier2: std::sync::Mutex<Tier2State>,
    /// Session-scoped conformance accumulator (G07 orphan, wired by
    /// lane LSPA-1): every did_save court run folds in via `record`.
    session: std::sync::Mutex<crate::conformance::SessionConformance>,
}

impl BranchlessLspServer {
    /// Build a server with the Tier 2 court rule pack loaded
    /// (conflict-validated).
    pub fn new(client: Client) -> Self {
        let packs = rps::ValidatedRulePackSet::new(&[rule_pack::tier2_pack()])
            .expect("single-pack set always validates");
        Self {
            client,
            packs,
            adapter: AutoLspAdapter::new_default(),
            index: rps::WorkspaceIndex::new(),
            tier2: std::sync::Mutex::new(Tier2State::default()),
            session: std::sync::Mutex::new(crate::conformance::SessionConformance::new()),
        }
    }

    /// Borrow the push client.
    pub fn client(&self) -> &Client {
        &self.client
    }

    /// Borrow the live Tier 2 state (court-facing).
    pub fn tier2_state(&self) -> &std::sync::Mutex<Tier2State> {
        &self.tier2
    }

    /// Borrow the session conformance accumulator (court-facing).
    pub fn session_conformance(&self) -> &std::sync::Mutex<crate::conformance::SessionConformance> {
        &self.session
    }

    /// Project a live ReceiptVector onto the wire `ConformanceVector`.
    pub fn conformance_from_vector(v: &crate::receipts::ReceiptVector) -> ConformanceVector {
        let total = v.total().max(1);
        let refused = if v.n_refused > 0 {
            vec![LawAxis::Custom("ANDON_REFUSED".into())]
        } else {
            vec![]
        };
        let admitted = if v.n_admitted + v.n_residual > 0 {
            vec![LawAxis::Custom("BRANCHLESS_ADMITTED".into())]
        } else {
            vec![]
        };
        ConformanceVector {
            admitted,
            refused,
            unknown: vec![],
            score: Some(100.0 * (v.n_admitted + v.n_residual) as f64 / total as f64),
            strict_mode: true,
            process_quality: None,
            ..Default::default()
        }
    }
}

impl rps::RulePackServer for BranchlessLspServer {
    fn rule_packs(&self) -> &rps::ValidatedRulePackSet {
        &self.packs
    }

    fn grammar(&self) -> tree_sitter::Language {
        tree_sitter_rust::LANGUAGE.into()
    }

    fn server_name(&self) -> &'static str {
        "branchless-lsp"
    }

    fn client(&self) -> &Client {
        &self.client
    }

    fn adapter(&self) -> &AutoLspAdapter {
        &self.adapter
    }

    fn workspace_index(&self) -> Option<&rps::WorkspaceIndex> {
        Some(&self.index)
    }
}

/// LSP lifecycle: delegates to RulePackServer default handlers.
#[lsp_max::async_trait]
impl lsp_max::LanguageServer for BranchlessLspServer {
    async fn initialize(
        &self,
        _params: InitializeParams,
    ) -> lsp_max::jsonrpc::Result<InitializeResult> {
        use lsp_max::rule_pack_server::RulePackServer;
        Ok(self.build_initialize_result())
    }

    async fn shutdown(&self) -> lsp_max::jsonrpc::Result<()> {
        // Lane LSPA-1: seal the session conformance totals (G07 orphan
        // wiring). The 80-byte seal lands in Tier2State.session_seal
        // (court/tooling consumers) plus one typed log line; the trait's
        // `Result<()>` signature carries no response extension slot.
        let seal = self
            .session
            .lock()
            .expect("session conformance lock")
            .seal();
        {
            let mut st = self.tier2.lock().expect("tier2 state lock");
            st.session_seal = Some(seal);
        }
        let (n_admitted, n_refused, n_residual, n_alu, n_load_store) = {
            let t = self.session.lock().expect("session conformance lock");
            (
                t.n_admitted,
                t.n_refused,
                t.n_residual,
                t.n_alu,
                t.n_load_store,
            )
        };
        let tag: String = seal[76..80].iter().map(|b| format!("{:02x}", b)).collect();
        // LSPA-3 SLA minimal-unblock (disclosed in both lanes' receipts):
        // fixed the in-flight LSPA-1 seal log line — `log_message` takes
        // (MessageType, impl Display) positionally, `[u8]` has no LowerHex
        // impl (hex via byte map), and the session MutexGuard was held
        // across the log_message await (future not Send); totals are now
        // copied out and the guard dropped before the await.
        self.client()
            .log_message(
                lsp_types_max::MessageType::INFO,
                format!(
                    "CONFORMANCE-SEAL admitted={} refused={} residual={} \
                     alu={} load_store={} tag={}",
                    n_admitted, n_refused, n_residual, n_alu, n_load_store, tag
                ),
            )
            .await;
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

    async fn did_save(&self, params: DidSaveTextDocumentParams) {
        use lsp_max::rule_pack_server::RulePackServer;
        // Tier 2 (authoritative): compile the saved content with debuginfo,
        // run the disasm court, publish diagnostics, update the live vector.
        let uri = params.text_document.uri;
        let content = params.text.clone().or_else(|| {
            self.workspace_index()
                .and_then(|idx| idx.get(uri.as_str()))
                .map(|doc| doc.content)
        });
        let Some(content) = content else {
            return; // never-seen document: nothing staged to judge
        };
        // G06: run_tier2 drives a real `rustc --emit=obj` subprocess
        // (seconds-scale). Offload to the blocking pool so the save never
        // stalls the async request loop; the handler still awaits the
        // verdict so diagnostics ordering stays save-ordered.
        let outcome = tokio::task::spawn_blocking(move || {
            rule_pack::run_tier2(&content, TargetKind::Aarch64AppleDarwin)
        })
        .await
        .expect("tier2 court blocking task must not panic");
        let diags = rule_pack::verdict_diagnostics(&outcome);
        self.client()
            .publish_diagnostics(uri.clone(), diags, None)
            .await;
        let cv = Self::conformance_from_vector(&outcome.vector);
        if let Some(idx) = self.workspace_index() {
            idx.set_conformance(uri.as_str(), cv);
        }
        // Lane LSPA-1: fold this court run into the session totals (the
        // shutdown seal is exactly these accumulated totals), before the
        // vector moves into the live state slot.
        self.session
            .lock()
            .expect("session conformance lock")
            .record(&outcome.vector);
        let mut st = self.tier2.lock().expect("tier2 state lock");
        st.last_vector = Some(outcome.vector);
        st.last_uri = Some(uri.as_str().to_string());
    }

    async fn max_conformance_vector(
        &self,
        _params: Option<lsp_max::max_protocol::SnapshotId>,
    ) -> lsp_max::jsonrpc::Result<ConformanceVector> {
        let st = self.tier2.lock().expect("tier2 state lock");
        Ok(match &st.last_vector {
            Some(v) => Self::conformance_from_vector(v),
            None => ConformanceVector {
                strict_mode: true,
                ..Default::default()
            },
        })
    }
}
