# Automatic Configuration

Goal: a person or agent that clones this repo gets a build-ready workspace without
needing to know it depends on sibling checkouts or how to wire them. The patterns
below favor zero-config defaults, one-command setup, self-diagnosis, and (where
authorized) zero-command setup via a session hook.

## One command: `just setup`

This workspace does not build standalone — Cargo path deps and `[patch.crates-io]`
require three siblings next to the repo root (`../lsp-types-max`,
`../wasm4pm-compat`, `../wasm4pm`). `scripts/bootstrap.sh` makes them PRESENT and
reports readiness; it is idempotent and non-interactive.

```bash
just setup     # clone missing siblings, then report readiness   (= bash scripts/bootstrap.sh)
just doctor    # read-only diagnosis; never clones                (= bash scripts/bootstrap.sh --check)
```

Sibling source mirrors CI (`https://github.com/<org>/<repo>.git`). Overrides:
`LSP_MAX_SIBLING_ORG`, `LSP_MAX_SIBLING_BASE`, `SIBLING_REPO_TOKEN` (private repos),
`LSP_MAX_BOOTSTRAP_DEPTH` (shallow clone).

## Layered configuration (precedence)

Runtime config resolves with sensible defaults so the common path needs no files:

```text
built-in default  <  config file (.lsp-max-config.json)  <  environment variable
```

The full key/env surface is enumerated in `docs/lsp-max-config-keys.md`. The
compositor auto-discovers `lsp-max.toml` by walking up from the cwd and falls back
to a static ANDON prefix set when it is absent, so a missing file degrades to a
documented default rather than an error.

## Zero-command setup on Claude Code (web) — PENDING AUTHORIZATION

A `SessionStart` hook makes setup automatic for web sessions: the bootstrap runs
before the session starts, so the agent never races ahead of the prerequisites.
Creating files under `.claude/` (a hook script + settings registration) is treated
as harness self-modification and was declined by the permission classifier during
this change. Apply the two artifacts below with explicit authorization.

`.claude/hooks/session-start.sh` (then `chmod +x`):

```bash
#!/usr/bin/env bash
# SessionStart hook: auto-bootstrap the lsp-max workspace for Claude Code on the
# web so a session can build/test without manual setup. Remote-only; local dev
# uses `just setup`. Always exits 0 so the session starts even if a sibling
# cannot be cloned.
set -uo pipefail
if [ "${CLAUDE_CODE_REMOTE:-}" != "true" ]; then
  exit 0
fi
ROOT="${CLAUDE_PROJECT_DIR:-$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)}"
bash "$ROOT/scripts/bootstrap.sh" || true
exit 0
```

Merge into `.claude/settings.json` (additive — leaves the existing PreToolUse /
PostToolUse gate hooks untouched):

```json
{
  "hooks": {
    "SessionStart": [
      { "hooks": [ { "type": "command", "command": "$CLAUDE_PROJECT_DIR/.claude/hooks/session-start.sh" } ] }
    ]
  }
}
```

Trade-off (synchronous): the session starts only after the bootstrap finishes,
which prevents a race where the agent runs cargo before siblings exist. Switching
to async (`{"async": true, "asyncTimeout": 300000}` as the hook's first stdout
line) starts the session faster but reintroduces that race. Once merged to the
default branch, all future web sessions use it.

## Verification (local runs — not signed receipt artifacts)

Run in a container after `bash scripts/bootstrap.sh` cloned the three siblings:

| Step | Command | Result |
|------|---------|--------|
| Bootstrap | `bash scripts/bootstrap.sh` | 3 siblings CLONED → "Environment READY", exit 0 |
| Workspace resolve | `cargo metadata --no-deps` | exit 0; 34 workspace members |
| Compile (scoped) | `cargo check -p lsp-max-protocol` | exit 0; `lsp-types-max v26.6.5 (/home/user/lsp-types-max)` + `wasm4pm-compat` compiled |
| Lint | `cargo clippy -p lsp-max-protocol --all-targets -- -D warnings` | exit 0 |
| Test | `cargo test -p lsp-max-protocol --lib` | 37 passed, 0 failed |

A build-blocking manifest bug was fixed as part of this work:
`crates/anti-llm-cheat-lsp/Cargo.toml` pointed `lsp-types-max` at
`../../lsp-types-max` (resolving inside the repo) with `version = "26.6.8"`; the
sibling is `26.6.5` and every other `crates/*` manifest uses `../../../lsp-types-max`.
Corrected to `path = "../../../lsp-types-max", version = "26.6.5"` — without this,
`just setup` would clone the siblings yet `cargo` would still fail to resolve.

## Proposed follow-ups (CANDIDATE — require a build to verify)

- **Embedded default `lsp-max.toml`**: compile the fallback server registry into the
  compositor so routing works with zero config, with the on-disk file as an override.
- **Single config loader**: the config-file resolution is currently re-implemented in
  three places (see `docs/lsp-max-config-keys.md`, finding 1); consolidating removes drift.
- **`lsp-max-cli config init` / `config doctor`**: scaffold a seed config and report
  effective values + their source (default / file / env), mirroring `just doctor`.
