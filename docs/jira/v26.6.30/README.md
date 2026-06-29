# v26.6.30 — Full Claude Code LSP Client Support

**CalVer**: 26.6.30  
**Status**: OPEN  
**Goal**: lsp-max becomes the transparent LSP proxy for every language server Claude Code connects
to. `lsp-max.toml` (and the auto-generated `.claude/lsp-max-auto.toml`) describe the full set of
servers; the compositor is the single LSP endpoint Claude Code binds; all traffic is routed,
merged, and law-checked before delivery.

## Epic Summary

Claude Code connects to LSP servers per file extension. Today it connects directly. After this
epic, lsp-max sits between Claude Code and every language server — as a transparent compositor
that adds diagnostics fanout, law-state enforcement, ANDON gating, and OCEL process mining to
every LSP session Claude Code opens.

## Tickets

| ID | Title | Status |
|---|---|---|
| [CC-001](CC-001-claude-code-lsp-discovery.md) | Claude Code LSP discovery — scan active servers | OPEN |
| [CC-002](CC-002-lsp-max-toml-auto-scan.md) | lsp-max.toml auto-scan and merge pipeline | OPEN |
| [CC-003](CC-003-compositor-transparent-proxy.md) | Compositor as transparent LSP proxy for Claude Code | OPEN |
| [CC-004](CC-004-notification-routing.md) | Notification routing — didOpen/didChange/didClose fan-out | OPEN |
| [CC-005](CC-005-diagnostic-merge-claude-code.md) | publishDiagnostics merge for Claude Code consumer | OPEN |
| [CC-006](CC-006-session-start-hook.md) | SessionStart hook — auto-configure lsp-max as Claude Code's LSP | OPEN |
| [CC-007](CC-007-lsif-tier.md) | LSIF DiagnosticsOnly tier for offline navigation | OPEN |

## Architecture After This Epic

```
Claude Code
    │  (one LSP connection per file type)
    ▼
lsp-max compositor  ←── lsp-max.toml + .claude/lsp-max-auto.toml
    │
    ├─ FanAll ──► rust-analyzer
    ├─ FanAll ──► wasm4pm-lsp
    ├─ FanAll ──► anti-llm-cheat-lsp
    ├─ FanAll ──► <any server discovered at session start>
    │
    └─ Merge diagnostics → publishDiagnostics → Claude Code
       FirstSuccess hover/definition/completion → Primary tier server
```

## Key Constraints

- lsp-max is **read-only**: it emits diagnostics/hovers/intents, never mutates files directly.
- The compositor is the **sole** LSP endpoint Claude Code binds — Claude Code must not also
  maintain direct connections to child servers (that would bypass law-state enforcement).
- Version tracking: the compositor must forward the version field from Claude Code's `didChange`
  unchanged to all child servers (do not re-sequence versions).
- Per-buffer segmentation: fan-out of `didChange` must be serialized per URI — notifications to
  each child are ordered, never mixed across buffers.
