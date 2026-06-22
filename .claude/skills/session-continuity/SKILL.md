---
name: session-continuity
description: Patterns for maintaining continuity across Claude Code Web sessions — ephemeral containers, git as persistence, session naming, parallel subagents, PR watching. Read this at the start of any new remote session.
tools: [Bash, Read, Glob]
---

# Session Continuity — Claude Code Web

## Container Model

Claude Code Web sessions run in Anthropic-managed ephemeral containers:
- Repository cloned fresh on session start
- Setup script runs once per environment, then cached
- Session state is lost when container reclaims — **git commits are the only persistence**
- Sibling repos (`../lsp-types-max`, `../wasm4pm-compat`, `../wasm4pm`) are NOT auto-cloned
- `CLAUDE_SESSION_ID` env var identifies the session in hooks

## Sibling Repos in Remote Sessions

The workspace does NOT build standalone. In a remote container:
```bash
ls ../lsp-types-max ../wasm4pm-compat ../wasm4pm 2>/dev/null || echo "MISSING — build will fail"
```

If missing, work is limited to:
- Documentation changes
- Code review and analysis
- Files that don't require compilation

For compilation: request that sibling repos be made available in the environment setup script, or work exclusively on the LSP protocol/CLI layer that can be partially checked.

## Git as Persistence

Every meaningful change must be committed and pushed before the session ends:
```bash
git add <files>
git commit -m "descriptive message"
git push -u origin <branch>
```

Branch convention: `claude/<kebab-description>-<6-char-id>`
PR convention: use `mcp__github__create_pull_request` or push and let the user create PR.

## Parallel Subagent Patterns

Spawn multiple agents in a single message to work in parallel:

```
# Research in parallel — don't wait for one before starting the other
Agent(description="Explore compositor flush pipeline", ...)
Agent(description="Explore anti-llm-cheat diagnostics", ...)
```

Rules:
- Parallel only when agents touch DIFFERENT files (no overlapping edits)
- Each subagent gets a fresh 1M token context window
- Use `isolation: "worktree"` for agents that write code (prevents git conflicts)
- Use foreground (default) when you need results before proceeding
- Use `run_in_background: true` for independent research tasks

## Subagent Gate Preamble (mandatory)

Subagents do NOT inherit PreToolUse hooks from the parent session. Every subagent prompt must include:

```bash
lsp-max-cli gate check || { echo "ANDON gate blocked"; exit 1; }
```

Include this as the FIRST bash action in every subagent prompt.

## Worktree Isolation

For agents that write code, use `isolation: "worktree"`:
```
Agent(
  description="Add DFG to compositor",
  isolation="worktree",
  prompt="... (include gate preamble) ..."
)
```

The worktree is automatically cleaned up if the agent makes no changes. Otherwise the path and branch are returned in the result.

## PR Watching

After creating a PR, subscribe to activity:
```
subscribe_pr_activity(pr_number=10)
```

Events arrive as `<github-webhook-activity>` messages. For each:
- CI failure → investigate and push fix
- Review comment → check with user if ambiguous; fix if clear
- Duplicate/no-action → skip silently

Check-in pattern (use `send_later` if available):
```
schedule self-check in 1 hour → re-check PR CI and mergeability
```

## Session Naming

Name sessions for discoverability when resuming:
```bash
claude -n "lsp-max-compositor-ocel"
# Later: claude --resume lsp-max-compositor-ocel
```

## Context Management

When context fills:
1. Push all uncommitted work first
2. `/compact` to trim history while keeping current task
3. Use subagents for investigation — their reads stay out of main context
4. Key decisions → commit messages (survive compaction)
5. Long-running plans → PLAN.md in repo (survives session loss)

## Environment Variables Available in Hooks

```bash
CLAUDE_SESSION_ID          # session identifier (v2.1.9+)
```

Use in PostToolUse hooks to tag diagnostic snapshots with session identity.

## Status Check Template

At the start of any resumed session, run:
```bash
git status                          # uncommitted work?
git log --oneline -5                # what was last committed?
lsp-max-cli gate check              # gate clear?
lsp-max-cli gate list               # active codes?
```
