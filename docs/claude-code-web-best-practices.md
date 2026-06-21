# Claude Code on the Web — Best Practices for lsp-max

Status: CANDIDATE reference. Synthesized from the official Claude Code docs and
applied to this workspace, whose full build is BLOCKED in web sessions (sibling
repos absent). Sources are listed at the bottom.

## The core constraint

Claude Code web sessions clone a **single** GitHub repository into an isolated,
ephemeral container (4 vCPU / 16 GB / 30 GB). The lsp-max workspace requires
sibling checkouts — `../lsp-types-max`, `../wasm4pm-compat`, `../wasm4pm` — that
are **not** present, so `cargo build --workspace` / `just test` are **BLOCKED**
there. `additionalDirectories` grants sibling access only in local/terminal
sessions, not on the web.

The bounded consequence: treat full-workspace build/test as **BLOCKED**, not
REFUSED, and verify the parts that are self-contained.

## What IS verifiable here

`src/pipeline/` (catalog, types, search, fitness, pareto, ocel, phase) depends
only on `serde`/`serde_json`/`std`. `scripts/tpot2-harness-verify.sh` mounts that
module tree in a throwaway crate and runs `cargo test` + `cargo clippy -D
warnings` + `rustfmt --check`, then emits a marker-style receipt and validates
it. This is the recommended verification path in any container where the
workspace cannot build.

```sh
bash scripts/tpot2-harness-verify.sh
# -> ADMITTED: 36 tests, clippy-clean, receipt bound + validated
```

## SessionStart hook (recommended, opt-in)

`scripts/web-session-setup.sh` detects the missing siblings and injects guidance
into the session so an agent does not waste a turn on a doomed workspace build.
It is **not** wired into `.claude/settings.json` automatically (an agent should
not self-modify its own startup config). To enable it, add this to
`.claude/settings.json` `hooks`:

```json
"SessionStart": [
  {
    "matcher": "startup|resume",
    "hooks": [
      {
        "type": "command",
        "command": "\"$CLAUDE_PROJECT_DIR\"/scripts/web-session-setup.sh"
      }
    ]
  }
]
```

SessionStart fires on every session start/resume; its stdout (plain text, or a
`hookSpecificOutput.additionalContext` JSON block — this script emits the JSON
form when `jq` is present) is injected into context before the first prompt.

## Setup scripts vs SessionStart hooks

| | Setup script | SessionStart hook |
|---|---|---|
| Attached to | the cloud environment (Web UI) | the repo (`.claude/settings.json`) |
| Runs | once before launch, then cached | every session start/resume |
| Best for | system packages, toolchains, large downloads | project deps, dependency checks, context injection |

Recommendation for lsp-max: install the Rust toolchain / build tools in a **setup
script**; do dependency validation + context injection in a **SessionStart
hook**. Keep both under the ~5-minute cache-build budget.

## Network access

Default **Trusted** egress allowlists crates.io, rustup.rs, and
static.rust-lang.org, so the harness's `cargo fetch` works out of the box. Use
**Custom** for private registries; **None** for air-gapped runs (the harness then
reports BLOCKED rather than REFUSED).

## Commits, branches, pushing

- Sessions push to the current branch only; they stay live after a push so CI
  errors and review comments can be handled in the same conversation.
- Commits carry a `Claude-Session: <url>` trailer for traceability.
- Do not open a PR unless explicitly asked.

## Sources

- Use Claude Code on the web — https://code.claude.com/docs/en/claude-code-on-the-web
- Hooks guide — https://code.claude.com/docs/en/hooks-guide
- Hooks reference (schemas) — https://code.claude.com/docs/en/hooks
- Large codebases / monorepos — https://code.claude.com/docs/en/large-codebases
- Web quickstart — https://code.claude.com/docs/en/web-quickstart
