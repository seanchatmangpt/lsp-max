# CC-006 — SessionStart Hook: Auto-Configure lsp-max as Claude Code's LSP

**Status**: OPEN  
**Component**: `.claude/hooks/`, `.claude/settings.json`  
**Depends on**: CC-001, CC-002, CC-003  
**Blocked by**: nothing

## Problem

The current `SessionStart` hook sequence is:

1. `session-start.sh` — status report (branch, dirty files, toolchain)
2. `discover-lsp-chains.sh` — writes `.claude/lsp-max-auto.toml`
3. `compositor-start.sh` (async) — starts the compositor process

The gap: after step 3, Claude Code does not automatically redirect its LSP connections to the
compositor. Claude Code determines which LSP server to use via its own config (project
`.claude/settings.json` or user-level config). There is no step that writes Claude Code's LSP
config to point at the compositor.

## What Claude Code Reads

Claude Code's LSP configuration (LSP tool in settings.json) supports:

```json
{
  "lsp": {
    "rust": {
      "command": "/path/to/server",
      "args": [],
      "enabled": true
    }
  }
}
```

The key insight from the auto-discovery research: if we write `.claude/settings.json` (or the
project-level override) with the compositor as the command for each discovered extension, Claude
Code will connect to the compositor instead of the child servers directly.

## Acceptance Criteria

- [ ] A new hook step `configure-claude-code-lsp.sh` runs after `compositor-start.sh` confirms
      the compositor is alive (reads `compositor-endpoint.json`).
- [ ] The script reads the merged server registry (from `lsp-max.toml` + auto.toml) and generates
      a `lsp` section in `.claude/settings.json` that maps each `primary_extension` to the
      compositor command.
- [ ] The script does not overwrite any user-set LSP entries unless `[auto_scan].manage_claude_config = true`
      is set in `lsp-max.toml`. Default: `false` (write only if the key is absent).
- [ ] On session end or compositor shutdown, the `lsp` entries added by lsp-max are removed or
      reverted so Claude Code falls back to its direct server connections.
- [ ] The `additionalContext` emitted by `session-start.sh` includes the LSP routing status:
      ```
      LSP routing: compositor ADMITTED — rust-analyzer, tsserver, wasm4pm-lsp → lsp-max-compositor
      ```
- [ ] If the compositor fails to start (timeout), the script logs the failure and leaves Claude
      Code's LSP config unchanged (graceful degradation — Claude Code connects directly).

## Hook Sequence After This Ticket

```
SessionStart:startup
  1. session-start.sh           → status report
  2. discover-lsp-chains.sh     → write lsp-max-auto.toml  (CC-001)
  3. compositor-start.sh        → start compositor, write compositor-endpoint.json  (CC-003)
  4. configure-claude-code-lsp.sh → write .claude/settings.json lsp section  (this ticket)
  5. (additionalContext injected into Claude Code's first prompt)
```

## Script Skeleton

```bash
#!/usr/bin/env bash
# configure-claude-code-lsp.sh
# Writes .claude/settings.json [lsp] section to route all Claude Code LSP
# connections through the lsp-max compositor.

ENDPOINT="${CLAUDE_PROJECT_DIR}/.claude/compositor-endpoint.json"
SETTINGS="${CLAUDE_PROJECT_DIR}/.claude/settings.json"
TOML="${CLAUDE_PROJECT_DIR}/lsp-max.toml"

# Wait for compositor to be live (up to 5s)
for i in {1..10}; do
    [[ -f "$ENDPOINT" ]] && break
    sleep 0.5
done
[[ ! -f "$ENDPOINT" ]] && { echo "CC-006: compositor endpoint not found; LSP config unchanged"; exit 0; }

COMPOSITOR_CMD=$(jq -r '.command' "$ENDPOINT")

# Check manage_claude_config flag
MANAGE=$(python3 -c "
import re, sys
t = open('$TOML').read()
m = re.search(r'manage_claude_config\s*=\s*(true|false)', t)
print(m.group(1) if m else 'false')
")

if [[ "$MANAGE" != "true" ]]; then
    echo "CC-006: manage_claude_config=false; skipping .claude/settings.json update"
    exit 0
fi

# Build lsp section from merged registry
# (implementation: parse both toml files, collect primary_extensions, emit JSON)
# ...

echo "CC-006: LSP routing configured → $COMPOSITOR_CMD"
```

## Files to Create / Modify

- `.claude/hooks/configure-claude-code-lsp.sh` — new script (create)
- `.claude/settings.json` — add hook entry for `configure-claude-code-lsp.sh` in `SessionStart:startup`
- `lsp-max.toml` — add `manage_claude_config = false` under `[auto_scan]`

## Test Plan

- [ ] `manage_claude_config = false` (default): script runs, logs skip message, settings.json unchanged
- [ ] `manage_claude_config = true`: script writes `lsp` section with compositor command for each extension
- [ ] Compositor endpoint absent after 5s: script exits 0, logs failure, settings unchanged
- [ ] Second session start: existing compositor command entries updated (not duplicated)
- [ ] Compositor shut down: on next start, stale entries removed before rewrite
