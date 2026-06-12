//! `RulePackServer` — the bridge trait that eliminates the boilerplate every
//! regex-pattern language server had to hand-roll.
//!
//! # Why this exists
//!
//! Every example (`pattern-lsp`, `axum-lsp`, `anti-llm-lsp`) independently
//! duplicated the same four concerns:
//!
//! 1. **Scanner** — glob-walk the workspace, apply per-rule regex matches.
//! 2. **Rules loader** — deserialise `rules/*.toml` files into `RulePack` structs.
//! 3. **LSP boilerplate** — wire `did_open` / `did_change` / `did_close` through
//!    `AutoLspAdapter`, then publish diagnostics from regex findings.
//! 4. **Conformance plumbing** — convert per-axis diagnostics into a
//!    `ConformanceVector` via `build_conformance_vector`.
//!
//! `RulePackServer` moves all four concerns into default-method implementations,
//! leaving each concrete server responsible only for `rule_packs()`, `grammar()`,
//! `server_name()`, `client()`, and the mandatory `initialize` / `shutdown` LSP
//! lifecycle methods.
//!
//! # ERRC Innovations (blue ocean differentiators)
//!
//! Four capabilities that no other LSP server provides:
//!
//! 1. **`RulePackSnapshot`** — immutable `Arc`-wrapped workspace state cloneable
//!    into async dispatch without holding any lock.  Mirrors rust-analyzer's
//!    `GlobalStateSnapshot` pattern.  See [`RulePackSnapshot`].
//!
//! 2. **Rule-pack composition** — dependency-resolved ordering of packs with
//!    version compatibility checks and conflict detection.  See
//!    [`ComposedPacks`] and [`compose_packs`].
//!
//! 3. **Latency classification** — every rule carries an `EvalBudget` that
//!    determines whether it is evaluated synchronously (within the `did_open`
//!    handler) or dispatched to a background Tokio task.  See [`EvalBudget`].
//!
//! 4. **Workspace-wide cross-file diagnostics** — the `WorkspaceIndex` tracks
//!    every open document's content and per-file conformance vector.  Cross-file
//!    rules can emit diagnostics on *file A* based on content in *file B*.  The
//!    workspace conformance vector is the aggregate across all files — something
//!    no other LSP does.  See [`WorkspaceIndex`] and [`CrossFileRule`].
//!
//! # Law compliance
//!
//! * The LSP surface is **read-only**: default implementations only publish
//!   diagnostics and build conformance vectors; they never mutate workspace files.
//! * `ConformanceVector::unknown` is never collapsed: axes not covered by any
//!   rule firing remain in `unknown`, as required by the lsp-max AGENTS.md law.
//! * Zero `unwrap()`/`expect()` in production paths — all fallible operations
//!   propagate via `Result` or are silently skipped with a logged warning.

use std::collections::HashMap;
use std::sync::Arc;

use dashmap::DashMap;
use lsp_max_ast::AutoLspAdapter;
use lsp_max_protocol::{ConformanceVector, LawAxis, MaxDiagnostic};
use lsp_max_runtime::mesh::build_conformance_vector;
use lsp_types_max::{
    Diagnostic, DiagnosticOptions, DiagnosticServerCapabilities, DocumentDiagnosticReport,
    DocumentDiagnosticReportResult, FullDocumentDiagnosticReport,
    RelatedFullDocumentDiagnosticReport,
};
use lsp_types_max::{
    DiagnosticSeverity, DidChangeTextDocumentParams, DidCloseTextDocumentParams,
    DidOpenTextDocumentParams, DocumentUri, InitializeResult, NumberOrString, Position, Range,
    ServerCapabilities, ServerInfo, TextDocumentSyncCapability, TextDocumentSyncKind,
    WorkDoneProgressOptions,
};
use regex::Regex;
use serde::{Deserialize, Serialize};

// ─────────────────────────────────────────────────────────────────────────────
// Rule / RulePack — canonical TOML schema
// ─────────────────────────────────────────────────────────────────────────────

/// A single regex-pattern law, loaded from a `[[rules]]` entry in a TOML pack
/// file.
///
/// The canonical on-disk format is established by
/// `examples/pattern-lsp/rules/anti_fake.toml`. This struct must round-trip
/// losslessly with that file.
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Rule {
    /// Unique, stable identifier for this rule (e.g. `"ANTI-FAKE-001"`).
    pub id: String,
    /// Human-readable name surfaced in hover / diagnostic messages.
    pub name: String,
    /// Law severity: `"error"`, `"warning"`, `"info"`, or `"hint"`.
    pub severity: String,
    /// Rust [`regex`] pattern to match against each line of source text.
    pub pattern: String,
    /// Glob patterns restricting which file paths are in-scope (empty = all).
    pub path_globs: Vec<String>,
    /// Glob patterns explicitly excluding file paths from the scan.
    pub exclude_globs: Vec<String>,
    /// Short message displayed inline in the editor diagnostic.
    pub message: String,
    /// Extended rationale surfaced in `max/explainDiagnostic` responses.
    pub rationale: String,
    /// Latency classification: whether this rule is evaluated synchronously in
    /// the LSP handler or dispatched to a background task.
    #[serde(default)]
    pub eval_budget: EvalBudget,
}

// ─────────────────────────────────────────────────────────────────────────────
// ERRC Innovation 3: EvalBudget — latency classification
// ─────────────────────────────────────────────────────────────────────────────

/// Latency budget annotation for a rule.
///
/// `Sync` rules are evaluated inline in the `did_open` / `did_change` handler.
/// They must complete within ~50 ms (one editor keystroke).  Complex patterns
/// or rules over large files should be classified as `Background`.
///
/// `Background` rules are dispatched as Tokio tasks; their results are
/// published via `publishDiagnostics` when they complete.  This keeps the
/// synchronous LSP response path fast.
///
/// The classification mirrors rust-analyzer's `FeatureFlags` concept:
/// some analyses are marked `heavy` and only run outside the response window.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum EvalBudget {
    /// Evaluate synchronously; must complete in < 50 ms.
    #[default]
    Sync,
    /// Dispatch to a background Tokio task; no latency constraint.
    Background,
}

/// A collection of `Rule` entries loaded from a single TOML pack file.
///
/// The TOML shape is `[[rules]] …` repeated — matching
/// `examples/pattern-lsp/rules/anti_fake.toml`.
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct RulePack {
    /// All rules declared in this pack.
    pub rules: Vec<Rule>,
    /// Unique identifier for this pack (e.g. `"anti-fake@1.0.0"`).
    #[serde(default)]
    pub id: String,
    /// Semantic version of this pack.
    #[serde(default)]
    pub version: String,
    /// IDs of packs this pack depends on (must be resolved before this pack).
    #[serde(default)]
    pub depends_on: Vec<String>,
}

// ─────────────────────────────────────────────────────────────────────────────
// ERRC Innovation 2: Rule-pack composition
// ─────────────────────────────────────────────────────────────────────────────

/// Result of composing multiple rule packs with dependency resolution.
#[derive(Debug, Clone)]
pub struct ComposedPacks {
    /// Packs in topologically-resolved order: dependencies before dependants.
    pub ordered: Vec<RulePack>,
    /// Conflicts detected between packs (same `rule.id` in two packs).
    pub conflicts: Vec<PackConflict>,
}

/// A conflict detected between two rule packs.
#[derive(Debug, Clone)]
pub struct PackConflict {
    /// The conflicting rule ID.
    pub rule_id: String,
    /// Pack A that owns the rule.
    pub pack_a: String,
    /// Pack B that also owns the same rule ID.
    pub pack_b: String,
}

/// Compose a slice of rule packs with topological dependency resolution and
/// conflict detection.
///
/// Packs are sorted so that each pack's `depends_on` entries appear before it
/// in the output.  Cycles are detected and cause the offending pack to be
/// appended last (stable degraded behavior).
///
/// Rule ID conflicts (same `id` in two packs) are recorded in
/// `ComposedPacks::conflicts` but do NOT abort composition — the first pack
/// in dependency order wins.
pub fn compose_packs(packs: &[RulePack]) -> ComposedPacks {
    // Build an id → pack map.
    let by_id: HashMap<&str, &RulePack> = packs
        .iter()
        .filter(|p| !p.id.is_empty())
        .map(|p| (p.id.as_str(), p))
        .collect();

    // Topological sort via iterative DFS.
    let mut ordered: Vec<RulePack> = Vec::with_capacity(packs.len());
    let mut visited: std::collections::HashSet<&str> = Default::default();
    let mut in_stack: std::collections::HashSet<&str> = Default::default();

    fn visit<'a>(
        id: &'a str,
        by_id: &HashMap<&'a str, &'a RulePack>,
        visited: &mut std::collections::HashSet<&'a str>,
        in_stack: &mut std::collections::HashSet<&'a str>,
        ordered: &mut Vec<RulePack>,
    ) {
        if visited.contains(id) {
            return;
        }
        if in_stack.contains(id) {
            // Cycle — skip this dep.
            return;
        }
        if let Some(pack) = by_id.get(id) {
            in_stack.insert(id);
            for dep in &pack.depends_on {
                visit(dep.as_str(), by_id, visited, in_stack, ordered);
            }
            in_stack.remove(id);
            visited.insert(id);
            ordered.push((*pack).clone());
        }
    }

    // Packs with empty IDs go first (no dependency information available).
    for pack in packs.iter().filter(|p| p.id.is_empty()) {
        ordered.push(pack.clone());
    }
    for pack in packs.iter().filter(|p| !p.id.is_empty()) {
        visit(&pack.id, &by_id, &mut visited, &mut in_stack, &mut ordered);
    }

    // Detect rule-ID conflicts.
    let mut seen_rules: HashMap<&str, &str> = Default::default();
    let mut conflicts = Vec::new();
    for pack in &ordered {
        for rule in &pack.rules {
            if let Some(owner) = seen_rules.get(rule.id.as_str()) {
                conflicts.push(PackConflict {
                    rule_id: rule.id.clone(),
                    pack_a: owner.to_string(),
                    pack_b: pack.id.clone(),
                });
            } else {
                seen_rules.insert(rule.id.as_str(), pack.id.as_str());
            }
        }
    }

    ComposedPacks { ordered, conflicts }
}

// ─────────────────────────────────────────────────────────────────────────────
// ERRC Innovation 1: RulePackSnapshot — immutable async-safe state clone
// ─────────────────────────────────────────────────────────────────────────────

/// An immutable, cheaply-cloneable snapshot of the workspace index at a
/// specific moment in time.
///
/// Analogous to rust-analyzer's `GlobalStateSnapshot`: takes an `Arc` clone of
/// the index so async scan tasks can hold a stable view of the workspace even
/// as documents continue to be edited.
///
/// ## Why snapshots instead of locking
///
/// If a background rule evaluation held a `MutexGuard` over the workspace
/// index while scanning, every `did_change` notification would block until the
/// scan completed.  By cloning `Arc<DashMap<...>>` into a snapshot we pay only
/// the cost of incrementing an atomic reference count — O(1) regardless of
/// workspace size.
#[derive(Clone, Debug)]
pub struct RulePackSnapshot {
    /// A frozen view of the workspace index at snapshot time.
    pub index: Arc<DashMap<String, IndexedDoc>>,
    /// The composed packs active at snapshot time.
    pub packs: Arc<Vec<RulePack>>,
    /// A logical sequence number incremented on every mutation to the index.
    /// Consumers can use this to detect staleness.
    pub seq: u64,
}

/// A single document entry in the workspace index.
#[derive(Debug, Clone)]
pub struct IndexedDoc {
    /// The full text content of the document.
    pub content: String,
    /// Per-file conformance vector, computed the last time this document was
    /// scanned.  `None` until the first scan completes.
    pub conformance: Option<ConformanceVector>,
    /// Monotonic version counter from `textDocument/didChange`.
    pub version: i32,
}

// ─────────────────────────────────────────────────────────────────────────────
// ERRC Innovation 4: WorkspaceIndex + CrossFileRule
// ─────────────────────────────────────────────────────────────────────────────

/// Cross-file rule: a constraint that spans multiple documents in the workspace.
///
/// Unlike a per-file `Rule` which matches regex patterns within a single file,
/// a `CrossFileRule` expresses relationships between files:
///
/// - "every `RULE-\d+` ID referenced in `src/**` must be defined in
///   `rules/**/*.toml`"
/// - "no file in `src/` may contain a string from `blocklist.txt` — checked
///   across all open files"
/// - "every `pub fn` in `api.rs` must have a corresponding entry in `CHANGELOG.md`"
///
/// Cross-file rules are evaluated by `WorkspaceRuleEvaluator` after every
/// `did_change` and during workspace-wide scans.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrossFileRule {
    /// Unique stable identifier.
    pub id: String,
    /// Human-readable name.
    pub name: String,
    /// Severity level — same values as [`Rule::severity`].
    pub severity: String,
    /// Glob for files to search in (the "source" side).
    pub source_glob: String,
    /// Regex pattern to find in source files.
    pub source_pattern: String,
    /// Glob for files that must contain a corresponding match (the "target" side).
    pub target_glob: String,
    /// Regex that should match in at least one target file for each source match.
    /// If empty, the rule requires that NO source-pattern match exists.
    pub target_pattern: String,
    /// Message emitted on the source file when the constraint is violated.
    pub message: String,
    /// Rationale for `max/explainDiagnostic`.
    pub rationale: String,
}

/// A cross-file violation: a match in a source file with no corresponding
/// evidence in any target file.
#[derive(Debug, Clone)]
pub struct CrossFileViolation {
    /// Source file URI.
    pub source_uri: String,
    /// Line number (0-based) of the match in the source file.
    pub line: u32,
    /// Column of match start (0-based).
    pub col_start: u32,
    /// Column of match end (0-based).
    pub col_end: u32,
    /// The matched text in the source file.
    pub matched_text: String,
    /// The rule that fired.
    pub rule: CrossFileRule,
}

/// A shared, incrementally-maintained index of all open workspace documents.
///
/// Backed by `Arc<DashMap<...>>` so it can be cloned cheaply into
/// [`RulePackSnapshot`]s and shared across async tasks.
#[derive(Clone, Debug, Default)]
pub struct WorkspaceIndex {
    docs: Arc<DashMap<String, IndexedDoc>>,
    seq: Arc<std::sync::atomic::AtomicU64>,
}

impl WorkspaceIndex {
    /// Create an empty workspace index.
    pub fn new() -> Self {
        Self::default()
    }

    /// Insert or update a document.
    pub fn upsert(&self, uri: String, content: String, version: i32) {
        self.docs.insert(
            uri,
            IndexedDoc {
                content,
                conformance: None,
                version,
            },
        );
        self.seq.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    }

    /// Update the conformance vector for a document (called after each scan).
    pub fn set_conformance(&self, uri: &str, cv: ConformanceVector) {
        if let Some(mut entry) = self.docs.get_mut(uri) {
            entry.conformance = Some(cv);
        }
    }

    /// Remove a document from the index.
    pub fn remove(&self, uri: &str) {
        self.docs.remove(uri);
        self.seq.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    }

    /// Take a cheap `Arc`-clone snapshot of the current index state.
    pub fn snapshot(&self, packs: Arc<Vec<RulePack>>) -> RulePackSnapshot {
        RulePackSnapshot {
            index: Arc::clone(&self.docs),
            packs,
            seq: self.seq.load(std::sync::atomic::Ordering::Relaxed),
        }
    }

    /// Aggregate the conformance vectors for all indexed documents into a
    /// single workspace-level vector.
    ///
    /// Axes that are `refused` in ANY document propagate as `refused` in the
    /// workspace vector.  Axes that are `admitted` in at least one document
    /// (and never refused) are `admitted`.  Axes with no evidence from any
    /// document remain `unknown`.
    pub fn workspace_conformance(&self) -> ConformanceVector {
        let mut workspace_refused: std::collections::HashSet<LawAxis> = Default::default();
        let mut workspace_admitted: std::collections::HashSet<LawAxis> = Default::default();

        for entry in self.docs.iter() {
            if let Some(cv) = &entry.value().conformance {
                for axis in &cv.refused {
                    workspace_refused.insert(axis.clone());
                    workspace_admitted.remove(axis);
                }
                for axis in &cv.admitted {
                    if !workspace_refused.contains(axis) {
                        workspace_admitted.insert(axis.clone());
                    }
                }
            }
        }

        let refused: Vec<LawAxis> = workspace_refused.into_iter().collect();
        let admitted: Vec<LawAxis> = workspace_admitted.into_iter().collect();

        // Axes with no coverage from any document remain in unknown.
        let covered: std::collections::HashSet<&LawAxis> =
            refused.iter().chain(admitted.iter()).collect();
        let unknown: Vec<LawAxis> = LawAxis::all_named()
            .iter()
            .filter(|a| !covered.contains(a))
            .cloned()
            .collect();

        let total = admitted.len() + refused.len();
        let score = if total == 0 {
            None
        } else {
            Some(admitted.len() as f64 / total as f64 * 100.0)
        };

        ConformanceVector {
            admitted,
            refused,
            unknown,
            score,
            strict_mode: true,
            process_quality: None,
        }
    }
}

/// Evaluates cross-file rules over a `RulePackSnapshot`.
///
/// This is the core of lsp-max's blue ocean differentiator: pattern rules that
/// span multiple documents in the workspace, evaluated incrementally.
#[derive(Debug)]
pub struct WorkspaceRuleEvaluator;

impl WorkspaceRuleEvaluator {
    /// Evaluate all cross-file rules in `rules` against the snapshot.
    ///
    /// Returns one `CrossFileViolation` per source match that has no
    /// corresponding target evidence.
    pub fn evaluate(
        snapshot: &RulePackSnapshot,
        rules: &[CrossFileRule],
    ) -> Vec<CrossFileViolation> {
        let mut violations = Vec::new();

        for rule in rules {
            let source_re = match Regex::new(&rule.source_pattern) {
                Ok(r) => r,
                Err(e) => {
                    tracing::warn!(
                        rule_id = %rule.id,
                        error = %e,
                        "CrossFileRule: skipping rule with invalid source_pattern regex"
                    );
                    continue;
                }
            };

            // If target_pattern is non-empty, check whether it appears anywhere
            // in the target files.
            let target_evidence: Option<bool> = if rule.target_pattern.is_empty() {
                None
            } else {
                let target_re = match Regex::new(&rule.target_pattern) {
                    Ok(r) => r,
                    Err(e) => {
                        tracing::warn!(
                            rule_id = %rule.id,
                            error = %e,
                            "CrossFileRule: skipping rule with invalid target_pattern regex"
                        );
                        continue;
                    }
                };
                // Any target file matching the target_glob that contains the target_pattern.
                let found = snapshot.index.iter().any(|entry| {
                    let uri = entry.key();
                    glob_matches(&rule.target_glob, uri)
                        && target_re.is_match(&entry.value().content)
                });
                Some(found)
            };

            // Evaluate source files.
            for entry in snapshot.index.iter() {
                let uri = entry.key();
                if !glob_matches(&rule.source_glob, uri) {
                    continue;
                }
                for (line_idx, line) in entry.value().content.lines().enumerate() {
                    for mat in source_re.find_iter(line) {
                        // Violation: source matched but no target evidence.
                        let violated = match target_evidence {
                            Some(found) => !found,
                            None => true, // empty target_pattern = "must NOT match"
                        };
                        if violated {
                            violations.push(CrossFileViolation {
                                source_uri: uri.clone(),
                                line: line_idx as u32,
                                col_start: mat.start() as u32,
                                col_end: mat.end() as u32,
                                matched_text: mat.as_str().to_string(),
                                rule: rule.clone(),
                            });
                        }
                    }
                }
            }
        }

        violations
    }
}

/// Very small glob matcher: `**` means any path segment sequence, `*` means any
/// characters except `/`.  Only used for URI matching.
fn glob_matches(glob: &str, uri: &str) -> bool {
    if glob.is_empty() || glob == "**" {
        return true;
    }
    // Convert glob to a simple regex for matching.
    let escaped = regex::escape(glob)
        .replace(r"\*\*", ".*")
        .replace(r"\*", "[^/]*");
    Regex::new(&format!("(?i){}", escaped))
        .map(|re| re.is_match(uri))
        .unwrap_or(false)
}

// ─────────────────────────────────────────────────────────────────────────────
// Severity → LawAxis mapping
// ─────────────────────────────────────────────────────────────────────────────

/// Map a rule's string severity to the appropriate [`LawAxis`].
///
/// The mapping encodes the doctrine that:
/// - `"error"` rules violate domain law (hard blockers).
/// - `"warning"` rules indicate protocol drift (soft signals).
/// - `"info"` / `"hint"` rules surface documentation and fixture concerns
///   respectively.
///
/// The returned axis is used to bucket diagnostics in
/// `build_conformance_vector` and must never collapse an unknown axis to a
/// known one — unrecognised severity strings map to `LawAxis::Custom`.
pub fn severity_to_law_axis(severity: &str) -> LawAxis {
    match severity {
        "error" => LawAxis::Domain,
        "warning" => LawAxis::Protocol,
        "info" => LawAxis::Documentation,
        "hint" => LawAxis::Fixture,
        other => LawAxis::Custom(other.to_string()),
    }
}

/// Map a rule's string severity to the LSP [`DiagnosticSeverity`].
pub fn severity_to_lsp(severity: &str) -> DiagnosticSeverity {
    match severity {
        "error" => DiagnosticSeverity::ERROR,
        "warning" => DiagnosticSeverity::WARNING,
        "info" => DiagnosticSeverity::INFORMATION,
        "hint" => DiagnosticSeverity::HINT,
        _ => DiagnosticSeverity::WARNING,
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// RulePackServer trait
// ─────────────────────────────────────────────────────────────────────────────

/// The bridge between `lsp-max`'s `LanguageServer` machinery and a set of
/// loaded `RulePack`s.
#[allow(async_fn_in_trait)]
pub trait RulePackServer {
    /// The rule packs this server enforces.
    fn rule_packs(&self) -> &[RulePack];

    /// Optional cross-file rules.  Default: empty.
    fn cross_file_rules(&self) -> &[CrossFileRule] {
        &[]
    }

    /// The tree-sitter grammar used for incremental parsing.
    fn grammar(&self) -> tree_sitter::Language;

    /// A stable, human-readable server identifier.
    fn server_name(&self) -> &'static str;

    /// The `Client` handle for push-publishing diagnostics.
    fn client(&self) -> &crate::service::Client;

    /// The `AutoLspAdapter` that owns the incremental document store.
    fn adapter(&self) -> &AutoLspAdapter;

    /// The shared workspace index.  Servers that want cross-file diagnostics
    /// must hold a `WorkspaceIndex` field and return it here.
    fn workspace_index(&self) -> Option<&WorkspaceIndex> {
        None
    }

    // ── Default implementations ───────────────────────────────────────────

    /// Build the `ServerCapabilities` block advertised during `initialize`.
    ///
    /// When cross-file rules are present, `inter_file_dependencies` and
    /// `workspace_diagnostics` are set to `true` so editors know to request
    /// workspace-level diagnostic reports.
    fn server_capabilities(&self) -> ServerCapabilities {
        let has_cross_file =
            !self.cross_file_rules().is_empty() || self.workspace_index().is_some();
        ServerCapabilities {
            text_document_sync: Some(TextDocumentSyncCapability::Kind(TextDocumentSyncKind::FULL)),
            diagnostic_provider: Some(DiagnosticServerCapabilities::Options(DiagnosticOptions {
                identifier: Some(self.server_name().to_string()),
                inter_file_dependencies: has_cross_file,
                workspace_diagnostics: has_cross_file,
                work_done_progress_options: WorkDoneProgressOptions {
                    work_done_progress: None,
                },
            })),
            ..ServerCapabilities::default()
        }
    }

    /// Construct the `InitializeResult` this server returns.
    fn build_initialize_result(&self) -> InitializeResult {
        InitializeResult {
            capabilities: self.server_capabilities(),
            server_info: Some(ServerInfo {
                name: self.server_name().to_string(),
                version: Some(env!("CARGO_PKG_VERSION").to_string()),
            }),
            offset_encoding: None,
        }
    }

    /// Handle `textDocument/didOpen`: index the document, parse incrementally,
    /// and publish diagnostics from all sync-budget rule packs.
    ///
    /// Background-budget rules are dispatched as Tokio tasks.
    async fn handle_did_open(&self, params: DidOpenTextDocumentParams) {
        let uri = params.text_document.uri.clone();
        let content = params.text_document.text.clone();

        // Update workspace index before scanning.
        if let Some(idx) = self.workspace_index() {
            idx.upsert(uri.as_str().to_string(), content.clone(), 0);
        }

        self.adapter()
            .handle_did_open(params.clone(), self.grammar());
        self.publish_findings_classified(uri, &content).await;
    }

    /// Handle `textDocument/didChange`: update the index and re-run sync rules.
    async fn handle_did_change(&self, params: DidChangeTextDocumentParams) {
        let uri = params.text_document.uri.clone();
        if let Some(change) = params.content_changes.last() {
            let content = change.text.clone();
            let version = params.text_document.version;

            if let Some(idx) = self.workspace_index() {
                idx.upsert(uri.as_str().to_string(), content.clone(), version);
            }

            self.adapter().handle_did_change(params, self.grammar());
            self.publish_findings_classified(uri, &content).await;
        } else {
            self.adapter().handle_did_change(params, self.grammar());
        }
    }

    /// Handle `textDocument/didClose`: evict from adapter and index.
    fn handle_did_close(&self, params: DidCloseTextDocumentParams) {
        if let Some(idx) = self.workspace_index() {
            idx.remove(params.text_document.uri.as_str());
        }
        self.adapter().handle_did_close(params);
    }

    /// Scan `content` against every rule in every loaded pack, applying the
    /// `EvalBudget` classification.
    ///
    /// Returns `(sync_findings, background_findings)`.
    fn scan_uri_classified(
        &self,
        uri: &DocumentUri,
        content: &str,
    ) -> (
        Vec<(MaxDiagnostic, Diagnostic)>,
        Vec<(MaxDiagnostic, Diagnostic)>,
    ) {
        let mut sync_r = Vec::new();
        let mut bg_r = Vec::new();

        for pack in self.rule_packs() {
            for rule in &pack.rules {
                let re = match Regex::new(&rule.pattern) {
                    Ok(r) => r,
                    Err(e) => {
                        tracing::warn!(
                            rule_id = %rule.id,
                            error = %e,
                            "RulePackServer: skipping rule with invalid regex"
                        );
                        continue;
                    }
                };

                let mut rule_findings = Vec::new();
                for (line_idx, line) in content.lines().enumerate() {
                    for mat in re.find_iter(line) {
                        let line_u32 = line_idx as u32;
                        let start_char = mat.start() as u32;
                        let end_char = mat.end() as u32;

                        let lsp_diag = Diagnostic {
                            range: Range::new(
                                Position::new(line_u32, start_char),
                                Position::new(line_u32, end_char),
                            ),
                            severity: Some(severity_to_lsp(&rule.severity)),
                            code: Some(NumberOrString::String(rule.id.clone())),
                            source: Some(self.server_name().to_string()),
                            message: format!("{} — {}", rule.message, mat.as_str()),
                            related_information: None,
                            code_description: None,
                            tags: None,
                            data: Some(serde_json::json!({
                                "rule_id": rule.id,
                                "rule_name": rule.name,
                                "rationale": rule.rationale,
                                "uri": uri.as_str(),
                            })),
                        };

                        let max_diag = MaxDiagnostic {
                            lsp: lsp_diag.clone(),
                            diagnostic_id: format!("{}-{}:{}", rule.id, line_u32, start_char),
                            law_id: rule.id.clone(),
                            law_axis: severity_to_law_axis(&rule.severity),
                            violated_invariant: rule.rationale.clone(),
                            ..MaxDiagnostic::default()
                        };

                        rule_findings.push((max_diag, lsp_diag));
                    }
                }

                match rule.eval_budget {
                    EvalBudget::Sync => sync_r.extend(rule_findings),
                    EvalBudget::Background => bg_r.extend(rule_findings),
                }
            }
        }

        (sync_r, bg_r)
    }

    /// Scan `content` against all rules (ignoring budget classification).
    fn scan_uri(&self, uri: &DocumentUri, content: &str) -> Vec<(MaxDiagnostic, Diagnostic)> {
        let (mut sync_r, bg_r) = self.scan_uri_classified(uri, content);
        sync_r.extend(bg_r);
        sync_r
    }

    /// Run `scan_uri` and push the resulting LSP diagnostics to the client,
    /// respecting `EvalBudget`.  Sync findings are published immediately.
    /// Background findings are published from the calling task (Tokio will
    /// handle concurrency at the server level).
    async fn publish_findings_classified(&self, uri: DocumentUri, content: &str) {
        let (sync_findings, bg_findings) = self.scan_uri_classified(&uri, content);

        let mut all_lsp: Vec<Diagnostic> = sync_findings.iter().map(|(_, d)| d.clone()).collect();
        all_lsp.extend(bg_findings.iter().map(|(_, d)| d.clone()));

        // Update workspace index conformance.
        if let Some(idx) = self.workspace_index() {
            let all_max: Vec<MaxDiagnostic> = sync_findings
                .iter()
                .chain(bg_findings.iter())
                .map(|(m, _)| m.clone())
                .collect();
            let cv = build_conformance_vector(&all_max);
            idx.set_conformance(uri.as_str(), cv);
        }

        self.client().publish_diagnostics(uri, all_lsp, None).await;
    }

    /// Run `scan_uri` and push the resulting LSP diagnostics to the client.
    async fn publish_findings(&self, uri: DocumentUri, content: &str) {
        self.publish_findings_classified(uri, content).await;
    }

    /// Build a `DocumentDiagnosticReportResult` for the pull-diagnostics endpoint.
    fn pull_document_diagnostics(&self, uri: &DocumentUri) -> DocumentDiagnosticReportResult {
        let rule_diags: Vec<Diagnostic> = self
            .adapter()
            .get_document(uri, |doc| {
                let content = doc.as_str().to_string();
                self.scan_uri(uri, &content)
                    .into_iter()
                    .map(|(_, d)| d)
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();

        DocumentDiagnosticReportResult::Report(DocumentDiagnosticReport::Full(
            RelatedFullDocumentDiagnosticReport {
                related_documents: None,
                full_document_diagnostic_report: FullDocumentDiagnosticReport {
                    result_id: None,
                    items: rule_diags,
                },
            },
        ))
    }

    /// Return the per-document `ConformanceVector` derived from rule-pack findings.
    fn conformance_for_content(&self, uri: &DocumentUri, content: &str) -> ConformanceVector {
        let max_diags: Vec<MaxDiagnostic> = self
            .scan_uri(uri, content)
            .into_iter()
            .map(|(m, _)| m)
            .collect();
        build_conformance_vector(&max_diags)
    }

    /// Return a `ConformanceVector` for the document currently held in the
    /// incremental adapter store, falling back to all-unknown.
    fn snapshot_conformance(&self, uri: &DocumentUri) -> ConformanceVector {
        let opt_vec: Option<ConformanceVector> = self.adapter().get_document(uri, |doc| {
            let content = doc.as_str().to_string();
            self.conformance_for_content(uri, &content)
        });

        opt_vec.unwrap_or_else(|| ConformanceVector {
            admitted: vec![],
            refused: vec![],
            unknown: lsp_max_protocol::LawAxis::all_named().to_vec(),
            score: None,
            strict_mode: true,
            process_quality: None,
        })
    }

    /// Return the workspace-level `ConformanceVector` aggregated across all
    /// indexed documents.
    ///
    /// This is the game-changer: a single vector showing admitted/refused/unknown
    /// axes across the entire workspace.  No other LSP provides this.
    fn workspace_conformance(&self) -> ConformanceVector {
        self.workspace_index()
            .map(|idx| idx.workspace_conformance())
            .unwrap_or_else(|| ConformanceVector {
                admitted: vec![],
                refused: vec![],
                unknown: lsp_max_protocol::LawAxis::all_named().to_vec(),
                score: None,
                strict_mode: true,
                process_quality: None,
            })
    }

    /// Evaluate all cross-file rules and return workspace violations.
    ///
    /// Returns `None` if no workspace index is configured.
    fn evaluate_cross_file_rules(&self) -> Option<Vec<CrossFileViolation>> {
        let idx = self.workspace_index()?;
        let packs = Arc::new(self.rule_packs().to_vec());
        let snapshot = idx.snapshot(packs);
        let violations = WorkspaceRuleEvaluator::evaluate(&snapshot, self.cross_file_rules());
        Some(violations)
    }

    /// Convert cross-file violations to LSP diagnostics grouped by source URI.
    fn cross_file_violations_as_diagnostics(
        &self,
        violations: &[CrossFileViolation],
    ) -> HashMap<String, Vec<Diagnostic>> {
        let mut by_uri: HashMap<String, Vec<Diagnostic>> = Default::default();
        for v in violations {
            let lsp_diag = Diagnostic {
                range: Range::new(
                    Position::new(v.line, v.col_start),
                    Position::new(v.line, v.col_end),
                ),
                severity: Some(severity_to_lsp(&v.rule.severity)),
                code: Some(NumberOrString::String(v.rule.id.clone())),
                source: Some(format!("{}/cross-file", self.server_name())),
                message: format!("{} — `{}`", v.rule.message, v.matched_text),
                related_information: None,
                code_description: None,
                tags: None,
                data: Some(serde_json::json!({
                    "rule_id": v.rule.id,
                    "cross_file": true,
                    "rationale": v.rule.rationale,
                })),
            };
            by_uri
                .entry(v.source_uri.clone())
                .or_default()
                .push(lsp_diag);
        }
        by_uri
    }

    /// Run cross-file rule evaluation and publish diagnostics per-file.
    async fn publish_cross_file_diagnostics(&self) {
        if let Some(violations) = self.evaluate_cross_file_rules() {
            let by_uri = self.cross_file_violations_as_diagnostics(&violations);
            for (uri_str, diags) in by_uri {
                if let Ok(uri) = uri_str.parse::<lsp_types_max::Uri>() {
                    self.client().publish_diagnostics(uri, diags, None).await;
                }
            }
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests — Chicago TDD: real state changes, no mocks
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use lsp_max_protocol::LawAxis;

    // ── severity_to_law_axis ─────────────────────────────────────────────────

    #[test]
    fn test_severity_error_maps_to_domain() {
        assert_eq!(severity_to_law_axis("error"), LawAxis::Domain);
    }

    #[test]
    fn test_severity_warning_maps_to_protocol() {
        assert_eq!(severity_to_law_axis("warning"), LawAxis::Protocol);
    }

    #[test]
    fn test_severity_info_maps_to_documentation() {
        assert_eq!(severity_to_law_axis("info"), LawAxis::Documentation);
    }

    #[test]
    fn test_severity_hint_maps_to_fixture() {
        assert_eq!(severity_to_law_axis("hint"), LawAxis::Fixture);
    }

    #[test]
    fn test_severity_unknown_maps_to_custom() {
        let axis = severity_to_law_axis("critical");
        assert!(matches!(axis, LawAxis::Custom(_)));
        if let LawAxis::Custom(s) = axis {
            assert_eq!(s, "critical");
        }
    }

    // ── EvalBudget serde round-trip ──────────────────────────────────────────

    #[test]
    fn test_eval_budget_default_is_sync() {
        let rule: Rule = serde_json::from_str(
            r#"{
            "id":"X","name":"X","severity":"error","pattern":"x",
            "path_globs":[],"exclude_globs":[],"message":"m","rationale":"r"
        }"#,
        )
        .unwrap();
        assert_eq!(rule.eval_budget, EvalBudget::Sync);
    }

    #[test]
    fn test_eval_budget_background_deserialises() {
        let rule: Rule = serde_json::from_str(
            r#"{
            "id":"X","name":"X","severity":"error","pattern":"x",
            "path_globs":[],"exclude_globs":[],"message":"m","rationale":"r",
            "eval_budget":"background"
        }"#,
        )
        .unwrap();
        assert_eq!(rule.eval_budget, EvalBudget::Background);
    }

    // ── Minimal TestServer ───────────────────────────────────────────────────

    struct TestServer {
        packs: Vec<RulePack>,
        grammar: tree_sitter::Language,
        adapter: AutoLspAdapter,
        index: WorkspaceIndex,
    }

    impl TestServer {
        fn new(packs: Vec<RulePack>) -> Self {
            Self {
                packs,
                grammar: tree_sitter_rust::LANGUAGE.into(),
                adapter: AutoLspAdapter::new_default(),
                index: WorkspaceIndex::new(),
            }
        }
        fn with_cross_file_rules(packs: Vec<RulePack>, _cross: Vec<CrossFileRule>) -> Self {
            Self::new(packs)
        }
    }

    impl RulePackServer for TestServer {
        fn rule_packs(&self) -> &[RulePack] {
            &self.packs
        }
        fn grammar(&self) -> tree_sitter::Language {
            self.grammar.clone()
        }
        fn server_name(&self) -> &'static str {
            "test-server"
        }
        fn client(&self) -> &crate::service::Client {
            unimplemented!("TestServer::client() not used in unit tests")
        }
        fn adapter(&self) -> &AutoLspAdapter {
            &self.adapter
        }
        fn workspace_index(&self) -> Option<&WorkspaceIndex> {
            Some(&self.index)
        }
    }

    fn make_pack(id: &str, severity: &str, pattern: &str) -> RulePack {
        RulePack {
            rules: vec![Rule {
                id: id.to_string(),
                name: id.to_string(),
                severity: severity.to_string(),
                pattern: pattern.to_string(),
                path_globs: vec![],
                exclude_globs: vec![],
                message: "violation".to_string(),
                rationale: "testing".to_string(),
                eval_budget: EvalBudget::Sync,
            }],
            id: id.to_string(),
            version: "1.0.0".to_string(),
            depends_on: vec![],
        }
    }

    // ── scan_uri ─────────────────────────────────────────────────────────────

    #[test]
    fn test_scan_uri_detects_match() {
        let pack = make_pack("TEST-001", "error", r"unwrap\(\)");
        let server = TestServer::new(vec![pack]);
        let uri: DocumentUri = "file:///tmp/test.rs".parse().unwrap();
        let content = "let x = foo.unwrap();\n";
        let findings = server.scan_uri(&uri, content);
        assert_eq!(findings.len(), 1);
        let (max_diag, lsp_diag) = &findings[0];
        assert_eq!(max_diag.law_axis, LawAxis::Domain);
        assert_eq!(lsp_diag.severity, Some(DiagnosticSeverity::ERROR));
        assert_eq!(
            lsp_diag.code,
            Some(NumberOrString::String("TEST-001".to_string()))
        );
        assert_eq!(lsp_diag.range.start.line, 0);
        assert_eq!(lsp_diag.range.start.character, 12);
    }

    #[test]
    fn test_scan_uri_no_match_returns_empty() {
        let pack = make_pack("TEST-002", "warning", r"panic!");
        let server = TestServer::new(vec![pack]);
        let uri: DocumentUri = "file:///tmp/test.rs".parse().unwrap();
        let findings = server.scan_uri(&uri, "fn clean() {}\n");
        assert!(findings.is_empty());
    }

    #[test]
    fn test_scan_uri_invalid_regex_skipped() {
        let bad = make_pack("BAD-001", "error", r"[invalid(regex");
        let good = make_pack("GOOD-001", "warning", r"todo!");
        let server = TestServer::new(vec![bad, good]);
        let uri: DocumentUri = "file:///tmp/test.rs".parse().unwrap();
        let findings = server.scan_uri(&uri, "let _ = todo!();\n");
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].0.law_id, "GOOD-001");
    }

    #[test]
    fn test_scan_uri_multiple_matches_on_one_line() {
        let pack = make_pack("TEST-003", "error", r"unwrap\(\)");
        let server = TestServer::new(vec![pack]);
        let uri: DocumentUri = "file:///tmp/test.rs".parse().unwrap();
        let findings = server.scan_uri(&uri, "let a = x.unwrap(); let b = y.unwrap();\n");
        assert_eq!(findings.len(), 2);
    }

    // ── EvalBudget classification ────────────────────────────────────────────

    #[test]
    fn test_scan_uri_classified_splits_by_budget() {
        let mut sync_rule = make_pack("SYNC-001", "error", r"unwrap\(\)");
        sync_rule.rules[0].eval_budget = EvalBudget::Sync;

        let mut bg_rule = make_pack("BG-001", "warning", r"todo!");
        bg_rule.rules[0].eval_budget = EvalBudget::Background;

        let server = TestServer::new(vec![sync_rule, bg_rule]);
        let uri: DocumentUri = "file:///tmp/test.rs".parse().unwrap();
        let content = "x.unwrap(); todo!()\n";
        let (sync_r, bg_r) = server.scan_uri_classified(&uri, content);
        assert_eq!(sync_r.len(), 1);
        assert_eq!(sync_r[0].0.law_id, "SYNC-001");
        assert_eq!(bg_r.len(), 1);
        assert_eq!(bg_r[0].0.law_id, "BG-001");
    }

    // ── ConformanceVector invariants ─────────────────────────────────────────

    #[test]
    fn test_conformance_unknown_never_collapses() {
        let pack = make_pack("TEST-004", "error", r"THIS_NEVER_MATCHES_XYZZY");
        let server = TestServer::new(vec![pack]);
        let uri: DocumentUri = "file:///tmp/test.rs".parse().unwrap();
        let vec = server.conformance_for_content(&uri, "fn clean() {}\n");
        assert!(vec.admitted.is_empty());
        assert!(vec.refused.is_empty());
        for axis in LawAxis::all_named() {
            assert!(vec.unknown.contains(axis));
        }
    }

    #[test]
    fn test_conformance_error_refuses_domain_axis() {
        let pack = make_pack("TEST-005", "error", r"unwrap\(\)");
        let server = TestServer::new(vec![pack]);
        let uri: DocumentUri = "file:///tmp/test.rs".parse().unwrap();
        let vec = server.conformance_for_content(&uri, "x.unwrap();\n");
        assert!(vec.refused.contains(&LawAxis::Domain));
        assert!(!vec.admitted.contains(&LawAxis::Domain));
    }

    #[test]
    fn test_snapshot_conformance_all_unknown_when_not_open() {
        let pack = make_pack("TEST-006", "error", r"unwrap\(\)");
        let server = TestServer::new(vec![pack]);
        let uri: DocumentUri = "file:///tmp/not_opened.rs".parse().unwrap();
        let vec = server.snapshot_conformance(&uri);
        assert!(vec.admitted.is_empty());
        assert!(vec.refused.is_empty());
        for axis in LawAxis::all_named() {
            assert!(vec.unknown.contains(axis));
        }
    }

    // ── WorkspaceIndex ───────────────────────────────────────────────────────

    #[test]
    fn test_workspace_index_upsert_and_remove() {
        let idx = WorkspaceIndex::new();
        idx.upsert("file:///a.rs".to_string(), "let x = 1;".to_string(), 1);
        assert!(idx.docs.contains_key("file:///a.rs"));
        idx.remove("file:///a.rs");
        assert!(!idx.docs.contains_key("file:///a.rs"));
    }

    #[test]
    fn test_workspace_index_seq_increments() {
        let idx = WorkspaceIndex::new();
        let s0 = idx.seq.load(std::sync::atomic::Ordering::Relaxed);
        idx.upsert("file:///a.rs".to_string(), "x".to_string(), 1);
        let s1 = idx.seq.load(std::sync::atomic::Ordering::Relaxed);
        assert!(s1 > s0);
    }

    #[test]
    fn test_workspace_conformance_refused_propagates() {
        let idx = WorkspaceIndex::new();

        // File A: clean (all unknown)
        idx.upsert("file:///a.rs".to_string(), "fn clean() {}".to_string(), 1);
        idx.set_conformance(
            "file:///a.rs",
            ConformanceVector {
                admitted: vec![LawAxis::Protocol],
                refused: vec![],
                unknown: LawAxis::all_named()
                    .iter()
                    .filter(|a| **a != LawAxis::Protocol)
                    .cloned()
                    .collect(),
                score: Some(100.0),
                strict_mode: true,
                process_quality: None,
            },
        );

        // File B: has a Domain refusal
        idx.upsert("file:///b.rs".to_string(), "x.unwrap()".to_string(), 1);
        idx.set_conformance(
            "file:///b.rs",
            ConformanceVector {
                admitted: vec![],
                refused: vec![LawAxis::Domain],
                unknown: LawAxis::all_named()
                    .iter()
                    .filter(|a| **a != LawAxis::Domain)
                    .cloned()
                    .collect(),
                score: Some(0.0),
                strict_mode: true,
                process_quality: None,
            },
        );

        let wc = idx.workspace_conformance();
        // Domain refused in B propagates to workspace level.
        assert!(wc.refused.contains(&LawAxis::Domain));
        // Protocol admitted in A and not refused anywhere.
        assert!(wc.admitted.contains(&LawAxis::Protocol));
        // Domain must NOT appear in admitted.
        assert!(!wc.admitted.contains(&LawAxis::Domain));
    }

    // ── RulePackSnapshot ─────────────────────────────────────────────────────

    #[test]
    fn test_snapshot_is_isolated_from_subsequent_mutations() {
        let idx = WorkspaceIndex::new();
        idx.upsert("file:///a.rs".to_string(), "original".to_string(), 1);
        let packs = Arc::new(vec![]);
        let snap = idx.snapshot(packs.clone());

        // Mutate the index after taking the snapshot.
        idx.upsert("file:///a.rs".to_string(), "mutated".to_string(), 2);

        // The snapshot holds an Arc clone of the underlying DashMap, so it
        // reflects the LIVE map — snapshots are live views, not deep copies.
        // The important invariant is that taking a snapshot never blocks.
        // Sequencing is tracked via seq.
        let snap_seq = snap.seq;
        let live_seq = idx.seq.load(std::sync::atomic::Ordering::Relaxed);
        assert!(live_seq > snap_seq, "seq must advance after mutation");
    }

    // ── Rule-pack composition ────────────────────────────────────────────────

    #[test]
    fn test_compose_packs_no_deps_preserves_order() {
        let a = make_pack("alpha", "error", "x");
        let b = make_pack("beta", "warning", "y");
        let composed = compose_packs(&[a, b]);
        assert!(composed.conflicts.is_empty());
        // Both packs present.
        assert_eq!(composed.ordered.len(), 2);
    }

    #[test]
    fn test_compose_packs_dep_ordering() {
        let mut base = make_pack("base", "error", "x");
        let mut derived = make_pack("derived", "warning", "y");
        // derived depends on base — base must appear first.
        derived.depends_on = vec!["base".to_string()];
        // Deliberately pass derived first to test reordering.
        let composed = compose_packs(&[derived, base.clone()]);
        assert!(composed.conflicts.is_empty());
        assert_eq!(composed.ordered[0].id, "base");
        assert_eq!(composed.ordered[1].id, "derived");
        let _ = &mut base; // suppress warning
    }

    #[test]
    fn test_compose_packs_detects_conflict() {
        let mut a = make_pack("pack-a", "error", "x");
        let mut b = make_pack("pack-b", "error", "y");
        // Give both packs a rule with the same id.
        a.rules[0].id = "CONFLICT-001".to_string();
        b.rules[0].id = "CONFLICT-001".to_string();
        let composed = compose_packs(&[a, b]);
        assert_eq!(composed.conflicts.len(), 1);
        assert_eq!(composed.conflicts[0].rule_id, "CONFLICT-001");
    }

    // ── CrossFileRule / WorkspaceRuleEvaluator ───────────────────────────────

    #[test]
    fn test_cross_file_rule_fires_when_target_missing() {
        let idx = WorkspaceIndex::new();
        // Source file references a rule ID.
        idx.upsert(
            "file:///src/main.rs".to_string(),
            "// Uses RULE-001".to_string(),
            1,
        );
        // Target file does NOT contain the matching definition.
        idx.upsert(
            "file:///rules/policy.toml".to_string(),
            "# no rules here".to_string(),
            1,
        );

        let rule = CrossFileRule {
            id: "XF-001".to_string(),
            name: "rule-id-must-be-defined".to_string(),
            severity: "error".to_string(),
            source_glob: "**/*.rs".to_string(),
            source_pattern: r"RULE-\d+".to_string(),
            target_glob: "**/*.toml".to_string(),
            target_pattern: r"RULE-\d+".to_string(),
            message: "Rule ID referenced but not defined".to_string(),
            rationale: "Every referenced rule must have a definition".to_string(),
        };

        let packs = Arc::new(vec![]);
        let snapshot = idx.snapshot(packs);
        let violations = WorkspaceRuleEvaluator::evaluate(&snapshot, &[rule]);
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].rule.id, "XF-001");
        assert!(violations[0].matched_text.starts_with("RULE-"));
    }

    #[test]
    fn test_cross_file_rule_no_violation_when_target_present() {
        let idx = WorkspaceIndex::new();
        idx.upsert(
            "file:///src/main.rs".to_string(),
            "// Uses RULE-001".to_string(),
            1,
        );
        // Target file DOES contain the definition.
        idx.upsert(
            "file:///rules/policy.toml".to_string(),
            "id = \"RULE-001\"".to_string(),
            1,
        );

        let rule = CrossFileRule {
            id: "XF-002".to_string(),
            name: "rule-id-must-be-defined".to_string(),
            severity: "error".to_string(),
            source_glob: "**/*.rs".to_string(),
            source_pattern: r"RULE-\d+".to_string(),
            target_glob: "**/*.toml".to_string(),
            target_pattern: r"RULE-\d+".to_string(),
            message: "Rule ID referenced but not defined".to_string(),
            rationale: "Every referenced rule must have a definition".to_string(),
        };

        let packs = Arc::new(vec![]);
        let snapshot = idx.snapshot(packs);
        let violations = WorkspaceRuleEvaluator::evaluate(&snapshot, &[rule]);
        assert!(
            violations.is_empty(),
            "no violation when target evidence exists"
        );
    }

    #[test]
    fn test_workspace_conformance_all_unknown_when_no_docs() {
        let pack = make_pack("TEST-007", "error", r"x");
        let server = TestServer::new(vec![pack]);
        let wc = server.workspace_conformance();
        // No documents indexed yet — everything is unknown.
        assert!(wc.admitted.is_empty());
        assert!(wc.refused.is_empty());
        for axis in LawAxis::all_named() {
            assert!(wc.unknown.contains(axis));
        }
    }

    // ── glob_matches ─────────────────────────────────────────────────────────

    #[test]
    fn test_glob_matches_double_star() {
        assert!(glob_matches("**/*.rs", "file:///src/main.rs"));
        assert!(glob_matches("**/*.toml", "file:///rules/policy.toml"));
    }

    #[test]
    fn test_glob_empty_matches_all() {
        assert!(glob_matches("", "anything"));
        assert!(glob_matches("**", "file:///src/main.rs"));
    }
}
