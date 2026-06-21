# Subagent Gate-Check Preamble (Λ_CD convention)

**Status: OPEN** — structural enforcement is **REFUSED** (the hook boundary is not
crossable). The preamble below is a **CANDIDATE** mitigation only; it holds *iff*
every subagent-prompt author pastes it. It is a convention, not a guarantee.

## Why a hook cannot do this

The `PreToolUse` ANDON hook in `.claude/settings.json` runs only in the parent
Claude Code session. Subagents spawned via the `Agent` tool run in isolated
sessions and inherit **no** hooks (see
`docs/agent-integration-guide.md` § Session Boundary Handling). No `settings.json`
key reaches into an Agent-tool child, so a subagent could run Bash/Edit/Write while
the parent gate byte is `b"1"` (ANDON set). The gate-safety invariant
`G(parent_BLOCKED → ¬child_mutates)` does **not** hold structurally.

## The preamble (paste verbatim as the first instruction in every subagent prompt that may run Bash/Edit/Write)

```text
Before any Bash/Edit/Write, run this and abort on non-zero:
  lsp-max-cli gate check || exit 1
If `lsp-max-cli` is absent on PATH, treat the gate as UNKNOWN and do NOT run
state-changing commands. Prefer read-only tools (Read/Grep/Glob).
```

## Gate file (read-only, single-syscall check)

```text
$XDG_RUNTIME_DIR/lsp-max-gate-{fnv1a(cwd):016x}   (fallback: /tmp/lsp-max-gate-{...})
byte "0" = clear, "1" = ANDON set, file absent = compositor not running (unenforced)
```

The authoritative path formula lives in **one** place: `src/primitives/gate_path.rs`
(`fnv1a` constants `offset_basis = 0xcbf29ce484222325`, `prime = 0x100000001b3`,
format `{hash:016x}`). The compositor (`crates/lsp-max-compositor/src/gate_file.rs`)
delegates to it. The CLI (`crates/lsp-max-cli/src/nouns/gate.rs`) currently keeps an
**inline duplicate** of the same constants — a drift risk (**PARTIAL**); collapsing
that copy into `lsp_max::primitives::gate_file_path()` would retire it. (Edit not
applied here — flagged for the owner.)

## Read-semantics caveat (do not collapse)

The compositor `read()` fail-closes on a stale heartbeat; the CLI `check()` treats a
missing file as `compositor_active = false` → exit 0 (clear). A subagent running the
preamble during a compositor crash therefore sees **CLEAR**, not BLOCKED. Whether
this is intended is **UNKNOWN**; it is recorded, not resolved.

## Placement

This file is the canonical artifact. Link it from `AGENTS.md` § Subagent Gate
Propagation and from `docs/agent-integration-guide.md` § Session Boundary Handling.
Overall propagation status remains **OPEN** until structural enforcement exists.
