# Claude Code Hooks — Quick Reference Card

Keep this file for quick lookup while working.

---

## Hook Types at a Glance

| Type | When | Blocks | Use | Max Time |
|------|------|--------|-----|----------|
| **PreToolUse** | Before tool | ✓ YES | Gates, validation | <500ms |
| **PostToolUse** | After tool | ✗ NO | Logging, archival | N/A |
| **SessionStart** | Session init | ✓ YES | Setup, validation | <10s |

---

## Configuration Template

```json
{
  "hooks": {
    "SessionStart": [
      {
        "type": "command",
        "command": "validation-command"
      }
    ],
    "PreToolUse": [
      {
        "matcher": "Bash|Edit|Write",
        "hooks": [
          {
            "type": "command",
            "command": "gate-or-check-command"
          }
        ]
      }
    ],
    "PostToolUse": [
      {
        "matcher": "Bash|Edit|Write",
        "hooks": [
          {
            "type": "command",
            "command": "logging-or-snapshot-command"
          }
        ]
      }
    ]
  }
}
```

---

## Exit Codes

| Code | PreToolUse | PostToolUse |
|------|-----------|------------|
| 0 | Allow tool | OK; continue |
| 1 | Block tool | Logged as error |
| >1 | Block tool | Logged as error |

---

## Common Commands

### lsp-max Project

```bash
lsp-max-cli gate check                    # Check ANDON gate (exit 1 if blocked)
lsp-max-cli diagnostic snapshot           # Save current diagnostics
lsp-max-cli diagnostic list --severity error  # List active errors
```

### Build & Quality

```bash
cargo check --workspace                   # Fast syntax check (~5s)
cargo fmt --check                         # Check formatting (~2s)
cargo clippy --workspace --all-targets -- -D warnings  # Lint (~10s)
cargo test --workspace                    # Full tests (~30s)
just dx-verify                            # Architecture check
just dx-polish                            # Format + lint
```

### Shell Utilities

```bash
[ -f file ]                               # File exists?
[ -d dir ]                                # Directory exists?
git branch --show-current                 # Current branch
git rev-parse --short HEAD                # Short commit hash
pwd                                       # Current directory
```

---

## Fastest Hooks (Fast < Slow)

**Fast (< 100ms):**
- `lsp-max-cli gate check`
- `git branch --show-current`
- `[ -f file ]` checks

**Medium (100ms - 1s):**
- `cargo check --workspace`
- `cargo fmt --check`

**Slow (> 1s):**
- `cargo clippy` (~10s)
- `cargo test` (~30s)
- Full CI suite (~2-5m)

**Rule:** PreToolUse must be fast. Move slow checks to PostToolUse (non-blocking).

---

## Hook Troubleshooting Checklist

- [ ] Test command locally: `bash ./.claude/my-hook.sh && echo "Exit: $?"`
- [ ] Check exit code: `echo $?` after running
- [ ] Verify working directory: `pwd` in hook
- [ ] Check $PATH: `which command-name` in hook
- [ ] Look for typos in command
- [ ] Use absolute paths (avoid relative paths)
- [ ] Check file permissions: `ls -la .claude/`
- [ ] Test with `bash -x` for tracing: `bash -x ./.claude/my-hook.sh 2>&1 | head -50`

---

## Cheat Sheet: Real Examples

### Minimal Gate

```json
{
  "matcher": "Bash|Edit|Write",
  "hooks": [
    {
      "type": "command",
      "command": "lsp-max-cli gate check"
    }
  ]
}
```

### Fast Quality Check

```json
{
  "matcher": "Edit|Write",
  "hooks": [
    {
      "type": "command",
      "command": "cargo fmt --check && cargo clippy --all-targets -- -D warnings"
    }
  ]
}
```

### Background Tests

```json
{
  "matcher": "Edit",
  "hooks": [
    {
      "type": "command",
      "command": "cargo test --workspace > /tmp/test-$(date +%s).log 2>&1 &"
    }
  ]
}
```

### Snapshot on Write

```json
{
  "matcher": "Bash|Edit|Write",
  "hooks": [
    {
      "type": "command",
      "command": "lsp-max-cli diagnostic snapshot 2>/dev/null || true"
    }
  ]
}
```

### Branch-Conditional

```json
{
  "matcher": "Bash",
  "hooks": [
    {
      "type": "command",
      "command": "[ $(git branch --show-current) = main ] && cargo test || echo 'Feature branch; skipping tests'"
    }
  ]
}
```

---

## Matchers Reference

Common tool names:

```
Bash               Execute shell
Edit               Modify file
Write              Create file
Read               Read file
Glob               Find files
Grep               Search files
TaskCreate         Create task
NotebookEdit       Edit notebook
```

Matcher syntax:

```json
"matcher": "Bash"                    // Single tool
"matcher": "Bash|Edit|Write"         // Multiple tools
"matcher": "Bash|.*"                 // Bash + wildcard (rare)
```

---

## Session Context

Hooks can access:

```bash
$(pwd)                               # Current directory
$(git branch --show-current)         # Current branch
$(git rev-parse --short HEAD)        # Short commit hash
$(whoami)                            # Username
$HOME                                # Home directory
$WORKSPACE                           # Not always set; use pwd
```

Hooks **cannot** directly access:

```bash
$TOOL_NAME                           # Tool being invoked
$FILE_PATH                           # File path from Edit/Write tool
$COMMAND_ARGS                        # Arguments to Bash tool
```

**Workaround:** Inspect filesystem for recently modified files.

---

## Debugging One-Liners

```bash
# Check if command exists
which lsp-max-cli

# Test gate locally
lsp-max-cli gate check && echo "OPEN" || echo "BLOCKED"

# Check git state
git status --short

# List recent files
ls -lt | head -10

# Show hook in settings
cat .claude/settings.json | jq '.hooks'

# Run hook with tracing
bash -x .claude/my-hook.sh 2>&1 | tee /tmp/hook-trace.log

# Time a hook
time bash .claude/my-hook.sh

# Check disk space
df -h .

# Check available memory
free -h
```

---

## Common Patterns

### Pattern: Conditional by Branch

```bash
branch=$(git branch --show-current)
if [ "$branch" = "main" ]; then
    cargo test --workspace
else
    cargo check
fi
```

### Pattern: Fail Fast

```bash
set -e
command1 || exit 1
command2 || exit 1
command3 || exit 1
```

### Pattern: Background Job

```bash
# Returns immediately; test runs in background
cargo test --workspace > /tmp/log-$(date +%s) 2>&1 &
echo "Tests running in background"
```

### Pattern: Suppress Errors

```bash
# Command may fail; hook still passes
cargo check 2>/dev/null || true
lsp-max-cli gate check 2>/dev/null || true
```

### Pattern: Retry Logic

```bash
for i in {1..3}; do
    lsp-max-cli gate check && exit 0
    sleep 1
done
exit 1  # Failed after 3 retries
```

---

## Performance Tips

| Operation | Time | Recommendation |
|-----------|------|-----------------|
| `lsp-max-cli gate check` | ~10ms | Use in PreToolUse |
| `cargo check --workspace` | ~5s | Use in PreToolUse if <500ms target |
| `cargo fmt --check` | ~2s | Use in PreToolUse |
| `cargo clippy` | ~10s | Move to PostToolUse |
| `cargo test` | ~30s | Move to PostToolUse |
| File checks (`[ -f ]`) | <1ms | Use liberally |
| Git commands | ~100ms | OK in PreToolUse |

---

## File Locations

```
Workspace root: /path/to/workspace
Settings: /path/to/workspace/.claude/settings.json
This file: /path/to/workspace/HOOKS_QUICK_REFERENCE.md
Main docs: /path/to/workspace/HOOKS_GUIDE.md
           /path/to/workspace/HOOKS_TEMPLATES.md
           /path/to/workspace/HOOKS_ADVANCED.md
           /path/to/workspace/HOOKS_INDEX.md
```

---

## Links

Full documentation:
- **HOOKS_GUIDE.md** — Comprehensive reference
- **HOOKS_TEMPLATES.md** — 15 ready-to-use templates
- **HOOKS_ADVANCED.md** — Advanced patterns & debugging
- **HOOKS_INDEX.md** — Navigation guide

Project docs:
- **CLAUDE.md** — Project conventions
- **AGENTS.md** — Agent-specific rules
- **AGENTS.md → Subagent Gate Propagation** — Hook boundaries in subagents

---

## Before You Ask...

**Q: Hook not running?**
→ Check: matcher in settings.json, hook syntax, working directory, file permissions

**Q: Hook is too slow?**
→ Check: if in PreToolUse (should be <500ms), move to PostToolUse if possible

**Q: Hook blocks me unexpectedly?**
→ Check: exit code (is it 0?), gate state, error message; use `lsp-max-cli gate check` to debug

**Q: How do I disable a hook?**
→ Edit `.claude/settings.json`, remove the hook, or change command to `exit 0`

**Q: Can hooks run other hooks?**
→ Yes: call scripts that invoke other commands. Keep total time reasonable.

---

**Print this card and keep it handy!**
