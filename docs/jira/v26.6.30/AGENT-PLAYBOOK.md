# v26.6.30 Agent Playbook: Ticket-Driven Implementation

**Status**: OPEN (pioneer workflow — no published precedent)  
**Research basis**: Deep-research workflow (100 agents, 25 claims verified)  
**Validated components**: Agent tool-integration (2025 CodeAgent paper); Rust build.rs code generation (Rust 1.0+)  
**Not documented**: ggen sync pipeline, JIRA-agent bridging, multi-agent SDLC orchestration — custom engineering required.

---

## Overview

This playbook enables Claude Code agents to implement v26.6.30 tickets end-to-end by reading structured ticket files and producing code. No JIRA API, no ggen sync — just **markdown tickets + agent instructions + Rust scaffolding**.

## Phase 0: Scaffold Setup (Done Locally)

### 1. Create Ticket-Stub Files

Each JIRA ticket (CC-001 through CC-007) becomes a **structured markdown file** that agents can parse:

```markdown
# CC-001: Claude Code LSP Discovery

**Ticket**: CC-001  
**Status**: OPEN  
**Files to modify**:
- `.claude/hooks/discover-lsp-chains.sh` (+150 LOC)

**Implementation stub location**:
- `.claude/hooks/discover-lsp-chains.sh` (exists; agents add functions)

## Acceptance Criteria

- [ ] `discover-lsp-chains.sh` implements `strategy_1_env_var()` function
- [ ] `discover-lsp-chains.sh` implements `strategy_2_settings_json()` function
- [ ] `discover-lsp-chains.sh` implements `strategy_3_process_scan()` function
- [ ] Dedup logic: check `lsp-max.toml` before writing auto stanza
- [ ] JSON `additionalContext` emitted with discovered servers

## Code Sketch

\`\`\`bash
strategy_1_env_var() {
    local env_value="${CLAUDE_LSP_SERVERS:-}"
    # TODO: Agents implement
    # Parse comma-separated: "rust-analyzer:/path,tsserver:command"
    # Extract server ID, command, args
    # Emit [[server]] stanza if not duplicate
}

strategy_2_settings_json() {
    # TODO: Agents implement
    # Read ~/.claude/settings.json or .claude/settings.json
    # Look for lspServers key
    # Parse with jq; extract server registrations
}

strategy_3_process_scan() {
    # TODO: Agents implement (heuristics already exist)
    # Fallback: scan child processes of Claude Code
    # Look for known binary names
}

main() {
    strategy_1_env_var || strategy_2_settings_json || strategy_3_process_scan
    emit_context "CC-001 discovery: $discovered_servers"
}
\`\`\`

## Test Plan

- [ ] Set `CLAUDE_LSP_SERVERS=rust-analyzer:/usr/bin/rust-analyzer` and run → stanza written
- [ ] Unset; verify fallback to process scan
- [ ] Duplicate in `lsp-max.toml` → auto stanza suppressed
- [ ] Script completes in <3s with 10 mock processes
```

### 2. Stub Out Target Files

For each ticket, create empty Rust stubs or shell functions:

```rust
// crates/lsp-max-compositor/src/config.rs — CC-002 additions

pub struct AutoScanConfig {
    pub enabled: bool,
    pub dedup_strategy: String,
    pub probe_timeout_ms: u32,
    pub manage_claude_config: bool,
}

// TODO: Agents implement `load_with_auto()`
// TODO: Agents implement `merge_with_strategy()`
// TODO: Agents implement probe logic
```

### 3. Create Agent Instructions File

`.claude/v26.6.30-agents.md` — tells agents how to work with tickets:

```markdown
# v26.6.30 Agent Implementation Instructions

## Reading a Ticket

1. Open `docs/jira/v26.6.30/CC-00X-*.md`
2. Extract **Files to modify** section (which Rust/shell files to change)
3. Extract **Acceptance Criteria** checklist (what "done" means)
4. Extract **Code Sketch** (starting point; agents complete it)

## Implementation Workflow

**Phase 1: Read (no edits)**
- Open ticket file
- Read ARD-PRD.md section for this ticket
- Identify all file paths and function stubs
- Run `cargo build` to see current compilation errors

**Phase 2: Implement (make edits)**
- Edit target file; fill in `// TODO` stubs
- Run `cargo test -p <crate> <test_name>` after each change
- Keep changes incremental (one function at a time)
- Commit after each acceptance criterion is met

**Phase 3: Verify (no new edits)**
- Run test plan from ticket
- Check all acceptance criteria boxes
- Run `just dx-polish` (fmt + clippy)
- Report: which criteria passed, which blocked

## Ticket Dependencies

- CC-001 → blocks CC-002
- CC-002 → blocks CC-003, CC-004, CC-005, CC-006, CC-007
- CC-003 → blocks CC-004
- CC-004 → blocks CC-005
- CC-006 → depends on CC-001, CC-002, CC-003 (parallel OK after)
- CC-007 → depends on CC-002 (parallel OK after CC-003)

**Safe parallelism**:
- Agent A works on CC-001
- Agent B works on CC-002 (but can't start until CC-001 stubs exist)
- Agent C waits for CC-002 to complete, then starts CC-003
- Agent D waits for CC-003 to complete, then starts CC-004

## Escalation

If compilation fails or a test fails:
1. Run `cargo build 2>&1 | head -50` to get first error
2. Read the error; identify which file/function is broken
3. If unsure how to fix, tag `@cc-review` in commit message
4. Do NOT skip the test or comment it out — escalate instead

## Success Metrics

- All acceptance criteria checkboxes checked
- `just test-pre-publish` passes
- `just dx-verify` passes
- All tests passing
```

---

## Phase 1: Foundation (CC-001, CC-002)

### Agent CC-001: Discovery Scanner

**Ticket**: `docs/jira/v26.6.30/CC-001-claude-code-lsp-discovery.md`

**Task**: Implement three discovery strategies in `.claude/hooks/discover-lsp-chains.sh`

**Agent prompt**:
```
You are implementing CC-001. Read the ticket file at docs/jira/v26.6.30/CC-001-claude-code-lsp-discovery.md
and the code sketch in that ticket. Implement the three strategies:

1. strategy_1_env_var() — parse CLAUDE_LSP_SERVERS env var
2. strategy_2_settings_json() — read Claude Code settings.json
3. strategy_3_process_scan() — scan child processes (heuristics + new)

Each function should:
- Check if the server is already in lsp-max.toml (if so, skip)
- Emit a [[server]] stanza to .claude/lsp-max-auto.toml
- Return 0 on success, 1 if no servers found

Test your implementation by:
- [ ] Setting CLAUDE_LSP_SERVERS and running the script
- [ ] Verifying a stanza is written
- [ ] Adding rust-analyzer to lsp-max.toml and verifying it's skipped in auto
- [ ] Running with 10 mock processes and measuring time (<3s)

After all tests pass, commit with message: "CC-001: Discovery scanner implementation (3-strategy, dedup, <3s)"
```

### Agent CC-002: Config Merge & Probe

**Ticket**: `docs/jira/v26.6.30/CC-002-lsp-max-toml-auto-scan.md`

**Task**: Implement config merge + probe in `crates/lsp-max-compositor/src/config.rs`

**Agent prompt**:
```
You are implementing CC-002. Read the ticket and ARD-PRD sections on CC-002.

Implement in crates/lsp-max-compositor/src/config.rs:

1. CompositorConfig::load_with_auto() — merge static + auto with dedup
2. ServerEntry::probe() — healthcheck server (--version or timeout)
3. [auto_scan] config section parsing

Also add CLI extension to crates/lsp-max-cli/src/nouns/server.rs:
- Add source field (static/auto) to ServerListEntry
- Add law_status field (ADMITTED/CANDIDATE/REFUSED)
- Extend list verb to show both

Test by:
- [ ] Loading lsp-max.toml + auto stanza with conflict → static wins (default)
- [ ] Changing dedup_strategy to "auto-wins" → auto wins
- [ ] Changing to "error-on-conflict" → COMPOSITOR-CONFLICT diagnostic
- [ ] Probing unreachable server → law_status = CANDIDATE
- [ ] Running lsp-max-cli server list → shows source + status

After tests pass, commit: "CC-002: Config merge, probe, hot-reload, CLI extensions"
```

---

## Phase 2: Proxy & Routing (CC-003, CC-004)

### Agent CC-003: Transparent Proxy

**Ticket**: `docs/jira/v26.6.30/CC-003-compositor-transparent-proxy.md`

**Task**: Implement endpoint descriptor + unknown method forwarding

**Agent prompt**:
```
You are implementing CC-003. This ticket depends on CC-002 (config merge).

Implement:

1. .claude/hooks/compositor-start.sh — write compositor-endpoint.json after spawn
   - Format: {"transport": "stdio", "command": "...", "args": [], "pid": NNN}

2. .claude/hooks/discover-lsp-chains.sh extensions — read endpoint and emit [compositor] stanza

3. crates/lsp-max-compositor/src/server.rs — forward unknown methods to primary child
   - In handle_request: if method not in routes, forward to Primary tier
   - Return MethodNotFound only if no Primary available

4. Verify capability_merge.rs is complete (all LSP 3.18 fields)

Test by:
- [ ] Starting compositor → compositor-endpoint.json written with correct PID
- [ ] Reading endpoint in discover-lsp-chains.sh → [compositor] stanza emitted
- [ ] Sending unknown method → forwarded to Primary child
- [ ] Sending initialize → merged capabilities returned (no duplicates)

Commit: "CC-003: Transparent proxy, endpoint descriptor, unknown method forwarding"
```

### Agent CC-004: Per-URI Serialization

**Ticket**: `docs/jira/v26.6.30/CC-004-notification-routing.md`

**Task**: Implement DocumentState + FanoutCoordinator

**Agent prompt**:
```
You are implementing CC-004. This ticket depends on CC-003 (proxy).

This is the most complex ticket — notification ordering is critical.

Implement in crates/lsp-max-compositor/src/server.rs:

1. DocumentState struct:
   - pub version: u32
   - pub open_in: HashSet<ServerId>

2. FanoutCoordinator struct:
   - uri_channels: DashMap<(server_id, uri), mpsc::Sender<DidChangeParams>>
   - doc_state: DashMap<uri, DocumentState>
   - open_gates: DashMap<uri, oneshot::Receiver<()>>

3. Methods:
   - handle_did_open() — fan-out to all children, set gate
   - handle_did_change() — wait for gate, validate version, send per (server, uri)
   - handle_did_close() — remove URI from doc_state, fan-out close

4. Extend crates/lsp-max-cli/src/nouns/snapshot.rs:
   - Add DocumentInfo to snapshot output (uri, version, open_in set)

Test by:
- [ ] Send didOpen then didChange → both received in order by children
- [ ] Send didChange before didOpen completes (mock slow child) → didChange held
- [ ] Send didChange for URI A and URI B concurrently → no interleaving per child
- [ ] Version regression → COMPOSITOR-VERSION-REGRESSION warning (not ANDON)
- [ ] lsp-max-cli snapshot shows open URIs

This is a stress test: 1000+ URIs, 5 children, concurrent changes.

Commit: "CC-004: Per-URI serialization, document state tracking, version validation"
```

---

## Phase 3: Integration (CC-005, CC-006, CC-007) — Parallel

### Agent CC-005: Upstream Diagnostics

**Ticket**: `docs/jira/v26.6.30/CC-005-diagnostic-merge-claude-code.md`

**Task**: Send merged diagnostics upstream to Claude Code

**Agent prompt**:
```
You are implementing CC-005. This depends on CC-004 (serialization complete).

Implement in crates/lsp-max-compositor/src/flush_coordinator.rs:

1. Wire upstream_client into FlushCoordinator (Arc<Client>)
2. After merge, send publishDiagnostics to upstream with merged list
3. Add relatedInformation (source server) to each diagnostic
4. Implement late-deposit follow-up (5ms after 30ms window if new data arrives)

Implement in crates/lsp-max-compositor/src/merge.rs:

5. merge_for_upstream() function:
   - Dedup by (range, code)
   - Keep higher severity
   - REFUSED_BY_LAW always survives
   - Attach server_id to each diagnostic

Extend crates/lsp-max-cli/src/nouns/diagnostics.rs:
6. Add --uri filter to list verb

Test by:
- [ ] Two children emit identical diagnostic → merged list has one
- [ ] ANTI-LLM- code at Hint survives dedup vs Warning from other server
- [ ] Late deposit within 5ms → follow-up sent
- [ ] lsp-max-cli diagnostic list --uri <file> returns merged list
- [ ] Mock Claude Code receives publishDiagnostics after every flush

Commit: "CC-005: Upstream publishDiagnostics, dedup, late-deposit follow-up, source attribution"
```

### Agent CC-006: Auto-Config Hook

**Ticket**: `docs/jira/v26.6.30/CC-006-session-start-hook.md`

**Task**: Create hook that configures Claude Code LSP settings

**Agent prompt**:
```
You are implementing CC-006. This depends on CC-001/002/003 (discovery + merge + proxy).

Create .claude/hooks/configure-claude-code-lsp.sh:

1. Wait for compositor-endpoint.json to exist (5s timeout)
2. Check manage_claude_config flag in lsp-max.toml
3. If flag is false, log and exit (graceful degradation)
4. If flag is true:
   - Read merged registry (lsp-max.toml + auto.toml)
   - Extract all unique primary_extensions
   - Map extensions to Claude Code language IDs
   - Generate lsp section in .claude/settings.json
   - Point each language to the compositor command

5. Add hook to .claude/settings.json SessionStart:startup (after compositor-start.sh)

Test by:
- [ ] manage_claude_config = false → hook skips update
- [ ] manage_claude_config = true → lsp section written
- [ ] Compositor endpoint missing → graceful exit 0
- [ ] Existing LSP entries → updated not duplicated
- [ ] additionalContext shows: "LSP routing: compositor ADMITTED — [servers]"

Commit: "CC-006: SessionStart hook, auto-configure Claude Code LSP routing, graceful fallback"
```

### Agent CC-007: LSIF Tier

**Ticket**: `docs/jira/v26.6.30/CC-007-lsif-tier.md`

**Task**: Add read-only LSIF tier support

**Agent prompt**:
```
You are implementing CC-007. This depends on CC-002 (config) and CC-003 (proxy).

Implement:

1. crates/lsp-max-compositor/src/registry.rs:
   - Add ChildTier::Lsif variant
   - Implement staleness check (dump mtime vs threshold + git commit)

2. crates/lsp-max-compositor/src/routing.rs:
   - Add RoutingStrategy::FallbackToLsif
   - Add RoutingDecision::FallbackToLsif
   - Routing: definition/references/hover → try Primary, fall back to LSIF
   - Routing: didChange/didClose → exclude LSIF (never notify)

3. crates/lsp-max-compositor/src/config.rs:
   - Extend ServerEntry with lsif_dump_path, lsif_max_age_hours
   - Implement ServerEntry::validate_lsif() (dump must exist, andon_code_prefixes must be empty)

4. crates/lsp-max-cli/src/nouns/server.rs:
   - Extend list verb: show tier=lsif with dump path and staleness status

5. lsp-max.toml:
   - Add comment block documenting priority = "lsif"

Test by:
- [ ] priority = "lsif" server: didChange fan-out excludes it
- [ ] definition with Primary returning null → LSIF server queried, result returned
- [ ] definition with Primary returning result → LSIF not queried
- [ ] Missing lsif_dump_path → CANDIDATE + COMPOSITOR-LSIF-DUMP-MISSING
- [ ] Stale dump (>24h or older than git commit) → COMPOSITOR-LSIF-STALE (warning)
- [ ] andon_code_prefixes non-empty → validation error at load

Commit: "CC-007: LSIF tier, fallback navigation, staleness detection, validation"
```

---

## Parallel Execution Strategy

**Safe to run in parallel** (after dependencies):

```
Session Start:
  CC-001 (Day 1)
    ↓
  CC-002 (Day 3) — depends CC-001 stubs ✓
    ├─→ CC-003 (Day 5) — can start now
    ├─→ CC-005 (Day 5) — can start now (mocks CC-004 if needed)
    ├─→ CC-006 (Day 5) — can start now
    └─→ CC-007 (Day 5) — can start now
    
  CC-004 (Day 7) — depends CC-003 ✓
    └─→ Re-run CC-005 (Day 9) — real integration
```

**Total duration**: 9 days (1 agent per ticket, sequential dependencies only)

**Concurrency**: Up to 4 agents after Day 5 (CC-003/005/006/007 parallel on CC-002)

---

## Testing Strategy

### Per-Ticket Testing

Each ticket includes a **test plan checklist**. Agents run tests and check boxes.

### Integration Testing (Final)

After all 7 tickets complete:

```bash
# 1. Fresh session bootstrap
bash .claude/setup.sh
git pull

# 2. Run full pipeline
just test-pre-publish

# 3. Smoke test: start compositor, check gate
lsp-max-cli gate check
lsp-max-cli server list

# 4. End-to-end: 5 servers, auto-discovery, diagnostics merge
# (run in Claude Code with a real editor)
```

---

## Escalation & Code Review

### When to Escalate

1. **Compilation fails** after >30 min of debugging → tag `@cc-review` in PR
2. **Test fails** (not your mistake) → check ARD-PRD for assumptions; escalate if unfounded
3. **Dependency ticket incomplete** → block and report; don't work around
4. **Acceptance criteria unclear** → read ARD-PRD **full ticket section** (not just checklist)

### Code Review Checklist

Before commit:

- [ ] `cargo build` succeeds
- [ ] `cargo test -p <crate>` passes all tests
- [ ] `just dx-polish` (fmt + clippy) passes
- [ ] All acceptance criteria checked
- [ ] Commit message includes ticket ID: `CC-00X: description`
- [ ] No TODOs left in code (or marked explicitly as blocked on another ticket)

---

## Success Criteria (DoD)

**v26.6.30 is shipped when**:

1. All 7 agents report: "All acceptance criteria passed ✓"
2. `just test-pre-publish` passes
3. `just dx-verify` clean
4. No escalations pending (all `@cc-review` tags resolved)
5. Fresh session: auto-discovery → merge → proxy → diagnostics merge ✓
