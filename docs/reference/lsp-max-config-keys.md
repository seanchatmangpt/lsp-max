# lsp-max Configuration Keys — Reference

**Status: PARTIAL** — the key surface is enumerated from source; some keys are read
from env only (not yet from the JSON config), noted below. Source of truth:
`crates/lsp-max-cli/src/nouns/{config.rs,mod.rs,agent.rs,gate.rs}`,
`lsp-max-agent/src/lib.rs`, `lsp-max-runtime/src/control_plane/admission/validation.rs`,
`src/composition/server.rs`.

## Resolution order

- **Config JSON** (three independent re-implementations — drift risk, **OPEN**):
  `LSP_MAX_CONFIG` → `$HOME/.lsp-max-config.json` → `./.lsp-max-config.json` (cwd).
- **Mesh state**: `LSP_MAX_STATE_PATH` → `./.mesh_state.json` (cwd).
- **Agent effective value**: env (`LSP_MAX_*`, then `OPENAI_*`) overrides the JSON
  key; the JSON value is used only when both env vars are absent.

## Key table

| Key (env / JSON) | Purpose | Default | Consumer |
|---|---|---|---|
| `LSP_MAX_CONFIG` | config JSON path | `$HOME/.lsp-max-config.json` | config.rs, agent, validation |
| `LSP_MAX_STATE_PATH` | mesh state JSON path | `.mesh_state.json` (cwd) | mod.rs, agent, telemetry |
| `LSP_MAX_API_KEY` / `OPENAI_API_KEY` | LLM API key | none → `Err` | lsp-max-agent |
| `LSP_MAX_API_BASE` / `OPENAI_API_BASE` | LLM endpoint | `https://api.openai.com/v1` | lsp-max-agent |
| `LSP_MAX_MODEL` / `OPENAI_MODEL` | model id | `gpt-4o` | lsp-max-agent |
| `LSP_MAX_DB_PATH` | graph DB dir | `$HOME/.local/share/lsp-max/db` | runtime/admission |
| `LSP_MAX_TIMEOUT` | upstream timeout (ms) | `150` | composition/server.rs |
| `XDG_RUNTIME_DIR` | gate-file dir | `/tmp` | gate.rs |
| JSON `api_key` / `openai_api_key` | LLM API key | — | agent, validation |
| JSON `api_base` / `openai_api_base` | LLM endpoint | — | agent |
| JSON `model` / `openai_model` | model id | — | agent |
| JSON `database_path` | graph DB dir | — | validation |

## Findings (bounded)

1. `[FRAMES]` Config schema is triplicated across three files with no shared loader — **OPEN**.
2. `[ELIZA]` Dual key names per concept (`api_key` vs `openai_api_key`); both are read, only the canonical is written by `config set` — silent shadowing — **PARTIAL**.
3. `[ASP]` `database_path`, `LSP_MAX_TIMEOUT`, `LSP_MAX_DB_PATH` are read but undocumented elsewhere — **OPEN** (this file documents them).
4. `[FRAMES]` `LSP_MAX_STATE_PATH` defaults to cwd, inconsistent with the XDG-style DB default — **OPEN**.
5. `[BAYES]` No macOS-only paths in code; uses `$HOME/.local/share` + `XDG_RUNTIME_DIR` — portable — **ADMITTED**.
6. `[META]` `config set` is an unvalidated flat KV store; arbitrary keys persist with no required-key enforcement — intent **UNKNOWN**.

## Canonical seed (apply to `$HOME/.lsp-max-config.json` — not written by this audit)

```json
{
  "api_key": "",
  "api_base": "https://api.openai.com/v1",
  "model": "gpt-4o",
  "database_path": ""
}
```

Note: `state_path` and `LSP_MAX_TIMEOUT` are **not** read from JSON today (env-only);
seeding them is aspirational until a loader reads them. The `openai_*` aliases remain
read-only for compatibility — keep them out of new documentation as canonical.

## Pipeline (TPOT2) Configuration

| Key | Env Var | Default | Description |
|-----|---------|---------|-------------|
| `pipeline_generations` | `LSP_MAX_PIPELINE_GENERATIONS` | `10` | Generations for genetic search |
| `pipeline_population_size` | `LSP_MAX_PIPELINE_POP_SIZE` | `20` | Population size per generation |
| `pipeline_mutation_rate` | `LSP_MAX_PIPELINE_MUTATION_RATE` | `0.15` | Mutation rate (0.0–1.0) |
| `pipeline_admission_threshold` | `LSP_MAX_PIPELINE_ADMISSION_THRESHOLD` | `0.7` | Minimum fitness for ADMITTED |
| `pipeline_ocel_path` | `LSP_MAX_PIPELINE_OCEL` | `` | OCEL event log path for fitness |
| `pipeline_max_length` | `LSP_MAX_PIPELINE_MAX_LENGTH` | `5` | Max pipeline length (breed count) |
