# lsp-max Roadmap

**As of:** 2026-06-12
**Baseline commit:** 6ee4255

---

## Completed

### ERRC Blue Ocean Innovations (committed 6ee4255)
- `RulePackSnapshot` — Arc<DashMap> O(1) async-safe workspace state clone; mirrors rust-analyzer GlobalStateSnapshot
- `compose_packs()` — topological dependency resolution + conflict detection between packs
- `EvalBudget` — per-rule `Sync` / `Background` latency classification; TOML serde; scan_uri_classified()
- `WorkspaceIndex` + `CrossFileRule` + `WorkspaceRuleEvaluator` — incremental DashMap doc index; workspace_conformance() aggregation; cross-file pattern constraints

### Protocol surface
- Four new `max/` Request types: `MaxRulePacks`, `MaxRulePackStatus`, `MaxRulePackDiff`, `MaxWorkspaceConformance`
- Method constants: `max/rulePacks`, `max/rulePackStatus`, `max/rulePackDiff`, `max/workspaceConformance`
- `RulePackDescriptor`, `RulePackStatusResult`, `RulePackDiffEntry` protocol structs
- `inter_file_dependencies` + `workspace_diagnostics` auto-enabled in `server_capabilities()` when cross-file rules present

### Coverage
- LSP 3.18 coverage matrix (`src/coverage/lsp_coverage.rs`) — watermark calibrated to 75% (22 client-push/transport methods correctly excluded)
- LSIF coverage matrix (`crates/lsp-max-lsif/`) — 18 coverage tests green
- 27 Chicago TDD tests for all ERRC innovations

### Infrastructure
- ggen-lsp plugin wired in `.claude/settings.json` (`enabledPlugins`)
- Default model set to `claude-sonnet-4-6`
- `.agents/` added to `.gitignore`

---

## Remaining Work (1 item)

### 1. `anti-llm-cheat-lsp` adoption of `RulePackServer` trait

**What:** `pattern-lsp`, `axum-lsp`, `anti-llm-cheat-lsp` each hand-roll the scanner / rules-loader / publish-diagnostics / conformance loop. `RulePackServer` was built to eliminate exactly this duplication. Nothing uses it yet.

**Status:** `pattern-lsp` ✅ and `axum-lsp` ✅ fully refactored to `RulePackServer` + `WorkspaceIndex`.

**Remaining:** `anti-llm-cheat-lsp` uses `AhoCorasick` + `engine::scan_directory()` + `virtual_docs` module — a fundamentally different architecture. Needs its own approach (replace `AntiLlmEngine` scanner with `RulePackServer::scan_uri()`).

**Files to change:**
- `examples/anti-llm-cheat-lsp/src/engine.rs` — replace `AntiLlmEngine` scanner with `RulePackServer::scan_uri()`
- `examples/anti-llm-cheat-lsp/src/main.rs` — add `index: WorkspaceIndex` field; override `workspace_index()`

**Definition of done:** `anti-llm-cheat-lsp` implements only `rule_packs()`, `grammar()`, `server_name()`, `client()`, `adapter()`. Zero hand-rolled AhoCorasick loops remain in the server path.

---

### 2. `WorkspaceIndex` wiring in examples

**What:** `workspace_index()` defaults to `None` in the `RulePackServer` trait. Cross-file diagnostics are silently disabled in every concrete server until they hold a `WorkspaceIndex` field and override `workspace_index()`.

**Files to change:**
- Each example server struct: add `index: WorkspaceIndex` field
- Each `impl RulePackServer`: override `workspace_index()` to return `Some(&self.index)`
- Each `LanguageServer::did_open` / `did_change` / `did_close`: delegate to `handle_did_open` / `handle_did_change` / `handle_did_close` (which already call `idx.upsert` / `idx.remove`)

**Unlock:** once wired, `publish_cross_file_diagnostics()` and `workspace_conformance()` activate automatically.

---

### 3. JSON-RPC handler wiring for new `max/` methods ✅ DONE

Implemented in `src/language_server/impls/snapshot.rs` and wired via `src/language_server.rs`:
- `max/rulePacks` → `max_rule_packs()` — groups diagnostics by pack_id prefix
- `max/rulePackStatus` → `max_rule_pack_status(pack_id)` — per-pack `RulePackStatusResult`
- `max/rulePackDiff` → `max_rule_pack_diff(params)` — diff against `cleared_diagnostics` set
- `max/workspaceConformance` → `max_workspace_conformance()` — refused-propagates lattice aggregation

---

### 4. ggen SLO: "one `ggen sync` produces a complete lsp-max" ✅ DONE

All required artifacts now exist:

```
ggen/
  .specify/specs/lsp-max/
    lsp.ttl             # OWL ontology: lsp:Server, lsp:RulePack, lsp:Rule, lsp:LawAxis
    ggen.toml           # Manifest wiring 4 SPARQL→template→output rules
  queries/lsp-max/
    list_servers.rq     # SELECT ?server_name ?diagnostic_source_id ?grammar_crate
    list_rules.rq       # SELECT all rule fields (pack→rule chain)
  templates/lsp-max/
    backend.rs.tera     # generates src/server.rs (struct + RulePackServer impl)
    cli.rs.tera         # generates src/main.rs (clap CLI + stdio/TCP entry points)
    semantics.rs.tera   # generates src/semantics.rs (static_packs() + type re-exports)
    build.rs.tera       # generates build.rs (tree-sitter compile hook)
```

**Invoke with:**
```bash
ggen sync --manifest .specify/specs/lsp-max/ggen.toml
# → generated/lsp-max/<server_name>/src/{server.rs,main.rs,semantics.rs} + build.rs
```

---

## Priority Order

| # | Item | Effort | Status |
|---|------|--------|--------|
| 1 | pattern-lsp → RulePackServer | Small | ✅ Done |
| 2 | axum-lsp → RulePackServer + WorkspaceIndex | Small | ✅ Done |
| 3 | max/ handler wiring (4 methods) | Medium | ✅ Done |
| 4 | ggen SLO (ontology + SPARQL + templates + manifest) | Large | ✅ Done |
| 5 | anti-llm-cheat-lsp → RulePackServer | Medium | ⬜ Remaining |
