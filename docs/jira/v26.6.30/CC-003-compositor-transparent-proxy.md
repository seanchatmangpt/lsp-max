# CC-003 — Compositor as Transparent LSP Proxy for Claude Code

**Status**: OPEN  
**Component**: `crates/lsp-max-compositor/src/server.rs`, `crates/lsp-max-compositor/src/main.rs`  
**Depends on**: CC-002  
**Blocked by**: nothing

## Problem

The compositor currently assumes it is started manually by the developer or CI. For Claude Code
integration, the compositor must be:

1. **Auto-started at session boot** — the `compositor-start.sh` hook already does this, but the
   compositor may not be in PATH in all environments.
2. **Addressable by Claude Code** — Claude Code needs to know the compositor's stdio endpoint or
   TCP port to bind its LSP connection to it instead of to child servers directly.
3. **Transparent to Claude Code** — Claude Code must not observe any behavior difference from
   talking to the compositor vs. a native language server. `initialize` handshake must succeed
   with merged capabilities from all child servers.

## Current State

`compositor-start.sh` starts the compositor in the background. Claude Code is not wired to use
it — the `discover-lsp-chains.sh` hook writes child server stanzas directly, so Claude Code
would connect to them directly if it reads the auto config. There is no mechanism to redirect
Claude Code's LSP binding to the compositor.

## Acceptance Criteria

- [ ] The compositor exposes its own `initialize` response with capabilities merged from all
      child servers (CC-004 covers the merge details; this ticket covers the handshake shell).
- [ ] `compositor-start.sh` writes a **compositor descriptor file** at
      `.claude/compositor-endpoint.json`:
      ```json
      { "transport": "stdio", "command": "/path/to/lsp-max-compositor", "args": [], "pid": 12345 }
      ```
- [ ] `discover-lsp-chains.sh` (CC-001) reads the descriptor and emits a special
      `[[compositor]]` stanza (or a `[compositor]` table) into `lsp-max-auto.toml` that Claude
      Code's config layer can use to substitute the compositor for direct child server connections.
- [ ] If the compositor is not running when `discover-lsp-chains.sh` runs, it starts it
      synchronously (with a 5s timeout) before writing the stanza.
- [ ] The compositor must forward **all** LSP methods it does not explicitly handle to the
      appropriate child server. Unknown methods must not return `MethodNotFound` to Claude Code.

## Transparent Proxy Requirement

Claude Code's LSP client negotiates capabilities during `initialize`. The compositor must:

1. Collect `ServerCapabilities` from all child `initialize` responses.
2. Merge them (see `capability_merge.rs`) and return the union to Claude Code.
3. Track which child advertised which capability so routing resolves correctly.

The compositor must **not** advertise capabilities that no child server supports. Advertising a
capability and then returning `null`/error on a request causes Claude Code to fall back or fail
silently — this is a law violation (ANTI-LLM-CLAIM category).

## Files to Modify

- `.claude/hooks/compositor-start.sh` — write `compositor-endpoint.json` after start
- `.claude/hooks/discover-lsp-chains.sh` — read descriptor; wire compositor as the Claude Code endpoint
- `crates/lsp-max-compositor/src/server.rs` — forward unknown methods to primary child
- `crates/lsp-max-compositor/src/capability_merge.rs` — ensure union merge is complete

## Test Plan

- [ ] Start compositor; verify `compositor-endpoint.json` written with correct PID
- [ ] Kill compositor; restart via `discover-lsp-chains.sh`; verify it starts and descriptor updated
- [ ] Send `initialize` to compositor; verify response includes capabilities from all children
- [ ] Send unknown method `textDocument/fakeMethod`; verify forwarded to primary child, not rejected
- [ ] Verify compositor never advertises capability absent from all children
