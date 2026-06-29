# CC-001 — Claude Code LSP Discovery: Scan Active Servers

**Status**: OPEN  
**Component**: `.claude/hooks/discover-lsp-chains.sh`  
**Depends on**: nothing  
**Blocked by**: nothing

## Problem

`discover-lsp-chains.sh` currently uses heuristic file-presence checks (`Cargo.toml` → rust-analyzer,
`package.json` → tsserver, etc.) to discover language servers. It does not know which LSP servers
Claude Code is *actually connected to* in the current session. If Claude Code connects to a server
not covered by the heuristics, lsp-max cannot intercept that traffic.

Claude Code exposes its active LSP connections through:

1. **`~/.claude/mcp_servers.json`** (or equivalent config) — the registered MCP/LSP server list.
2. **`CLAUDE_LSP_SERVERS` env var** — set by newer Claude Code builds to enumerate active servers.
3. **Process inspection** — Claude Code spawns language servers as child processes; their command
   lines are discoverable via `ps` or `/proc`.

## Acceptance Criteria

- [ ] `discover-lsp-chains.sh` attempts all three discovery strategies in order, stopping at the
      first that yields results.
- [ ] Each discovered server produces one `[[server]]` stanza in `.claude/lsp-max-auto.toml`.
- [ ] If a server already appears in `lsp-max.toml` (by `id`), the auto stanza is **skipped** to
      avoid duplicate registration.
- [ ] The hook emits a JSON `additionalContext` line listing which servers were discovered by which
      strategy: `CC-001 discovery: rust-analyzer (heuristic), tsserver (process-scan)`.
- [ ] Discovery completes in <3s. Process scan uses `pgrep` / `ps -o pid,comm,args` with a
      10-server depth limit.

## Implementation Notes

### Strategy 1 — CLAUDE_LSP_SERVERS env var

```bash
if [[ -n "${CLAUDE_LSP_SERVERS:-}" ]]; then
    # comma-separated: "rust-analyzer:/path/to/bin,.ts:typescript-language-server --stdio"
    IFS=',' read -ra entries <<< "$CLAUDE_LSP_SERVERS"
    for entry in "${entries[@]}"; do
        # parse and emit [[server]] stanza
    done
fi
```

### Strategy 2 — Claude Code settings.json

Claude Code stores LSP server configurations in `~/.claude/settings.json` under a `lspServers`
key (format TBD — check Claude Code docs / source). Parse with `jq` if available.

### Strategy 3 — Process scan (existing heuristics + new)

Keep the current file-presence heuristics as fallback. Augment with:

```bash
# Find language server processes spawned as children of the Claude Code process
CLAUDE_PID=$(pgrep -f "claude" | head -1)
if [[ -n "$CLAUDE_PID" ]]; then
    ps --ppid "$CLAUDE_PID" -o comm,args --no-headers 2>/dev/null | head -20
fi
```

Map known binary names to server IDs and `lsp-max.toml` stanza templates.

## Files to Modify

- `.claude/hooks/discover-lsp-chains.sh` — add strategies 1 and 2 before heuristics
- `.claude/lsp-max-auto.toml` — output file (unchanged format)
- `.claude/settings.json` — no changes needed (hook is already wired to SessionStart:startup)

## Test Plan

- [ ] Set `CLAUDE_LSP_SERVERS=rust-analyzer:/usr/bin/rust-analyzer` and run hook; verify stanza written
- [ ] Remove env var; verify fallback to process scan (mock with a background `sleep` process)
- [ ] Duplicate detection: add rust-analyzer to `lsp-max.toml`; verify auto stanza is suppressed
- [ ] Hook completes in <3s with 10 mock processes in the process table
