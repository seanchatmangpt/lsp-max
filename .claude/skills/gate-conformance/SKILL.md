---
name: gate-conformance
description: Inspect the ANDON gate state, resolve blocking diagnostics, and understand the Λ_CD gate predicate. Use when gate is blocked or when writing subagent preambles.
tools: [Bash, Read]
---

# Gate Conformance — Λ_CD^runtime

## Quick State Check

```bash
lsp-max-cli gate check   # exit 0 = clear, exit 1 = ANDON blocked
lsp-max-cli gate list    # JSON: active_codes, agent_scope, gate_file, compositor_active
```

`gate list` output:
```json
{
  "andon_blocked": false,
  "gate_file": "/tmp/lsp-max-gate-a3b2c1...",
  "compositor_active": false,
  "active_codes": [],
  "agent_scope": "global"
}
```

When blocked: `active_codes` shows `["WASM4PM-*", "GGEN-*"]` (the blocking families).

## Gate File Internals

Path formula (FNV-1a of workspace path, zero-padded 16-hex):
```
$XDG_RUNTIME_DIR/lsp-max-gate-{fnv1a(cwd):016x}
  or
/tmp/lsp-max-gate-{fnv1a(cwd):016x}
```

Content: single byte — `b"0"` clear, `b"1"` ANDON set. File absent = compositor not running.

## What Blocks the Gate

Only `WASM4PM-*` and `GGEN-*` diagnostic codes trigger ANDON. Resolution path:
1. Run `lsp-max-cli gate list` to confirm which families are active
2. Find the WASM4PM-* / GGEN-* diagnostic in the relevant crate
3. Fix the underlying process-model or ggen violation
4. Rerun diagnostics; compositor clears gate once `D_t` drains

## Subagent Preamble (mandatory)

Every subagent prompt MUST start with a gate check:
```bash
lsp-max-cli gate check || { echo "ANDON gate blocked — resolve before proceeding"; exit 1; }
```

Subagents spawned via `Agent` tool do NOT inherit the parent session's PreToolUse hooks.
Without this preamble, subagents can bypass the gate entirely.

## Formal Predicate

```
Λ_CD(a) = Λ(a) ∧ ¬∃ d ∈ D_t : d.law_id ∈ A ∧ d.severity = Error
```

Where:
- `Λ(a)` — base admissibility (receipts, no victory language, no forbidden implications)
- `D_t` — active diagnostics at time t
- `A` — governed code prefixes: `WASM4PM-*`, `ANTI-LLM-*`, `GGEN-*`
- Gate blocks when any Error-severity diagnostic with governed law_id is present

Warning-severity and Hint-severity diagnostics do NOT block the gate.

## Conjunct Status

| Conjunct | Status |
|----------|--------|
| Gate file write | ADMITTED — eager on first ANDON Error |
| PreToolUse hook (parent session) | ADMITTED — `.claude/settings.json` |
| `gate check` available to subagents | ADMITTED — syscall-level read |
| Structural enforcement in subagent sessions | REFUSED — hook boundary not crossable |
| Per-agent partitioning (`agent_scope`) | OPEN — currently always "global" |

## Receipt Gating

`CompositorReceipt::has_andon_block = true` propagates to the gate file write. The gate write precedes the receipt emission — no receipt can claim admission while the gate is set.
