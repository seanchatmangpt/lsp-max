# CC-007 — LSIF DiagnosticsOnly Tier for Offline Navigation

**Status**: OPEN  
**Component**: `crates/lsp-max-compositor/src/registry.rs`, `lsp-max.toml`  
**Depends on**: CC-002  
**Blocked by**: CC-003 (compositor must be the endpoint before LSIF makes sense)

## Problem

LSIF (Language Server Index Format) provides pre-computed go-to-definition, find-all-references,
and hover data without requiring a live language server. It is well-suited as a fallback or
augmentation tier for Claude Code's navigation features when:

1. A live server is not available (offline, slow startup, cloud environment).
2. Navigation over a large monorepo where indexing would be prohibitive per-session.
3. PR/code-review contexts where Claude Code needs symbol navigation but no running server exists.

LSIF's constraint: **no incremental updates**. An LSIF dump is a snapshot; mutations invalidate
it. It must never receive `didChange` notifications. It serves only `definition`, `references`,
and `hover` — never `publishDiagnostics`.

## Acceptance Criteria

- [ ] `lsp-max.toml` supports a new `priority = "lsif"` tier:

```toml
[[server]]
id = "my-lsif-index"
command = "lsif-util serve"   # or any LSIF server binary
args = ["--dump", "lsif/index.lsif.json"]
priority = "lsif"
primary_extensions = [".rs", ".ts"]
secondary_extensions = []
lsif_dump_path = "lsif/index.lsif.json"  # path to the dump file
```

- [ ] The routing table treats `lsif` tier as:
  - `textDocument/definition`, `textDocument/references`, `textDocument/hover` → `FallbackToLsif`:
    try Primary tier first; if Primary returns null/error, fall back to LSIF tier.
  - `textDocument/publishDiagnostics` → excluded (LSIF servers do not push diagnostics).
  - `textDocument/didOpen`, `didChange`, `didClose` → **not forwarded** (LSIF servers are
    read-only snapshots; sending change notifications to them is incorrect).

- [ ] The compositor validates that `lsif_dump_path` exists at startup. If missing, the server
      is registered as `CANDIDATE` with diagnostic `COMPOSITOR-LSIF-DUMP-MISSING`.

- [ ] `lsp-max-cli server list` shows LSIF servers with tier=lsif and dump path.

- [ ] LSIF servers are never included in the ANDON gate (`andon_code_prefixes` must be empty for
      lsif tier servers; validation rejects non-empty values).

## Routing Table Extension

Current `ChildTier` enum:

```rust
pub enum ChildTier {
    Primary,
    Secondary,
    DiagnosticsOnly,
}
```

Add:

```rust
pub enum ChildTier {
    Primary,
    Secondary,
    DiagnosticsOnly,
    Lsif,  // read-only snapshot; fallback navigation only
}
```

Routing changes:

```rust
match (method, tier) {
    ("textDocument/definition" | "textDocument/references" | "textDocument/hover", Lsif) =>
        // included in FallbackToLsif decision; excluded from FanAll
    ("textDocument/didOpen" | "textDocument/didChange" | "textDocument/didClose", Lsif) =>
        // never forwarded
    ("textDocument/publishDiagnostics", Lsif) =>
        // LSIF servers do not push diagnostics; ignore any that arrive
    _ => // existing routing unchanged
}
```

## LSIF Dump Staleness

LSIF dumps go stale when source files change. The compositor should emit a `COMPOSITOR-LSIF-STALE`
warning diagnostic when:

- The dump file's mtime is older than any file in `primary_extensions` by more than a configurable
  threshold (default: 24h, configurable via `lsif_max_age_hours` in the stanza).
- The dump was generated before the last `git commit` (check `git log -1 --format=%ct` vs dump mtime).

Staleness is a warning, not ANDON — degraded navigation is acceptable, not a gate failure.

## Files to Modify

- `crates/lsp-max-compositor/src/registry.rs` — add `Lsif` to `ChildTier`; add staleness check
- `crates/lsp-max-compositor/src/routing.rs` — add `FallbackToLsif` strategy; exclude from
  `didChange` fan-out
- `lsp-max.toml` — document `priority = "lsif"` and `lsif_dump_path` fields (comment block)
- `crates/lsp-max-cli/src/nouns/server.rs` — show tier=lsif in `list` verb

## Test Plan

- [ ] `priority = "lsif"` server: `didChange` fan-out does not include it
- [ ] `textDocument/definition` with Primary returning null → compositor falls back to LSIF server
- [ ] `textDocument/definition` with Primary returning a result → LSIF server not queried
- [ ] Missing `lsif_dump_path` → `CANDIDATE` status + `COMPOSITOR-LSIF-DUMP-MISSING` diagnostic
- [ ] Dump older than threshold → `COMPOSITOR-LSIF-STALE` warning (not ANDON)
- [ ] `andon_code_prefixes` non-empty on lsif tier → validation error at load time
