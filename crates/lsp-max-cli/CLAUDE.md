# lsp-max-cli

Actuation grammar layer: noun/verb CLI built on `clap-noun-verb`. Each file in `src/nouns/` is a noun; `#[verb("name")]` attributes are actions.

## Architecture

```
src/main.rs          — entry point; registers all nouns
src/nouns/           — one file per noun (gate, receipt, pack, diagnostics, ...)
src/nouns/gate.rs    — gate check + gate list verbs; GateService; GateListResult
```

## Key Nouns

| Noun | Key Verbs | Purpose |
|------|-----------|---------|
| `gate` | `check`, `list` | ANDON gate state inspection |
| `receipt` | `validate`, `show` | Receipt chain verification |
| `pack` | `list`, `status`, `diff` | RulePack inspection |
| `diagnostics` | `snapshot`, `list` | Diagnostic state dump |
| `conformance` | `check`, `vector` | ConformanceVector inspection |
| `workspace` | `conformance` | Workspace-wide conformance aggregation |

## gate noun (PreToolUse hook)

`gate check` is the single-call PreToolUse gate:
- Exit 0 → ANDON clear, tool proceeds
- Exit 1 → ANDON blocked, tool is prevented

`gate list` → `GateListResult`:
```json
{
  "andon_blocked": false,
  "gate_file": "/tmp/lsp-max-gate-{hash}",
  "compositor_active": false,
  "active_codes": ["WASM4PM-*", "GGEN-*"],
  "agent_scope": "global"
}
```

`active_codes` is empty when gate is clear; lists blocking families when set. `agent_scope` is always `"global"` until RFC A per-agent partitioning is wired.

## Adding a Noun

1. Create `src/nouns/<noun>.rs` with `#[verb("action")]` functions
2. Register in `src/nouns/mod.rs` with `pub mod <noun>;`
3. Wire in `src/main.rs` noun registration

Each verb function returns `Result<T>` where `T: Serialize`. The framework handles JSON output, exit codes, and error formatting.

## Testing

```bash
cargo test -p lsp-max-cli
```

Unit tests in `src/nouns/<noun>.rs` `#[cfg(test)]` blocks. The gate noun tests verify path determinism and clear-state behavior.

## Law Status

- `gate check` verb: ADMITTED
- `gate list` verb: CANDIDATE — wired and tested; per-agent partitioning OPEN
- `diagnostic snapshot` verb: CANDIDATE — wired; session tagging not yet wired
