# CC-002 — lsp-max.toml Auto-Scan and Merge Pipeline

**Status**: OPEN  
**Component**: `crates/lsp-max-compositor/src/registry_init.rs`, `lsp-max.toml`  
**Depends on**: CC-001  
**Blocked by**: nothing

## Problem

The compositor currently loads config from two sources:

1. `lsp-max.toml` — hand-authored, project-specific servers (wasm4pm-lsp, anti-llm-cheat-lsp, etc.)
2. `.claude/lsp-max-auto.toml` — auto-generated at session start by `discover-lsp-chains.sh`

These are merged at startup via `CompositorConfig::load_with_auto()`. The current merge is
additive: auto stanzas are appended after static stanzas. There is no:

- Deduplication by server `id`
- Conflict resolution when the same server appears in both files with different `command` paths
- Hot-reload when `lsp-max.toml` changes (the `FileChanged` hook calls `lsp_reload_config` but
  the reload path in the compositor is not implemented)
- Validation that discovered servers are actually reachable before registration

## Acceptance Criteria

- [ ] `CompositorConfig::load_with_auto()` deduplicates by `id`: static `lsp-max.toml` entries
      win over auto entries on conflict (project config is authoritative).
- [ ] After merge, each server stanza is **probed**: compositor attempts to spawn the command with
      `--version` or a zero-timeout healthcheck. Unreachable servers are logged as `CANDIDATE`
      (not `ADMITTED`) and skipped from routing until a reload confirms them.
- [ ] `lsp_reload_config` MCP tool path is wired: `FileChanged` on `lsp-max.toml` triggers a
      live re-merge without restarting the compositor.
- [ ] `lsp-max.toml` gains an `[auto_scan]` section:

```toml
[auto_scan]
enabled = true           # default true; set false to disable discover-lsp-chains
dedup_strategy = "static-wins"  # or "auto-wins" | "error-on-conflict"
probe_timeout_ms = 500
```

- [ ] CLI verb `lsp-max-cli server list` shows merged config with source (`static` / `auto`) and
      probe status (`ADMITTED` / `CANDIDATE` / `REFUSED`).

## Merge Algorithm

```
merged = {}
for stanza in load("lsp-max.toml"):
    merged[stanza.id] = (stanza, "static")

for stanza in load(".claude/lsp-max-auto.toml"):
    if stanza.id not in merged:
        merged[stanza.id] = (stanza, "auto")
    elif dedup_strategy == "auto-wins":
        merged[stanza.id] = (stanza, "auto")
    elif dedup_strategy == "error-on-conflict":
        emit ANDON diagnostic: COMPOSITOR-CONFLICT-{id}
    # else: static-wins → skip auto stanza (default)

for (stanza, source) in merged.values():
    probe(stanza.command)  → sets law_status = ADMITTED | CANDIDATE
    registry.register(stanza, source, law_status)
```

## Files to Modify

- `crates/lsp-max-compositor/src/registry_init.rs` — implement merge algorithm + probe
- `lsp-max.toml` — add `[auto_scan]` section
- `crates/lsp-max-cli/src/nouns/server.rs` — extend `list` verb to show source + probe status
- `crates/lsp-max-compositor/src/server.rs` — wire `lsp_reload_config` handler

## Test Plan

- [ ] Both files have `id = "rust-analyzer"` → static entry wins; auto suppressed
- [ ] `dedup_strategy = "auto-wins"` → auto entry wins
- [ ] `dedup_strategy = "error-on-conflict"` → ANDON diagnostic emitted
- [ ] Unreachable command → server registered as CANDIDATE, not routed
- [ ] Edit `lsp-max.toml` → `FileChanged` hook fires → compositor hot-reloads within 1s
- [ ] `lsp-max-cli server list` shows `source=static|auto` and `status=ADMITTED|CANDIDATE`
